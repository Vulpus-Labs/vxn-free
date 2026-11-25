//! Wesley's engine against ticket 0033's acceptance criteria.

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::ThreeBand;
use wesley::engine::WesleyEngine;
use wesley::{BANDS, BandParam, HIGH, LOW, MID, MIX, PARAMS, band_param};

const RATE: f32 = 48_000.0;

/// White noise, repeatable.
fn noise(len: usize, mut seed: u32) -> Vec<f32> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect()
}

fn sine(len: usize, hz: f32, amp: f32) -> Vec<f32> {
    (0..len)
        .map(|n| amp * (n as f32 * hz * std::f32::consts::TAU / RATE).sin())
        .collect()
}

fn engine(params: &[(ParamIndex, f32)]) -> WesleyEngine {
    let mut e = WesleyEngine::new(RATE);
    for &(p, v) in params {
        e.set_param(p, v);
    }
    e.reset();
    e
}

/// Stereo through `e` in blocks of `block`.
fn render(e: &mut WesleyEngine, l: &[f32], r: &[f32], block: usize) -> (Vec<f32>, Vec<f32>) {
    let (mut l, mut r) = (l.to_vec(), r.to_vec());
    for (cl, cr) in l.chunks_mut(block).zip(r.chunks_mut(block)) {
        e.process(&mut [cl, cr]);
    }
    (l, r)
}

/// `x` through the default split, with `keep` choosing which bands are
/// summed.
fn split_sum(x: &[f32], keep: [bool; BANDS]) -> Vec<f32> {
    let mut t = ThreeBand::new(200.0, 2500.0, RATE);
    x.iter()
        .map(|&s| {
            let b = t.split(s);
            (0..BANDS).filter(|&i| keep[i]).map(|i| b[i]).sum()
        })
        .collect()
}

fn all_off() -> Vec<(ParamIndex, f32)> {
    (0..BANDS)
        .map(|b| (band_param(b, BandParam::On), 0.0))
        .collect()
}

fn worst(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

#[test]
fn every_band_off_is_the_split_summed() {
    let x = noise(48_000, 1);
    let want = split_sum(&x, [true; 3]);
    for extra in [vec![], vec![(MIX, 50.0)], vec![(MIX, 0.0)]] {
        let mut set = all_off();
        set.extend(extra);
        let (l, r) = render(&mut engine(&set), &x, &x, 64);
        assert!(worst(&l, &want) < 1e-6, "{set:?}: {}", worst(&l, &want));
        assert!(worst(&r, &want) < 1e-6);
    }
}

#[test]
fn every_band_mix_at_zero_is_the_split_summed() {
    let x = noise(48_000, 2);
    let set: Vec<_> = (0..BANDS)
        .flat_map(|b| {
            [
                (band_param(b, BandParam::On), 1.0),
                (band_param(b, BandParam::Mix), 0.0),
            ]
        })
        .collect();
    let (l, _) = render(&mut engine(&set), &x, &x, 64);
    assert!(worst(&l, &split_sum(&x, [true; 3])) < 1e-6);
}

#[test]
fn a_partial_mix_doesnt_comb_at_the_crossovers() {
    // Bands crushed hard, MIX 50 %: the dry half is the split summed, in
    // phase with the crushed half, so there is no notch at a crossover. A
    // sine at each crossover keeps its level within a fraction of a dB
    // of the crushed output's and the dry's average.
    for hz in [200.0, 2500.0] {
        let x = sine(48_000, hz, 0.5);
        let crush: Vec<_> = (0..BANDS)
            .flat_map(|b| {
                [
                    (band_param(b, BandParam::On), 1.0),
                    (band_param(b, BandParam::BitDepth), 16.0),
                ]
            })
            .collect();
        let mut half = crush.clone();
        half.push((MIX, 50.0));
        let (out, _) = render(&mut engine(&half), &x, &x, 64);
        let level = 20.0 * (rms(&out[24_000..]) / rms(&x[24_000..])).log10();
        assert!(level.abs() < 0.1, "{hz} Hz at MIX 50 %: {level} dB");
    }
}

#[test]
fn at_the_defaults_the_bass_passes_clean() {
    // LOW is off: a 60 Hz sine comes out as the split passes it, but for
    // what MID's and HIGH's crushers make of the little of it that leaks
    // into their bands (auto gain lifts it before crushing).
    let x = sine(96_000, 60.0, 0.5);
    let (out, _) = render(&mut engine(&[]), &x, &x, 64);
    let want = split_sum(&x, [true; 3]);
    let err: Vec<f32> = out.iter().zip(&want).map(|(a, b)| a - b).collect();
    let db = 20.0 * (rms(&err[48_000..]) / rms(&want[48_000..])).log10();
    println!("60 Hz at the defaults: difference from clean {db:.1} dB");
    assert!(db < -30.0, "{db} dB");
    // With LOW on, the bass is crushed: 4 bits leaves the error about
    // 26 dB under the signal (6 dB a bit), against −66 dB clean.
    let (on, _) = render(
        &mut engine(&[(band_param(LOW, BandParam::On), 1.0)]),
        &x,
        &x,
        64,
    );
    let err: Vec<f32> = on.iter().zip(&want).map(|(a, b)| a - b).collect();
    let crushed = 20.0 * (rms(&err[48_000..]) / rms(&want[48_000..])).log10();
    println!("60 Hz with LOW on: {crushed:.1} dB");
    assert!(crushed > -35.0, "{crushed} dB");
}

#[test]
fn solo_hears_only_the_soloed_bands() {
    let x = noise(48_000, 3);
    let mut set = all_off();
    set.push((band_param(MID, BandParam::Solo), 1.0));
    let (out, _) = render(&mut engine(&set), &x, &x, 64);
    assert!(worst(&out, &split_sum(&x, [false, true, false])) < 1e-6);
    set.push((band_param(HIGH, BandParam::Solo), 1.0));
    let (out, _) = render(&mut engine(&set), &x, &x, 64);
    assert!(worst(&out, &split_sum(&x, [false, true, true])) < 1e-6);
}

#[test]
fn level_scales_its_band() {
    let x = noise(48_000, 4);
    let mut set = all_off();
    set.push((band_param(HIGH, BandParam::Level), -6.0));
    let (out, _) = render(&mut engine(&set), &x, &x, 64);
    let mut t = ThreeBand::new(200.0, 2500.0, RATE);
    let g = 10f32.powf(-6.0 / 20.0);
    let want: Vec<f32> = x
        .iter()
        .map(|&s| {
            let b = t.split(s);
            b[0] + b[1] + g * b[2]
        })
        .collect();
    assert!(worst(&out, &want) < 1e-6);
}

#[test]
fn switching_a_band_fades_rather_than_clicks() {
    // MID switched on part way: the change it makes starts from nothing.
    let x = noise(24_000, 5);
    let run = |switch: bool| {
        let mut e = engine(&[(band_param(MID, BandParam::BitDepth), 2.0)]);
        e.set_param(band_param(MID, BandParam::On), 0.0);
        e.reset();
        let (mut l, mut r) = (x.clone(), x.clone());
        let at = 12_000;
        e.process(&mut [&mut l[..at], &mut r[..at]]);
        if switch {
            e.set_param(band_param(MID, BandParam::On), 1.0);
        }
        e.process(&mut [&mut l[at..], &mut r[at..]]);
        l
    };
    let (off, switched) = (run(false), run(true));
    let diff: Vec<f32> = off.iter().zip(&switched).map(|(a, b)| a - b).collect();
    assert!(diff[..12_000].iter().all(|&d| d == 0.0));
    let settled = rms(&diff[18_000..]);
    let first = diff[12_000..12_048]
        .iter()
        .fold(0.0f32, |m, d| m.max(d.abs()));
    assert!(settled > 0.01);
    assert!(first < 0.25 * settled, "{first} against {settled}");
}

#[test]
fn output_is_the_same_whatever_the_block_size() {
    let (l, r) = (noise(20_000, 6), noise(20_000, 7));
    let set = vec![
        (band_param(LOW, BandParam::On), 1.0),
        (band_param(MID, BandParam::BitDepth), 2.3),
        (band_param(HIGH, BandParam::Smoothing), 40.0),
        (MIX, 80.0),
    ];
    let (want_l, want_r) = render(&mut engine(&set), &l, &r, 20_000);
    for block in [1, 7, 32, 33, 512] {
        let (got_l, got_r) = render(&mut engine(&set), &l, &r, block);
        assert!(
            got_l
                .iter()
                .zip(&want_l)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "block {block}"
        );
        assert!(
            got_r
                .iter()
                .zip(&want_r)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "block {block}"
        );
    }
}

#[test]
fn a_mono_input_is_processed() {
    let x = noise(10_000, 8);
    let mut e = engine(&all_off());
    let mut one = x.clone();
    e.process(&mut [&mut one]);
    assert!(worst(&one, &split_sum(&x, [true; 3])) < 1e-6);
}

#[test]
fn every_param_has_an_effect_or_is_known_to_need_one() {
    // A smoke test of the mapping: moving each param from its default
    // changes the output of a band that is on, crushed and heard.
    let x = noise(24_000, 9);
    let base: Vec<_> = vec![
        (band_param(LOW, BandParam::On), 1.0),
        (band_param(LOW, BandParam::AutoGain), 0.0),
        (band_param(MID, BandParam::AutoGain), 0.0),
        (band_param(HIGH, BandParam::AutoGain), 0.0),
    ];
    let (want, _) = render(&mut engine(&base), &x, &x, 64);
    for (i, spec) in PARAMS.iter().enumerate() {
        let p = ParamIndex(i as u32);
        if base.iter().any(|&(q, _)| q == p) {
            continue;
        }
        // Toggles flip; ranges go to whichever end is further.
        let v = if (spec.max - spec.default).abs() > (spec.default - spec.min).abs() {
            spec.max
        } else {
            spec.min
        };
        let mut set = base.clone();
        set.push((p, v));
        let (got, _) = render(&mut engine(&set), &x, &x, 64);
        // STEREO and DETECT need a stereo difference or auto gain to act.
        let quiet = ["Stereo", "Detect", "Decay"].contains(&spec.name);
        assert!(
            quiet || worst(&got, &want) > 1e-4,
            "{} had no effect",
            spec.name
        );
    }
}
