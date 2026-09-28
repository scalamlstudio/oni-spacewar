//! Oni Spacewar client: rendering, UI, input and netcode around the Bevy-free
//! `sim` crate.
//!
//! Modes:
//!   synctest  GGRS SyncTestSession: every frame is re-simulated and checksummed.
//!   p2p       Up to 4 peers connected through a matchbox signaling server.
//!
//! Gameplay is a placeholder that exercises the foundation (see `sim`):
//! click-to-move battleships, a Q shot, and waves of chasing enemies.

mod input;
mod net;
mod render;
mod rollback;
mod stats;

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::WinitSettings;
use bevy_ggrs::prelude::*;
use ggrs::DesyncDetection;
use matchbox_socket::WebRtcSocket;
use sim::{SimParams, SimState, MAX_PLAYERS};

use input::{BotBrains, KeyboardPlayer};
use net::{EmulatedGgrsSocket, NetEmuConfig};
use rollback::{GameConfig, InjectDesync, RollbackPlugin, SimWorld};
use stats::{RunLimit, StatsPlugin};

const USAGE: &str = "\
usage:
  oni-spacewar synctest [--minutes M] [--check-distance D] [--headless] [--bot]
  oni-spacewar p2p [--room ws://127.0.0.1:3536/oni?next=2]
                   [--delay-ms 50] [--jitter-ms 0] [--loss 0.0] [--input-delay 2]
                   [--minutes M] [--headless] [--bot]
common: [--players 2] [--seed 42] [--no-vsync] [--inject-desync]";

#[derive(Clone, Debug)]
struct Args {
    mode: String,
    headless: bool,
    bot: bool,
    vsync: bool,
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
}

fn usage_exit(msg: &str) -> ! {
    eprintln!("{msg}{USAGE}");
    std::process::exit(2)
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1);
    let mode = it.next().unwrap_or_else(|| usage_exit(""));
    let mut a = Args {
        mode,
        headless: false,
        bot: false,
        vsync: true,
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

/// The matchbox socket. Kept for the whole run: the GGRS session owns its
/// data channel, but the socket must stay alive for the connection to live.
#[derive(Resource)]
struct MatchboxSocket(WebRtcSocket);

#[derive(Resource, Clone)]
struct AppArgs(Args);

fn main() {
    let args = parse_args();
    let mut app = App::new();

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
                resolution: WindowResolution::new(1000, 580),
                present_mode: if args.vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                ..default()
            }),
            ..default()
        }))
        // Keep simulating when the window is in the background (two windows side by side).
        .insert_resource(WinitSettings::continuous())
        .add_plugins(render::RenderPlugin);
    }

    let frames = if args.minutes > 0.0 {
        (args.minutes * 60.0 * 60.0) as u32
    } else {
        0
    };
    app.add_plugins(GgrsPlugin::<GameConfig>::default())
        .insert_resource(RollbackFrameRate(60))
        .insert_resource(SimWorld(SimState::new(SimParams {
            num_players: args.players,
            seed: args.seed,
        })))
        .insert_resource(RunLimit {
            frames,
            warmup_frames: 180,
        })
        .insert_resource(InjectDesync(args.inject_desync))
        .insert_resource(KeyboardPlayer(!args.bot && !args.headless))
        .init_resource::<NetStatus>()
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
            let (socket, message_loop) = WebRtcSocket::builder(args.room.clone())
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

fn wait_for_peers(
    mut commands: Commands,
    socket: Option<ResMut<MatchboxSocket>>,
    session: Option<Res<Session<GameConfig>>>,
    args: Res<AppArgs>,
    mut status: ResMut<NetStatus>,
) {
    let Some(mut socket) = socket else { return };
    if session.is_some() {
        return;
    }
    let args = &args.0;
    if socket.0.try_update_peers().is_err() {
        status.0 = "signaling connection failed".into();
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
