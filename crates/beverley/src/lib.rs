//! Beverley: a bitcrusher that adds harmonic grit. Its parameters and
//! engine, shared by every host.

use silkscreen::model::Taper;
use silkscreen::{ParamIndex, ParamSpec};

pub mod crush;
pub mod engine;

pub const GAIN: ParamIndex = ParamIndex(0);
pub const AUTO_GAIN: ParamIndex = ParamIndex(1);
pub const GAMMA: ParamIndex = ParamIndex(2);
pub const BIT_DEPTH: ParamIndex = ParamIndex(3);
pub const SMOOTHING: ParamIndex = ParamIndex(4);
pub const CRUSH_MODE: ParamIndex = ParamIndex(5);
pub const MIX: ParamIndex = ParamIndex(6);
pub const STEREO: ParamIndex = ParamIndex(7);

/// GAMMA's knob runs from −1 to 1; the power it sets is ten to that.
fn gamma_from_knob(knob: f32) -> f32 {
    10f32.powf(knob)
}

fn gamma_to_knob(gamma: f32) -> f32 {
    gamma.max(1.0e-6).log10()
}

/// The parameter table: index `i` is `ParamIndex(i)`. Each `id` is what
/// presets and host projects store, so never change or reuse one.
pub static PARAMS: [ParamSpec; 8] = [
    ParamSpec::new("Gain", 0.0, 20.0, 0.0)
        .unit("dB")
        .decimals(1)
        .id(1),
    ParamSpec::toggle("Auto Gain", false).id(2),
    ParamSpec::new("Gamma", -1.0, 1.0, 0.0)
        .bipolar()
        .display(gamma_from_knob, gamma_to_knob)
        .decimals(2)
        .id(3),
    ParamSpec::new("Bit Depth", 1.0, 16.0, 4.0)
        .taper(Taper::Exp { mid: 4.0 })
        .unit("bits")
        .decimals(1)
        .id(4),
    ParamSpec::new("Smoothing", 0.0, 100.0, 0.0)
        .taper(Taper::Exp { mid: 20.0 })
        .unit("%")
        .decimals(0)
        .id(5),
    ParamSpec::stepped("Crush Mode", &["Asymmetric", "Symmetric"], 0).id(6),
    ParamSpec::new("Mix", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(7),
    // How the two channels are crushed (`engine::Stereo`).
    ParamSpec::stepped("Stereo", &["Mono", "Separate", "Linked"], 2).id(8),
];
