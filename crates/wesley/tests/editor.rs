//! Wesley's editor (ticket 0036: the rack panel), driven headlessly.
//! Snapshots live in `tests/snapshots/`; after a deliberate visual change,
//! re-bless them with `SILKSCREEN_BLESS=1 cargo test` and look at the
//! diffs.

use silkscreen::ControlId;
use silkscreen::ParamIndex;
use silkscreen_harness::{Harness, assert_snapshot};
use wesley::faceplate::{LAYOUT, ids};
use wesley::{
    BandParam, HIGH, HIGH_XOVER, LOW, LOW_XOVER, MID, MIX, PARAMS, SOLO, STEREO, band_param,
};

#[test]
fn the_faceplate_draws() {
    let mut h = Harness::new(wesley::editor, 1.0);
    assert_snapshot!(h, "initial_1x");
}

/// Band `band`'s controls, in `BandParam::ALL`'s order.
fn band_ids(band: usize) -> [ControlId; 10] {
    match band {
        LOW => [
            ids::LOW_GAIN,
            ids::LOW_AUTO_GAIN,
            ids::LOW_GAMMA,
            ids::LOW_BIT_DEPTH,
            ids::LOW_SMOOTHING,
            ids::LOW_CRUSH_MODE,
            ids::LOW_MIX,
            ids::LOW_LEVEL,
            ids::LOW_ON,
            ids::LOW_DETECT,
        ],
        MID => [
            ids::MID_GAIN,
            ids::MID_AUTO_GAIN,
            ids::MID_GAMMA,
            ids::MID_BIT_DEPTH,
            ids::MID_SMOOTHING,
            ids::MID_CRUSH_MODE,
            ids::MID_MIX,
            ids::MID_LEVEL,
            ids::MID_ON,
            ids::MID_DETECT,
        ],
        _ => [
            ids::HIGH_GAIN,
            ids::HIGH_AUTO_GAIN,
            ids::HIGH_GAMMA,
            ids::HIGH_BIT_DEPTH,
            ids::HIGH_SMOOTHING,
            ids::HIGH_CRUSH_MODE,
            ids::HIGH_MIX,
            ids::HIGH_LEVEL,
            ids::HIGH_ON,
            ids::HIGH_DETECT,
        ],
    }
}

/// Every control bound to a param, with that param.
fn controls() -> Vec<(ControlId, ParamIndex)> {
    let mut all = Vec::new();
    for band in [LOW, MID, HIGH] {
        for (p, id) in BandParam::ALL.iter().zip(band_ids(band)) {
            all.push((id, band_param(band, *p)));
        }
        let decay = [ids::LOW_DECAY, ids::MID_DECAY, ids::HIGH_DECAY][band];
        all.push((decay, band_param(band, BandParam::Decay)));
        let solo = [ids::LOW_SOLO, ids::MID_SOLO, ids::HIGH_SOLO][band];
        all.push((solo, SOLO));
    }
    all.extend([
        (ids::LOW_XOVER, LOW_XOVER),
        (ids::HIGH_XOVER, HIGH_XOVER),
        (ids::STEREO, STEREO),
        (ids::MIX, MIX),
    ]);
    all
}

#[test]
fn every_control_moves_its_param() {
    let all = controls();
    // Every param has a control (SOLO three), and a display a band.
    let mut params: Vec<u32> = all.iter().map(|(_, p)| p.0).collect();
    params.sort_unstable();
    params.dedup();
    assert_eq!(params.len(), PARAMS.len());
    assert_eq!(LAYOUT.controls.len(), all.len() + 3);
    for (control, param) in all {
        let mut h = Harness::new(wesley::editor, 1.0);
        let spec = &PARAMS[param.0 as usize];
        let before = h.model().get(param);
        if spec.decimals == 0 && spec.max - spec.min <= 3.0 {
            // Switches, buttons and the selector: a click (STEREO is
            // dragged, as Beverley's).
            if param == STEREO {
                h.drag(control, 60.0);
            } else {
                h.click(control);
            }
        } else {
            let drag = if before >= spec.max { 40.0 } else { -40.0 };
            h.drag(control, drag);
        }
        assert_ne!(h.model().get(param), before, "{}", spec.name);
    }
}

#[test]
fn only_one_solo_lights_at_once() {
    let mut h = Harness::new(wesley::editor, 1.0);
    assert_eq!(h.model().get(SOLO), 0.0);
    h.click(ids::LOW_SOLO);
    assert_eq!(h.model().get(SOLO), 1.0, "LOW soloed");
    h.click(ids::MID_SOLO);
    assert_eq!(h.model().get(SOLO), 2.0, "MID soloed; LOW's goes out");
    assert_snapshot!(h, "mid_soloed_1x");
    h.click(ids::MID_SOLO);
    assert_eq!(h.model().get(SOLO), 0.0, "pressed again: no solo");
}

#[test]
fn a_bands_controls_dim_while_it_is_off() {
    // LOW is off by default: its row is dimmed, MID's isn't.
    let mut h = Harness::new(wesley::editor, 1.0);
    let off = h.snapshot().clone();
    h.click(ids::LOW_ON);
    assert_eq!(h.model().get(band_param(LOW, BandParam::On)), 1.0);
    assert_ne!(off, *h.snapshot(), "turning LOW on undims its row");
}

#[test]
fn a_display_follows_its_bands_settings() {
    let mut h = Harness::new(wesley::editor, 1.0);
    h.snapshot();
    h.drag(ids::MID_BIT_DEPTH, 30.0);
    h.snapshot();
    let mid = LAYOUT.control(ids::MID_DISPLAY).rect.to_physical(1.0);
    let high = LAYOUT.control(ids::HIGH_DISPLAY).rect.to_physical(1.0);
    assert!(
        h.last_damage().iter().any(|d| d.intersect(&mid).is_some()),
        "MID's display repaints"
    );
    assert!(
        h.last_damage().iter().all(|d| d.intersect(&high).is_none()),
        "HIGH's doesn't"
    );
}
