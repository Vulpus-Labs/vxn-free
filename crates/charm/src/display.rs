//! The flock display (ADR 0002, point 10; ticket 0019): what the flocks
//! are doing, drawn from the engine's telemetry frame.
//!
//! - Horizontal: pan, hard left to hard right. Each band's AUTO WIDTH
//!   walls (WIDTH times AUTO WIDTH's share) are drawn in its colour, and
//!   the mono guard as faint lines inside them.
//! - Vertical: delay, from the floor at the bottom ("front of the stage")
//!   to the floor plus SPREAD at the top.
//! - Each voice is a dot, MID gold and HIGH teal. Its tail points back
//!   along the delay axis, as long as its velocity's share of DETUNE's
//!   speed limit: how far out of tune it is.
//! - Duck and Bloom scale each dot's size and brightness where the voice
//!   reads, so an onset shows as a pulse moving up through the flock.
//!
//! Only whole frames reach the editor (silkscreen's telemetry frames), and
//! a frame older than [`STALE_MS`] isn't drawn: with the host stopped, the
//! flock goes dark rather than freezing.

use silkscreen::EditorState;
use silkscreen::EditorWidget;
use silkscreen::core::tiny_skia::{
    Color, FillRule, LineCap, Paint, PathBuilder, PixmapMut, Rect as SkRect, Stroke, Transform,
};
use silkscreen::core::{Rect, WidgetState};

use crate::engine::PAN_GUARD;
use crate::flock::FLOOR;
use crate::observe::{MID, Observation};

/// A frame this old is out of date and not drawn, ms.
pub const STALE_MS: f64 = 250.0;

/// Gaps between the well's edge and the plot, logical px.
const INSET_X: f32 = 14.0;
const INSET_Y: f32 = 12.0;
/// A dot's radius between onsets, and its tail at the speed limit.
const DOT: f32 = 4.5;
const TAIL: f32 = 26.0;

/// MID's and HIGH's colours (r, g, b).
const COLOURS: [(u8, u8, u8); 2] = [(0xe8, 0xbe, 0x5e), (0x6c, 0xcf, 0xd6)];
const WELL: (u8, u8, u8) = (0x0b, 0x0d, 0x12);
const GRID: (u8, u8, u8) = (0x23, 0x27, 0x32);

/// The flock display widget.
#[derive(Debug, Default)]
pub struct FlockDisplay {
    /// The latest frame, decoded, and the generation it came with.
    obs: Option<Observation>,
    seen: Option<u64>,
    /// When the latest frame arrived, by the editor's clock.
    arrived_ms: f64,
    /// Whether the last draw showed the flock (not out of date).
    live: bool,
}

impl FlockDisplay {
    /// Take `frame` as if it had just arrived: for tests and stills.
    pub fn with_frame(frame: &[f32]) -> Self {
        Self {
            obs: Observation::from_frame(frame),
            seen: None,
            arrived_ms: 0.0,
            live: true,
        }
    }
}

impl EditorWidget for FlockDisplay {
    fn update(&mut self, state: &EditorState) -> bool {
        let (frame, generation) = state.engine_frame();
        let now = state.now_ms();
        let mut changed = false;
        if generation != 0 && self.seen != Some(generation) {
            self.seen = Some(generation);
            self.obs = Observation::from_frame(frame);
            self.arrived_ms = now;
            changed = true;
        }
        let live = self.obs.is_some() && now - self.arrived_ms < STALE_MS;
        changed |= live != self.live;
        self.live = live;
        changed
    }

    fn draw(&self, canvas: &mut PixmapMut<'_>, rect: Rect, transform: Transform, _: &WidgetState) {
        draw_well(canvas, rect, transform);
        if !self.live {
            return;
        }
        if let Some(obs) = &self.obs {
            draw_flock(canvas, rect, transform, obs);
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

fn line(
    canvas: &mut PixmapMut<'_>,
    t: Transform,
    from: (f32, f32),
    to: (f32, f32),
    p: &Paint,
    w: f32,
) {
    let mut pb = PathBuilder::new();
    pb.move_to(from.0, from.1);
    pb.line_to(to.0, to.1);
    if let Some(path) = pb.finish() {
        let stroke = Stroke {
            width: w,
            line_cap: LineCap::Round,
            ..Stroke::default()
        };
        canvas.stroke_path(&path, p, &stroke, t, None);
    }
}

/// The plot's area inside the well.
fn plot(rect: Rect) -> Rect {
    Rect::new(
        rect.x + INSET_X,
        rect.y + INSET_Y,
        rect.w - 2.0 * INSET_X,
        rect.h - 2.0 * INSET_Y,
    )
}

/// The well and its grid: the centre line, the floor, quarter pans.
fn draw_well(canvas: &mut PixmapMut<'_>, rect: Rect, t: Transform) {
    if let Some(r) = SkRect::from_xywh(rect.x, rect.y, rect.w, rect.h) {
        let mut pb = PathBuilder::new();
        pb.push_rect(r);
        if let Some(path) = pb.finish() {
            canvas.fill_path(&path, &paint(WELL, 1.0), FillRule::Winding, t, None);
        }
    }
    let p = plot(rect);
    let grid = paint(GRID, 1.0);
    for k in [-0.5, 0.5] {
        let x = p.x + p.w * (0.5 + 0.5 * k);
        line(canvas, t, (x, p.y), (x, p.bottom()), &grid, 0.75);
    }
    let cx = p.x + 0.5 * p.w;
    line(
        canvas,
        t,
        (cx, p.y),
        (cx, p.bottom()),
        &paint(GRID, 1.0),
        1.25,
    );
    line(
        canvas,
        t,
        (p.x, p.bottom()),
        (p.right(), p.bottom()),
        &paint(GRID, 1.0),
        1.25,
    );
}

/// Where a pan (−1 to 1) and delay (seconds) land in the plot.
fn place(p: Rect, obs: &Observation, pan: f32, delay: f32) -> (f32, f32) {
    let x = p.x + p.w * 0.5 * (1.0 + pan.clamp(-1.0, 1.0));
    let up = if obs.spread > 0.0 {
        ((delay - FLOOR as f32) / obs.spread).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (x, p.bottom() - up * p.h)
}

fn draw_flock(canvas: &mut PixmapMut<'_>, rect: Rect, xf: Transform, obs: &Observation) {
    let area = plot(rect);
    // Walls first, then the guard, then every band's voices, HIGH over MID.
    for (b, band) in obs.bands.iter().enumerate() {
        let wall = paint(COLOURS[b], 0.45);
        // MID's walls run the top half, HIGH's the bottom, so equal walls
        // both show.
        let (y0, y1) = if b == MID {
            (area.y, area.y + 0.5 * area.h)
        } else {
            (area.y + 0.5 * area.h, area.bottom())
        };
        for side in [-1.0, 1.0] {
            let x = place(area, obs, side * band.width, 0.0).0;
            line(canvas, xf, (x, y0), (x, y1), &wall, 1.5);
        }
    }
    let guard = paint(GRID, 1.0);
    for side in [-1.0, 1.0] {
        let x = place(area, obs, side * PAN_GUARD, 0.0).0;
        let mut y = area.y;
        while y < area.bottom() {
            line(
                canvas,
                xf,
                (x, y),
                (x, (y + 3.0).min(area.bottom())),
                &guard,
                1.0,
            );
            y += 7.0;
        }
    }
    for (b, band) in obs.bands.iter().enumerate() {
        // Each voice's level is 1/√VOICES at full: scale it back to 0–1,
        // so a voice fading in or out fades on screen.
        let full = (f32::from(band.active.max(1))).sqrt();
        for v in &band.voices {
            let presence = (v.gain * full).clamp(0.0, 1.0);
            if presence <= 0.0 {
                continue;
            }
            let shape = v.shape.clamp(0.0, 1.5);
            let (x, y) = place(area, obs, v.pan, v.delay);
            let speed = if obs.max_velocity > 0.0 {
                (v.velocity / obs.max_velocity).clamp(-1.0, 1.0)
            } else {
                0.0
            };
            // A growing delay climbs the plot, so its tail hangs below it.
            let tail = paint(COLOURS[b], 0.45 * presence * (0.4 + 0.6 * shape));
            line(canvas, xf, (x, y), (x, y + speed * TAIL), &tail, 2.0);
            let r = DOT * (0.55 + 0.45 * shape);
            let dot = paint(COLOURS[b], presence * (0.3 + 0.7 * shape.min(1.0)));
            if let Some(path) = PathBuilder::from_circle(x, y, r) {
                canvas.fill_path(&path, &dot, FillRule::Winding, xf, None);
            }
        }
    }
}
