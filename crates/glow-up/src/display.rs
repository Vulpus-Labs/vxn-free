//! The tracking display (ticket 0009), set into the faceplate's window:
//! what the bank has done over the last [`SPAN_SECONDS`], from the engine's
//! telemetry frames ([`Observation`]).
//!
//! - Time runs left to right, the newest at the right edge; frequency up a
//!   log axis from [`LOWEST`] to [`HIGHEST`], with a line at each decade's
//!   1, 2 and 5.
//! - Each filter is an amber trace at its centre, its brightness and
//!   weight its level. A singing filter (self-oscillating) burns
//!   near-white; a fading one (released, or stolen) is drawn thinner.
//!   Each filter's latest position ends in a dot.
//! - The analysis's peaks are faint blue-grey ticks, brighter the more
//!   prominent.
//!
//! The history lives on the editor's side: every new frame is placed by
//! the engine's own clock, so the traces scroll at the audio's pace however
//! often the editor draws. A frame older than [`STALE_MS`] dims the
//! traces: with the host stopped, the screen holds what it last showed.

use silkscreen::EditorState;
use silkscreen::EditorWidget;
use silkscreen::core::tiny_skia::{
    Color, FillRule, LineCap, Paint, PathBuilder, PixmapMut, Rect as SkRect, Stroke, Transform,
};
use silkscreen::core::{Rect, WidgetState};

use crate::bank::POOL;
use crate::observe::{Observation, Shown};

/// The frequency axis, Hz.
pub const LOWEST: f32 = 40.0;
pub const HIGHEST: f32 = 16_000.0;
/// How much history the screen holds.
pub const SPAN_SECONDS: f32 = 2.0;
/// A frame this old means the host has stopped.
pub const STALE_MS: f64 = 250.0;
/// Frames kept: more than the span holds at any likely rate.
const HISTORY: usize = 512;
/// Levels are drawn in this many steps, a path each.
const STEPS: usize = 4;
/// Px between the plot and the glass's edges.
const INSET: f32 = 4.0;

const GLASS: (u8, u8, u8) = (0x0b, 0x0d, 0x10);
const GRID: (u8, u8, u8) = (0x1c, 0x20, 0x26);
const DECADE: (u8, u8, u8) = (0x2a, 0x2f, 0x37);
const AMBER: (u8, u8, u8) = (0xf2, 0xa3, 0x3a);
const HOT: (u8, u8, u8) = (0xff, 0xec, 0xc8);
const PEAK: (u8, u8, u8) = (0x7f, 0x9c, 0xbf);

/// Segments of a trace that share a look, as one path.
struct Run {
    colour: (u8, u8, u8),
    alpha: f32,
    width: f32,
    path: PathBuilder,
}

/// One frame, as the display keeps it.
#[derive(Clone, Copy, Debug)]
struct Entry {
    /// Seconds on the engine's clock (unwrapped).
    at: f64,
    obs: Observation,
}

/// The tracking display widget.
#[derive(Debug)]
pub struct TrackDisplay {
    history: Vec<Entry>,
    /// The next slot to write, and how many are filled.
    head: usize,
    filled: usize,
    /// The last frame's generation, its time in samples (modulo 2^24),
    /// and the clock it was placed at.
    seen: Option<u64>,
    last_time: Option<u64>,
    clock: f64,
    /// When the last frame arrived, by the editor's clock.
    arrived_ms: f64,
    live: bool,
}

impl Default for TrackDisplay {
    fn default() -> Self {
        Self {
            history: Vec::with_capacity(HISTORY),
            head: 0,
            filled: 0,
            seen: None,
            last_time: None,
            clock: 0.0,
            arrived_ms: 0.0,
            live: false,
        }
    }
}

/// Where `hz` lands, from 0 at the bottom to 1 at the top.
pub fn height_of(hz: f32) -> f32 {
    ((hz / LOWEST).ln() / (HIGHEST / LOWEST).ln()).clamp(0.0, 1.0)
}

impl TrackDisplay {
    /// Take `frames` as if they had arrived in turn, the last just now:
    /// for tests and stills.
    pub fn with_frames<'a>(frames: impl IntoIterator<Item = &'a [f32]>) -> Self {
        let mut d = Self::default();
        for f in frames {
            if let Some(obs) = Observation::from_frame(f) {
                d.push(&obs);
            }
        }
        d.live = true;
        d
    }

    /// Add a frame, placing it on the engine's clock.
    fn push(&mut self, obs: &Observation) {
        const WRAP: u64 = 1 << 24;
        let rate = f64::from(obs.sample_rate.max(1.0));
        let step = match self.last_time {
            // The time is modulo 2^24 samples; a jump back further than
            // that is a reset, and starts the history again.
            Some(t) if obs.time >= t => (obs.time - t) as f64 / rate,
            Some(t) if t - obs.time > WRAP / 2 => (obs.time + WRAP - t) as f64 / rate,
            Some(_) => {
                self.filled = 0;
                0.0
            }
            None => 0.0,
        };
        self.last_time = Some(obs.time);
        self.clock += step;
        let entry = Entry {
            at: self.clock,
            obs: *obs,
        };
        if self.history.len() < HISTORY {
            self.history.push(entry);
        } else {
            self.history[self.head] = entry;
        }
        self.head = (self.head + 1) % HISTORY;
        self.filled = (self.filled + 1).min(HISTORY);
    }

    /// The kept entries, oldest first.
    fn entries(&self) -> impl Iterator<Item = &Entry> {
        let len = self.history.len();
        let start = (self.head + HISTORY - self.filled) % HISTORY;
        (0..self.filled).map(move |i| &self.history[(start + i) % HISTORY.min(len.max(1))])
    }
}

impl EditorWidget for TrackDisplay {
    fn update(&mut self, state: &EditorState) -> bool {
        let (frame, generation) = state.engine_frame();
        let now = state.now_ms();
        let mut changed = false;
        if generation != 0 && self.seen != Some(generation) {
            self.seen = Some(generation);
            if let Some(obs) = Observation::from_frame(frame) {
                self.push(&obs);
                self.arrived_ms = now;
                changed = true;
            }
        }
        let live = self.filled > 0 && now - self.arrived_ms < STALE_MS;
        changed |= live != self.live;
        self.live = live;
        changed
    }

    fn draw(&self, canvas: &mut PixmapMut<'_>, rect: Rect, t: Transform, _: &WidgetState) {
        if let Some(r) = SkRect::from_xywh(rect.x, rect.y, rect.w, rect.h) {
            canvas.fill_rect(r, &paint(GLASS, 1.0), t, None);
        }
        let plot = Rect::new(
            rect.x + INSET,
            rect.y + INSET,
            rect.w - 2.0 * INSET,
            rect.h - 2.0 * INSET,
        );
        draw_grid(canvas, plot, t);
        if self.filled > 0 {
            draw_history(canvas, plot, t, self, if self.live { 1.0 } else { 0.45 });
        }
    }
}

fn paint(rgb: (u8, u8, u8), alpha: f32) -> Paint<'static> {
    let mut p = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    p.set_color(Color::from_rgba8(rgb.0, rgb.1, rgb.2, a));
    p
}

fn stroke(width: f32) -> Stroke {
    Stroke {
        width,
        line_cap: LineCap::Round,
        ..Stroke::default()
    }
}

fn y_of(plot: Rect, hz: f32) -> f32 {
    plot.bottom() - height_of(hz) * plot.h
}

fn draw_grid(canvas: &mut PixmapMut<'_>, plot: Rect, t: Transform) {
    let mut decade = 10.0f32;
    while decade < HIGHEST {
        for (step, colour) in [(1.0, DECADE), (2.0, GRID), (5.0, GRID)] {
            let hz = decade * step;
            if !(LOWEST..=HIGHEST).contains(&hz) {
                continue;
            }
            let y = y_of(plot, hz);
            let mut pb = PathBuilder::new();
            pb.move_to(plot.x, y);
            pb.line_to(plot.right(), y);
            if let Some(path) = pb.finish() {
                canvas.stroke_path(&path, &paint(colour, 1.0), &stroke(1.0), t, None);
            }
        }
        decade *= 10.0;
    }
}

/// A filter's colour, brightness and weight for its state and level.
fn look(shown: Shown, level: f32) -> Option<((u8, u8, u8), f32, f32)> {
    let level = level.clamp(0.0, 1.0);
    match shown {
        Shown::Free => None,
        Shown::Boosting => Some((AMBER, 0.25 + 0.75 * level, 1.0 + 1.5 * level)),
        Shown::Singing => Some((HOT, 0.6 + 0.4 * level, 1.5 + 1.5 * level)),
        Shown::Fading => Some((AMBER, 0.15 + 0.5 * level, 1.0)),
    }
}

fn draw_history(
    canvas: &mut PixmapMut<'_>,
    plot: Rect,
    xf: Transform,
    display: &TrackDisplay,
    dim: f32,
) {
    let Some(newest) = display.entries().last().map(|e| e.at) else {
        return;
    };
    let x_of = |at: f64| plot.right() - ((newest - at) as f32 / SPAN_SECONDS) * plot.w;
    let shown: Vec<&Entry> = display
        .entries()
        .filter(|e| newest - e.at <= f64::from(SPAN_SECONDS) + 0.1)
        .collect();

    // Peaks: a short tick at each, per frame.
    let mut ticks: [PathBuilder; STEPS] = std::array::from_fn(|_| PathBuilder::new());
    for e in &shown {
        let x = x_of(e.at).max(plot.x);
        for p in &e.obs.peaks[..usize::from(e.obs.peak_count)] {
            if !(LOWEST..=HIGHEST).contains(&p.hz) {
                continue;
            }
            let step = ((p.prominence.clamp(0.0, 1.0) * STEPS as f32) as usize).min(STEPS - 1);
            let y = y_of(plot, p.hz);
            ticks[step].move_to(x, y);
            ticks[step].line_to(x + 1.0, y);
        }
    }
    for (step, pb) in ticks.into_iter().enumerate() {
        if let Some(path) = pb.finish() {
            let alpha = dim * (0.12 + 0.18 * step as f32);
            canvas.stroke_path(&path, &paint(PEAK, alpha), &stroke(1.5), xf, None);
        }
    }

    // Filters: a trace a slot, joined from frame to frame while the slot
    // stays in use, its segments grouped by look into a few paths.
    for slot in 0..POOL {
        let mut runs: Vec<Run> = Vec::new();
        let mut last: Option<(f32, f32)> = None;
        for e in &shown {
            let filter = e.obs.filters[slot];
            let Some((colour, alpha, width)) = look(filter.shown, filter.level) else {
                last = None;
                continue;
            };
            if !(LOWEST..=HIGHEST).contains(&filter.hz) {
                last = None;
                continue;
            }
            let point = (x_of(e.at), y_of(plot, filter.hz));
            if let Some(from) = last {
                // Snap the look to a few steps, so segments share paths.
                let snapped_alpha = (alpha * STEPS as f32).round() / STEPS as f32;
                let snapped_width = (width * 2.0).round() / 2.0;
                let found = runs.iter().position(|r| {
                    r.colour == colour && r.alpha == snapped_alpha && r.width == snapped_width
                });
                let i = found.unwrap_or_else(|| {
                    runs.push(Run {
                        colour,
                        alpha: snapped_alpha,
                        width: snapped_width,
                        path: PathBuilder::new(),
                    });
                    runs.len() - 1
                });
                let path = &mut runs[i].path;
                path.move_to(from.0.max(plot.x), from.1);
                path.line_to(point.0, point.1);
            }
            last = Some(point);
        }
        for run in runs {
            if let Some(path) = run.path.finish() {
                let paint = paint(run.colour, dim * run.alpha);
                canvas.stroke_path(&path, &paint, &stroke(run.width), xf, None);
            }
        }
    }

    // Each filter's latest position, a dot at the right edge.
    if let Some(e) = shown.last() {
        for f in &e.obs.filters {
            let Some((colour, alpha, width)) = look(f.shown, f.level) else {
                continue;
            };
            if !(LOWEST..=HIGHEST).contains(&f.hz) {
                continue;
            }
            if let Some(path) =
                PathBuilder::from_circle(plot.right() - 2.0, y_of(plot, f.hz), 1.0 + width)
            {
                canvas.fill_path(
                    &path,
                    &paint(colour, dim * alpha),
                    FillRule::Winding,
                    xf,
                    None,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observe::{FRAME_WORDS, FilterObs};

    #[test]
    fn the_axis_runs_from_its_lowest_to_its_highest() {
        assert_eq!(height_of(LOWEST), 0.0);
        assert!((height_of(HIGHEST) - 1.0).abs() < 1e-6);
        assert!(height_of(1000.0) > 0.5 && height_of(1000.0) < 0.6);
    }

    fn frame(time: u64) -> Vec<f32> {
        let mut obs = Observation {
            time,
            ..Observation::default()
        };
        obs.filters[0] = FilterObs {
            hz: 880.0,
            level: 0.5,
            shown: Shown::Boosting,
        };
        let mut out = [0.0; FRAME_WORDS];
        obs.to_frame(&mut out);
        out.to_vec()
    }

    #[test]
    fn frames_are_placed_by_the_engines_clock_and_wrap() {
        let near_wrap = (1u64 << 24) - 480;
        let frames = [frame(near_wrap - 480), frame(near_wrap), frame(480)];
        let d = TrackDisplay::with_frames(frames.iter().map(Vec::as_slice));
        let at: Vec<f64> = d.entries().map(|e| e.at).collect();
        assert_eq!(at.len(), 3);
        assert!((at[1] - at[0] - 0.01).abs() < 1e-9, "{at:?}");
        assert!((at[2] - at[1] - 0.02).abs() < 1e-9, "{at:?}");
    }

    #[test]
    fn a_reset_starts_the_history_again() {
        let frames = [frame(48_000), frame(96_000), frame(480)];
        let d = TrackDisplay::with_frames(frames.iter().map(Vec::as_slice));
        assert_eq!(d.entries().count(), 1);
    }

    #[test]
    fn the_history_keeps_its_newest() {
        let frames: Vec<Vec<f32>> = (0..HISTORY as u64 + 50).map(|k| frame(k * 256)).collect();
        let d = TrackDisplay::with_frames(frames.iter().map(Vec::as_slice));
        let times: Vec<u64> = d.entries().map(|e| e.obs.time).collect();
        assert_eq!(times.len(), HISTORY);
        assert_eq!(times[0], 50 * 256);
        assert!(times.windows(2).all(|w| w[0] < w[1]));
    }
}
