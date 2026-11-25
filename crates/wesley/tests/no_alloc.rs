//! The engine must not allocate on the audio thread.

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use silkscreen_bridge::testing::{CountingAllocator, allocations};
use wesley::engine::WesleyEngine;
use wesley::{BandParam, HIGH_XOVER, LOW, LOW_XOVER, MID, MIX, PARAMS, STEREO, band_param};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator;

#[test]
fn process_and_set_param_never_allocate() {
    let mut e = WesleyEngine::new(48_000.0);
    e.reset();
    let mut l: Vec<f32> = (0..512).map(|i| (i as f32 * 0.05).sin()).collect();
    let mut r = l.clone();
    let n = allocations(|| {
        for k in 0..6 {
            let k = k as f32;
            for (i, spec) in PARAMS.iter().enumerate() {
                e.set_param(
                    ParamIndex(i as u32),
                    spec.min + (spec.max - spec.min) * k / 5.0,
                );
            }
            e.set_param(band_param(LOW, BandParam::On), 1.0);
            e.set_param(band_param(MID, BandParam::Solo), k % 2.0);
            e.set_param(LOW_XOVER, 100.0 + 50.0 * k);
            e.set_param(HIGH_XOVER, 1000.0 + 1000.0 * k);
            e.set_param(STEREO, k % 3.0);
            e.set_param(MIX, 60.0);
            e.process(&mut [&mut l, &mut r]);
            e.process(&mut [&mut l[..77]]);
        }
        e.reset();
    });
    assert_eq!(n, 0);
}
