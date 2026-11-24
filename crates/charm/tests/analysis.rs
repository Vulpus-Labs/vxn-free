//! Charm's analysis sidechain against ticket 0015's acceptance criteria.

use charm::analysis::window_for;
use charm::engine::{CharmEngine, TICK};
use charm::observe::{HIGH, MID, Observation};
use charm::{LOW_XOVER, MIX};
use silkscreen::bridge::Engine;
use std::f32::consts::TAU;

fn engine(rate: f32) -> CharmEngine {
    let mut e = CharmEngine::new(rate);
    e.set_param(LOW_XOVER, 150.0);
    e.set_param(MIX, 50.0);
    e.reset();
    e
}

/// Run stereo input a tick at a time, keeping every tick's observation.
fn observe(rate: f32, l: &[f32], r: &[f32]) -> Vec<Observation> {
    let mut e = engine(rate);
    let (mut l, mut r) = (l.to_vec(), r.to_vec());
    let mut out = Vec::new();
    for (cl, cr) in l.chunks_mut(TICK).zip(r.chunks_mut(TICK)) {
        e.process(&mut [cl, cr]);
        out.push(*e.observation());
    }
    out
}

fn noise(len: usize, mut seed: u32) -> Vec<f32> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect()
}

#[test]
fn an_impulse_trains_onsets_are_found_in_frames_that_hold_them() {
    for rate in [44_100.0f32, 48_000.0, 96_000.0] {
        let window = window_for(rate) as u64;
        let every = (rate * 0.25) as usize;
        let mut x = vec![0.0f32; every * 8];
        let clicks: Vec<usize> = (1..8).map(|k| k * every + 17 * k).collect();
        for &t in &clicks {
            x[t] = 0.9;
        }
        let obs = observe(rate, &x, &x);
        for band in [MID, HIGH] {
            let mut found: Vec<u64> = obs
                .iter()
                .filter(|o| o.bands[band].onset > 0.0)
                .map(|o| o.bands[band].onset_start)
                .collect();
            found.dedup();
            assert_eq!(found.len(), clicks.len(), "{rate} band {band}: {found:?}");
            for (&start, &t) in found.iter().zip(&clicks) {
                // The onset's frame holds the click.
                let t = t as u64;
                assert!(
                    start <= t && t < start + window,
                    "{rate}: click {t}, frame {start}"
                );
            }
        }
    }
}

#[test]
fn sines_above_the_crossover_are_peaks_within_a_bin() {
    let rate = 48_000.0;
    let bin = rate / window_for(rate) as f32;
    let (f_mid, f_high) = (1234.0f32, 6789.0f32);
    let x: Vec<f32> = (0..rate as usize)
        .map(|n| {
            let t = n as f32 / rate;
            0.3 * (TAU * f_mid * t).sin() + 0.2 * (TAU * f_high * t).sin()
        })
        .collect();
    let obs = observe(rate, &x, &x);
    let last = obs.last().unwrap();
    let mid = last.bands[MID].peaks[0];
    let high = last.bands[HIGH].peaks[0];
    assert!((mid.hz - f_mid).abs() < bin, "{mid:?}");
    assert!((high.hz - f_high).abs() < bin, "{high:?}");
    assert!((mid.strength - 0.3).abs() < 0.05, "{mid:?}");
    assert!((high.strength - 0.2).abs() < 0.05, "{high:?}");
}

#[test]
fn coherence_reads_the_channels_relation() {
    let rate = 48_000.0;
    let a = noise(rate as usize * 2, 1);
    let b = noise(rate as usize * 2, 2);
    let neg: Vec<f32> = a.iter().map(|x| -x).collect();
    for (r, want) in [(&a, 1.0f32), (&b, 0.0), (&neg, -1.0)] {
        let obs = observe(rate, &a, r);
        let last = obs.last().unwrap();
        for band in [MID, HIGH] {
            let c = last.bands[band].coherence;
            assert!((c - want).abs() < 0.1, "want {want}, band {band}: {c}");
        }
    }
}

#[test]
fn the_analysis_is_the_same_however_the_host_splits_blocks() {
    let rate = 48_000.0;
    let mut l = noise(rate as usize, 3);
    // Clicks for onsets.
    for t in (4000..l.len()).step_by(9000) {
        l[t] = 1.0;
    }
    let r = noise(rate as usize, 4);
    // Observations at every multiple of 512 samples, which every split
    // below lands on.
    let run = |size: usize| {
        let mut e = engine(rate);
        let (mut cl, mut cr) = (l.clone(), r.clone());
        let mut at512 = Vec::new();
        let mut at = 0;
        while at < cl.len() {
            let n = size.min(cl.len() - at);
            e.process(&mut [&mut cl[at..at + n], &mut cr[at..at + n]]);
            at += n;
            if at % 512 == 0 {
                at512.push(*e.observation());
            }
        }
        at512
    };
    let whole = run(512);
    assert_eq!(run(1), whole);
    assert_eq!(run(64), whole);
}

#[test]
fn the_analysis_does_not_touch_the_audio() {
    // A loud click train changes what the analysis reports; the output
    // must be the same as before 0015, which the engine tests pin down.
    // Here: the dry path at sample 0 is exact, and nothing is heard
    // before the floor.
    let rate = 48_000.0;
    let mut e = engine(rate);
    let mut x = vec![0.0f32; 1024];
    x[0] = 1.0;
    let mut y = x.clone();
    e.process(&mut [&mut x, &mut y]);
    assert_eq!(x[0], 1.0);
    assert!(x[1..300].iter().all(|&s| s == 0.0));
}
