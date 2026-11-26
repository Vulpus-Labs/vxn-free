//! The tracking display (ticket 0009), set into the faceplate's window:
//! time runs left to right, frequency up a log axis from 40 Hz to 16 kHz.
//! For now it draws the screen and its grid; the engine's telemetry (the
//! peaks, and the filters' centres and levels) is drawn over them next.

use silkscreen::EditorState;
use silkscreen::EditorWidget;
use silkscreen::core::tiny_skia::{
    Color, Paint, PathBuilder, PixmapMut, Rect as SkRect, Stroke, Transform,
};
use silkscreen::core::{Rect, WidgetState};

/// The frequency axis, Hz.
pub const LOWEST: f32 = 40.0;
pub const HIGHEST: f32 = 16_000.0;

const GLASS: (u8, u8, u8) = (0x0b, 0x0d, 0x10);
const GRID: (u8, u8, u8) = (0x1c, 0x20, 0x26);
const OCTAVE: (u8, u8, u8) = (0x2a, 0x2f, 0x37);

/// The tracking display widget.
#[derive(Debug, Default)]
pub struct TrackDisplay;

/// Where `hz` lands, from 0 at the bottom to 1 at the top.
pub fn height_of(hz: f32) -> f32 {
    ((hz / LOWEST).ln() / (HIGHEST / LOWEST).ln()).clamp(0.0, 1.0)
}

fn paint(rgb: (u8, u8, u8)) -> Paint<'static> {
    let mut p = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    p.set_color(Color::from_rgba8(rgb.0, rgb.1, rgb.2, 255));
    p
}

impl EditorWidget for TrackDisplay {
    fn update(&mut self, _state: &EditorState) -> bool {
        false
    }

    fn draw(&self, canvas: &mut PixmapMut<'_>, rect: Rect, t: Transform, _: &WidgetState) {
        if let Some(r) = SkRect::from_xywh(rect.x, rect.y, rect.w, rect.h) {
            canvas.fill_rect(r, &paint(GLASS), t, None);
        }
        // A line at each decade's 1, 2 and 5, the decades brighter.
        let mut decade = 10.0f32;
        while decade < HIGHEST {
            for (step, colour) in [(1.0, OCTAVE), (2.0, GRID), (5.0, GRID)] {
                let hz = decade * step;
                if !(LOWEST..=HIGHEST).contains(&hz) {
                    continue;
                }
                let y = rect.bottom() - height_of(hz) * rect.h;
                let mut pb = PathBuilder::new();
                pb.move_to(rect.x, y);
                pb.line_to(rect.right(), y);
                if let Some(path) = pb.finish() {
                    let stroke = Stroke {
                        width: 1.0,
                        ..Stroke::default()
                    };
                    canvas.stroke_path(&path, &paint(colour), &stroke, t, None);
                }
            }
            decade *= 10.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_axis_runs_from_its_lowest_to_its_highest() {
        assert_eq!(height_of(LOWEST), 0.0);
        assert!((height_of(HIGHEST) - 1.0).abs() < 1e-6);
        assert!(height_of(1000.0) > 0.5 && height_of(1000.0) < 0.6);
    }
}
