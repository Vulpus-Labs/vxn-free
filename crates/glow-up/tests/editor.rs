//! The faceplate's editor (ticket 0009: a modern rack unit), driven
//! headlessly.
//! Snapshots live in `tests/snapshots/`; after a deliberate visual change,
//! re-bless them with `SILKSCREEN_BLESS=1 cargo test` and look at the
//! diffs.

use glow_up::faceplate::ids;
use glow_up::{
    A_AMOUNT, A_DEN, A_NUM, B_AMOUNT, B_DEN, B_NUM, DRY_CUT, FOCUS, GRID, MIX, MODE, PARAMS,
    RESONANCE, SPEED, THRESHOLD, TONE, WIDTH, WINDOW,
};
use silkscreen_harness::{Harness, assert_snapshot};

#[test]
fn the_faceplate_draws() {
    let mut h = Harness::new(glow_up::editor, 1.0);
    assert_snapshot!(h, "initial_1x");
}

#[test]
fn the_keys_light_for_their_positions() {
    let mut h = Harness::new(glow_up::editor, 1.0);
    h.click(ids::GRID_TET)
        .click(ids::STEREO)
        .click(ids::DRY_CUT);
    assert_snapshot!(h, "tet_stereo_dry_cut_1x");
}

#[test]
fn every_control_moves_its_param() {
    let mut h = Harness::new(glow_up::editor, 1.0);
    for (control, param) in [
        (ids::A_AMOUNT, A_AMOUNT),
        (ids::A_NUM, A_NUM),
        (ids::A_DEN, A_DEN),
        (ids::B_AMOUNT, B_AMOUNT),
        (ids::B_NUM, B_NUM),
        (ids::B_DEN, B_DEN),
        (ids::WIDTH, WIDTH),
        (ids::RESONANCE, RESONANCE),
        (ids::FOCUS, FOCUS),
        (ids::MIX, MIX),
        (ids::THRESHOLD, THRESHOLD),
        (ids::TONE, TONE),
        (ids::SPEED, SPEED),
        (ids::WINDOW, WINDOW),
    ] {
        let before = h.model().get(param);
        // Down from the top for params that start there; far enough for
        // a knob of few steps.
        let drag = if before >= PARAMS[param.0 as usize].max {
            120.0
        } else {
            -120.0
        };
        h.drag(control, drag);
        assert_ne!(h.model().get(param), before, "{control:?}");
    }
    // Each key of a row picks its position.
    for (control, param, position) in [
        (ids::GRID_LIN, GRID, 0.0),
        (ids::GRID_TET, GRID, 2.0),
        (ids::GRID_IRR, GRID, 1.0),
        (ids::STEREO, MODE, 1.0),
        (ids::MONO, MODE, 0.0),
    ] {
        h.click(control);
        assert_eq!(h.model().get(param), position, "{control:?}");
        // Pressing it again keeps it.
        h.click(control);
        assert_eq!(h.model().get(param), position, "{control:?}");
    }
    assert_eq!(h.model().get(DRY_CUT), 0.0);
    h.click(ids::DRY_CUT);
    assert_eq!(h.model().get(DRY_CUT), 1.0);
}
