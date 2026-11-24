//! Charm: a listening ensemble chorus. Its parameters and its engine
//! (ADR 0002).
//!
//! The input is split into LOW, MID and HIGH bands. LOW is never chorused.
//! MID and HIGH each feed a flock of voices: delay taps whose delay and pan
//! move by flocking rules, with DETUNE as the speed limit on the delay
//! axis, so no voice is ever further out of tune than DETUNE. The dry
//! input passes at unity; MIX sets the ensemble's level against it.
//!
//! On each onset the ensemble ducks (DUCK) and can fade back in (BLOOM).
//! AUTO WIDTH narrows the ensemble for bands whose input is already wide,
//! and no voice is panned so far that it loses more than 2 dB in mono.
//! The editor arrives with the plugin (ticket 0020).
//! (CLARITY, which steered voices' comb notches off strong partials, was
//! dropped after measurement: ticket 0017.)

use silkscreen::model::{ParamSpec, Taper};
use silkscreen::{
    Binding, Editor, EditorState, EngineLink, ParamIndex, ParamKnob, ParamToggle, PluginUi,
};

pub mod analysis;
pub mod decorrelate;
pub mod engine;
pub mod flock;
pub mod observe;
pub mod wet;

/// Generated from `charm.svg` by `silkscreen-build`: the plain faceplate
/// (ticket 0020), until 0019's design replaces it.
pub mod faceplate {
    include!(concat!(env!("OUT_DIR"), "/charm.rs"));
}

pub const VOICES: ParamIndex = ParamIndex(0);
pub const DETUNE: ParamIndex = ParamIndex(1);
pub const SPREAD: ParamIndex = ParamIndex(2);
pub const COHESION: ParamIndex = ParamIndex(3);
pub const SEPARATION: ParamIndex = ParamIndex(4);
pub const TURBULENCE: ParamIndex = ParamIndex(5);
pub const WIDTH: ParamIndex = ParamIndex(6);
pub const LOW_XOVER: ParamIndex = ParamIndex(7);
pub const MID_DEPTH: ParamIndex = ParamIndex(8);
pub const HIGH_DEPTH: ParamIndex = ParamIndex(9);
pub const MIX: ParamIndex = ParamIndex(10);
pub const DUCK: ParamIndex = ParamIndex(11);
pub const BLOOM: ParamIndex = ParamIndex(12);
pub const AUTO_WIDTH: ParamIndex = ParamIndex(13);

/// Ids reserved so none is taken by anything else: a control for the
/// voices' decorrelation, if 0024 gives it one.
pub const RESERVED_IDS: [u32; 1] = [15];

/// VOICES's positions: the number of voices in each flock.
const VOICE_COUNTS: &[&str] = &["2", "3", "4", "5", "6", "7", "8"];

/// The fewest voices a flock has; VOICES's position 0.
pub const MIN_VOICES: usize = 2;
/// The most voices a flock has.
pub const MAX_VOICES: usize = 8;

/// The parameter table: index `i` is `ParamIndex(i)`. Each `id` is what
/// presets and host projects store, so never change or reuse one. Ranges
/// and defaults are provisional until ticket 0024 tunes them.
pub static PARAMS: [ParamSpec; 14] = [
    // Voices per flock (MID and HIGH each have this many).
    ParamSpec::stepped("Voices", VOICE_COUNTS, 2).id(1),
    // The furthest out of tune any voice may be: the flock's speed limit
    // on the delay axis.
    ParamSpec::new("Detune", 1.0, 40.0, 12.0)
        .taper(Taper::Exp { mid: 10.0 })
        .unit("cents")
        .decimals(1)
        .id(2),
    // The range of delays above the 7.5 ms floor.
    ParamSpec::new("Spread", 1.0, 25.0, 10.0)
        .taper(Taper::Exp { mid: 7.0 })
        .unit("ms")
        .decimals(1)
        .id(3),
    ParamSpec::new("Cohesion", 0.0, 100.0, 50.0)
        .unit("%")
        .decimals(0)
        .id(4),
    ParamSpec::new("Separation", 0.0, 100.0, 50.0)
        .unit("%")
        .decimals(0)
        .id(5),
    ParamSpec::new("Turbulence", 0.0, 100.0, 40.0)
        .unit("%")
        .decimals(0)
        .id(6),
    // The voices' pan range, as a share of hard left to hard right.
    ParamSpec::new("Width", 0.0, 100.0, 70.0)
        .unit("%")
        .decimals(0)
        .id(7),
    // Below this, nothing is chorused.
    ParamSpec::new("Low Xover", 50.0, 500.0, 150.0)
        .taper(Taper::Exp { mid: 158.113_88 })
        .unit("Hz")
        .decimals(0)
        .id(8),
    ParamSpec::new("Mid Depth", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(9),
    ParamSpec::new("High Depth", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(10),
    // The ensemble's level against the dry signal, which stays at unity:
    // 100 % is equal level, as a classic chorus's even blend.
    ParamSpec::new("Mix", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(11),
    // How far the ensemble drops on each attack, so attacks stay dry
    // (ADR 0002, point 7).
    ParamSpec::new("Duck", 0.0, 100.0, 50.0)
        .unit("%")
        .decimals(0)
        .id(12),
    // After each attack the ensemble fades back in over this long; 0 is
    // off.
    ParamSpec::new("Bloom", 0.0, 2000.0, 0.0)
        .taper(Taper::Exp { mid: 300.0 })
        .unit("ms")
        .decimals(0)
        .id(13),
    // Narrow WIDTH for bands whose input is already wide (ADR 0002,
    // point 7).
    ParamSpec::toggle("Auto Width", true).id(14),
];

/// The editor's long-lived state, bound to the engine through `link`.
pub fn state(link: Box<dyn EngineLink>) -> EditorState {
    let knob = |p| -> Box<dyn Binding> { Box::new(ParamKnob::new(p)) };
    let controls = faceplate::Controls::<Box<dyn Binding>> {
        voices: knob(VOICES),
        detune: knob(DETUNE),
        spread: knob(SPREAD),
        cohesion: knob(COHESION),
        separation: knob(SEPARATION),
        turbulence: knob(TURBULENCE),
        mix: knob(MIX),
        width: knob(WIDTH),
        auto_width: Box::new(ParamToggle::new(AUTO_WIDTH)),
        low_xover: knob(LOW_XOVER),
        mid_depth: knob(MID_DEPTH),
        high_depth: knob(HIGH_DEPTH),
        duck: knob(DUCK),
        bloom: knob(BLOOM),
    };
    EditorState::new(&faceplate::LAYOUT, &PARAMS, controls.into_array(), link)
}

/// What every host needs to know about the editor.
pub const UI: PluginUi = PluginUi {
    vendor: "Vulpus Labs",
    product: "Charm",
    version: env!("CARGO_PKG_VERSION"),
    web_prefix: "charm",
    specs: &PARAMS,
    factory: &[],
    faceplate: &faceplate::FACEPLATE,
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
    fn ids_are_unique_and_none_is_reserved() {
        let mut ids: Vec<u32> = PARAMS.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PARAMS.len());
        for id in RESERVED_IDS {
            assert!(!ids.contains(&id), "{id} is reserved");
        }
    }

    #[test]
    fn voices_positions_cover_the_range() {
        assert_eq!(VOICE_COUNTS.len(), MAX_VOICES - MIN_VOICES + 1);
    }
}
