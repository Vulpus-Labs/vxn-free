//! The scope (ticket 0009), driven by telemetry frames.
//!
//! Snapshots use frames built here by hand, not rendered by the engine, so
//! they don't move with the engine's rounding on each platform. What the
//! engine puts in a frame is checked separately, by property.

use std::f64::consts::TAU;

use glow_up::engine::GlowUpEngine;
use glow_up::observe::{
    FRAME_WORDS, FilterObs, Observation, PeakObs, SPECTRUM, Shown, spectrum_hz,
};
use glow_up::{A_AMOUNT, A_DIV, A_MULT, B_AMOUNT, FOCUS, RESONANCE};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use silkscreen_harness::{Harness, assert_snapshot};

const RATE: f64 = 48_000.0;

/// The engine's frame after `seconds` of a sine at `hz`, with A at 2/1.
fn engine_frame(hz: f64, params: &[(ParamIndex, f32)], seconds: f64) -> Observation {
    let mut e = GlowUpEngine::new(RATE as f32);
    for &(p, v) in [
        (A_AMOUNT, 100.0),
        (A_MULT, 1.0),
        (A_DIV, 0.0),
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
    // The input's spectrum: the sine at its level (0.4, −8 dB), little
    // far from it.
    let nearest = (0..SPECTRUM)
        .min_by(|&a, &b| {
            (spectrum_hz(a) / 440.0)
                .ln()
                .abs()
                .total_cmp(&(spectrum_hz(b) / 440.0).ln().abs())
        })
        .unwrap();
    let at = obs.spectrum[nearest];
    assert!(
        (at - 20.0 * 0.4f32.log10()).abs() < 1.5,
        "{at} dB at 440 Hz"
    );
    let far = obs.spectrum[(0..SPECTRUM).find(|&i| spectrum_hz(i) > 5000.0).unwrap()];
    assert!(far < -60.0, "{far} dB at 5 kHz");
    // The output over the input: the full boost at 880 Hz, flat far off.
    let boost = 20.0 * obs.response(880.0).log10();
    assert!((boost - 13.98).abs() < 0.3, "{boost} dB at 880 Hz");
    assert!((20.0 * obs.response(3000.0).log10()).abs() < 0.3);
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

/// Hann's magnitude response `d` bins off a sine's centre, its main lobe
/// alone.
fn lobe(d: f32) -> f32 {
    let d = d.abs();
    if d >= 2.0 {
        return 0.0;
    }
    if d < 1e-6 {
        return 1.0;
    }
    if (d - 1.0).abs() < 1e-4 {
        return 0.5;
    }
    let x = std::f32::consts::PI * d;
    (x.sin() / x / (1.0 - d * d)).abs()
}

/// The spectrum the analysis would show of a saw at `hz` (partials 0.3/k)
/// at 2048, over a floor at −84 dB: each point the loudest of its share.
fn saw_spectrum(hz: f32) -> [f32; SPECTRUM] {
    let bin = 48_000.0 / 2048.0;
    let mut out = [0.0; SPECTRUM];
    let half = (spectrum_hz(1) / spectrum_hz(0)).sqrt();
    for (i, v) in out.iter_mut().enumerate() {
        let centre = spectrum_hz(i);
        let mut most = 10f32.powf(-84.0 / 20.0);
        for s in 0..8 {
            let f = centre / half * half.powf(2.0 * s as f32 / 7.0);
            for k in 1..60 {
                let p = hz * k as f32;
                if p > 18_000.0 {
                    break;
                }
                most = most.max(0.3 / k as f32 * lobe((f - p) / bin));
            }
        }
        *v = 20.0 * most.log10();
    }
    out
}

/// A note at `hz` gliding up a tone, then held, as the scope gets it at
/// frame `k` of `n` 10 ms apart: its partials as peaks, filters at 2/1
/// and 3/2 of the strongest, one singing at the end if `singing`, and a
/// filter on a grid point fading as the note leaves it.
fn frame(k: usize, n: usize, singing: bool) -> Vec<f32> {
    let t = k as f32 / n as f32;
    let hz = 196.0 * 2f32.powf(2.0 * ((t - 0.35) / 0.3).clamp(0.0, 1.0) / 12.0);
    let k0 = 2.0 * (2f32.ln() * 50.0 / 2400.0).sinh();
    let mut obs = Observation {
        time: (k * 480) as u64,
        k0,
        spectrum: saw_spectrum(hz),
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
        let sings = singing && slot == 0 && t > 0.6;
        obs.filters[slot] = FilterObs {
            hz: hz * (i + 1) as f32 * ratio,
            level: 0.9 / (1.0 + 0.5 * i as f32) * (t * 4.0).min(1.0),
            damping: if sings { 0.0 } else { k0 },
            shown: if sings {
                Shown::Singing
            } else {
                Shown::Boosting
            },
        };
        slot += 1;
    }
    if t < 0.5 {
        obs.filters[slot] = FilterObs {
            hz: 1250.0,
            level: if t < 0.4 { 0.5 } else { 0.5 * (0.5 - t) * 10.0 },
            damping: k0,
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

/// The same, with DRY CUT on at full MIX: only the filters are heard.
fn dry_cut(k: usize, n: usize) -> Vec<f32> {
    let mut obs = Observation::from_frame(&frame(k, n, false)).unwrap();
    obs.direct = 0.0;
    let mut out = [0.0; FRAME_WORDS];
    obs.to_frame(&mut out);
    out.to_vec()
}

fn played(make: impl Fn(usize, usize) -> Vec<f32>) -> Harness {
    let mut h = Harness::new(glow_up::editor, 1.0);
    let n = 200;
    for k in 0..n {
        h.engine_frame(&make(k, n));
        h.advance(10.0);
    }
    h
}

#[test]
fn the_scope_draws_the_input_and_the_boosts_over_it() {
    let mut h = played(|k, n| frame(k, n, false));
    assert_snapshot!(h, "display_bend_1x");
}

#[test]
fn a_singing_filter_burns_bright() {
    let mut h = played(|k, n| frame(k, n, true));
    assert_snapshot!(h, "display_singing_1x");
}

#[test]
fn with_dry_cut_the_output_falls_under_the_input() {
    let mut h = played(dry_cut);
    assert_snapshot!(h, "display_dry_cut_1x");
}

#[test]
fn with_the_host_stopped_the_scope_dims() {
    let mut h = played(|k, n| frame(k, n, false));
    h.advance(500.0);
    assert_snapshot!(h, "display_stopped_1x");
}
