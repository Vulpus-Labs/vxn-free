//! The tracking display (ticket 0009), driven by telemetry frames.
//!
//! Snapshots use frames built here by hand, not rendered by the engine, so
//! they don't move with the engine's rounding on each platform. What the
//! engine puts in a frame is checked separately, by property.

use std::f64::consts::TAU;

use glow_up::engine::GlowUpEngine;
use glow_up::observe::{FRAME_WORDS, FilterObs, Observation, PeakObs, Shown};
use glow_up::{A_AMOUNT, A_DEN, A_NUM, B_AMOUNT, FOCUS, RESONANCE};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use silkscreen_harness::{Harness, assert_snapshot};

const RATE: f64 = 48_000.0;

/// The engine's frame after `seconds` of a sine at `hz`, with A at 2/1.
fn engine_frame(hz: f64, params: &[(ParamIndex, f32)], seconds: f64) -> Observation {
    let mut e = GlowUpEngine::new(RATE as f32);
    for &(p, v) in [
        (A_AMOUNT, 100.0),
        (A_NUM, 1.0),
        (A_DEN, 0.0),
        (B_AMOUNT, 0.0),
    ]
    .iter()
    .chain(params)
    {
        e.set_param(p, v);
    }
    e.reset();
    let mut x: Vec<f32> = (0..(seconds * RATE) as usize)
        .map(|n| (0.4 * (TAU * hz * n as f64 / RATE).sin()) as f32)
        .collect();
    for chunk in x.chunks_mut(256) {
        e.process(&mut [chunk]);
    }
    let mut got = None;
    e.frame(&mut |f| got = Observation::from_frame(f));
    got.expect("a frame of the right size")
}

#[test]
fn the_engine_reports_its_peaks_and_filters() {
    let obs = engine_frame(440.0, &[(FOCUS, 100.0)], 1.0);
    assert_eq!(obs.time, 48_000);
    assert_eq!(obs.sample_rate, 48_000.0);
    let peaks = &obs.peaks[..usize::from(obs.peak_count)];
    assert!(
        peaks
            .iter()
            .any(|p| (p.hz - 440.0).abs() < 1.0 && p.prominence > 0.9),
        "{peaks:?}"
    );
    let boosting: Vec<&FilterObs> = obs
        .filters
        .iter()
        .filter(|f| f.shown == Shown::Boosting)
        .collect();
    assert!(
        boosting
            .iter()
            .any(|f| (f.hz - 880.0).abs() < 1.0 && f.level > 0.9),
        "{boosting:?}"
    );
    // With RESONANCE up, it sings.
    let obs = engine_frame(440.0, &[(FOCUS, 100.0), (RESONANCE, 100.0)], 1.0);
    assert!(
        obs.filters
            .iter()
            .any(|f| f.shown == Shown::Singing && (f.hz - 880.0).abs() < 1.0),
        "{:?}",
        obs.filters
    );
}

#[test]
fn at_most_the_most_prominent_peaks_are_sent() {
    // Noise has many peaks; the frame keeps the most prominent.
    let mut e = GlowUpEngine::new(RATE as f32);
    e.set_param(glow_up::THRESHOLD, 0.0);
    e.reset();
    let mut seed = 3u32;
    let mut x: Vec<f32> = (0..48_000)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect();
    for chunk in x.chunks_mut(256) {
        e.process(&mut [chunk]);
    }
    let mut got = None;
    e.frame(&mut |f| got = Observation::from_frame(f));
    let obs = got.unwrap();
    assert!(usize::from(obs.peak_count) <= glow_up::observe::PEAKS);
    let shown = &obs.peaks[..usize::from(obs.peak_count)];
    assert!(shown.windows(2).all(|w| w[0].hz < w[1].hz), "{shown:?}");
}

/// A note at `hz` gliding by `bend` semitones over the span, with its
/// first eight partials as peaks and filters at 2/1 and 3/2 of the
/// strongest, at `k` of `n` frames 10 ms apart.
fn frame(k: usize, n: usize, singing: bool) -> Vec<f32> {
    let t = k as f32 / n as f32;
    let hz = 196.0 * 2f32.powf(2.0 * ((t - 0.35) / 0.3).clamp(0.0, 1.0) / 12.0);
    let mut obs = Observation {
        time: (k * 480) as u64,
        ..Observation::default()
    };
    for (i, p) in obs.peaks.iter_mut().take(8).enumerate() {
        *p = PeakObs {
            hz: hz * (i + 1) as f32,
            prominence: 1.0 / (1.0 + 0.4 * i as f32),
        };
    }
    obs.peak_count = 8;
    let mut slot = 0;
    for (i, ratio) in [(0usize, 2.0f32), (0, 1.5), (1, 2.0), (2, 2.0), (1, 1.5)] {
        let level = 0.9 / (1.0 + 0.5 * i as f32) * (t * 4.0).min(1.0);
        obs.filters[slot] = FilterObs {
            hz: hz * (i + 1) as f32 * ratio,
            level,
            shown: if singing && slot == 0 && t > 0.6 {
                Shown::Singing
            } else {
                Shown::Boosting
            },
        };
        slot += 1;
    }
    // A filter on a grid point that faded as the note left it.
    if t < 0.5 {
        obs.filters[slot] = FilterObs {
            hz: 1250.0,
            level: 0.5,
            shown: if t < 0.4 {
                Shown::Boosting
            } else {
                Shown::Fading
            },
        };
    }
    let mut out = [0.0; FRAME_WORDS];
    obs.to_frame(&mut out);
    out.to_vec()
}

fn played(singing: bool) -> Harness {
    let mut h = Harness::new(glow_up::editor, 1.0);
    let n = 200;
    for k in 0..n {
        h.engine_frame(&frame(k, n, singing));
        h.advance(10.0);
    }
    h
}

#[test]
fn the_display_draws_the_bank_over_time() {
    let mut h = played(false);
    assert_snapshot!(h, "display_bend_1x");
}

#[test]
fn a_singing_filter_burns_bright() {
    let mut h = played(true);
    assert_snapshot!(h, "display_singing_1x");
}

#[test]
fn with_the_host_stopped_the_traces_dim() {
    let mut h = played(false);
    h.advance(500.0);
    assert_snapshot!(h, "display_stopped_1x");
}
