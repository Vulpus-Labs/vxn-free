//! The scope (ticket 0009), set into the faceplate's window: an EQ-style
//! plot of what the bank is doing to the input, from the engine's telemetry
//! frames ([`Observation`]).
//!
//! - Frequency runs left to right on a log axis from [`LOWEST`] to
//!   [`HIGHEST`], with a line at each decade's 1, 2 and 5; level up, from
//!   [`DB_BOTTOM`] to [`DB_TOP`] under full scale, a line every 12 dB.
//! - The input's spectrum is a red line over a faint area, so where the
//!   amber output leaves it the boost reads as the input brightened. It
//!   rises at once and
//!   falls at [`FALL_DB_PER_SECOND`], as an analyser's does, timed by the
//!   engine's clock.
//! - The output's is an amber line, drawn where it is [`LEAST_LINE_DB`] or
//!   more off the input's: the
//!   input's, times the output's gain over it ([`Observation::response`]),
//!   every filter's band-pass summed with the direct signal as the engine
//!   sums them.
//! - Between the two, each column is filled with a gradient: amber, bright
//!   at the boosted line and fading towards the input's, so it shows where
//!   each boost lands and how far its skirts spread. Where the output falls
//!   under the input (DRY CUT, or MIX turned down), the gap is a dark red.
//! - Each detected peak is a vertical line at its frequency, brighter the
//!   more prominent; each filter a dot on the output's line at its centre,
//!   near-white while it sings.
//!
//! A frame older than [`STALE_MS`] dims it all: with the host stopped, the
//! screen holds what it last showed.

use silkscreen::EditorState;
use silkscreen::EditorWidget;
use silkscreen::core::tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, PathBuilder,
    PixmapMut, Point, Rect as SkRect, Shader, SpreadMode, Stroke, Transform,
};
use silkscreen::core::{Rect, WidgetState};

pub use crate::observe::{HIGHEST, LOWEST};
use crate::observe::{Observation, SPECTRUM, Shown};

/// The level axis, dB under full scale.
pub const DB_TOP: f32 = 6.0;
pub const DB_BOTTOM: f32 = -78.0;
/// How fast the input's line falls when the input does.
pub const FALL_DB_PER_SECOND: f32 = 60.0;
/// A frame this old means the host has stopped.
pub const STALE_MS: f64 = 250.0;
/// Px between the plot and the glass's edges.
const INSET: f32 = 4.0;
/// A gap between the lines under this many px is not filled.
const LEAST_FILL: f32 = 0.5;
/// The output's line is drawn where it is at least this far off the
/// input's: a dozen filters' skirts lift the midrange by a tenth of a dB,
/// and a line there would hide the input's.
pub const LEAST_LINE_DB: f32 = 0.5;

const GLASS: (u8, u8, u8) = (0x0b, 0x0d, 0x10);
const GRID: (u8, u8, u8) = (0x1a, 0x1e, 0x24);
const DECADE: (u8, u8, u8) = (0x2a, 0x2f, 0x37);
const AMBER: (u8, u8, u8) = (0xf2, 0xa3, 0x3a);
const HOT: (u8, u8, u8) = (0xff, 0xec, 0xc8);
const INPUT: (u8, u8, u8) = (0xd2, 0x3a, 0x2c);
const PEAK: (u8, u8, u8) = (0x7f, 0x9c, 0xbf);
const CUT: (u8, u8, u8) = (0x8a, 0x24, 0x1c);

/// The scope widget.
#[derive(Debug, Default)]
pub struct ScopeDisplay {
    /// The latest frame, if any.
    obs: Option<Observation>,
    /// The input's line as drawn, dB: the frames' spectra with the fall.
    shown: Vec<f32>,
    /// The last frame's generation and time in samples (modulo 2^24).
    seen: Option<u64>,
    last_time: Option<u64>,
    /// When the last frame arrived, by the editor's clock.
    arrived_ms: f64,
    live: bool,
}

/// Where `hz` lands, from 0 at the left to 1 at the right.
pub fn across(hz: f32) -> f32 {
    ((hz / LOWEST).ln() / (HIGHEST / LOWEST).ln()).clamp(0.0, 1.0)
}

/// Where `db` lands, from 0 at the bottom to 1 at the top.
pub fn up(db: f32) -> f32 {
    ((db - DB_BOTTOM) / (DB_TOP - DB_BOTTOM)).clamp(0.0, 1.0)
}

impl ScopeDisplay {
    /// Take `frames` as if they had arrived in turn, the last just now:
    /// for tests and stills.
    pub fn with_frames<'a>(frames: impl IntoIterator<Item = &'a [f32]>) -> Self {
        let mut d = Self::default();
        for f in frames {
            if let Some(obs) = Observation::from_frame(f) {
                d.push(&obs);
            }
        }
        d.live = d.obs.is_some();
        d
    }

    /// The input's line as it would be drawn, dB at each spectrum point.
    pub fn input_line(&self) -> &[f32] {
        &self.shown
    }

    /// Take a frame: the input's line rises to it at once, and falls
    /// toward it at [`FALL_DB_PER_SECOND`] over the time since the last,
    /// by the engine's clock.
    fn push(&mut self, obs: &Observation) {
        const WRAP: u64 = 1 << 24;
        let rate = obs.sample_rate.max(1.0);
        let elapsed = match self.last_time {
            Some(t) if obs.time >= t => Some((obs.time - t) as f32 / rate),
            // Across the 2^24 wrap.
            Some(t) if t - obs.time > WRAP / 2 => Some((obs.time + WRAP - t) as f32 / rate),
            // A reset: start again.
            _ => None,
        };
        self.last_time = Some(obs.time);
        match elapsed {
            Some(dt) if self.shown.len() == SPECTRUM => {
                let fall = FALL_DB_PER_SECOND * dt;
                for (s, &new) in self.shown.iter_mut().zip(&obs.spectrum) {
                    *s = new.max(*s - fall);
                }
            }
            _ => {
                self.shown.clear();
                self.shown.extend_from_slice(&obs.spectrum);
            }
        }
        self.obs = Some(*obs);
    }

    /// The input's line at `hz`, dB: interpolated between spectrum points.
    fn input_at(&self, hz: f32) -> f32 {
        let at = across(hz) * (SPECTRUM - 1) as f32;
        let i = (at.floor() as usize).min(SPECTRUM - 2);
        let t = at - i as f32;
        self.shown[i] + t * (self.shown[i + 1] - self.shown[i])
    }
}

impl EditorWidget for ScopeDisplay {
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
        let live = self.obs.is_some() && now - self.arrived_ms < STALE_MS;
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
        if let Some(obs) = &self.obs {
            draw_scope(
                canvas,
                plot,
                t,
                self,
                obs,
                if self.live { 1.0 } else { 0.45 },
            );
        }
    }
}

fn colour(rgb: (u8, u8, u8), alpha: f32) -> Color {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_rgba8(rgb.0, rgb.1, rgb.2, a)
}

fn paint(rgb: (u8, u8, u8), alpha: f32) -> Paint<'static> {
    let mut p = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    p.set_color(colour(rgb, alpha));
    p
}

fn stroke(width: f32) -> Stroke {
    Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    }
}

fn x_of(plot: Rect, hz: f32) -> f32 {
    plot.x + across(hz) * plot.w
}

fn y_of(plot: Rect, db: f32) -> f32 {
    plot.bottom() - up(db) * plot.h
}

fn line(canvas: &mut PixmapMut<'_>, t: Transform, from: (f32, f32), to: (f32, f32), p: &Paint) {
    let mut pb = PathBuilder::new();
    pb.move_to(from.0, from.1);
    pb.line_to(to.0, to.1);
    if let Some(path) = pb.finish() {
        canvas.stroke_path(&path, p, &stroke(1.0), t, None);
    }
}

fn draw_grid(canvas: &mut PixmapMut<'_>, plot: Rect, t: Transform) {
    let mut decade = 10.0f32;
    while decade < HIGHEST {
        for (step, rgb) in [(1.0, DECADE), (2.0, GRID), (5.0, GRID)] {
            let hz = decade * step;
            if !(LOWEST..=HIGHEST).contains(&hz) {
                continue;
            }
            let x = x_of(plot, hz);
            line(canvas, t, (x, plot.y), (x, plot.bottom()), &paint(rgb, 1.0));
        }
        decade *= 10.0;
    }
    let mut db = 0.0f32;
    while db > DB_BOTTOM {
        let y = y_of(plot, db);
        let rgb = if db == 0.0 { DECADE } else { GRID };
        line(canvas, t, (plot.x, y), (plot.right(), y), &paint(rgb, 1.0));
        db -= 12.0;
    }
}

/// A filter's dot: colour, brightness and radius for its state and level.
fn look(shown: Shown, level: f32) -> Option<((u8, u8, u8), f32, f32)> {
    let level = level.clamp(0.0, 1.0);
    match shown {
        Shown::Free => None,
        Shown::Boosting => Some((AMBER, 0.35 + 0.65 * level, 1.5 + 1.0 * level)),
        Shown::Singing => Some((HOT, 0.7 + 0.3 * level, 2.5 + 1.0 * level)),
        Shown::Fading => Some((AMBER, 0.2 + 0.4 * level, 1.5)),
    }
}

/// A column `width` wide at `left`, from `top` to `bottom`, filled with
/// `rgb` at alpha `bright` along its top edge fading to `faint` at its
/// bottom.
fn gradient_column(
    canvas: &mut PixmapMut<'_>,
    xf: Transform,
    (left, width): (f32, f32),
    (top, bottom): (f32, f32),
    rgb: (u8, u8, u8),
    (bright, faint): (f32, f32),
) {
    let Some(rect) = SkRect::from_xywh(left, top, width, bottom - top) else {
        return;
    };
    let shader = LinearGradient::new(
        Point::from_xy(left, top),
        Point::from_xy(left, bottom),
        vec![
            GradientStop::new(0.0, colour(rgb, bright)),
            GradientStop::new(1.0, colour(rgb, faint)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    )
    .unwrap_or(Shader::SolidColor(colour(rgb, 0.5 * (bright + faint))));
    let fill = Paint {
        shader,
        anti_alias: false,
        ..Paint::default()
    };
    canvas.fill_rect(rect, &fill, xf, None);
}

fn draw_scope(
    canvas: &mut PixmapMut<'_>,
    plot: Rect,
    t: Transform,
    scope: &ScopeDisplay,
    obs: &Observation,
    dim: f32,
) {
    // The peaks: a vertical line at each, brighter with prominence.
    for p in &obs.peaks[..usize::from(obs.peak_count)] {
        if !(LOWEST..=HIGHEST).contains(&p.hz) {
            continue;
        }
        let x = x_of(plot, p.hz);
        let alpha = dim * (0.15 + 0.45 * p.prominence.clamp(0.0, 1.0));
        line(
            canvas,
            t,
            (x, plot.y),
            (x, plot.bottom()),
            &paint(PEAK, alpha),
        );
    }

    // Each column's input and output, px.
    let columns = plot.w.ceil().max(1.0) as usize;
    let points: Vec<(f32, f32, f32)> = (0..=columns)
        .map(|c| {
            let x = plot.x + (c as f32).min(plot.w);
            let hz = LOWEST * (HIGHEST / LOWEST).powf((x - plot.x) / plot.w);
            let db_in = scope.input_at(hz);
            let db_out = db_in + 20.0 * obs.response(hz).max(1e-6).log10();
            (x, y_of(plot, db_in), y_of(plot, db_out))
        })
        .collect();

    // The input: a faint area under a pale line.
    let mut area = PathBuilder::new();
    let mut input = PathBuilder::new();
    area.move_to(plot.x, plot.bottom());
    for (c, &(x, y_in, _)) in points.iter().enumerate() {
        area.line_to(x, y_in);
        if c == 0 {
            input.move_to(x, y_in);
        } else {
            input.line_to(x, y_in);
        }
    }
    area.line_to(plot.right(), plot.bottom());
    area.close();
    if let Some(path) = area.finish() {
        canvas.fill_path(&path, &paint(INPUT, dim * 0.07), FillRule::Winding, t, None);
    }

    // The gap between the lines, a column at a time.
    for pair in points.windows(2) {
        let ((x, y_in, y_out), (next, _, _)) = (pair[0], pair[1]);
        let span = (x, next - x);
        if y_in - y_out > LEAST_FILL {
            // A boost: bright at the boosted line, fading to the input.
            gradient_column(canvas, t, span, (y_out, y_in), AMBER, (dim, dim * 0.2));
        } else if y_out - y_in > LEAST_FILL {
            // A cut: the input over the output, fading down to it.
            gradient_column(canvas, t, span, (y_in, y_out), CUT, (dim * 0.4, dim * 0.05));
        }
    }
    if let Some(path) = input.finish() {
        canvas.stroke_path(&path, &paint(INPUT, dim * 0.9), &stroke(1.0), t, None);
    }

    // The output's line, only where it leaves the input's, from the column
    // before to the column after so it joins it.
    let least = LEAST_LINE_DB / (DB_TOP - DB_BOTTOM) * plot.h;
    let apart = |&(_, y_in, y_out): &(f32, f32, f32)| (y_in - y_out).abs() > least;
    let mut output = PathBuilder::new();
    let mut drawing = false;
    for (c, p) in points.iter().enumerate() {
        let near = apart(p)
            || points.get(c + 1).is_some_and(apart)
            || c.checked_sub(1)
                .and_then(|b| points.get(b))
                .is_some_and(apart);
        if near {
            if drawing {
                output.line_to(p.0, p.2);
            } else {
                output.move_to(p.0, p.2);
            }
        }
        drawing = near;
    }
    if let Some(path) = output.finish() {
        canvas.stroke_path(&path, &paint(AMBER, dim), &stroke(1.5), t, None);
    }

    // Each filter, a dot on the output's line at its centre.
    for f in &obs.filters {
        let Some((rgb, alpha, radius)) = look(f.shown, f.level) else {
            continue;
        };
        if !(LOWEST..=HIGHEST).contains(&f.hz) {
            continue;
        }
        let db = scope.input_at(f.hz) + 20.0 * obs.response(f.hz).max(1e-6).log10();
        if let Some(path) = PathBuilder::from_circle(x_of(plot, f.hz), y_of(plot, db), radius) {
            canvas.fill_path(&path, &paint(rgb, dim * alpha), FillRule::Winding, t, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observe::FRAME_WORDS;

    #[test]
    fn the_axes_run_from_their_ends() {
        assert_eq!(across(LOWEST), 0.0);
        assert!((across(HIGHEST) - 1.0).abs() < 1e-6);
        assert!(across(1000.0) > 0.5 && across(1000.0) < 0.6);
        assert_eq!(up(DB_BOTTOM), 0.0);
        assert_eq!(up(DB_TOP), 1.0);
    }

    fn frame(time: u64, db: f32) -> Vec<f32> {
        let mut obs = Observation {
            time,
            ..Observation::default()
        };
        obs.spectrum.fill(db);
        let mut out = [0.0; FRAME_WORDS];
        obs.to_frame(&mut out);
        out.to_vec()
    }

    #[test]
    fn the_input_rises_at_once_and_falls_by_the_engines_clock() {
        // Up at once; then 0.1 s later, 6 dB down.
        let frames = [frame(0, -60.0), frame(480, -20.0), frame(5280, -80.0)];
        let d = ScopeDisplay::with_frames(frames[..2].iter().map(Vec::as_slice));
        assert!(d.input_line().iter().all(|&v| v == -20.0));
        let d = ScopeDisplay::with_frames(frames.iter().map(Vec::as_slice));
        assert!(
            d.input_line().iter().all(|&v| (v + 26.0).abs() < 1e-3),
            "{:?}",
            &d.input_line()[..4]
        );
    }

    #[test]
    fn the_fall_is_timed_across_the_wrap_and_a_reset_starts_again() {
        let near_wrap = (1u64 << 24) - 2400;
        let frames = [frame(near_wrap, -20.0), frame(2400, -80.0)];
        let d = ScopeDisplay::with_frames(frames.iter().map(Vec::as_slice));
        // 0.1 s across the wrap: 6 dB.
        assert!(
            (d.input_line()[0] + 26.0).abs() < 1e-3,
            "{}",
            d.input_line()[0]
        );
        let frames = [frame(96_000, -20.0), frame(480, -80.0)];
        let d = ScopeDisplay::with_frames(frames.iter().map(Vec::as_slice));
        assert_eq!(d.input_line()[0], -80.0);
    }
}
