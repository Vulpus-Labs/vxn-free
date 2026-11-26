//! Glow-Up's engine against ticket 0008's criteria.

use std::f64::consts::TAU;

use glow_up::bank::State;
use glow_up::engine::GlowUpEngine;
use glow_up::{
    A_AMOUNT, A_DEN, A_NUM, B_AMOUNT, DRY_CUT, FOCUS, GRID, MIX, MODE, RESONANCE, SPEED, THRESHOLD,
    WIDTH,
};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;

const RATE: f32 = 48_000.0;

/// An engine at `rate` with `params` set (plain values), reset as a host
/// does.
fn engine_at(rate: f32, params: &[(ParamIndex, f32)]) -> GlowUpEngine {
    let mut e = GlowUpEngine::new(rate);
    for &(p, v) in params {
        e.set_param(p, v);
    }
    e.reset();
    e
}

fn engine(params: &[(ParamIndex, f32)]) -> GlowUpEngine {
    engine_at(RATE, params)
}

/// Boost A only, at `num`/`den` and `amount` %.
fn only_a(num: usize, den: usize, amount: f32) -> Vec<(ParamIndex, f32)> {
    vec![
        (A_AMOUNT, amount),
        (A_NUM, (num - 1) as f32),
        (A_DEN, (den - 1) as f32),
        (B_AMOUNT, 0.0),
    ]
}

/// Mono: `x` through `e` in blocks of `block`.
fn run(e: &mut GlowUpEngine, x: &[f32], block: usize) -> Vec<f32> {
    let mut y = x.to_vec();
    for chunk in y.chunks_mut(block) {
        e.process(&mut [chunk]);
    }
    y
}

/// Stereo: `(l, r)` through `e` in blocks of `block`.
fn run2(e: &mut GlowUpEngine, l: &[f32], r: &[f32], block: usize) -> (Vec<f32>, Vec<f32>) {
    let (mut yl, mut yr) = (l.to_vec(), r.to_vec());
    for (a, b) in yl.chunks_mut(block).zip(yr.chunks_mut(block)) {
        e.process(&mut [a, b]);
    }
    (yl, yr)
}

fn sines(parts: &[(f64, f64)], seconds: f64) -> Vec<f32> {
    let rate = f64::from(RATE);
    (0..(seconds * rate) as usize)
        .map(|n| {
            parts
                .iter()
                .map(|&(hz, a)| a * (TAU * hz * n as f64 / rate).sin())
                .sum::<f64>() as f32
        })
        .collect()
}

/// A band-limited sawtooth at `hz`, amplitude `a`: harmonics to 16 kHz.
fn saw(hz: f64, a: f64, seconds: f64) -> Vec<f32> {
    let top = (16_000.0 / hz).ceil() as usize;
    let parts: Vec<(f64, f64)> = (1..top).map(|k| (k as f64 * hz, a / k as f64)).collect();
    sines(&parts, seconds)
}

fn noise(len: usize, a: f32, mut seed: u32) -> Vec<f32> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            a * (2.0 * (seed >> 8) as f32 / (1u32 << 24) as f32 - 1.0)
        })
        .collect()
}

/// The amplitude of `hz` in the last `seconds` of `x`.
fn level(x: &[f32], hz: f64, seconds: f64) -> f64 {
    let n = (seconds * f64::from(RATE)) as usize;
    let tail = &x[x.len() - n..];
    let w = TAU * hz / f64::from(RATE);
    let (mut re, mut im) = (0.0, 0.0);
    for (i, &s) in tail.iter().enumerate() {
        re += f64::from(s) * (w * i as f64).cos();
        im += f64::from(s) * (w * i as f64).sin();
    }
    2.0 * (re * re + im * im).sqrt() / n as f64
}

fn db(a: f64, b: f64) -> f64 {
    20.0 * (a / b).log10()
}

#[test]
fn every_amount_at_zero_passes_the_input_sample_for_sample() {
    let x: Vec<f32> = saw(110.0, 0.3, 1.0)
        .iter()
        .zip(noise(48_000, 0.2, 5))
        .map(|(a, b)| a + b)
        .collect();
    for focus in [0.0, 100.0] {
        let mut e = engine(&[
            (A_AMOUNT, 0.0),
            (B_AMOUNT, 0.0),
            (FOCUS, focus),
            (RESONANCE, 100.0),
        ]);
        assert_eq!(run(&mut e, &x, 64), x, "FOCUS {focus}");
    }
}

#[test]
fn a_sine_passes_almost_untouched() {
    let x = sines(&[(440.0, 0.5)], 2.0);
    for focus in [0.0, 100.0] {
        let mut e = engine(&[(FOCUS, focus)]);
        let y = run(&mut e, &x, 256);
        let change = db(level(&y, 440.0, 1.0), level(&x, 440.0, 1.0));
        // TRACK puts the boosts exactly on 880 and 660; the grid end
        // drives several resonances about each, and their skirts add.
        let most = if focus > 0.0 { 0.3 } else { 1.0 };
        assert!(change.abs() < most, "FOCUS {focus}: {change} dB");
    }
}

#[test]
fn the_octave_lifts_a_saws_even_partials() {
    let x = saw(110.0, 0.3, 3.0);
    let mut params = only_a(2, 1, 100.0);
    params.push((FOCUS, 100.0));
    let mut e = engine(&params);
    let y = run(&mut e, &x, 512);
    for k in 2..=8 {
        let hz = 110.0 * f64::from(k);
        let change = db(level(&y, hz, 1.0), level(&x, hz, 1.0));
        if k % 2 == 0 {
            assert!(change > 6.0, "partial {k}: {change} dB");
        } else {
            assert!(change.abs() < 1.5, "partial {k}: {change} dB");
        }
    }
}

#[test]
fn a_ratio_of_three_lifts_the_third_partial() {
    let x = sines(&[(440.0, 0.5), (1320.0, 0.05), (880.0, 0.05)], 2.0);
    for (num, lifted, kept) in [(3, 1320.0, 880.0), (2, 880.0, 1320.0)] {
        let mut params = only_a(num, 1, 100.0);
        params.push((FOCUS, 100.0));
        let mut e = engine(&params);
        let y = run(&mut e, &x, 128);
        let up = db(level(&y, lifted, 1.0), level(&x, lifted, 1.0));
        let same = db(level(&y, kept, 1.0), level(&x, kept, 1.0));
        assert!(up > 10.0, "{num}/1 lifts {lifted} by {up} dB");
        assert!(same.abs() < 1.0, "{num}/1 moves {kept} by {same} dB");
    }
}

#[test]
fn a_ratio_below_one_boosts_below_the_peak() {
    // A strong partial at 880 and a weaker one an octave below.
    let x = sines(&[(880.0, 0.5), (440.0, 0.05)], 2.0);
    let mut params = only_a(1, 2, 100.0);
    params.extend([(FOCUS, 100.0), (THRESHOLD, 3.0)]);
    let mut e = engine(&params);
    let y = run(&mut e, &x, 128);
    let up = db(level(&y, 440.0, 1.0), level(&x, 440.0, 1.0));
    assert!(up > 10.0, "{up} dB");
}

#[test]
fn mono_ignores_the_right_input_and_copies_left_to_right() {
    let l = saw(220.0, 0.3, 1.0);
    let r = noise(l.len(), 0.5, 9);
    let mut e = engine(&[]);
    let (yl, yr) = run2(&mut e, &l, &r, 100);
    assert_eq!(yl, yr);
    let mut alone = engine(&[]);
    assert_eq!(run(&mut alone, &l, 100), yl);
}

#[test]
fn stereo_filters_each_channel_after_the_same_targets() {
    // The right is the left at half level: filters are linear and the
    // banks follow the same targets with the same gains, so the right
    // comes out at exactly half the left.
    let l = saw(165.0, 0.1, 2.0);
    let r: Vec<f32> = l.iter().map(|s| 0.5 * s).collect();
    let mut params = only_a(2, 1, 100.0);
    params.extend([(MODE, 1.0), (FOCUS, 50.0)]);
    let mut e = engine(&params);
    let (yl, yr) = run2(&mut e, &l, &r, 333);
    assert!(yl.iter().zip(&yr).all(|(a, b)| 0.5 * a == *b));
    // Halfway to TRACK, the filters sit between their grid points and the
    // partials: less than the full lift.
    let up = db(level(&yr, 330.0, 1.0), level(&r, 330.0, 1.0));
    assert!(up > 3.0, "{up} dB");
    // And with different material each side, each is its own. The right's
    // sine is clear of the octaves of the left's partials (990, 1320 Hz)
    // and quieter, so the octave of it is boosted, not it.
    let other = sines(&[(1160.0, 0.05)], 2.0);
    let mut e = engine(&params);
    let (yl, yr) = run2(&mut e, &l, &other, 333);
    assert!(db(level(&yl, 330.0, 1.0), level(&l, 330.0, 1.0)) > 3.0);
    let one = db(level(&yr, 1160.0, 1.0), level(&other, 1160.0, 1.0));
    assert!(one.abs() < 1.0, "{one} dB");
}

/// A sine gliding `from` to `to` over 0.5 s after `hold` s, with its octave.
fn glide(from: f64, to: f64, hold: f64, seconds: f64) -> Vec<f32> {
    let rate = f64::from(RATE);
    let mut phase = 0.0;
    (0..(seconds * rate) as usize)
        .map(|n| {
            let t = n as f64 / rate;
            let hz = from * (to / from).powf(((t - hold) / 0.5).clamp(0.0, 1.0));
            phase += TAU * hz / rate;
            (0.4 * phase.sin() + 0.1 * (2.0 * phase).sin()) as f32
        })
        .collect()
}

#[test]
fn track_follows_a_glide_and_never_retunes_a_sounding_filter() {
    let x = glide(300.0, 450.0, 0.5, 2.0);
    for (focus, resonance) in [(100.0, 0.0), (100.0, 100.0), (0.0, 0.0), (50.0, 60.0)] {
        let mut e = engine(&[(FOCUS, focus), (RESONANCE, resonance)]);
        let y = run(&mut e, &x, 64);
        assert!(y.iter().all(|s| s.is_finite()));
        let stats = e.bank().stats;
        assert_eq!(stats.retunes_while_audible, 0, "FOCUS {focus}: {stats:?}");
        // No filter is stolen sooner than its hold.
        if stats.steals > 0 {
            assert!(stats.steal_soonest >= e.bank().hold_samples(), "{stats:?}");
        }
        assert_eq!(stats.tail_chokes_with_free, 0);
    }
    // At TRACK the octave's filter lands on 900 Hz.
    let mut e = engine(&[(FOCUS, 100.0)]);
    run(&mut e, &x, 64);
    let on = e
        .bank()
        .filters()
        .iter()
        .filter(|f| f.state == State::Active)
        .any(|f| (1200.0 * (f.hz / 900.0).log2()).abs() < 2.0);
    assert!(on, "{:?}", e.bank().filters().map(|f| (f.state, f.hz)));
}

#[test]
fn resonance_sings_on_a_strong_note_and_fades_with_it() {
    let mut x = sines(&[(440.0, 0.5)], 1.5);
    x.extend(vec![0.0; 48_000]);
    let mut params = only_a(2, 1, 100.0);
    params.extend([(FOCUS, 100.0), (RESONANCE, 100.0)]);
    let mut e = engine(&params);
    let n = 72_000 - 1;
    let sung = run(&mut e, &x[..n], 128);
    let singing: Vec<f64> = e
        .bank()
        .filters()
        .iter()
        .filter(|f| f.oscillating)
        .map(|f| f.hz)
        .collect();
    assert!(!singing.is_empty());
    for hz in &singing {
        let cents = 1200.0 * (hz / 880.0).log2();
        assert!(cents.abs() < 1.0, "{hz} Hz, {cents} cents");
    }
    // It sings: the output's 880 Hz is far above the input's (none).
    assert!(level(&sung, 880.0, 0.5) > 0.05);
    let rest = run(&mut e, &x[n..], 128);
    // Within half a second of the note's end it is 60 dB down.
    let tail = &rest[24_000..];
    let peak = tail.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak < 5e-4, "{peak}");
}

#[test]
fn below_oscillation_a_filter_rings_on_after_its_partial() {
    let mut x = sines(&[(440.0, 0.5)], 1.0);
    x.extend(vec![0.0; 24_000]);
    let mut params = only_a(2, 1, 100.0);
    params.extend([(FOCUS, 100.0), (RESONANCE, 45.0)]);
    let mut e = engine(&params);
    let y = run(&mut e, &x, 128);
    assert!(!e.bank().filters().iter().any(|f| f.oscillating));
    // Ringing: something is still there 20 ms after the input stops.
    let after = &y[48_000 + 960..48_000 + 1920];
    assert!(after.iter().any(|s| s.abs() > 1e-3));
    assert!(y[y.len() - 100..].iter().all(|s| s.abs() < 1e-4));
}

#[test]
fn loud_noise_with_everything_up_stays_bounded() {
    let x = noise(10 * 48_000, 1.6, 77);
    let params = [
        (A_AMOUNT, 100.0),
        (B_AMOUNT, 100.0),
        (RESONANCE, 100.0),
        (WIDTH, 200.0),
        (THRESHOLD, 0.0),
        (SPEED, 100.0),
    ];
    for focus in [0.0, 100.0] {
        let mut ps = params.to_vec();
        ps.push((FOCUS, focus));
        let mut e = engine(&ps);
        let y = run(&mut e, &x, 512);
        assert!(
            y.iter().all(|s| s.is_finite() && s.abs() <= 2.0),
            "FOCUS {focus}"
        );
    }
}

#[test]
fn the_output_is_the_same_however_the_input_is_split() {
    let l: Vec<f32> = glide(200.0, 330.0, 0.3, 1.5)
        .iter()
        .zip(noise(72_000, 0.05, 3))
        .map(|(a, b)| a + b)
        .collect();
    let r = saw(147.0, 0.2, 1.5);
    let params = [(MODE, 1.0), (FOCUS, 60.0), (RESONANCE, 70.0)];
    let mut e = engine(&params);
    let whole = run2(&mut e, &l, &r, l.len());
    let mut seed = 11u32;
    for sizes in [vec![1usize], vec![7], vec![64], vec![500], vec![]] {
        let mut e = engine(&params);
        let (mut yl, mut yr) = (l.clone(), r.clone());
        let mut at = 0;
        let mut blocks = 0;
        while at < yl.len() {
            let size = if sizes.is_empty() {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                1 + (seed >> 23) as usize
            } else {
                sizes[blocks % sizes.len()]
            };
            let n = size.min(yl.len() - at);
            e.process(&mut [&mut yl[at..at + n], &mut yr[at..at + n]]);
            at += n;
            blocks += 1;
        }
        assert!(whole.0 == yl && whole.1 == yr, "blocks {sizes:?}");
    }
}

#[test]
fn the_dry_path_has_the_latency_the_engine_says() {
    // With DRY CUT the wet side loses the input; the difference between
    // the two outputs is the input's wet share, wherever the engine puts
    // it. It must be on the input's own sample.
    assert_eq!(GlowUpEngine::LATENCY, 0);
    let at = 3001;
    let mut impulse = vec![0.0f32; 8192];
    impulse[at] = 0.5;
    let step: Vec<f32> = (0..8192)
        .map(|n| if n >= at { 0.25 } else { 0.0 })
        .collect();
    for rate in [44_100.0, 48_000.0, 96_000.0] {
        for mode in [0.0, 1.0] {
            for mix in [0.0, 50.0, 100.0] {
                let params = [(MODE, mode), (MIX, mix)];
                for x in [&impulse, &step] {
                    let mut on = params.to_vec();
                    on.push((DRY_CUT, 1.0));
                    let (y, _) = run2(&mut engine_at(rate, &params), x, x, 256);
                    let (cut, _) = run2(&mut engine_at(rate, &on), x, x, 256);
                    let wet = (0.5 * std::f32::consts::PI * mix * 0.01).sin();
                    let wet = if mix >= 100.0 { 1.0 } else { wet };
                    for n in 0..x.len() {
                        let d = y[n] - cut[n];
                        assert!(
                            (d - wet * x[n]).abs() < 1e-6,
                            "{rate} mode {mode} mix {mix}: \
                             sample {n} {d} against {}",
                            wet * x[n]
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_quiet_input_and_clicks_pass_untouched() {
    let quiet = sines(&[(440.0, 5e-4), (880.0, 2.5e-4)], 1.0);
    let mut e = engine(&only_a(2, 1, 100.0));
    assert_eq!(run(&mut e, &quiet, 128), quiet);
    let mut clicks = vec![0.0f32; 48_000];
    for n in (0..clicks.len()).step_by(4800) {
        clicks[n] = 0.8;
    }
    let mut e = engine(&only_a(2, 1, 100.0));
    assert_eq!(run(&mut e, &clicks, 128), clicks);
}

#[test]
fn every_grid_plays() {
    let x = saw(196.0, 0.3, 1.5);
    for grid in [0.0, 1.0, 2.0] {
        let mut e = engine(&[(GRID, grid), (A_AMOUNT, 60.0)]);
        let y = run(&mut e, &x, 256);
        let up = db(level(&y, 392.0, 0.5), level(&x, 392.0, 0.5));
        assert!(up > 0.5, "GRID {grid}: {up} dB");
    }
}

/// C3, E3, G3 and C4 as saws, held: the chord whose close and beating
/// partials made the bank warble (ticket 0043).
fn chord(seconds: f64) -> Vec<f32> {
    let mut parts = Vec::new();
    for hz in [130.81, 164.81, 196.0, 261.63] {
        for k in 1..60 {
            let f = hz * f64::from(k);
            if f < 16_000.0 {
                parts.push((f, 0.08 / f64::from(k)));
            }
        }
    }
    sines(&parts, seconds)
}

#[test]
fn a_held_chord_holds_its_filters_still() {
    let x = chord(3.0);
    for focus in [0.0, 50.0, 100.0] {
        let mut e = engine(&[(FOCUS, focus)]);
        let mut prev = [(State::Free, 0.0f64); glow_up::bank::POOL];
        let (mut starts, mut moved, mut held) = (0u32, 0.0f64, 0u32);
        let mut y = x.clone();
        for (b, chunk) in y.chunks_mut(glow_up::bank::BLOCK).enumerate() {
            e.process(&mut [chunk]);
            let after = b * glow_up::bank::BLOCK > 72_000;
            for (i, f) in e.bank().filters().iter().enumerate() {
                let (state, hz) = prev[i];
                if after && f.state == State::Active {
                    if state == State::Active {
                        moved += (1200.0 * (f.hz / hz).log2()).abs();
                        held += 1;
                    } else {
                        starts += 1;
                    }
                }
                prev[i] = (f.state, f.hz);
            }
        }
        // 1.5 s held. Before 0043: 4 to 27 starts a second, and sounding
        // filters moving 1.7 to 3.1 cents a block.
        let mean = moved / f64::from(held.max(1));
        assert!(starts <= 12, "FOCUS {focus}: {starts} starts in 1.5 s");
        assert!(mean < 0.3, "FOCUS {focus}: {mean} cents a block");
    }
}
