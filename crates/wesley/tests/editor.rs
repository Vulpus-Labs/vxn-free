//! The plain faceplate's editor (ticket 0034), driven headlessly.
//! Snapshots live in `tests/snapshots/`; after a deliberate visual change,
//! re-bless them with `SILKSCREEN_BLESS=1 cargo test` and look at the
//! diffs.

use silkscreen::ParamIndex;
use silkscreen_harness::{Harness, assert_snapshot};
use wesley::faceplate::{LAYOUT, ids};
use wesley::{
    BandParam, DECAY, DETECT, HIGH, HIGH_XOVER, LOW, LOW_XOVER, MID, MIX, PARAMS, STEREO,
    band_param,
};

#[test]
fn the_faceplate_draws() {
    let mut h = Harness::new(wesley::editor, 1.0);
    assert_snapshot!(h, "initial_1x");
}

/// Every control on the faceplate, with the param it moves.
fn controls() -> Vec<(silkscreen::ControlId, ParamIndex)> {
    let band = |b, ids: [silkscreen::ControlId; 10]| {
        BandParam::ALL
            .iter()
            .zip(ids)
            .map(move |(&p, id)| (id, band_param(b, p)))
            .collect::<Vec<_>>()
    };
    let mut all = Vec::new();
    all.extend(band(
        LOW,
        [
            ids::LOW_GAIN,
            ids::LOW_AUTO_GAIN,
            ids::LOW_GAMMA,
            ids::LOW_BIT_DEPTH,
            ids::LOW_SMOOTHING,
            ids::LOW_CRUSH_MODE,
            ids::LOW_MIX,
            ids::LOW_LEVEL,
            ids::LOW_ON,
            ids::LOW_SOLO,
        ],
    ));
    all.extend(band(
        MID,
        [
            ids::MID_GAIN,
            ids::MID_AUTO_GAIN,
            ids::MID_GAMMA,
            ids::MID_BIT_DEPTH,
            ids::MID_SMOOTHING,
            ids::MID_CRUSH_MODE,
            ids::MID_MIX,
            ids::MID_LEVEL,
            ids::MID_ON,
            ids::MID_SOLO,
        ],
    ));
    all.extend(band(
        HIGH,
        [
            ids::HIGH_GAIN,
            ids::HIGH_AUTO_GAIN,
            ids::HIGH_GAMMA,
            ids::HIGH_BIT_DEPTH,
            ids::HIGH_SMOOTHING,
            ids::HIGH_CRUSH_MODE,
            ids::HIGH_MIX,
            ids::HIGH_LEVEL,
            ids::HIGH_ON,
            ids::HIGH_SOLO,
        ],
    ));
    all.extend([
        (ids::LOW_XOVER, LOW_XOVER),
        (ids::HIGH_XOVER, HIGH_XOVER),
        (ids::DECAY, DECAY),
        (ids::DETECT, DETECT),
        (ids::STEREO, STEREO),
        (ids::MIX, MIX),
    ]);
    all
}

#[test]
fn every_control_moves_its_param() {
    let all = controls();
    assert_eq!(all.len(), PARAMS.len(), "one control a param");
    assert_eq!(LAYOUT.controls.len(), PARAMS.len());
    for (control, param) in all {
        let mut h = Harness::new(wesley::editor, 1.0);
        let spec = &PARAMS[param.0 as usize];
        let before = h.model().get(param);
        if spec.max - spec.min <= 2.0 && spec.decimals == 0 {
            // Toggles and the selector: a click moves them.
            h.click(control);
        } else {
            let drag = if before >= spec.max { 40.0 } else { -40.0 };
            h.drag(control, drag);
        }
        assert_ne!(h.model().get(param), before, "{}", spec.name);
    }
}

#[test]
fn a_bands_controls_dim_while_it_is_off() {
    // LOW is off by default: its strip is dimmed, MID's isn't.
    let mut h = Harness::new(wesley::editor, 1.0);
    let off = h.snapshot().clone();
    h.click(ids::LOW_ON);
    assert_eq!(h.model().get(band_param(LOW, BandParam::On)), 1.0);
    let on = h.snapshot().clone();
    assert_ne!(off, on, "turning LOW on undims its strip");
}
