//! Rollback / frame-time measurement (from the TAKOAI-18 spike). Nothing here
//! is rolled back: these resources observe the simulation from the outside.

use std::time::{Duration, Instant};

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy_ggrs::prelude::*;
use bevy_ggrs::RunGgrsSystems;

use crate::rollback::{GameConfig, SimWorld};

/// When the run should stop and print its report.
#[derive(Resource, Clone, Copy, Debug)]
pub struct RunLimit {
    /// Stop once the simulation has reached this frame (0 = run forever).
    pub frames: u32,
    /// Frames ignored for frame-time stats (startup, shader compilation).
    pub warmup_frames: u32,
}

#[derive(Resource, Default, Debug)]
pub struct Stats {
    pub started: Option<Instant>,
    // Simulation / rollback
    pub advances: u64,
    pub resimulated: u64,
    pub rollbacks: u64,
    pub max_rollback: u32,
    pub last_frame: Option<u32>,
    pub max_frame: u32,
    pub max_advances_per_update: u32,
    advances_this_update: u32,
    // Rendered frame times (after warmup)
    pub frame_times_ms: Vec<f32>,
    /// (frame time, sim frame reached) for the five worst rendered frames.
    pub worst_frames: Vec<(f32, u32)>,
    pub rendered_frames: u64,
    // Time spent inside the GGRS update (save/load/advance incl. rollback)
    pub ggrs_ms: Vec<f32>,
    ggrs_started: Option<Instant>,
    // Correctness
    pub synctest_mismatches: u64,
    pub desyncs: u64,
    pub disconnected: bool,
    pub prediction_stalls: u64,
    pub finished: bool,
}

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Stats>()
            .add_systems(PreUpdate, ggrs_timer_start.before(RunGgrsSystems))
            .add_systems(PreUpdate, ggrs_timer_end.after(RunGgrsSystems))
            .add_systems(Update, (drain_p2p_events, check_finished).chain())
            .add_systems(Last, (record_frame_time, report_on_exit))
            .add_observer(on_synctest_mismatch);
    }
}

/// Runs first in `GgrsSchedule`; notices rollbacks by watching the sim frame
/// counter jump backwards.
pub fn track_sim_frame(world: Res<SimWorld>, mut stats: ResMut<Stats>) {
    let f = world.frame;
    stats.advances += 1;
    stats.advances_this_update += 1;
    if let Some(last) = stats.last_frame {
        if f <= last {
            stats.rollbacks += 1;
            stats.max_rollback = stats.max_rollback.max(last - f + 1);
        }
    }
    if f < stats.max_frame {
        stats.resimulated += 1;
    } else {
        stats.max_frame = f + 1;
    }
    stats.last_frame = Some(f);
}

fn ggrs_timer_start(mut stats: ResMut<Stats>) {
    stats.ggrs_started = Some(Instant::now());
    stats.advances_this_update = 0;
    if stats.started.is_none() {
        stats.started = Some(Instant::now());
    }
}

fn ggrs_timer_end(mut stats: ResMut<Stats>, limit: Res<RunLimit>) {
    if let Some(t) = stats.ggrs_started.take() {
        if stats.max_frame > limit.warmup_frames {
            stats.ggrs_ms.push(t.elapsed().as_secs_f32() * 1000.0);
        }
    }
    stats.max_advances_per_update = stats
        .max_advances_per_update
        .max(stats.advances_this_update);
}

fn record_frame_time(time: Res<Time<Real>>, mut stats: ResMut<Stats>, limit: Res<RunLimit>) {
    stats.rendered_frames += 1;
    if stats.max_frame > limit.warmup_frames {
        let ms = time.delta_secs() * 1000.0;
        stats.frame_times_ms.push(ms);
        let at = stats.max_frame;
        stats.worst_frames.push((ms, at));
        stats.worst_frames.sort_by(|a, b| b.0.total_cmp(&a.0));
        stats.worst_frames.truncate(5);
    }
}

fn on_synctest_mismatch(trigger: On<SyncTestMismatch>, mut stats: ResMut<Stats>) {
    stats.synctest_mismatches += 1;
    error!(
        "SYNCTEST MISMATCH at frame {}: {:?}",
        trigger.event().current_frame,
        trigger.event().mismatched_frames
    );
}

fn drain_p2p_events(session: Option<ResMut<Session<GameConfig>>>, mut stats: ResMut<Stats>) {
    let Some(mut session) = session else { return };
    let Session::P2P(s) = session.as_mut() else {
        return;
    };
    for event in s.events() {
        match event {
            GgrsEvent::DesyncDetected {
                frame,
                local_checksum,
                remote_checksum,
                ..
            } => {
                stats.desyncs += 1;
                error!(
                    "DESYNC at frame {frame}: local {local_checksum:X} remote {remote_checksum:X}"
                );
            }
            GgrsEvent::Disconnected { .. } => {
                stats.disconnected = true;
                warn!("GGRS event: {event:?}");
            }
            GgrsEvent::WaitRecommendation { .. } => {
                stats.prediction_stalls += 1;
                debug!("GGRS event: {event:?}");
            }
            _ => info!("GGRS event: {event:?}"),
        }
    }
}

fn check_finished(
    mut stats: ResMut<Stats>,
    limit: Res<RunLimit>,
    mut exit: MessageWriter<AppExit>,
) {
    if stats.finished {
        return;
    }
    // A synctest mismatch stalls the session (bevy_ggrs stops advancing), so
    // stop right away and fail.
    if stats.synctest_mismatches > 0 {
        stats.finished = true;
        println!("{}", stats.report());
        exit.write(AppExit::error());
        return;
    }
    if limit.frames == 0 {
        return;
    }
    // The other peer may quit a few frames before us; treat that as the end too.
    let peer_left_near_end = stats.disconnected && stats.max_frame + 600 >= limit.frames;
    if stats.max_frame >= limit.frames || peer_left_near_end {
        stats.finished = true;
        println!("{}", stats.report());
        exit.write(AppExit::Success);
    }
}

/// Prints the report when the app exits for any other reason (window closed).
fn report_on_exit(mut exits: MessageReader<AppExit>, mut stats: ResMut<Stats>) {
    if exits.read().next().is_some() && !stats.finished {
        stats.finished = true;
        println!("{}", stats.report());
    }
}

fn percentile(sorted: &[f32], p: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

impl Stats {
    pub fn report(&self) -> String {
        let wall = self.started.map_or(Duration::ZERO, |t| t.elapsed());
        let mut ft = self.frame_times_ms.clone();
        ft.sort_by(|a, b| a.total_cmp(b));
        let mut gg = self.ggrs_ms.clone();
        gg.sort_by(|a, b| a.total_cmp(b));
        let over = |limit: f32| ft.iter().filter(|&&t| t > limit).count();
        let worst: Vec<String> = self
            .worst_frames
            .iter()
            .map(|(t, f)| format!("{t:.2}@f{f}"))
            .collect();
        let avg_fps = if ft.is_empty() {
            0.0
        } else {
            1000.0 * ft.len() as f64 / ft.iter().map(|&t| t as f64).sum::<f64>()
        };
        format!(
            "==== run report ====\n\
             sim frames reached      : {} ({:.1} min of simulated play at 60 Hz)\n\
             wall time               : {:.1} s\n\
             advances (incl. resim)  : {}  resimulated: {}  rollbacks: {}\n\
             max rollback depth      : {} frames   max sim steps in one update: {}\n\
             synctest mismatches     : {}\n\
             p2p desyncs detected    : {}\n\
             rendered frames         : {}  avg fps {:.1}\n\
             frame time ms           : p50 {:.2}  p99 {:.2}  p99.9 {:.2}  worst5 [{}]\n\
             frames > 16.7/20/33 ms  : {} / {} / {}\n\
             ggrs update ms (sim+rb) : p50 {:.3}  p99 {:.3}  max {:.3}\n\
             ======================",
            self.max_frame,
            self.max_frame as f64 / 3600.0,
            wall.as_secs_f64(),
            self.advances,
            self.resimulated,
            self.rollbacks,
            self.max_rollback,
            self.max_advances_per_update,
            self.synctest_mismatches,
            self.desyncs,
            self.rendered_frames,
            avg_fps,
            percentile(&ft, 0.5),
            percentile(&ft, 0.99),
            percentile(&ft, 0.999),
            worst.join(", "),
            over(16.7),
            over(20.0),
            over(33.4),
            percentile(&gg, 0.5),
            percentile(&gg, 0.99),
            gg.last().copied().unwrap_or(0.0),
        )
    }
}
