//! Glow-Up: a targeted harmonic filter (ADR 0005). Its parameters and its
//! engine.
//!
//! A running spectrum finds the input's peaks; a bank of narrow band-pass
//! filters boosts what is already there at two ratios of each peak, boost
//! A's and boost B's, each a NUMERATOR over a DENOMINATOR. FOCUS blends
//! resonances fixed on a grid with filters that track each peak exactly,
//! and RESONANCE takes them up to self-oscillation. The audio never passes
//! through the analysis, so there is no latency.

use silkscreen::ParamIndex;
use silkscreen::model::{ParamSpec, Taper};

pub mod bank;
pub mod engine;
pub mod grids;

pub const A_AMOUNT: ParamIndex = ParamIndex(0);
pub const A_NUM: ParamIndex = ParamIndex(1);
pub const A_DEN: ParamIndex = ParamIndex(2);
pub const B_AMOUNT: ParamIndex = ParamIndex(3);
pub const B_NUM: ParamIndex = ParamIndex(4);
pub const B_DEN: ParamIndex = ParamIndex(5);
pub const WIDTH: ParamIndex = ParamIndex(6);
pub const RESONANCE: ParamIndex = ParamIndex(7);
pub const FOCUS: ParamIndex = ParamIndex(8);
pub const MIX: ParamIndex = ParamIndex(9);
pub const WINDOW: ParamIndex = ParamIndex(10);
pub const THRESHOLD: ParamIndex = ParamIndex(11);
pub const TONE: ParamIndex = ParamIndex(12);
pub const GRID: ParamIndex = ParamIndex(13);
pub const SPEED: ParamIndex = ParamIndex(14);
pub const DRY_CUT: ParamIndex = ParamIndex(15);
pub const MODE: ParamIndex = ParamIndex(16);

/// Each boost's params: AMOUNT, NUM, DEN.
pub const BOOSTS: [[ParamIndex; 3]; 2] = [[A_AMOUNT, A_NUM, A_DEN], [B_AMOUNT, B_NUM, B_DEN]];

/// NUM's and DEN's positions: position `k` is `k + 1`.
const STEPS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8"];

/// WINDOW's positions, in samples.
pub const WINDOWS: [usize; 3] = [512, 1024, 2048];

/// GRID's positions.
pub const GRID_LIN: usize = 0;
pub const GRID_IRR: usize = 1;
pub const GRID_TET: usize = 2;

/// WIDTH's range, cents.
pub const WIDTH_MIN: f32 = 10.0;
pub const WIDTH_MAX: f32 = 200.0;

/// The parameter table: index `i` is `ParamIndex(i)`. Each `id` is what
/// presets and host projects store, so never change or reuse one once
/// released. Ranges and defaults are drafts until tuned by ear (ADR 0005,
/// point 10).
pub static PARAMS: [ParamSpec; 17] = [
    ParamSpec::new("A Amount", 0.0, 100.0, 25.0)
        .unit("%")
        .decimals(0)
        .id(1),
    ParamSpec::stepped("A Numerator", STEPS, 1).id(2),
    ParamSpec::stepped("A Denominator", STEPS, 0).id(3),
    ParamSpec::new("B Amount", 0.0, 100.0, 25.0)
        .unit("%")
        .decimals(0)
        .id(4),
    ParamSpec::stepped("B Numerator", STEPS, 2).id(5),
    ParamSpec::stepped("B Denominator", STEPS, 1).id(6),
    // Each boost's bandwidth, the same in cents at every ratio. Half
    // travel is the geometric middle of the range.
    ParamSpec::new("Width", WIDTH_MIN, WIDTH_MAX, 50.0)
        .taper(Taper::Exp { mid: 44.72136 })
        .unit("cents")
        .decimals(0)
        .id(7),
    // Past half way a strong peak makes its filter sing.
    ParamSpec::new("Resonance", 0.0, 100.0, 0.0)
        .unit("%")
        .decimals(0)
        .id(8),
    // 0 is GRID (resonances that stay put), 100 is TRACK.
    ParamSpec::new("Focus", 0.0, 100.0, 0.0)
        .unit("%")
        .decimals(0)
        .id(9),
    ParamSpec::new("Mix", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(10),
    ParamSpec::stepped("Window", &["512", "1024", "2048"], 2).id(11),
    ParamSpec::new("Threshold", 0.0, 30.0, 6.0)
        .unit("dB")
        .decimals(1)
        .id(12),
    // A tilt of the boosts by frequency about 1 kHz.
    ParamSpec::new("Tone", -6.0, 6.0, 0.0)
        .bipolar()
        .unit("dB/oct")
        .decimals(1)
        .id(13),
    ParamSpec::stepped("Grid", &["LIN", "IRR", "TET"], GRID_IRR).id(14),
    // Slow and smooth (0) to fast and twitchy (100).
    ParamSpec::new("Speed", 0.0, 100.0, 80.0)
        .unit("%")
        .decimals(0)
        .id(15),
    ParamSpec::toggle("Dry Cut", false).id(16),
    ParamSpec::stepped("Mode", &["Mono", "Stereo"], 0).id(17),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_in_order() {
        for (i, p) in PARAMS.iter().enumerate() {
            assert_eq!(p.id, i as u32 + 1, "{}", p.name);
        }
    }

    #[test]
    fn width_is_geometric_about_its_middle() {
        let mid = (WIDTH_MIN * WIDTH_MAX).sqrt();
        assert!((mid - 44.72136).abs() < 1e-4);
    }
}
