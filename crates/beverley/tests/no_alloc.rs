//! The engine must not allocate on the audio thread.

use beverley::engine::BeverleyEngine;
use beverley::{AUTO_GAIN, BIT_DEPTH, GAMMA, MIX, STEREO};
use silkscreen::bridge::Engine;
use silkscreen_bridge::testing::{CountingAllocator, allocations};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator;

#[test]
fn process_and_set_param_never_allocate() {
    let mut e = BeverleyEngine::new(48_000.0);
    e.reset();
    let mut l: Vec<f32> = (0..512).map(|i| (i as f32 * 0.05).sin()).collect();
    let mut r = l.clone();
    let n = allocations(|| {
        for stereo in [0.0, 1.0, 2.0] {
            e.set_param(STEREO, stereo);
            e.set_param(AUTO_GAIN, stereo - 1.0);
            e.set_param(BIT_DEPTH, 3.7 + stereo);
            e.set_param(GAMMA, -0.5);
            e.set_param(MIX, 70.0);
            e.process(&mut [&mut l, &mut r]);
            e.process(&mut [&mut l[..77]]);
        }
        e.reset();
    });
    assert_eq!(n, 0);
}
