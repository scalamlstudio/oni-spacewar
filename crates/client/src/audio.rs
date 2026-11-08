//! Placeholder battle sound (TAKOAI-112, decision B6): CC0 effects played
//! from the sim's events and one looping battle track, at the volumes in
//! the player's settings. Sources and licences are in
//! `assets/source/core/audio/CREDITS.md`. Client only, outside rollback:
//! events arrive once each through [`BattleEvents`](crate::fx::BattleEvents).

use std::collections::HashMap;

use bevy::audio::Volume;
use bevy::prelude::*;
use bevy_ggrs::LocalPlayers;
use sim::{Attack, EnemyKind, LootKind, SimEvent, Skill};

use crate::art::ContentImages;
use crate::fx::{local_handle, BattleEvents};
use crate::rollback::SimWorld;
use crate::settings::{AudioSettings, SettingsFile};

pub struct BattleAudioPlugin;

impl Plugin for BattleAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastPlayed>().add_systems(
            Update,
            (preload, play_effects, battle_music, music_volume).chain(),
        );
    }
}

/// One sound effect: content ID, volume (0..=1, before the settings), and
/// the shortest gap between two plays, so a volley doesn't stack into noise.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sfx {
    pub id: &'static str,
    pub volume: f32,
    pub min_gap: f32,
}

const fn sfx(id: &'static str, volume: f32, min_gap: f32) -> Sfx {
    Sfx {
        id,
        volume,
        min_gap,
    }
}

pub const BASIC_FIRE: Sfx = sfx("core.audio.sfx.basic_fire", 0.25, 0.06);
pub const HIT: Sfx = sfx("core.audio.sfx.hit", 0.3, 0.05);
pub const ENEMY_DEATH: Sfx = sfx("core.audio.sfx.enemy_death", 0.5, 0.04);
pub const SHIP_HIT: Sfx = sfx("core.audio.sfx.ship_hit", 0.7, 0.1);
pub const KITE_AFTERBURN: Sfx = sfx("core.audio.sfx.kite_afterburn", 0.6, 0.0);
pub const KITE_SCATTER: Sfx = sfx("core.audio.sfx.kite_scatter", 0.6, 0.0);
pub const BULWARK_BASTION: Sfx = sfx("core.audio.sfx.bulwark_bastion", 0.6, 0.0);
pub const BULWARK_SHOCKWAVE: Sfx = sfx("core.audio.sfx.bulwark_shockwave", 0.8, 0.0);
pub const CREDIT_PICKUP: Sfx = sfx("core.audio.sfx.credit_pickup", 0.4, 0.05);
pub const CRYSTAL_PICKUP: Sfx = sfx("core.audio.sfx.crystal_pickup", 0.6, 0.05);
/// An elite's loot drop (TAKOAI-117); later also rare module drops.
pub const RARE_DROP: Sfx = sfx("core.audio.sfx.rare_drop", 0.8, 0.0);
pub const MISSION_SUCCESS: Sfx = sfx("core.audio.sfx.mission_success", 0.8, 0.0);
pub const MISSION_FAILURE: Sfx = sfx("core.audio.sfx.mission_failure", 0.8, 0.0);

/// Every shipped effect.
pub const SOUNDS: [Sfx; 13] = [
    BASIC_FIRE,
    HIT,
    ENEMY_DEATH,
    SHIP_HIT,
    KITE_AFTERBURN,
    KITE_SCATTER,
    BULWARK_BASTION,
    BULWARK_SHOCKWAVE,
    CREDIT_PICKUP,
    CRYSTAL_PICKUP,
    RARE_DROP,
    MISSION_SUCCESS,
    MISSION_FAILURE,
];

pub const MUSIC: &str = "core.audio.music.battle";
/// The music sits under the effects.
const MUSIC_VOLUME: f32 = 0.35;

/// The sound an event makes, if any. Our own ship's damage only: another
/// player's hits on their hull would be noise.
pub fn sound_for(ev: &SimEvent, me: usize) -> Option<Sfx> {
    Some(match *ev {
        SimEvent::Fired { .. } => BASIC_FIRE,
        SimEvent::Hit { attack, .. } if !matches!(attack, Attack::Shockwave | Attack::Blast) => HIT,
        // Placeholder: the Bloater's burst borrows the Shockwave's boom.
        SimEvent::Burst { .. } => BULWARK_SHOCKWAVE,
        // Placeholders until real boss audio (TAKOAI-118): the kill borrows
        // the Shockwave's boom, its coming out and its phase change the
        // low failure sting, its rings the Scatter's blast.
        SimEvent::Killed {
            kind: EnemyKind::Guardian,
            ..
        } => BULWARK_SHOCKWAVE,
        SimEvent::GuardianEmerged { .. } | SimEvent::GuardianEnraged { .. } => MISSION_FAILURE,
        SimEvent::GuardianRing { .. } => KITE_SCATTER,
        SimEvent::Killed { elite: true, .. } => RARE_DROP,
        SimEvent::Killed { .. } => ENEMY_DEATH,
        SimEvent::ShipDamaged { ship, .. } if ship == me => SHIP_HIT,
        SimEvent::SkillUsed { skill, .. } => match skill {
            Skill::Afterburn => KITE_AFTERBURN,
            Skill::Scatter => KITE_SCATTER,
            Skill::Bastion => BULWARK_BASTION,
            Skill::Shockwave => BULWARK_SHOCKWAVE,
        },
        SimEvent::Collected { kind, .. } => match kind {
            LootKind::Credits => CREDIT_PICKUP,
            LootKind::VoidCrystal => CRYSTAL_PICKUP,
        },
        SimEvent::MissionEnded { success } => {
            if success {
                MISSION_SUCCESS
            } else {
                MISSION_FAILURE
            }
        }
        _ => return None,
    })
}

/// Reads every effect when a battle starts, so the first hit doesn't wait
/// on the disk.
fn preload(
    world: Option<Res<SimWorld>>,
    mut art: ResMut<ContentImages>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    if world.is_some_and(|w| w.is_added()) {
        for sfx in SOUNDS {
            art.sound(&mut sources, sfx.id);
        }
    }
}

/// When each effect last started (`Time` seconds).
#[derive(Resource, Default)]
struct LastPlayed(HashMap<&'static str, f32>);

#[allow(clippy::too_many_arguments)]
fn play_effects(
    mut commands: Commands,
    events: Res<BattleEvents>,
    time: Res<Time>,
    local: Option<Res<LocalPlayers>>,
    settings: Option<Res<SettingsFile>>,
    mut last: ResMut<LastPlayed>,
    mut art: ResMut<ContentImages>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let audio = settings
        .map(|s| s.settings.audio.clone())
        .unwrap_or_default();
    let me = local_handle(local.as_deref());
    let now = time.elapsed_secs();
    for ev in events.fresh() {
        let Some(sfx) = sound_for(ev, me) else {
            continue;
        };
        let volume = audio.sfx_volume() * sfx.volume;
        if volume <= 0.0 {
            continue;
        }
        if last.0.get(sfx.id).is_some_and(|t| now - t < sfx.min_gap) {
            continue;
        }
        let Some(handle) = art.sound(&mut sources, sfx.id) else {
            continue;
        };
        last.0.insert(sfx.id, now);
        commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
        ));
    }
}

#[derive(Component)]
struct BattleMusic;

/// The battle track loops while a battle is running (muted it plays at
/// volume 0, so unmuting brings it back) and stops when the battle ends.
#[allow(clippy::too_many_arguments)]
fn battle_music(
    mut commands: Commands,
    world: Option<Res<SimWorld>>,
    settings: Option<Res<SettingsFile>>,
    music: Query<Entity, With<BattleMusic>>,
    mut art: ResMut<ContentImages>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let playing = !music.is_empty();
    let ended = world.as_ref().is_some_and(|w| w.outcome().is_some());
    let wanted = world.is_some() && !ended;
    if playing && !wanted {
        for entity in &music {
            commands.entity(entity).despawn();
        }
    } else if !playing && wanted {
        let audio = settings
            .map(|s| s.settings.audio.clone())
            .unwrap_or_default();
        let Some(handle) = art.sound(&mut sources, MUSIC) else {
            return;
        };
        commands.spawn((
            BattleMusic,
            AudioPlayer(handle),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(music_volume_of(&audio))),
        ));
    }
}

fn music_volume_of(audio: &AudioSettings) -> f32 {
    audio.music_volume() * MUSIC_VOLUME
}

/// Settings changes apply to the playing track at once.
fn music_volume(
    settings: Option<Res<SettingsFile>>,
    mut sinks: Query<&mut AudioSink, With<BattleMusic>>,
) {
    let Some(settings) = settings else { return };
    if !settings.is_changed() {
        return;
    }
    let volume = Volume::Linear(music_volume_of(&settings.settings.audio));
    for mut sink in &mut sinks {
        sink.set_volume(volume);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::FxVec2;

    #[test]
    fn every_sound_is_shipped() {
        let mut art = ContentImages::repo();
        let mut sources = Assets::<AudioSource>::default();
        for sfx in SOUNDS {
            assert!(art.sound(&mut sources, sfx.id).is_some(), "{}", sfx.id);
            assert!((0.0..=1.0).contains(&sfx.volume), "{}", sfx.id);
        }
        assert!(art.sound(&mut sources, MUSIC).is_some());
    }

    #[test]
    fn every_shipped_sound_is_credited() {
        let credits = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/source/core/audio/CREDITS.md"
        ))
        .unwrap();
        for id in SOUNDS.iter().map(|s| s.id).chain([MUSIC]) {
            assert!(
                credits.contains(&format!("`{id}`")),
                "{id} not in CREDITS.md"
            );
        }
        assert!(credits.contains("CC0"));
    }

    #[test]
    fn events_pick_their_sounds() {
        let skill = |skill| SimEvent::SkillUsed {
            ship: 1,
            skill,
            pos: FxVec2::ZERO,
        };
        assert_eq!(sound_for(&skill(Skill::Afterburn), 0), Some(KITE_AFTERBURN));
        assert_eq!(
            sound_for(&skill(Skill::Shockwave), 0),
            Some(BULWARK_SHOCKWAVE)
        );
        let damaged = |ship| SimEvent::ShipDamaged {
            ship,
            source: 1,
            amount: 5,
            hull_lost: 5,
            pos: FxVec2::ZERO,
        };
        assert_eq!(sound_for(&damaged(0), 0), Some(SHIP_HIT));
        assert_eq!(sound_for(&damaged(1), 0), None);
        let shockwave_hit = SimEvent::Hit {
            enemy: 1,
            ship: 0,
            attack: Attack::Shockwave,
            source: 0,
            amount: 15,
            pos: FxVec2::ZERO,
        };
        assert_eq!(sound_for(&shockwave_hit, 0), None);
        assert_eq!(
            sound_for(&SimEvent::MissionEnded { success: false }, 0),
            Some(MISSION_FAILURE)
        );
        let killed = |elite| SimEvent::Killed {
            enemy: 1,
            kind: sim::EnemyKind::Spitter,
            elite,
            pos: FxVec2::ZERO,
        };
        assert_eq!(sound_for(&killed(false), 0), Some(ENEMY_DEATH));
        assert_eq!(sound_for(&killed(true), 0), Some(RARE_DROP));
        let guardian = SimEvent::Killed {
            enemy: 1,
            kind: EnemyKind::Guardian,
            elite: false,
            pos: FxVec2::ZERO,
        };
        assert_eq!(sound_for(&guardian, 0), Some(BULWARK_SHOCKWAVE));
        let enraged = SimEvent::GuardianEnraged {
            enemy: 1,
            pos: FxVec2::ZERO,
        };
        assert!(sound_for(&enraged, 0).is_some());
    }
}
