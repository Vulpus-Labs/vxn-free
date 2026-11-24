//! The editor, driven headlessly: scripted input in, params and pixels out.
//! Snapshots live in `tests/snapshots/`; after a deliberate visual change,
//! re-bless them with `SILKSCREEN_BLESS=1 cargo test` and look at the diffs.

use beverley::faceplate::ids;
use beverley::{
    AUTO_GAIN, BIT_DEPTH, CRUSH_MODE, DECAY, DETECT, GAIN, GAMMA, MIX, SMOOTHING, STEREO,
};
use silkscreen::ControlId;
use silkscreen_harness::{Harness, assert_snapshot};

/// The tip shown after resting on `control`.
fn tip(h: &mut Harness, control: ControlId) -> String {
    let c = h.centre(control);
    h.move_to(c).advance(700.0);
    h.editor().tip().expect("a tip after the dwell").to_owned()
}

#[test]
fn the_faceplate_draws() {
    let mut h = Harness::new(beverley::editor, 1.0);
    assert_snapshot!(h, "initial_1x");
}

#[test]
fn the_faceplate_draws_at_2x() {
    let mut h = Harness::new(beverley::editor, 2.0);
    assert_snapshot!(h, "initial_2x");
}

#[test]
fn every_control_moves_its_param() {
    let mut h = Harness::new(beverley::editor, 1.0);
    for (control, param) in [
        (ids::GAIN, GAIN),
        (ids::GAMMA, GAMMA),
        (ids::BIT_DEPTH, BIT_DEPTH),
        (ids::SMOOTHING, SMOOTHING),
        (ids::DECAY, DECAY),
    ] {
        let before = h.model().get(param);
        h.drag(control, -40.0);
        assert!(h.model().get(param) > before, "{control:?}");
    }
    h.drag(ids::MIX, 40.0);
    assert!(h.model().get(MIX) < 100.0);

    h.click(ids::AUTO_GAIN);
    assert_eq!(h.model().get(AUTO_GAIN), 1.0);
    // DETECT starts at RANGE; a click flips it to PEAK.
    assert_eq!(h.model().get(DETECT), 1.0);
    h.click(ids::DETECT);
    assert_eq!(h.model().get(DETECT), 0.0);
    h.click(ids::CRUSH_MODE);
    assert_eq!(h.model().get(CRUSH_MODE), 1.0);
    // STEREO starts at LINKED, the top; dragging down steps to MONO.
    assert_eq!(h.model().get(STEREO), 2.0);
    h.drag(ids::STEREO, 60.0);
    assert_eq!(h.model().get(STEREO), 0.0);
    assert_snapshot!(h, "changed_1x");
}

#[test]
fn hovering_shows_values_in_their_units() {
    let mut h = Harness::new(beverley::editor, 1.0);
    assert_eq!(tip(&mut h, ids::GAIN), "Gain  0.0 dB");
    assert_eq!(tip(&mut h, ids::BIT_DEPTH), "Bit Depth  4.0 bits");
    assert_eq!(tip(&mut h, ids::GAMMA), "Gamma  1.00");
    assert_eq!(tip(&mut h, ids::SMOOTHING), "Smoothing  0 %");
    assert_eq!(tip(&mut h, ids::MIX), "Mix  100 %");
    assert_eq!(tip(&mut h, ids::CRUSH_MODE), "Crush Mode  Asymmetric");
    assert_eq!(tip(&mut h, ids::STEREO), "Stereo  Linked");
    assert_eq!(tip(&mut h, ids::AUTO_GAIN), "Auto Gain  Off");
    assert_eq!(tip(&mut h, ids::DECAY), "Decay  115 ms");
    assert_eq!(tip(&mut h, ids::DETECT), "Detect  Range");
    assert_snapshot!(h, "tooltip_auto_1x");
}

#[test]
fn detect_and_decay_light_up_with_auto_gain() {
    // At rest AUTO is off, so DETECT and DECAY are dimmed (initial_1x);
    // switched on, they are drawn at full strength.
    let mut h = Harness::new(beverley::editor, 1.0);
    h.click(ids::AUTO_GAIN);
    assert_snapshot!(h, "auto_on_1x");
}
