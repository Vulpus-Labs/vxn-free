//! The engine must not allocate on the audio thread.

use charm::engine::CharmEngine;
use charm::{
    AUTO_WIDTH, BLOOM, COHESION, DETUNE, DUCK, LOW_XOVER, MIX, SPREAD, TURBULENCE, VOICES, WIDTH,
};
use silkscreen::bridge::Engine;
use silkscreen_bridge::testing::{CountingAllocator, allocations};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator;

#[test]
fn process_and_set_param_never_allocate() {
    let mut e = CharmEngine::new(48_000.0);
    e.reset();
    let mut l: Vec<f32> = (0..512)
        .map(|i| {
            if i % 128 == 0 {
                1.0
            } else {
                (i as f32 * 0.05).sin() * 0.1
            }
        })
        .collect();
    let mut r = l.clone();
    let n = allocations(|| {
        for k in 0..20 {
            let k = k as f32;
            e.set_param(VOICES, k % 7.0);
            e.set_param(DETUNE, 1.0 + k * 2.0);
            e.set_param(SPREAD, 1.0 + k);
            e.set_param(COHESION, k * 5.0);
            e.set_param(TURBULENCE, 100.0 - k * 5.0);
            e.set_param(WIDTH, k * 5.0);
            e.set_param(LOW_XOVER, 50.0 + k * 20.0);
            e.set_param(MIX, 50.0);
            e.set_param(DUCK, k * 5.0);
            e.set_param(BLOOM, k * 100.0);
            e.set_param(AUTO_WIDTH, k % 2.0);
            e.process(&mut [&mut l, &mut r]);
            e.process(&mut [&mut l[..77]]);
            let _ = e.observation();
        }
        e.reset();
    });
    assert_eq!(n, 0);
}
