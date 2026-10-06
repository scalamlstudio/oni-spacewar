//! Oni Spacewar client: rendering, UI, input and netcode around the Bevy-free
//! `sim` crate.
//!
//! Modes:
//!   synctest  GGRS SyncTestSession: every frame is re-simulated and checksummed.
//!   p2p       Up to 4 peers connected through a matchbox signaling server.
//!
//! With no mode, runs the single-player first-playable flow (`flow`). The
//! gameplay is the Elimination mission in `sim`; synctest / p2p runs give the
//! players alternating Kite / Bulwark loadouts so both ships are exercised.

mod art;
mod carrier;
mod flow;
mod hints;
mod input;
mod layout;
mod mission;
mod net;
mod pacing;
mod render;
mod rollback;
mod save;
mod sky;
mod stats;
mod workshop;

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::ecs::system::NonSendMarker;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{PresentMode, PrimaryWindow, WindowLevel, WindowResolution};
use bevy::winit::{WinitSettings, WINIT_WINDOWS};
use bevy_ggrs::prelude::*;
use content::ContentManifest;
use ggrs::DesyncDetection;
use matchbox_socket::{RtcIceServerConfig, WebRtcSocket};
use sim::{Loadout, ShipKind, SimState, MAX_PLAYERS};

use input::{BotBrains, KeyboardPlayer};
use net::{EmulatedGgrsSocket, NetEmuConfig};
use rollback::{GameConfig, InjectDesync, RollbackPlugin, SimWorld};
use stats::{RunLimit, Stats, StatsPlugin};

/// Default window size in px. The sim's `INITIAL_VIEW_HALF_*` is this at
/// `render::BATTLE_ZOOM` (checked by a test).
pub const DEFAULT_WINDOW: UVec2 = UVec2::new(1000, 580);

const USAGE: &str = "\
usage:
  oni-spacewar [--autoplay [--ship kite|bulwark] [--missions N] [--continue] [--abandon]
                [--shots DIR]]
  oni-spacewar synctest [--minutes M] [--check-distance D] [--headless] [--bot]
  oni-spacewar p2p [--room ws://127.0.0.1:3536/oni?next=2]
                   [--delay-ms 50] [--jitter-ms 0] [--loss 0.0] [--input-delay 2]
                   [--minutes M] [--headless] [--bot]
                   [--ice URL]...   STUN/TURN servers (default: public Google STUN;
                                    `--ice none` = host candidates only, LAN)
                   TURN auth from env: ONI_ICE_USERNAME, ONI_ICE_CREDENTIAL
common: [--players 2] [--seed 42] [--no-vsync] [--frame-latency N] [--inject-desync]";

#[derive(Clone, Debug)]
struct Args {
    mode: String,
    headless: bool,
    bot: bool,
    vsync: bool,
    /// Swapchain images the GPU may queue ahead (wgpu default 2).
    frame_latency: Option<u32>,
    minutes: f64,
    seed: u64,
    check_distance: usize,
    room: String,
    players: usize,
    input_delay: usize,
    max_prediction: usize,
    desync_interval: u32,
    inject_desync: bool,
    emu: NetEmuConfig,
    /// `--ice` URLs; empty = matchbox's default STUN servers.
    ice: Vec<String>,
    /// Demo QA: fly `missions` battles with a scripted pilot, then quit.
    autoplay: bool,
    ship: String,
    missions: u32,
    /// `--continue`: autoplay starts from the saved game instead of New Game.
    resume: bool,
    /// `--abandon`: autoplay quits its last mission (counts as Failed).
    abandon: bool,
    shots: Option<std::path::PathBuf>,
}

fn usage_exit(msg: &str) -> ! {
    eprintln!("{msg}{USAGE}");
    std::process::exit(2)
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1).peekable();
    // No mode (or only flags) = the demo flow.
    let mode = match it.peek() {
        Some(m) if !m.starts_with("--") => it.next().unwrap(),
        _ => "demo".to_string(),
    };
    let mut a = Args {
        mode,
        headless: false,
        bot: false,
        vsync: true,
        frame_latency: None,
        minutes: 0.0,
        seed: 42,
        check_distance: 7,
        room: String::new(),
        players: 2,
        input_delay: 2,
        max_prediction: 8,
        desync_interval: 10,
        inject_desync: false,
        emu: NetEmuConfig::default(),
        ice: Vec::new(),
        autoplay: false,
        ship: mission::KITE_ID.to_string(),
        missions: 1,
        resume: false,
        abandon: false,
        shots: None,
    };

    fn val<T: std::str::FromStr>(name: &str, it: &mut dyn Iterator<Item = String>) -> T {
        let raw = it
            .next()
            .unwrap_or_else(|| usage_exit(&format!("missing value for {name}\n")));
        raw.parse()
            .unwrap_or_else(|_| usage_exit(&format!("bad value for {name}: {raw}\n")))
    }
    while let Some(flag) = it.next() {
        let it = &mut it;
        match flag.as_str() {
            "--headless" => a.headless = true,
            "--bot" => a.bot = true,
            "--no-vsync" => a.vsync = false,
            "--frame-latency" => a.frame_latency = Some(val(&flag, it)),
            "--inject-desync" => a.inject_desync = true,
            "--minutes" => a.minutes = val(&flag, it),
            "--seed" => a.seed = val(&flag, it),
            "--check-distance" => a.check_distance = val(&flag, it),
            "--room" => a.room = val(&flag, it),
            "--players" => a.players = val(&flag, it),
            "--input-delay" => a.input_delay = val(&flag, it),
            "--max-prediction" => a.max_prediction = val(&flag, it),
            "--desync-interval" => a.desync_interval = val(&flag, it),
            "--delay-ms" => a.emu.delay = Duration::from_millis(val(&flag, it)),
            "--jitter-ms" => a.emu.jitter = Duration::from_millis(val(&flag, it)),
            "--loss" => a.emu.loss = val(&flag, it),
            "--ice" => a.ice.push(val(&flag, it)),
            "--autoplay" => a.autoplay = true,
            "--ship" => a.ship = val(&flag, it),
            "--missions" => a.missions = val(&flag, it),
            "--continue" => a.resume = true,
            "--abandon" => a.abandon = true,
            "--shots" => a.shots = Some(val(&flag, it)),
            _ => usage_exit(&format!("unknown flag {flag}\n")),
        }
    }
    if !(1..=MAX_PLAYERS).contains(&a.players) {
        usage_exit(&format!("--players must be 1..={MAX_PLAYERS}\n"));
    }
    if a.room.is_empty() {
        a.room = format!("ws://127.0.0.1:3536/oni?next={}", a.players);
    }
    a
}

/// Short connection/status line for the HUD and logs.
#[derive(Resource, Default)]
pub struct NetStatus(pub String);

/// Human-readable proof that the client resolved a shipped asset by stable ID.
#[derive(Resource, Clone)]
pub struct ContentStatus(pub String);

/// The matchbox socket. Kept for the whole run: the GGRS session owns its
/// data channel, but the socket must stay alive for the connection to live.
#[derive(Resource)]
struct MatchboxSocket(WebRtcSocket);

#[derive(Resource, Clone)]
struct AppArgs(Args);

fn main() {
    let args = parse_args();
    let mut app = App::new();
    let content_status = load_content_status();

    if args.headless {
        // No window, no renderer. Synctest runs as fast as the CPU allows (each
        // update is fed exactly one 60 Hz tick); p2p runs in real time.
        let wait = if args.mode == "synctest" {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(1.0 / 240.0)
        };
        app.add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(wait)),
            LogPlugin::default(),
        ));
        if args.mode == "synctest" {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                16_666_667,
            )));
        }
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Oni Spacewar — {}", args.mode),
                resolution: WindowResolution::new(DEFAULT_WINDOW.x, DEFAULT_WINDOW.y),
                present_mode: if args.vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                desired_maximum_frame_latency: args.frame_latency.and_then(std::num::NonZero::new),
                // macOS presents no frames for a fully covered window, so
                // autoplay screenshots would be black: keep it on top.
                window_level: if args.autoplay {
                    WindowLevel::AlwaysOnTop
                } else {
                    WindowLevel::Normal
                },
                ..default()
            }),
            ..default()
        }))
        // Keep simulating when the window is in the background (two windows side by side).
        .insert_resource(WinitSettings::continuous())
        .add_plugins(render::RenderPlugin);
        // Pacing feeds the synctest/p2p run report and needs their RunLimit.
        if args.mode != "demo" {
            app.add_plugins(pacing::PacingPlugin);
        }
    }

    let frames = if args.minutes > 0.0 {
        (args.minutes * 60.0 * 60.0) as u32
    } else {
        0
    };
    if args.mode == "demo" {
        if args.headless {
            usage_exit("demo mode requires a window\n");
        }
        // The battle inserts its own SimWorld on entry; none exists outside it.
        app.insert_resource(NetStatus("demo flow".into()))
            .insert_resource(ContentStatus(content_status))
            .init_resource::<Stats>()
            .add_plugins((flow::FlowPlugin, carrier::CarrierPlugin, hints::HintsPlugin));
        if args.autoplay {
            if let Some(dir) = &args.shots {
                std::fs::create_dir_all(dir).expect("create --shots dir");
            }
            app.add_systems(Update, bring_window_to_front);
            app.insert_resource(flow::Autoplay {
                ship: args.ship.clone(),
                shots: args.shots.clone(),
                missions: args.missions,
                resume: args.resume,
                abandon: args.abandon,
                ..Default::default()
            });
        }
        if app.run().is_error() {
            std::process::exit(1);
        }
        return;
    }

    app.add_plugins(GgrsPlugin::<GameConfig>::default())
        .insert_resource(RollbackFrameRate(60))
        .insert_resource(SimWorld(SimState::with_loadouts(
            args.seed,
            &test_loadouts(args.players),
        )))
        .insert_resource(RunLimit {
            frames,
            warmup_frames: 180,
        })
        .insert_resource(InjectDesync(args.inject_desync))
        .insert_resource(KeyboardPlayer(!args.bot && !args.headless))
        .init_resource::<NetStatus>()
        .insert_resource(ContentStatus(content_status))
        .init_resource::<BotBrains>()
        .insert_resource(AppArgs(args.clone()))
        .add_plugins((RollbackPlugin, StatsPlugin))
        .add_systems(ReadInputs, input::read_local_inputs);

    match args.mode.as_str() {
        "synctest" => {
            let mut builder = SessionBuilder::<GameConfig>::new()
                .with_num_players(args.players)
                .unwrap()
                .with_check_distance(args.check_distance)
                .with_input_delay(0);
            for handle in 0..args.players {
                builder = builder.add_player(PlayerType::Local, handle).unwrap();
            }
            let session = builder.start_synctest_session().expect("synctest session");
            app.insert_resource(Session::SyncTest(session));
            app.insert_resource(NetStatus(format!(
                "synctest (check distance {})",
                args.check_distance
            )));
        }
        "p2p" => {
            info!("connecting to matchbox signaling server at {}", args.room);
            let ice = ice_config(&args.ice);
            info!("ICE servers: {:?}", ice.urls);
            let (socket, message_loop) = WebRtcSocket::builder(args.room.clone())
                .ice_server(ice)
                .add_unreliable_channel()
                .build();
            // The message loop drives signaling + WebRTC; run it on its own thread.
            std::thread::spawn(move || {
                if let Err(e) = bevy::tasks::block_on(message_loop) {
                    eprintln!("matchbox message loop ended: {e:?}");
                }
            });
            app.insert_resource(MatchboxSocket(socket))
                .insert_resource(NetStatus("waiting for peers".into()))
                .add_systems(Update, wait_for_peers);
        }
        _ => usage_exit(""),
    }

    // The report is printed by StatsPlugin on any AppExit (limit reached,
    // window closed).
    if app.run().is_error() {
        std::process::exit(1);
    }
}

/// Synctest / p2p fleet: even handles fly Kite, odd handles Bulwark;
/// handles 2 and up have the Training Room, so the gate covers its
/// cooldowns too.
fn test_loadouts(players: usize) -> Vec<Loadout> {
    (0..players)
        .map(|h| Loadout {
            ship: if h % 2 == 0 {
                ShipKind::Kite
            } else {
                ShipKind::Bulwark
            },
            upgrades: sim::Upgrades {
                training: u8::from(h >= 2),
                ..Default::default()
            },
        })
        .collect()
}

fn load_content_status() -> String {
    let manifest = match ContentManifest::load("assets/manifest.json") {
        Ok(manifest) => manifest,
        Err(e) => return format!("content unavailable ({e})"),
    };
    match manifest.load_text("assets", "core.ui.hud_status") {
        Ok(text) => {
            let pack = manifest
                .pack("core")
                .map(|pack| pack.version.as_str())
                .unwrap_or("unknown");
            format!("{} | core pack {}", text.trim(), pack)
        }
        Err(e) => format!("content asset unavailable ({e})"),
    }
}

/// STUN/TURN servers for NAT traversal. matchbox takes one ICE server entry
/// with a URL list and a single username/credential, which TURN servers use
/// and STUN servers ignore. Credentials come from the environment so they
/// never end up in the repo, shell history or process list.
fn ice_config(urls: &[String]) -> RtcIceServerConfig {
    let mut ice = RtcIceServerConfig::default();
    if urls.iter().any(|u| u == "none") {
        ice.urls.clear();
    } else if !urls.is_empty() {
        ice.urls = urls.to_vec();
    }
    ice.username = std::env::var("ONI_ICE_USERNAME").ok();
    ice.credential = std::env::var("ONI_ICE_CREDENTIAL").ok();
    if ice.urls.iter().any(|u| u.starts_with("turn")) && ice.credential.is_none() {
        warn!("a turn: server is configured but ONI_ICE_CREDENTIAL is not set");
    }
    ice
}

fn wait_for_peers(
    mut commands: Commands,
    socket: Option<ResMut<MatchboxSocket>>,
    session: Option<Res<Session<GameConfig>>>,
    args: Res<AppArgs>,
    mut status: ResMut<NetStatus>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut socket) = socket else { return };
    if session.is_some() {
        return;
    }
    let args = &args.0;
    if socket.0.try_update_peers().is_err() {
        // matchbox gave up on the signaling server; without it no peer can
        // ever join, so fail instead of waiting forever.
        status.0 = "signaling connection failed".into();
        error!("{} ({})", status.0, args.room);
        exit.write(AppExit::error());
        return;
    }
    let connected = socket.0.connected_peers().count();
    if connected + 1 < args.players {
        status.0 = format!("waiting for peers ({}/{})", connected + 1, args.players);
        return;
    }
    let Some(players) = net::ggrs_players(&mut socket.0) else {
        return;
    };
    let mut builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(args.players)
        .unwrap()
        .with_input_delay(args.input_delay)
        .with_max_prediction_window(args.max_prediction)
        .with_desync_detection_mode(DesyncDetection::On {
            interval: args.desync_interval,
        });
    let mut local = 0;
    for (handle, player) in players.into_iter().enumerate() {
        if matches!(player, PlayerType::Local) {
            local = handle;
        }
        builder = builder.add_player(player, handle).unwrap();
    }
    let channel = socket.0.take_channel(0).unwrap();
    let session = builder
        .start_p2p_session(EmulatedGgrsSocket::new(channel, args.emu))
        .expect("p2p session");
    status.0 = format!(
        "p2p P{} | +{}ms±{} one-way, {:.1}% loss, input delay {}",
        local + 1,
        args.emu.delay.as_millis(),
        args.emu.jitter.as_millis(),
        args.emu.loss * 100.0,
        args.input_delay
    );
    info!("session started: {}", status.0);
    commands.insert_resource(Session::P2P(session));
}

/// Activates the app and raises its window once, as soon as winit has
/// created it (autoplay: a covered window renders black screenshots).
fn bring_window_to_front(
    mut done: Local<bool>,
    window: Query<Entity, With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    if *done {
        return;
    }
    let Ok(entity) = window.single() else {
        return;
    };
    WINIT_WINDOWS.with_borrow(|windows| {
        if let Some(w) = windows.get_window(entity) {
            w.focus_window();
            *done = true;
        }
    });
}
