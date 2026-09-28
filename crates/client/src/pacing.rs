//! Frame-pacing profiler (TAKOAI-23). Windowed runs only; nothing here touches
//! the simulation.
//!
//! Bevy renders pipelined: the main app updates frame N while the render
//! sub-app (its own thread) draws frame N-1, and the two meet once per frame
//! to extract. A long gap between two main-app frames therefore comes from one
//! of these, which are timed separately:
//!
//! - **main**: main-app `First`..`Last` (input, GGRS/sim, HUD, ...)
//! - **prepare**: render sub-app CPU work before the render graph (extract
//!   commands, prepare, queue, sort), minus `acquire`
//! - **acquire**: `prepare_windows`, which blocks in `get_current_texture`
//!   until the OS hands back a swapchain image (vsync back-pressure / the
//!   compositor)
//! - **submit**: `RenderSystems::Render` (render graph encode, queue submit,
//!   present)
//! - **other**: none of the above (winit event loop, time outside Bevy)
//!
//! Every main-app frame longer than [`HITCH_MS`] is attributed to whichever of
//! these took the most time inside its interval.
//!
//! Any of these spans can also be long because the OS did not run our thread.
//! To tell that apart, a probe thread sleeps 1 ms in a loop and records how
//! late it wakes up; a wake-up more than [`STALL_MS`] late is a scheduler
//! stall, and each hitch reports the worst stall that overlapped it.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::view::prepare_windows;
use bevy::render::{Render, RenderApp, RenderSystems};

use crate::rollback::SimWorld;
use crate::stats::RunLimit;

/// A main-app frame longer than this is a hitch (a missed 60 Hz vsync).
pub const HITCH_MS: f32 = 20.0;
/// A probe wake-up this late (beyond its 1 ms sleep) counts as a scheduler stall.
pub const STALL_MS: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
struct Span {
    start: Instant,
    end: Instant,
}

impl Span {
    fn at(t: Instant) -> Self {
        Span { start: t, end: t }
    }
    /// Milliseconds of this span that fall inside `[from, to]`.
    fn overlap_ms(&self, from: Instant, to: Instant) -> f32 {
        let s = self.start.max(from);
        let e = self.end.min(to);
        if e > s {
            (e - s).as_secs_f32() * 1000.0
        } else {
            0.0
        }
    }
    fn ms(&self) -> f32 {
        (self.end - self.start).as_secs_f32() * 1000.0
    }
}

/// One render sub-app frame. `acquire` and `submit` lie inside `total`.
#[derive(Clone, Copy)]
struct RenderFrame {
    total: Span,
    acquire: Span,
    submit: Span,
}

/// Timestamps the render thread fills in for the frame it is drawing.
#[derive(Default)]
struct InFlight {
    start: Option<Instant>,
    acquire: Option<Span>,
    submit: Option<Span>,
    pending: Option<Instant>,
}

#[derive(Default)]
struct Shared {
    /// Finished render frames not yet collected by the main app.
    done: Vec<RenderFrame>,
    in_flight: InFlight,
    /// Scheduler stalls seen by the probe: (when it should have woken, ms late).
    stalls: Vec<(Instant, f32)>,
    probe_wakeups: u64,
}

/// Shared between the main world, the render world and the probe thread.
#[derive(Resource, Clone, Default)]
struct PacingShared(Arc<Mutex<Shared>>);

#[derive(Default, Clone, Copy, Debug)]
pub struct Hitch {
    pub frame_ms: f32,
    pub sim_frame: u32,
    pub main_ms: f32,
    pub prepare_ms: f32,
    pub acquire_ms: f32,
    pub submit_ms: f32,
    /// Worst scheduler stall overlapping this frame (0 = none).
    pub stall_ms: f32,
    pub cause: &'static str,
}

/// Results, read by the stats report. Everything after warm-up only.
#[derive(Resource, Default)]
pub struct Pacing {
    pub main_ms: Vec<f32>,
    pub prepare_ms: Vec<f32>,
    pub acquire_ms: Vec<f32>,
    pub submit_ms: Vec<f32>,
    pub hitches: Vec<Hitch>,
    pub stalls: Vec<f32>,
    pub probe_wakeups: u64,
    main_start: Option<Instant>,
    prev_frame_start: Option<Instant>,
    prev_main: Option<Span>,
    /// Render frames that may still overlap the next main-app interval.
    render: Vec<RenderFrame>,
    /// Stalls from the previous frame (a stall is reported only once the probe
    /// has woken up, so it can land one frame late).
    prev_stalls: Vec<(Instant, f32)>,
}

pub struct PacingPlugin;

impl Plugin for PacingPlugin {
    fn build(&self, app: &mut App) {
        let shared = PacingShared::default();
        let probe = shared.clone();
        std::thread::Builder::new()
            .name("pacing-probe".into())
            .spawn(move || probe_loop(probe))
            .expect("spawn pacing probe");
        app.insert_resource(shared.clone())
            .init_resource::<Pacing>()
            .add_systems(First, main_start)
            .add_systems(Last, main_end);
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.insert_resource(shared).add_systems(
            Render,
            (
                render_start.before(RenderSystems::ExtractCommands),
                mark.before(prepare_windows)
                    .in_set(RenderSystems::PrepareViews),
                acquire_end
                    .after(prepare_windows)
                    .in_set(RenderSystems::PrepareViews),
                mark.after(RenderSystems::PrepareBindGroups)
                    .before(RenderSystems::Render),
                submit_end
                    .after(RenderSystems::Render)
                    .before(RenderSystems::Cleanup),
                render_end.after(RenderSystems::PostCleanup),
            ),
        );
    }
}

fn probe_loop(shared: PacingShared) {
    let nap = Duration::from_millis(1);
    loop {
        let t = Instant::now();
        std::thread::sleep(nap);
        let late = t.elapsed().saturating_sub(nap).as_secs_f32() * 1000.0;
        let mut s = shared.0.lock().unwrap();
        s.probe_wakeups += 1;
        if late > STALL_MS {
            s.stalls.push((t + nap, late));
        }
    }
}

fn render_start(shared: Res<PacingShared>) {
    shared.0.lock().unwrap().in_flight = InFlight {
        start: Some(Instant::now()),
        ..default()
    };
}

/// Start of a sub-span; closed by `acquire_end` / `submit_end`.
fn mark(shared: Res<PacingShared>) {
    shared.0.lock().unwrap().in_flight.pending = Some(Instant::now());
}

fn close(f: &mut InFlight) -> Option<Span> {
    f.pending.take().map(|start| Span {
        start,
        end: Instant::now(),
    })
}

fn acquire_end(shared: Res<PacingShared>) {
    let f = &mut shared.0.lock().unwrap().in_flight;
    f.acquire = close(f);
}

fn submit_end(shared: Res<PacingShared>) {
    let f = &mut shared.0.lock().unwrap().in_flight;
    f.submit = close(f);
}

fn render_end(shared: Res<PacingShared>) {
    let s = &mut *shared.0.lock().unwrap();
    let f = std::mem::take(&mut s.in_flight);
    if let Some(start) = f.start {
        let end = Instant::now();
        s.done.push(RenderFrame {
            total: Span { start, end },
            // No swapchain image this frame (e.g. minimised): zero-length.
            acquire: f.acquire.unwrap_or(Span::at(end)),
            submit: f.submit.unwrap_or(Span::at(end)),
        });
    }
}

fn main_start(mut pacing: ResMut<Pacing>) {
    pacing.main_start = Some(Instant::now());
}

fn main_end(
    mut pacing: ResMut<Pacing>,
    shared: Res<PacingShared>,
    limit: Res<RunLimit>,
    world: Res<SimWorld>,
) {
    let p = &mut *pacing;
    let Some(start) = p.main_start.take() else {
        return;
    };
    let main = Span {
        start,
        end: Instant::now(),
    };
    let (stalls, wakeups) = {
        let mut sh = shared.0.lock().unwrap();
        p.render.append(&mut sh.done);
        (
            std::mem::take(&mut sh.stalls),
            std::mem::take(&mut sh.probe_wakeups),
        )
    };
    let warm = world.frame > limit.warmup_frames;

    // Frame interval = start of this main update minus start of the previous
    // one (what `Time<Real>` measures). The previous main update lies inside it.
    if let (true, Some(prev)) = (warm, p.prev_frame_start) {
        p.main_ms.push(main.ms());
        p.probe_wakeups += wakeups;
        p.stalls.extend(stalls.iter().map(|s| s.1));
        let frame_ms = (start - prev).as_secs_f32() * 1000.0;
        if frame_ms > HITCH_MS {
            let mut h = Hitch {
                frame_ms,
                sim_frame: world.frame,
                main_ms: p.prev_main.map_or(0.0, |m| m.overlap_ms(prev, start)),
                ..default()
            };
            for r in &p.render {
                let a = r.acquire.overlap_ms(prev, start);
                let s = r.submit.overlap_ms(prev, start);
                h.acquire_ms += a;
                h.submit_ms += s;
                h.prepare_ms += r.total.overlap_ms(prev, start) - a - s;
            }
            h.stall_ms = stalls
                .iter()
                .chain(&p.prev_stalls)
                .filter(|(t, late)| {
                    *t <= start && *t + Duration::from_secs_f32(late / 1000.0) >= prev
                })
                .map(|s| s.1)
                .fold(0.0, f32::max);
            let busy = h.main_ms.max(h.prepare_ms + h.acquire_ms + h.submit_ms);
            h.cause = [
                ("main", h.main_ms),
                ("prepare", h.prepare_ms),
                ("acquire", h.acquire_ms),
                ("submit", h.submit_ms),
                ("other", frame_ms - busy),
            ]
            .into_iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
            p.hitches.push(h);
        }
    }

    // Per-render-frame stats for frames that can no longer overlap the next
    // interval, then drop them.
    let (old, keep): (Vec<RenderFrame>, Vec<RenderFrame>) =
        p.render.iter().partition(|r| r.total.end <= start);
    if warm {
        for r in old {
            p.acquire_ms.push(r.acquire.ms());
            p.submit_ms.push(r.submit.ms());
            p.prepare_ms
                .push(r.total.ms() - r.acquire.ms() - r.submit.ms());
        }
    }
    p.render = keep;
    p.prev_frame_start = Some(start);
    p.prev_main = Some(main);
    p.prev_stalls = stalls;
}

fn percentile(sorted: &[f32], p: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn summary(v: &[f32]) -> String {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    format!(
        "p50 {:.2}  p99 {:.2}  max {:.2}",
        percentile(&s, 0.5),
        percentile(&s, 0.99),
        s.last().copied().unwrap_or(0.0)
    )
}

impl Pacing {
    pub fn report(&self) -> String {
        let count = |c: &str| self.hitches.iter().filter(|h| h.cause == c).count();
        let stalled = self.hitches.iter().filter(|h| h.stall_ms > 0.0).count();
        let mut worst = self.hitches.clone();
        worst.sort_by(|a, b| b.frame_ms.total_cmp(&a.frame_ms));
        let worst: Vec<String> = worst
            .iter()
            .take(5)
            .map(|h| {
                format!(
                    "{:.1}@f{} {} [main {:.1} prep {:.1} acq {:.1} submit {:.1} | stall {:.1}]",
                    h.frame_ms,
                    h.sim_frame,
                    h.cause,
                    h.main_ms,
                    h.prepare_ms,
                    h.acquire_ms,
                    h.submit_ms,
                    h.stall_ms
                )
            })
            .collect();
        format!(
            "\n---- frame pacing (ms) ----\n\
             main update             : {}\n\
             render prepare          : {}\n\
             swapchain acquire       : {}\n\
             render graph + submit   : {}\n\
             hitches > {HITCH_MS}           : {}  by cause: main {} / prepare {} / acquire {} / submit {} / other {}\n\
             hitches during OS stall : {}  (probe woke > {STALL_MS} ms late)\n\
             OS stalls (probe)       : {} of {} wake-ups, {}\n\
             worst hitches           : {}",
            summary(&self.main_ms),
            summary(&self.prepare_ms),
            summary(&self.acquire_ms),
            summary(&self.submit_ms),
            self.hitches.len(),
            count("main"),
            count("prepare"),
            count("acquire"),
            count("submit"),
            count("other"),
            stalled,
            self.stalls.len(),
            self.probe_wakeups,
            summary(&self.stalls),
            if worst.is_empty() {
                "-".into()
            } else {
                worst.join("\n                           ")
            },
        )
    }
}
