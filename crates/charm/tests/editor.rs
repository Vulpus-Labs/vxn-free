//! The plain faceplate's editor (ticket 0020), driven headlessly. Snapshots
//! live in `tests/snapshots/`; after a deliberate visual change, re-bless
//! them with `SILKSCREEN_BLESS=1 cargo test` and look at the diffs.

use charm::faceplate::ids;
use charm::{
    AUTO_WIDTH, BLOOM, COHESION, DECORRELATE, DETUNE, DUCK, HIGH_DEPTH, LOW_XOVER, MID_DEPTH, MIX,
    SEPARATION, SPREAD, TURBULENCE, VOICES, WIDTH,
};
use silkscreen_harness::{Harness, assert_snapshot};

#[test]
fn the_faceplate_draws() {
    let mut h = Harness::new(charm::editor, 1.0);
    assert_snapshot!(h, "initial_1x");
}

#[test]
fn every_control_moves_its_param() {
    let mut h = Harness::new(charm::editor, 1.0);
    for (control, param) in [
        (ids::VOICES, VOICES),
        (ids::DETUNE, DETUNE),
        (ids::SPREAD, SPREAD),
        (ids::COHESION, COHESION),
        (ids::SEPARATION, SEPARATION),
        (ids::TURBULENCE, TURBULENCE),
        (ids::WIDTH, WIDTH),
        (ids::LOW_XOVER, LOW_XOVER),
        (ids::MID_DEPTH, MID_DEPTH),
        (ids::HIGH_DEPTH, HIGH_DEPTH),
        (ids::MIX, MIX),
        (ids::DUCK, DUCK),
        (ids::BLOOM, BLOOM),
    ] {
        let before = h.model().get(param);
        // Down from the top for params that start there.
        let drag = if before >= charm::PARAMS[param.0 as usize].max {
            40.0
        } else {
            -40.0
        };
        h.drag(control, drag);
        assert_ne!(h.model().get(param), before, "{control:?}");
    }
    assert_eq!(h.model().get(AUTO_WIDTH), 1.0);
    h.click(ids::AUTO_WIDTH);
    assert_eq!(h.model().get(AUTO_WIDTH), 0.0);
    assert_eq!(h.model().get(DECORRELATE), 1.0);
    h.click(ids::DECORRELATE);
    assert_eq!(h.model().get(DECORRELATE), 0.0);
}
