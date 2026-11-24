//! Duck and Bloom against ticket 0016's acceptance criteria.

use charm::engine::CharmEngine;
use charm::flock::FLOOR;
use charm::wet::BLOOM_HALF_POWER;
use charm::{BLOOM, DUCK, MIX, SPREAD};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;

fn render(rate: f32, params: &[(ParamIndex, f32)], input: &[f32]) -> Vec<f32> {
    let mut e = CharmEngine::new(rate);
    for &(p, v) in params {
        e.set_param(p, v);
    }
    e.reset();
    let (mut l, mut r) = (input.to_vec(), input.to_vec());
    for (cl, cr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        e.process(&mut [cl, cr]);
    }
    // The wet part, mono.
    l.iter()
        .zip(&r)
        .zip(input)
        .map(|((a, b), x)| 0.5 * (a + b) - x)
        .collect()
}

fn energy(x: &[f32]) -> f64 {
    x.iter().map(|&v| f64::from(v) * f64::from(v)).sum()
}

#[test]
fn a_full_duck_silences_the_ensemble_around_each_click() {
    for rate in [44_100.0f32, 48_000.0, 96_000.0] {
        let every = (rate * 0.25) as usize;
        let mut x = vec![0.0f32; every * 8];
        let clicks: Vec<usize> = (1..8).map(|k| k * every + 31 * k).collect();
        for &t in &clicks {
            x[t] = 0.9;
        }
        let spread = 10.0f32;
        let base = [(MIX, 100.0), (SPREAD, spread), (BLOOM, 0.0)];
        let open = render(rate, &[&base[..], &[(DUCK, 0.0)]].concat(), &x);
        let ducked = render(rate, &[&base[..], &[(DUCK, 100.0)]].concat(), &x);
        // Every voice's copy of a click lands within the floor plus
        // SPREAD after it.
        let reach = ((FLOOR + f64::from(spread) * 0.001 + 0.002) * f64::from(rate)) as usize;
        for &t in &clicks {
            let window = t..t + reach;
            let a = energy(&open[window.clone()]);
            let b = energy(&ducked[window]);
            let db = 10.0 * (b / a).log10();
            assert!(db < -40.0, "{rate}: click at {t} ducked by only {db} dB");
        }
    }
}

#[test]
fn bloom_passes_minus_3_db_where_the_design_says() {
    let rate = 48_000.0;
    let start = (rate * 0.2) as usize;
    let bloom_ms = 1000.0;
    let len = start + rate as usize * 3;
    let mut seed = 1u32;
    let x: Vec<f32> = (0..len)
        .map(|n| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            if n < start {
                0.0
            } else {
                0.25 * ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5)
            }
        })
        .collect();
    let wet = render(rate, &[(MIX, 100.0), (DUCK, 0.0), (BLOOM, bloom_ms)], &x);
    let level = |from_s: f32, to_s: f32| {
        let (a, b) = (
            start + (from_s * rate) as usize,
            start + (to_s * rate) as usize,
        );
        energy(&wet[a..b]) / (b - a) as f64
    };
    let full = level(2.0, 2.8);
    // The voices sit about 12 ms behind the input; the onset is marked a
    // little before the noise starts.
    let at = BLOOM_HALF_POWER * bloom_ms * 0.001 + 0.012;
    let db = 10.0 * (level(at - 0.01, at + 0.01) / full).log10();
    assert!((db + 3.0).abs() < 1.0, "{db} dB at {at} s");
    // Early on it is far down.
    assert!(10.0 * (level(0.05, 0.07) / full).log10() < -15.0);
}
