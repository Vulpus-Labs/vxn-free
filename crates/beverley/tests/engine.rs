//! The engine end to end: settings reach the chain, stereo modes, mix,
//! smoothing and latency.

use beverley::crush::{ATTACK_MS, Mode, Prepared, RELEASE_MS, Settings, auto_gain};
use beverley::engine::{BeverleyEngine, TICK};
use beverley::{AUTO_GAIN, BIT_DEPTH, CRUSH_MODE, GAIN, GAMMA, MIX, SMOOTHING, STEREO};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::PeakFollower;

const RATE: f32 = 48_000.0;

/// An engine at `values`, settled there.
fn engine(rate: f32, values: &[(ParamIndex, f32)]) -> BeverleyEngine {
    let mut e = BeverleyEngine::new(rate);
    for &(p, v) in values {
        e.set_param(p, v);
    }
    e.reset();
    e
}

fn sine(amp: f32, hz: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| amp * (std::f32::consts::TAU * hz * i as f32 / RATE).sin())
        .collect()
}

fn stereo(e: &mut BeverleyEngine, l: &[f32], r: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let (mut l, mut r) = (l.to_vec(), r.to_vec());
    e.process(&mut [&mut l, &mut r]);
    (l, r)
}

#[test]
fn a_sine_comes_out_as_the_chain_says() {
    let settings = Settings {
        gain: 10f32.powf(6.0 / 20.0),
        gamma: 10f32.powf(0.3),
        bits: 3.5,
        smoothing: 0.25,
        mode: Mode::Symmetric,
    };
    let mut e = engine(
        RATE,
        &[
            (GAIN, 6.0),
            (GAMMA, 0.3),
            (BIT_DEPTH, 3.5),
            (SMOOTHING, 25.0),
            (CRUSH_MODE, 1.0),
        ],
    );
    let x = sine(0.7, 440.0, 1000);
    let (l, r) = stereo(&mut e, &x, &x);
    let chain = Prepared::new(&settings);
    for (i, &x) in x.iter().enumerate() {
        let want = chain.crush(x, settings.gain);
        assert!((l[i] - want).abs() < 1e-5, "sample {i}: {} vs {want}", l[i]);
    }
    assert_eq!(l, r);
}

#[test]
fn one_channel_is_crushed_alone() {
    let mut e = engine(RATE, &[(BIT_DEPTH, 2.0)]);
    let x = sine(0.7, 440.0, 512);
    let mut one = x.clone();
    e.process(&mut [&mut one]);
    let chain = Prepared::new(&Settings {
        bits: 2.0,
        ..Settings::default()
    });
    assert!(one.iter().zip(&x).all(|(&y, &x)| y == chain.crush(x, 1.0)));
}

#[test]
fn mono_feeds_both_outputs_from_either_input() {
    let mut e = engine(RATE, &[(STEREO, 0.0), (BIT_DEPTH, 2.0)]);
    let x = sine(0.9, 440.0, 512);
    let silence = vec![0.0; 512];
    let (l, r) = stereo(&mut e, &x, &silence);
    assert_eq!(l, r, "left alone reaches both");
    let chain = Prepared::new(&Settings {
        bits: 2.0,
        ..Settings::default()
    });
    assert!(
        l.iter()
            .zip(&x)
            .all(|(&y, &x)| y == chain.crush(0.5 * x, 1.0))
    );
    let (l2, r2) = stereo(&mut e, &silence, &x);
    assert_eq!((l2, r2), (l, r), "and so does right alone");
}

/// Auto gain on a loud left and quiet right channel.
fn auto(stereo_mode: f32) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut e = engine(RATE, &[(AUTO_GAIN, 1.0), (STEREO, stereo_mode)]);
    let l = sine(0.8, 300.0, 4800);
    let r = sine(0.1, 300.0, 4800);
    let (ol, or) = stereo(&mut e, &l, &r);
    (l, r, ol, or)
}

#[test]
fn separate_auto_gain_follows_each_channel() {
    let (l, r, ol, or) = auto(1.0);
    let chain = Prepared::new(&Settings::default());
    let mut pl = PeakFollower::new(ATTACK_MS, RELEASE_MS, RATE);
    let mut pr = PeakFollower::new(ATTACK_MS, RELEASE_MS, RATE);
    for i in 0..l.len() {
        let wl = chain.crush(l[i], auto_gain(pl.next(l[i].abs())));
        let wr = chain.crush(r[i], auto_gain(pr.next(r[i].abs())));
        assert_eq!((ol[i], or[i]), (wl, wr), "sample {i}");
    }
}

#[test]
fn linked_auto_gain_follows_the_mid_for_both() {
    let (l, r, ol, or) = auto(2.0);
    let chain = Prepared::new(&Settings::default());
    let mut mid = PeakFollower::new(ATTACK_MS, RELEASE_MS, RATE);
    for i in 0..l.len() {
        let gain = auto_gain(mid.next((0.5 * (l[i] + r[i])).abs()));
        assert_eq!(
            (ol[i], or[i]),
            (chain.crush(l[i], gain), chain.crush(r[i], gain)),
            "sample {i}"
        );
    }
    // Not the same as separate: the quiet channel is crushed harder.
    let (_, _, _, separate_r) = auto(1.0);
    assert_ne!(or, separate_r);
}

#[test]
fn mix_blends_dry_and_wet() {
    let x = sine(0.6, 440.0, 512);
    let (dry, _) = stereo(&mut engine(RATE, &[(MIX, 0.0)]), &x, &x);
    assert_eq!(dry, x);
    let (wet, _) = stereo(&mut engine(RATE, &[]), &x, &x);
    let (half, _) = stereo(&mut engine(RATE, &[(MIX, 50.0)]), &x, &x);
    for i in 0..x.len() {
        assert!((half[i] - 0.5 * (x[i] + wet[i])).abs() < 1e-6);
    }
}

/// The largest step between consecutive samples of `y`.
fn largest_step(y: &[f32]) -> f32 {
    y.windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0, f32::max)
}

#[test]
fn automation_glides_rather_than_steps() {
    // A steady 0.3 at one bit is 1.0 wet. Dropping MIX from 100 to 0
    // moves the output from 1.0 to 0.3: in small steps, not one.
    let mut e = engine(RATE, &[(BIT_DEPTH, 1.0)]);
    let x = vec![0.3f32; 4800];
    let (before, _) = stereo(&mut e, &x[..480], &x[..480]);
    assert!(before.iter().all(|&y| (y - 1.0).abs() < 1e-6));
    e.set_param(MIX, 0.0);
    let (after, _) = stereo(&mut e, &x, &x);
    assert!(largest_step(&after) < 0.01, "{}", largest_step(&after));
    assert!((after[4799] - 0.3).abs() < 1e-4);

    // Likewise GAIN, with the chain transparent (full smoothing) and the
    // output clipping so the gain is heard: 0.5 at 0 dB passes, at
    // +20 dB clips to 0.1.
    let mut e = engine(RATE, &[(SMOOTHING, 100.0)]);
    let x = vec![0.5f32; 4800];
    stereo(&mut e, &x[..480], &x[..480]);
    e.set_param(GAIN, 20.0);
    let (after, _) = stereo(&mut e, &x, &x);
    assert!(largest_step(&after) < 0.01, "{}", largest_step(&after));
    assert!((after[4799] - 0.1).abs() < 1e-4);
}

#[test]
fn block_splits_do_not_change_the_output() {
    let x = sine(0.6, 440.0, 2000);
    let run = |sizes: &[usize]| {
        let mut e = engine(RATE, &[]);
        let mut out = x.clone();
        let mut at = 0;
        let mut k = 0;
        while at < out.len() {
            if at == 300 {
                e.set_param(BIT_DEPTH, 7.0);
                e.set_param(MIX, 40.0);
            }
            let n = sizes[k % sizes.len()].min(out.len() - at).min(if at < 300 {
                300 - at
            } else {
                usize::MAX
            });
            e.process(&mut [&mut out[at..at + n]]);
            at += n;
            k += 1;
        }
        out
    };
    assert_eq!(run(&[2000]), run(&[1, 7, 64, 3, TICK]));
}

/// Every setting that could move the output in time, at each rate.
fn every_setting() -> Vec<Vec<(ParamIndex, f32)>> {
    let mut all = Vec::new();
    for stereo in [0.0, 1.0, 2.0] {
        for auto in [0.0, 1.0] {
            for mode in [0.0, 1.0] {
                for (bits, smoothing) in [(1.0, 0.0), (4.0, 0.0), (5.5, 50.0), (16.0, 100.0)] {
                    for mix in [0.0, 50.0, 100.0] {
                        all.push(vec![
                            (STEREO, stereo),
                            (AUTO_GAIN, auto),
                            (CRUSH_MODE, mode),
                            (BIT_DEPTH, bits),
                            (SMOOTHING, smoothing),
                            (MIX, mix),
                            (GAMMA, 0.4),
                            (GAIN, 12.0),
                        ]);
                    }
                }
            }
        }
    }
    all
}

#[test]
fn latency_is_what_the_engine_says() {
    const AT: usize = 100;
    assert_eq!(BeverleyEngine::LATENCY, 0);
    let lag = BeverleyEngine::LATENCY as usize;
    for rate in [44_100.0, 48_000.0, 96_000.0] {
        for values in every_setting() {
            for input in ["impulse", "step"] {
                let mut x = vec![0.0f32; 400];
                match input {
                    "impulse" => x[AT] = 0.8,
                    _ => x[AT..].fill(0.8),
                }
                let mut e = engine(rate, &values);
                let (l, r) = stereo(&mut e, &x, &x);
                for y in [l, r] {
                    let first = y.iter().position(|&s| s != 0.0);
                    assert_eq!(first, Some(AT + lag), "{input} at {rate} Hz, {values:?}");
                }
            }
        }
    }
}
