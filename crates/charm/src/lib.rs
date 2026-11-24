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
//! The editor arrives with the plugin (ticket 0020); AUTO WIDTH with
//! ticket 0018, under an id reserved below.
//! (CLARITY, which steered voices' comb notches off strong partials, was
//! dropped after measurement: ticket 0017.)

use silkscreen::ParamIndex;
use silkscreen::model::{ParamSpec, Taper};

pub mod analysis;
pub mod decorrelate;
pub mod engine;
pub mod flock;
pub mod observe;
pub mod wet;

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

/// Ids reserved for params later tickets add, so none is taken by
/// anything else: AUTO WIDTH (0018), and a control for the voices'
/// decorrelation if 0024 gives it one.
pub const RESERVED_IDS: [u32; 2] = [14, 15];

/// VOICES's positions: the number of voices in each flock.
const VOICE_COUNTS: &[&str] = &["2", "3", "4", "5", "6", "7", "8"];

/// The fewest voices a flock has; VOICES's position 0.
pub const MIN_VOICES: usize = 2;
/// The most voices a flock has.
pub const MAX_VOICES: usize = 8;

/// The parameter table: index `i` is `ParamIndex(i)`. Each `id` is what
/// presets and host projects store, so never change or reuse one. Ranges
/// and defaults are provisional until ticket 0024 tunes them.
pub static PARAMS: [ParamSpec; 13] = [
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
    // 100 % is equal level.
    ParamSpec::new("Mix", 0.0, 100.0, 50.0)
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
];

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
