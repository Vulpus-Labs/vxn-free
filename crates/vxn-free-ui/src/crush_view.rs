//! The crush display (ticket 0038): what one crush chain's settings do to
//! one cycle of a sine, drawn over the sine itself.
//!
//! It reads the settings from the model, not the engine, so it moves with
//! the knobs whether or not audio is playing, and runs the engine's own
//! crush (`vxn_free_dsp::crush`) on them.
//!
//! Which sine: with AUTO GAIN on, the chain sees any signal at full scale,
//! so the sine is at full scale. With it off, the result depends on the
//! input's level, which the display can't know; it takes a sine at
//! [`FIXED_GAIN_REFERENCE`] (−6 dBFS) before GAIN, so 0 dB uses half the
//! quantiser's range, +6 dB all of it, and more clips. Both curves are
//! drawn relative to that sine's peak: the sine fills the height, and the
//! output shows its shape and level against it (a clipped sine comes out
//! quieter, as Beverley's gain is undone after the crush).

use silkscreen::core::tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Rect as SkRect, Stroke,
    Transform,
};
use silkscreen::core::{Rect, WidgetState};
use silkscreen::{EditorState, EditorWidget, Model, ParamIndex};
use vxn_free_dsp::crush::{Mode, Prepared, Settings};

/// The sine's peak with AUTO GAIN off, before GAIN: −6 dBFS.
pub const FIXED_GAIN_REFERENCE: f32 = 0.5;

/// One chain's params, as its plugin numbers them, in Beverley's units:
/// GAIN dB, AUTO GAIN and CRUSH MODE switches, GAMMA's knob (−1 to 1),
/// BIT DEPTH bits, SMOOTHING and MIX %.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainParams {
    pub gain: ParamIndex,
    pub auto_gain: ParamIndex,
    pub gamma: ParamIndex,
    pub bit_depth: ParamIndex,
    pub smoothing: ParamIndex,
    pub crush_mode: ParamIndex,
    pub mix: ParamIndex,
}

impl ChainParams {
    fn values(&self, model: &Model) -> [f32; 7] {
        [
            self.gain,
            self.auto_gain,
            self.gamma,
            self.bit_depth,
            self.smoothing,
            self.crush_mode,
            self.mix,
        ]
        .map(|p| model.get(p))
    }
}

/// The settings' values, as [`ChainParams`] reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainValues {
    pub gain_db: f32,
    pub auto_gain: bool,
    pub gamma_knob: f32,
    pub bits: f32,
    pub smoothing_pct: f32,
    pub symmetric: bool,
    pub mix_pct: f32,
}

impl ChainValues {
    fn from_array(v: [f32; 7]) -> Self {
        Self {
            gain_db: v[0],
            auto_gain: v[1] >= 0.5,
            gamma_knob: v[2],
            bits: v[3],
            smoothing_pct: v[4],
            symmetric: v[5] >= 0.5,
            mix_pct: v[6],
        }
    }
}

/// One cycle of the sine and its crushed output at `n` evenly spaced
/// points, each relative to the sine's peak: `(sine, output)`.
pub fn crush_cycle(v: &ChainValues, n: usize) -> Vec<(f32, f32)> {
    let (peak, gain) = if v.auto_gain {
        (1.0, 1.0)
    } else {
        (FIXED_GAIN_REFERENCE, 10f32.powf(v.gain_db / 20.0))
    };
    let prepared = Prepared::new(&Settings {
        gain,
        gamma: 10f32.powf(v.gamma_knob),
        bits: v.bits,
        smoothing: v.smoothing_pct * 0.01,
        mode: if v.symmetric {
            Mode::Symmetric
        } else {
            Mode::Asymmetric
        },
    });
    let mix = (v.mix_pct * 0.01).clamp(0.0, 1.0);
    (0..n)
        .map(|i| {
            let s = (std::f32::consts::TAU * (i as f32 + 0.5) / n as f32).sin();
            let x = peak * s;
            let crushed = prepared.crush(x, gain);
            let out = crushed * mix + x * (1.0 - mix);
            (s, out / peak)
        })
        .collect()
}

/// How a [`CrushView`] looks. Colours are RGB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrushStyle {
    pub background: (u8, u8, u8),
    /// The centre line and frame.
    pub grid: (u8, u8, u8),
    /// The sine, drawn faint under the output.
    pub sine: (u8, u8, u8),
    pub sine_alpha: f32,
    /// The output.
    pub curve: (u8, u8, u8),
    pub curve_width: f32,
    /// Space between the rect's edge and the plot, logical px.
    pub inset: f32,
}

impl CrushStyle {
    /// A dark well with an amber trace: for plain faceplates.
    pub const WELL: CrushStyle = CrushStyle {
        background: (0x0e, 0x0d, 0x0c),
        grid: (0x2c, 0x29, 0x25),
        sine: (0xb8, 0xae, 0x9e),
        sine_alpha: 0.35,
        curve: (0xf0, 0xa3, 0x40),
        curve_width: 1.6,
        inset: 6.0,
    };
}

/// The display widget for one chain.
#[derive(Debug)]
pub struct CrushView {
    params: ChainParams,
    style: CrushStyle,
    values: Option<[f32; 7]>,
}

impl CrushView {
    pub fn new(params: ChainParams, style: CrushStyle) -> Self {
        Self {
            params,
            style,
            values: None,
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

impl EditorWidget for CrushView {
    fn update(&mut self, state: &EditorState) -> bool {
        let now = self.params.values(state.model());
        let changed = self
            .values
            .is_none_or(|v| v.map(f32::to_bits) != now.map(f32::to_bits));
        self.values = Some(now);
        changed
    }

    fn draw(&self, canvas: &mut PixmapMut<'_>, rect: Rect, xf: Transform, _: &WidgetState) {
        let st = &self.style;
        if let Some(r) = SkRect::from_xywh(rect.x, rect.y, rect.w, rect.h) {
            let mut pb = PathBuilder::new();
            pb.push_rect(r);
            if let Some(path) = pb.finish() {
                canvas.fill_path(
                    &path,
                    &paint(st.background, 1.0),
                    FillRule::Winding,
                    xf,
                    None,
                );
            }
        }
        let Some(values) = self.values else {
            return;
        };
        let (x0, y0) = (rect.x + st.inset, rect.y + st.inset);
        let (width_px, height_px) = (rect.w - 2.0 * st.inset, rect.h - 2.0 * st.inset);
        let mid = y0 + 0.5 * height_px;
        let stroke = |width| Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        let mut pb = PathBuilder::new();
        pb.move_to(x0, mid);
        pb.line_to(x0 + width_px, mid);
        if let Some(path) = pb.finish() {
            canvas.stroke_path(&path, &paint(st.grid, 1.0), &stroke(1.0), xf, None);
        }
        // One point a logical pixel column.
        let n = width_px.max(2.0) as usize;
        let cycle = crush_cycle(&ChainValues::from_array(values), n);
        let at = |i: usize, v: f32| {
            (
                x0 + width_px * i as f32 / (n - 1) as f32,
                mid - 0.5 * height_px * v.clamp(-1.0, 1.0),
            )
        };
        for (which, colour, alpha, width) in [
            (0, st.sine, st.sine_alpha, 1.0),
            (1, st.curve, 1.0, st.curve_width),
        ] {
            let mut pb = PathBuilder::new();
            for (i, &(s, out)) in cycle.iter().enumerate() {
                let (x, y) = at(i, if which == 0 { s } else { out });
                if i == 0 {
                    pb.move_to(x, y);
                } else {
                    pb.line_to(x, y);
                }
            }
            if let Some(path) = pb.finish() {
                canvas.stroke_path(&path, &paint(colour, alpha), &stroke(width), xf, None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> ChainValues {
        ChainValues {
            gain_db: 0.0,
            auto_gain: true,
            gamma_knob: 0.0,
            bits: 4.0,
            smoothing_pct: 0.0,
            symmetric: false,
            mix_pct: 100.0,
        }
    }

    #[test]
    fn the_curve_is_the_engines_crush() {
        let v = ChainValues {
            auto_gain: false,
            gain_db: 3.0,
            bits: 2.5,
            gamma_knob: 0.3,
            smoothing_pct: 20.0,
            ..defaults()
        };
        let g = 10f32.powf(3.0 / 20.0);
        let p = Prepared::new(&Settings {
            gain: g,
            gamma: 10f32.powf(0.3),
            bits: 2.5,
            smoothing: 0.2,
            mode: Mode::Asymmetric,
        });
        for (i, (s, out)) in crush_cycle(&v, 64).into_iter().enumerate() {
            let x = FIXED_GAIN_REFERENCE * (std::f32::consts::TAU * (i as f32 + 0.5) / 64.0).sin();
            assert_eq!(s * FIXED_GAIN_REFERENCE, x);
            assert_eq!(out, p.crush(x, g) / FIXED_GAIN_REFERENCE, "point {i}");
        }
    }

    #[test]
    fn every_setting_changes_the_curve() {
        let base = crush_cycle(&defaults(), 128);
        let changed = [
            ChainValues {
                auto_gain: false,
                ..defaults()
            },
            ChainValues {
                gamma_knob: 0.5,
                ..defaults()
            },
            ChainValues {
                bits: 3.0,
                ..defaults()
            },
            ChainValues {
                smoothing_pct: 50.0,
                ..defaults()
            },
            ChainValues {
                symmetric: true,
                ..defaults()
            },
            ChainValues {
                mix_pct: 50.0,
                ..defaults()
            },
        ];
        for v in changed {
            assert_ne!(crush_cycle(&v, 128), base, "{v:?}");
        }
        // GAIN acts only with AUTO GAIN off.
        let fixed = ChainValues {
            auto_gain: false,
            ..defaults()
        };
        assert_ne!(
            crush_cycle(
                &ChainValues {
                    gain_db: 12.0,
                    ..fixed
                },
                128
            ),
            crush_cycle(&fixed, 128)
        );
        assert_eq!(
            crush_cycle(
                &ChainValues {
                    gain_db: 12.0,
                    ..defaults()
                },
                128
            ),
            base
        );
    }

    #[test]
    fn full_depth_and_no_mix_are_the_sine() {
        let dry = crush_cycle(
            &ChainValues {
                mix_pct: 0.0,
                ..defaults()
            },
            64,
        );
        assert!(dry.iter().all(|(s, o)| (s - o).abs() < 1e-6));
        let fine = crush_cycle(
            &ChainValues {
                bits: 16.0,
                ..defaults()
            },
            64,
        );
        assert!(fine.iter().all(|(s, o)| (s - o).abs() < 1e-3));
    }

    #[test]
    fn driving_past_full_scale_clips_and_comes_out_quieter() {
        // −6 dBFS with +18 dB is 4× full scale: clipped at a quarter of
        // the sine's peak, relative to it.
        let v = ChainValues {
            auto_gain: false,
            gain_db: 18.06,
            bits: 16.0,
            ..defaults()
        };
        let top = crush_cycle(&v, 256)
            .iter()
            .map(|&(_, o)| o)
            .fold(0.0f32, f32::max);
        assert!((top - 0.25).abs() < 0.01, "{top}");
    }
}
