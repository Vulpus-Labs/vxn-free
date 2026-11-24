//! Charm's analysis sidechain against ticket 0015's acceptance criteria.

use charm::analysis::REFERENCE_SECONDS;
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
fn an_impulse_trains_onsets_are_found_promptly_and_once() {
    for rate in [44_100.0f32, 48_000.0, 96_000.0] {
        let every = (rate * 0.25) as usize;
        let mut x = vec![0.0f32; every * 8];
        let clicks: Vec<usize> = (1..8).map(|k| k * every + 17 * k).collect();
        for &t in &clicks {
            x[t] = 0.9;
        }
        let obs = observe(rate, &x, &x);
        for band in [MID, HIGH] {
            let found: Vec<(u64, u64)> = obs
                .iter()
                .filter(|o| o.bands[band].onset > 0.0)
                .map(|o| (o.bands[band].onset_start, o.tick * TICK as u64))
                .collect();
            assert_eq!(found.len(), clicks.len(), "{rate} band {band}: {found:?}");
            for (&(start, seen), &t) in found.iter().zip(&clicks) {
                // Marked as starting before the click, and reported within
                // 3 ms of it.
                let t = t as u64;
                assert!(start <= t, "{rate}: click {t}, marked from {start}");
                assert!(
                    seen > t && seen - t <= (0.003 * rate) as u64,
                    "{rate}: click {t}, seen at {seen}"
                );
            }
        }
    }
}

#[test]
fn sustained_sounds_give_no_onsets_after_their_start() {
    let rate = 48_000.0;
    let len = rate as usize * 3;
    // A harmonic tone whose partials beat in shared bands, and noise.
    let tone: Vec<f32> = (0..len)
        .map(|n| {
            let t = n as f32 / rate;
            (1..=12)
                .map(|k| 0.4 / k as f32 * (TAU * 220.0 * k as f32 * t).sin())
                .sum()
        })
        .collect();
    let noise = noise(len, 7);
    for x in [&tone, &noise] {
        let obs = observe(rate, x, x);
        let after_start = (2.0 * REFERENCE_SECONDS * rate) as u64 / TICK as u64;
        for band in [MID, HIGH] {
            let late = obs
                .iter()
                .filter(|o| o.tick > after_start && o.bands[band].onset > 0.0)
                .count();
            assert_eq!(late, 0, "band {band}");
        }
    }
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
