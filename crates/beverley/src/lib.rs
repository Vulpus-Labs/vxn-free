//! Beverley: a bitcrusher that adds harmonic grit. Its parameters, its
//! editor and its engine, shared by every host.
//!
//! `beverley.svg` is the whole visual design. `build.rs` turns it into
//! PNGs and the `faceplate` module below; this file declares the params
//! and binds each control to one.

use silkscreen::model::Taper;
use silkscreen::{
    Binding, BindingExt, Editor, EditorState, EngineLink, Model, ParamIndex, ParamKnob,
    ParamSelector, ParamSpec, ParamToggle, PluginUi, Unbound,
};
use vxn_free_ui::{ChainParams, CrushStyle, CrushView};

/// Beverley's crush chain, in `vxn-free-dsp` so Wesley's engine shares
/// it (ADR 0003).
pub use vxn_free_dsp::{crush, crush_chain as chain};
pub mod engine;

/// Generated from `beverley.svg` by `silkscreen-build`.
pub mod faceplate {
    include!(concat!(env!("OUT_DIR"), "/beverley.rs"));
}

pub const GAIN: ParamIndex = ParamIndex(0);
pub const AUTO_GAIN: ParamIndex = ParamIndex(1);
pub const GAMMA: ParamIndex = ParamIndex(2);
pub const BIT_DEPTH: ParamIndex = ParamIndex(3);
pub const SMOOTHING: ParamIndex = ParamIndex(4);
pub const CRUSH_MODE: ParamIndex = ParamIndex(5);
pub const MIX: ParamIndex = ParamIndex(6);
pub const STEREO: ParamIndex = ParamIndex(7);
pub const DECAY: ParamIndex = ParamIndex(8);
pub const DETECT: ParamIndex = ParamIndex(9);

/// GAMMA's knob runs from −1 to 1; the power it sets is ten to that.
fn gamma_from_knob(knob: f32) -> f32 {
    10f32.powf(knob)
}

fn gamma_to_knob(gamma: f32) -> f32 {
    gamma.max(1.0e-6).log10()
}

/// The parameter table: index `i` is `ParamIndex(i)`. Each `id` is what
/// presets and host projects store, so never change or reuse one.
pub static PARAMS: [ParamSpec; 10] = [
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
    ParamSpec::toggle("Crush Mode", false)
        .labels(&["Asymmetric", "Symmetric"])
        .id(6),
    ParamSpec::new("Mix", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(7),
    // How the two channels are crushed (`engine::Stereo`).
    ParamSpec::stepped("Stereo", &["Mono", "Separate", "Linked"], 2).id(8),
    // Auto gain's detector lets go 20 dB in this time. Geometric over the
    // whole travel: the middle is √(1 × 5000) ms. The default is the
    // module's release (a 50 ms time constant).
    ParamSpec::new("Decay", 1.0, 5000.0, 115.0)
        .taper(Taper::Exp { mid: 70.710_68 })
        .unit("ms")
        .decimals(0)
        .id(9),
    // How auto gain follows the input (`engine::Detect`).
    ParamSpec::toggle("Detect", true)
        .labels(&["Peak", "Range"])
        .id(10),
];

fn auto_off(model: &Model) -> bool {
    model.get(AUTO_GAIN) < 0.5
}

fn auto_on(model: &Model) -> bool {
    !auto_off(model)
}

/// The editor's long-lived state, bound to the engine through `link`.
pub fn state(link: Box<dyn EngineLink>) -> EditorState {
    let controls = faceplate::Controls::<Box<dyn Binding>> {
        // Fixed gain does nothing while auto gain sets it.
        gain: Box::new(ParamKnob::new(GAIN).dimmed_when(&[AUTO_GAIN], auto_on)),
        auto_gain: Box::new(ParamToggle::new(AUTO_GAIN)),
        // Auto gain's detector and its DECAY do nothing with AUTO off.
        detect: Box::new(ParamToggle::new(DETECT).dimmed_when(&[AUTO_GAIN], auto_off)),
        decay: Box::new(ParamKnob::new(DECAY).dimmed_when(&[AUTO_GAIN], auto_off)),
        gamma: Box::new(ParamKnob::new(GAMMA)),
        bit_depth: Box::new(ParamKnob::new(BIT_DEPTH)),
        smoothing: Box::new(ParamKnob::new(SMOOTHING)),
        crush_mode: Box::new(ParamToggle::new(CRUSH_MODE)),
        stereo: Box::new(ParamSelector::new(STEREO)),
        mix: Box::new(ParamKnob::new(MIX)),
        // The crush display (ticket 0039) takes no input.
        display: Box::new(Unbound),
    };
    let mut state = EditorState::new(&faceplate::LAYOUT, &PARAMS, controls.into_array(), link);
    state.set_widget(faceplate::ids::DISPLAY, |_| {
        Box::new(CrushView::new(
            ChainParams {
                gain: GAIN,
                auto_gain: AUTO_GAIN,
                gamma: GAMMA,
                bit_depth: BIT_DEPTH,
                smoothing: SMOOTHING,
                crush_mode: CRUSH_MODE,
                mix: MIX,
            },
            CrushStyle::LCD,
        ))
    });
    state
}

/// What every host needs to know about the editor.
pub const UI: PluginUi = PluginUi {
    vendor: "Vulpus Labs",
    product: "Beverley",
    version: env!("CARGO_PKG_VERSION"),
    web_prefix: "beverley",
    specs: &PARAMS,
    factory: &[],
    faceplate: &faceplate::FACEPLATE,
    frame: 0,
    state,
};

/// A whole editor, state and view, for hosts that own both.
pub fn editor(link: Box<dyn EngineLink>) -> Editor {
    Editor::new(state(link), UI.raster())
}
