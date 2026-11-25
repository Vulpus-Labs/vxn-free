//! Wesley: Beverley in three bands (ADR 0003). Its parameters and its
//! engine.
//!
//! The input is split into LOW, MID and HIGH by two Linkwitz-Riley
//! crossovers whose bands sum flat (`vxn_free_dsp::ThreeBand`). Each band
//! has Beverley's crush chain with its own settings; STEREO and auto
//! gain's DETECT and DECAY are shared. LOW is off by default, so out of
//! the box the bass passes clean and MID and HIGH are crushed.
//!
//! `wesley.svg` is the plain faceplate (ticket 0034) until 0036's design;
//! `build.rs` turns it into PNGs and the `faceplate` module below.

use silkscreen::model::{ParamSpec, Taper};
use silkscreen::{
    Binding, BindingExt, Editor, EditorState, EngineLink, Model, ParamIndex, ParamKnob,
    ParamSelector, ParamToggle, PluginUi, Unbound,
};
use vxn_free_ui::{ChainParams, CrushStyle, CrushView};

pub mod engine;

/// Generated from `wesley.svg` by `silkscreen-build`.
pub mod faceplate {
    include!(concat!(env!("OUT_DIR"), "/wesley.rs"));
}

/// The three bands, low to high.
pub const BANDS: usize = 3;
pub const LOW: usize = 0;
pub const MID: usize = 1;
pub const HIGH: usize = 2;

/// Each band's params, in order: band `b`'s param `p` is at index
/// `b * PER_BAND + p` ([`band_param`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BandParam {
    Gain = 0,
    AutoGain = 1,
    Gamma = 2,
    BitDepth = 3,
    Smoothing = 4,
    CrushMode = 5,
    Mix = 6,
    Level = 7,
    On = 8,
    Solo = 9,
}

/// Params a band has.
pub const PER_BAND: u32 = 10;

impl BandParam {
    /// Every band param, in index order.
    pub const ALL: [BandParam; PER_BAND as usize] = [
        BandParam::Gain,
        BandParam::AutoGain,
        BandParam::Gamma,
        BandParam::BitDepth,
        BandParam::Smoothing,
        BandParam::CrushMode,
        BandParam::Mix,
        BandParam::Level,
        BandParam::On,
        BandParam::Solo,
    ];
}

/// Band `band`'s `param`.
pub const fn band_param(band: usize, param: BandParam) -> ParamIndex {
    ParamIndex(band as u32 * PER_BAND + param as u32)
}

/// Which band param `index` is, if it is one.
pub fn which_band_param(index: ParamIndex) -> Option<(usize, BandParam)> {
    let i = index.0;
    (i < BANDS as u32 * PER_BAND).then(|| {
        (
            (i / PER_BAND) as usize,
            BandParam::ALL[(i % PER_BAND) as usize],
        )
    })
}

const SHARED: u32 = BANDS as u32 * PER_BAND;
pub const LOW_XOVER: ParamIndex = ParamIndex(SHARED);
pub const HIGH_XOVER: ParamIndex = ParamIndex(SHARED + 1);
pub const STEREO: ParamIndex = ParamIndex(SHARED + 2);
pub const DETECT: ParamIndex = ParamIndex(SHARED + 3);
pub const DECAY: ParamIndex = ParamIndex(SHARED + 4);
pub const MIX: ParamIndex = ParamIndex(SHARED + 5);

/// GAMMA's knob runs from −1 to 1; the power it sets is ten to that.
fn gamma_from_knob(knob: f32) -> f32 {
    10f32.powf(knob)
}

fn gamma_to_knob(gamma: f32) -> f32 {
    gamma.max(1.0e-6).log10()
}

/// One band's params, `name` prefixed (`"Low"`), ids from `id`. Ranges
/// and defaults are Beverley's but for AUTO GAIN, on: a band is quieter
/// than the whole, and auto gain brings it to full scale before it is
/// crushed (ADR 0003, point 4). Drafts until ticket 0035.
macro_rules! band {
    ($name:literal, $id:literal, $on:literal) => {
        [
            ParamSpec::new(concat!($name, " Gain"), 0.0, 20.0, 0.0)
                .unit("dB")
                .decimals(1)
                .id($id),
            ParamSpec::toggle(concat!($name, " Auto Gain"), true).id($id + 1),
            ParamSpec::new(concat!($name, " Gamma"), -1.0, 1.0, 0.0)
                .bipolar()
                .display(gamma_from_knob, gamma_to_knob)
                .decimals(2)
                .id($id + 2),
            ParamSpec::new(concat!($name, " Bit Depth"), 1.0, 16.0, 4.0)
                .taper(Taper::Exp { mid: 4.0 })
                .unit("bits")
                .decimals(1)
                .id($id + 3),
            ParamSpec::new(concat!($name, " Smoothing"), 0.0, 100.0, 0.0)
                .taper(Taper::Exp { mid: 20.0 })
                .unit("%")
                .decimals(0)
                .id($id + 4),
            ParamSpec::toggle(concat!($name, " Crush Mode"), false)
                .labels(&["Asymmetric", "Symmetric"])
                .id($id + 5),
            // The band's crushed signal against its own dry signal.
            ParamSpec::new(concat!($name, " Mix"), 0.0, 100.0, 100.0)
                .unit("%")
                .decimals(0)
                .id($id + 6),
            // The band's output.
            ParamSpec::new(concat!($name, " Level"), -24.0, 12.0, 0.0)
                .unit("dB")
                .decimals(1)
                .id($id + 7),
            // Off passes the band uncrushed.
            ParamSpec::toggle(concat!($name, " On"), $on).id($id + 8),
            // Any band soloed mutes the rest: for setting up by ear.
            ParamSpec::toggle(concat!($name, " Solo"), false).id($id + 9),
        ]
    };
}

const LOW_PARAMS: [ParamSpec; PER_BAND as usize] = band!("Low", 1, false);
const MID_PARAMS: [ParamSpec; PER_BAND as usize] = band!("Mid", 11, true);
const HIGH_PARAMS: [ParamSpec; PER_BAND as usize] = band!("High", 21, true);

/// Params in all.
pub const PARAM_COUNT: usize = BANDS * PER_BAND as usize + 6;

/// The parameter table: index `i` is `ParamIndex(i)`. Each `id` is what
/// presets and host projects store, so never change or reuse one.
pub static PARAMS: [ParamSpec; PARAM_COUNT] = {
    let mut all = [LOW_PARAMS[0]; PARAM_COUNT];
    let mut i = 0;
    while i < PER_BAND as usize {
        all[i] = LOW_PARAMS[i];
        all[PER_BAND as usize + i] = MID_PARAMS[i];
        all[2 * PER_BAND as usize + i] = HIGH_PARAMS[i];
        i += 1;
    }
    let s = SHARED as usize;
    // Below this, LOW; up to HIGH XOVER, MID. Geometric over the travel.
    all[s] = ParamSpec::new("Low Xover", 40.0, 1000.0, 200.0)
        .taper(Taper::Exp { mid: 200.0 })
        .unit("Hz")
        .decimals(0)
        .id(31);
    // Held at least an octave above LOW XOVER.
    all[s + 1] = ParamSpec::new("High Xover", 500.0, 12_000.0, 2500.0)
        .taper(Taper::Exp { mid: 2449.49 })
        .unit("Hz")
        .decimals(0)
        .id(32);
    // How the two channels are crushed, as Beverley's STEREO.
    all[s + 2] = ParamSpec::stepped("Stereo", &["Mono", "Separate", "Linked"], 2).id(33);
    // How auto gain follows each band, as Beverley's DETECT.
    all[s + 3] = ParamSpec::toggle("Detect", true)
        .labels(&["Peak", "Range"])
        .id(34);
    // Auto gain lets go 20 dB in this time, as Beverley's DECAY.
    all[s + 4] = ParamSpec::new("Decay", 1.0, 5000.0, 115.0)
        .taper(Taper::Exp { mid: 70.710_68 })
        .unit("ms")
        .decimals(0)
        .id(35);
    // The summed bands against the input split and summed uncrushed.
    all[s + 5] = ParamSpec::new("Mix", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(36);
    all
};

fn on(model: &Model, p: ParamIndex) -> bool {
    model.get(p) >= 0.5
}

/// Band `band`'s bindings: its crush controls dim while the band is off,
/// and its GAIN while AUTO GAIN sets the gain instead.
fn band_controls(band: usize) -> [Box<dyn Binding>; PER_BAND as usize] {
    let p = |q| band_param(band, q);
    let band_on = p(BandParam::On);
    let auto = p(BandParam::AutoGain);
    let knob = move |q| -> Box<dyn Binding> {
        Box::new(ParamKnob::new(p(q)).dimmed_when(&[band_on], move |m| !on(m, band_on)))
    };
    let toggle = move |q| -> Box<dyn Binding> {
        Box::new(ParamToggle::new(p(q)).dimmed_when(&[band_on], move |m| !on(m, band_on)))
    };
    [
        Box::new(
            ParamKnob::new(p(BandParam::Gain))
                .dimmed_when(&[band_on, auto], move |m| !on(m, band_on) || on(m, auto)),
        ),
        toggle(BandParam::AutoGain),
        knob(BandParam::Gamma),
        knob(BandParam::BitDepth),
        knob(BandParam::Smoothing),
        toggle(BandParam::CrushMode),
        knob(BandParam::Mix),
        Box::new(ParamKnob::new(p(BandParam::Level))),
        Box::new(ParamToggle::new(band_on)),
        Box::new(ParamToggle::new(p(BandParam::Solo))),
    ]
}

/// The editor's long-lived state, bound to the engine through `link`.
pub fn state(link: Box<dyn EngineLink>) -> EditorState {
    let [
        low_gain,
        low_auto_gain,
        low_gamma,
        low_bit_depth,
        low_smoothing,
        low_crush_mode,
        low_mix,
        low_level,
        low_on,
        low_solo,
    ] = band_controls(LOW);
    let [
        mid_gain,
        mid_auto_gain,
        mid_gamma,
        mid_bit_depth,
        mid_smoothing,
        mid_crush_mode,
        mid_mix,
        mid_level,
        mid_on,
        mid_solo,
    ] = band_controls(MID);
    let [
        high_gain,
        high_auto_gain,
        high_gamma,
        high_bit_depth,
        high_smoothing,
        high_crush_mode,
        high_mix,
        high_level,
        high_on,
        high_solo,
    ] = band_controls(HIGH);
    let controls = faceplate::Controls::<Box<dyn Binding>> {
        low_gain,
        low_auto_gain,
        low_gamma,
        low_bit_depth,
        low_smoothing,
        low_crush_mode,
        low_mix,
        low_level,
        low_on,
        low_solo,
        mid_gain,
        mid_auto_gain,
        mid_gamma,
        mid_bit_depth,
        mid_smoothing,
        mid_crush_mode,
        mid_mix,
        mid_level,
        mid_on,
        mid_solo,
        high_gain,
        high_auto_gain,
        high_gamma,
        high_bit_depth,
        high_smoothing,
        high_crush_mode,
        high_mix,
        high_level,
        high_on,
        high_solo,
        low_xover: Box::new(ParamKnob::new(LOW_XOVER)),
        high_xover: Box::new(ParamKnob::new(HIGH_XOVER)),
        decay: Box::new(ParamKnob::new(DECAY)),
        detect: Box::new(ParamToggle::new(DETECT)),
        stereo: Box::new(ParamSelector::new(STEREO)),
        mix: Box::new(ParamKnob::new(MIX)),
        low_display: display_binding(LOW),
        mid_display: display_binding(MID),
        high_display: display_binding(HIGH),
    };
    let mut state = EditorState::new(&faceplate::LAYOUT, &PARAMS, controls.into_array(), link);
    for (band, id) in [
        (LOW, faceplate::ids::LOW_DISPLAY),
        (MID, faceplate::ids::MID_DISPLAY),
        (HIGH, faceplate::ids::HIGH_DISPLAY),
    ] {
        state.set_widget(id, move |_| {
            Box::new(CrushView::new(chain_params(band), CrushStyle::WELL))
        });
    }
    state
}

/// Band `band`'s chain params, for its crush display (ticket 0040).
fn chain_params(band: usize) -> ChainParams {
    let p = |q| band_param(band, q);
    ChainParams {
        gain: p(BandParam::Gain),
        auto_gain: p(BandParam::AutoGain),
        gamma: p(BandParam::Gamma),
        bit_depth: p(BandParam::BitDepth),
        smoothing: p(BandParam::Smoothing),
        crush_mode: p(BandParam::CrushMode),
        mix: p(BandParam::Mix),
    }
}

/// A band's display takes no input, and dims with the band while it is
/// off.
fn display_binding(band: usize) -> Box<dyn Binding> {
    let band_on = band_param(band, BandParam::On);
    Box::new(Unbound.dimmed_when(&[band_on], move |m| !on(m, band_on)))
}

/// What every host needs to know about the editor.
pub const UI: PluginUi = PluginUi {
    vendor: "Vulpus Labs",
    product: "Wesley",
    version: env!("CARGO_PKG_VERSION"),
    web_prefix: "wesley",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_in_order() {
        let ids: Vec<u32> = PARAMS.iter().map(|p| p.id).collect();
        assert_eq!(ids, (1..=36).collect::<Vec<u32>>());
    }

    #[test]
    fn band_params_index_round_trip() {
        for band in 0..BANDS {
            for p in BandParam::ALL {
                let i = band_param(band, p);
                assert_eq!(which_band_param(i), Some((band, p)));
            }
        }
        assert_eq!(which_band_param(LOW_XOVER), None);
        assert_eq!(
            PARAMS[band_param(MID, BandParam::BitDepth).0 as usize].name,
            "Mid Bit Depth"
        );
    }

    #[test]
    fn low_is_off_by_default() {
        let on = |b| PARAMS[band_param(b, BandParam::On).0 as usize].default;
        assert_eq!((on(LOW), on(MID), on(HIGH)), (0.0, 1.0, 1.0));
    }
}
