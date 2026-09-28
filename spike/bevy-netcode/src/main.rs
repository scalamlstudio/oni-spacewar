//! Bevy + bevy_ggrs + matchbox rollback spike (TAKOAI-18).
//!
//! Modes (see README.md):
//!   synctest  GGRS SyncTestSession: every frame is re-simulated and checksummed.
//!   p2p       Two (or more) peers connected through a matchbox signaling server.

mod net;
mod render;
mod sim;
mod stats;
mod trig;

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::WinitSettings;
use bevy_ggrs::prelude::*;
use bevy_ggrs::{LocalInputs, LocalPlayers};
use ggrs::DesyncDetection;
use matchbox_socket::WebRtcSocket;

use net::{EmulatedGgrsSocket, NetEmuConfig};
use sim::*;
use stats::{RunLimit, Stats, StatsPlugin};

const USAGE: &str = "\
usage:
  bevy-netcode-spike synctest [--minutes M] [--check-distance D] [--headless] [--bot]
  bevy-netcode-spike p2p [--room ws://127.0.0.1:3536/spike?next=2] [--players 2]
                         [--delay-ms 50] [--jitter-ms 0] [--loss 0.0] [--input-delay 2]
                         [--minutes M] [--headless] [--bot]
common: [--bullets 1000] [--seed 42] [--no-vsync]";

#[derive(Clone, Debug)]
struct Args {
    mode: String,
    headless: bool,
    bot: bool,
    vsync: bool,
    minutes: f64,
    bullets: usize,
    seed: u64,
    check_distance: usize,
    room: String,
    players: usize,
    input_delay: usize,
    max_prediction: usize,
    desync_interval: u32,
    emu: NetEmuConfig,
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1);
    let mode = it.next().unwrap_or_else(|| {
        eprintln!("{USAGE}");
        std::process::exit(2)
    });
    let mut a = Args {
        mode,
        headless: false,
        bot: false,
        vsync: true,
        minutes: 0.0,
        bullets: 1000,
        seed: 42,
        check_distance: 7,
        room: String::new(),
        players: 2,
        input_delay: 2,
        max_prediction: 8,
        desync_interval: 10,
        emu: NetEmuConfig::default(),
    };
    let mut val = |name: &str, it: &mut dyn Iterator<Item = String>| -> String {
        it.next().unwrap_or_else(|| {
            eprintln!("missing value for {name}\n{USAGE}");
            std::process::exit(2)
        })
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--headless" => a.headless = true,
            "--bot" => a.bot = true,
            "--no-vsync" => a.vsync = false,
            "--minutes" => a.minutes = val(&flag, &mut it).parse().unwrap(),
            "--bullets" => a.bullets = val(&flag, &mut it).parse().unwrap(),
            "--seed" => a.seed = val(&flag, &mut it).parse().unwrap(),
            "--check-distance" => a.check_distance = val(&flag, &mut it).parse().unwrap(),
            "--room" => a.room = val(&flag, &mut it),
            "--players" => a.players = val(&flag, &mut it).parse().unwrap(),
            "--input-delay" => a.input_delay = val(&flag, &mut it).parse().unwrap(),
            "--max-prediction" => a.max_prediction = val(&flag, &mut it).parse().unwrap(),
            "--desync-interval" => a.desync_interval = val(&flag, &mut it).parse().unwrap(),
            "--delay-ms" => {
                a.emu.delay = Duration::from_millis(val(&flag, &mut it).parse().unwrap())
            }
            "--jitter-ms" => {
                a.emu.jitter = Duration::from_millis(val(&flag, &mut it).parse().unwrap())
            }
            "--loss" => a.emu.loss = val(&flag, &mut it).parse().unwrap(),
            _ => {
                eprintln!("unknown flag {flag}\n{USAGE}");
                std::process::exit(2)
            }
        }
    }
    if a.room.is_empty() {
        a.room = format!("ws://127.0.0.1:3536/spike?next={}", a.players);
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
        app.add_plugins((MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(wait)), LogPlugin::default()));
        if args.mode == "synctest" {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(16_666_667)));
        }
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Oni Spacewar netcode spike — {}", args.mode),
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
    app.add_plugins(GgrsPlugin::<SpikeConfig>::default())
        .insert_resource(RollbackFrameRate(60))
        .insert_resource(SimParams {
            num_players: args.players,
            seed: args.seed,
            bullet_target: args.bullets,
        })
        .insert_resource(RunLimit {
            frames,
            warmup_frames: 180,
        })
        .init_resource::<NetStatus>()
        .init_resource::<BotBrains>()
        .insert_resource(AppArgs(args.clone()))
        .add_plugins((SimPlugin, StatsPlugin))
        .add_systems(ReadInputs, read_local_inputs);

    match args.mode.as_str() {
        "synctest" => {
            let mut builder = SessionBuilder::<SpikeConfig>::new()
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
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }

    app.run();
    let stats = app.world().resource::<Stats>();
    if !stats.finished {
        println!("{}", stats.report());
    }
}

fn wait_for_peers(
    mut commands: Commands,
    socket: Option<ResMut<MatchboxSocket>>,
    session: Option<Res<Session<SpikeConfig>>>,
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
    let players = socket.0.players();
    let mut builder = SessionBuilder::<SpikeConfig>::new()
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

/// Scripted local players for unattended runs: random 8-way direction changes
/// every 6..40 frames. Only the resulting inputs matter to the simulation.
#[derive(Resource, Default)]
struct BotBrains(HashMap<usize, (u64, u8, u32)>);

fn bot_input(brains: &mut BotBrains, handle: usize) -> u8 {
    let (rng, buttons, left) = brains
        .0
        .entry(handle)
        .or_insert(((handle as u64 + 1) * 0x9E37_79B9_7F4A_7C15, 0, 0));
    if *left == 0 {
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        *rng ^= *rng << 17;
        let r = *rng;
        const DIRS: [u8; 9] = [
            0,
            INPUT_UP,
            INPUT_DOWN,
            INPUT_LEFT,
            INPUT_RIGHT,
            INPUT_UP | INPUT_LEFT,
            INPUT_UP | INPUT_RIGHT,
            INPUT_DOWN | INPUT_LEFT,
            INPUT_DOWN | INPUT_RIGHT,
        ];
        *buttons = DIRS[(r % 9) as usize] | if (r >> 8) % 4 == 0 { INPUT_FOCUS } else { 0 };
        *left = 6 + ((r >> 16) % 35) as u32;
    }
    *left -= 1;
    *buttons
}

fn keyboard_input(keys: &ButtonInput<KeyCode>) -> u8 {
    let mut b = 0;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        b |= INPUT_UP;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        b |= INPUT_DOWN;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        b |= INPUT_LEFT;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        b |= INPUT_RIGHT;
    }
    if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        b |= INPUT_FOCUS;
    }
    b
}

fn read_local_inputs(
    mut commands: Commands,
    local_players: Res<LocalPlayers>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    args: Res<AppArgs>,
    mut brains: ResMut<BotBrains>,
) {
    let mut local_inputs = HashMap::new();
    for (i, handle) in local_players.0.iter().enumerate() {
        // The first local player is on the keyboard unless --bot/--headless;
        // any further local players (synctest) are always bots.
        let human = i == 0 && !args.0.bot && keys.is_some();
        let buttons = match (&keys, human) {
            (Some(keys), true) => keyboard_input(keys),
            _ => bot_input(&mut brains, *handle),
        };
        local_inputs.insert(*handle, NetInput { buttons });
    }
    commands.insert_resource(LocalInputs::<SpikeConfig>(local_inputs));
}
