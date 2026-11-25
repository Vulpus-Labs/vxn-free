//! Measurements for the manual, taken by running Wesley's engine as a
//! host does: params set, the engine reset so they hold from the first
//! sample, a signal processed. Each table is a CSV with a header row.

use std::fmt::Write as _;
use std::path::Path;

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use wesley::engine::WesleyEngine;
use wesley::{
    BandParam, HIGH, HIGH_XOVER, LOW, LOW_XOVER, MID, MIX, PARAMS, SOLO, STEREO, band_param,
};

const RATE: f32 = 48_000.0;
const TAU: f64 = std::f64::consts::TAU;
/// STEREO's SEPARATE position: one channel is crushed on its own.
const SEPARATE: f32 = 1.0;

pub fn write(out: &Path) {
    bands(out);
    low_on(out);
    decay(out);
    tapers(out);
    params(out);
}

/// An engine at `RATE` with `values` set and settled, running one channel.
fn engine(values: &[(ParamIndex, f32)]) -> WesleyEngine {
    let mut e = WesleyEngine::new(RATE);
    e.set_param(STEREO, SEPARATE);
    for &(p, v) in values {
        e.set_param(p, v);
    }
    e.reset();
    e
}

/// `input` through an engine with `values`.
fn run(values: &[(ParamIndex, f32)], input: &[f32]) -> Vec<f32> {
    let mut e = engine(values);
    let mut x = input.to_vec();
    for block in x.chunks_mut(512) {
        e.process(&mut [block]);
    }
    x
}

fn csv(out: &Path, name: &str, header: &[&str], rows: &[Vec<f32>]) {
    let mut text = header.join(",");
    text.push('\n');
    for row in rows {
        let cells: Vec<String> = row.iter().map(|v| format!("{v:.5}")).collect();
        text.push_str(&cells.join(","));
        text.push('\n');
    }
    std::fs::write(out.join(format!("{name}.csv")), text).expect("write a CSV");
}

/// Columns of equal length into rows.
fn rows(columns: &[Vec<f32>]) -> Vec<Vec<f32>> {
    (0..columns[0].len())
        .map(|i| columns.iter().map(|c| c[i]).collect())
        .collect()
}

/// The magnitude of `h`'s spectrum at `hz`, in dB.
fn response_db(h: &[f32], hz: f64) -> f32 {
    let w = TAU * hz / f64::from(RATE);
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (n, &v) in h.iter().enumerate() {
        let phase = w * n as f64;
        re += f64::from(v) * phase.cos();
        im -= f64::from(v) * phase.sin();
    }
    (10.0 * (re * re + im * im).max(1e-12).log10()) as f32
}

/// The power of `signal` at `cycles` whole cycles over its length, as dB
/// against a full-scale sine.
fn level_db(signal: &[f32], cycles: f64) -> f32 {
    let len = signal.len() as f64;
    let omega = TAU * cycles / len;
    let coeff = 2.0 * omega.cos();
    let (mut prev, mut prev2) = (0.0f64, 0.0f64);
    for &v in signal {
        let next = f64::from(v) + coeff * prev - prev2;
        prev2 = prev;
        prev = next;
    }
    let re = prev - prev2 * omega.cos();
    let im = prev2 * omega.sin();
    let amp = 2.0 * (re * re + im * im).sqrt() / len;
    (20.0 * amp.max(1e-6).log10()) as f32
}

/// Every band switched off, so each passes uncrushed.
fn all_off() -> Vec<(ParamIndex, f32)> {
    [LOW, MID, HIGH]
        .iter()
        .map(|&b| (band_param(b, BandParam::On), 0.0))
        .collect()
}

/// The split: each band's frequency response (soloed, uncrushed), and
/// the three together, at three crossover settings. An impulse through
/// the engine, its spectrum at 61 frequencies from 20 Hz to 20 kHz.
/// Columns `hz`, then for setting `i`, `{i}_low`, `{i}_mid`, `{i}_high`,
/// `{i}_sum`.
fn bands(out: &Path) {
    let settings = [(100.0, 1000.0), (200.0, 2500.0), (500.0, 8000.0)];
    let freqs: Vec<f64> = (0..=60)
        .map(|i| 20.0 * 1000f64.powf(f64::from(i) / 60.0))
        .collect();
    let mut impulse = vec![0.0f32; RATE as usize];
    impulse[0] = 1.0;
    let mut header = vec!["hz".to_owned()];
    let mut columns = vec![freqs.iter().map(|&f| f as f32).collect::<Vec<_>>()];
    for (i, &(lo, hi)) in settings.iter().enumerate() {
        for (name, solo) in [("low", 1.0), ("mid", 2.0), ("high", 3.0), ("sum", 0.0)] {
            let mut values = all_off();
            values.extend([(LOW_XOVER, lo), (HIGH_XOVER, hi), (SOLO, solo)]);
            let h = run(&values, &impulse);
            header.push(format!("{i}_{name}"));
            columns.push(freqs.iter().map(|&f| response_db(&h, f)).collect());
        }
    }
    let header: Vec<&str> = header.iter().map(String::as_str).collect();
    csv(out, "bands", &header, &rows(&columns));
}

/// A bass line and a lead together: 80 Hz at 0.5 and 660 Hz at 0.2. Both
/// are whole multiples of 20 Hz, so over a second everything the crush
/// adds falls on a multiple of 20 Hz too.
fn bass_and_lead(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f64 / f64::from(RATE);
            (0.5 * (TAU * 80.0 * t).sin() + 0.2 * (TAU * 660.0 * t).sin()) as f32
        })
        .collect()
}

/// The bass and lead at the defaults (LOW off) and with LOW on: 25 ms of
/// the waveform (`wave.csv`), and the level at every 20 Hz to 4 kHz over
/// one second, after half a second to settle (`spectrum.csv`).
fn low_on(out: &Path) {
    let settle = (RATE / 2.0) as usize;
    let n = settle + RATE as usize;
    let x = bass_and_lead(n);
    let defaults = run(&[], &x);
    let low = run(&[(band_param(LOW, BandParam::On), 1.0)], &x);

    let span = (0.025 * RATE) as usize;
    let t: Vec<f32> = (0..span).map(|i| 1000.0 * i as f32 / RATE).collect();
    let window = |y: &[f32]| y[settle..settle + span].to_vec();
    csv(
        out,
        "wave",
        &["ms", "input", "defaults", "low"],
        &rows(&[t, window(&x), window(&defaults), window(&low)]),
    );

    let bins: Vec<f64> = (1..=200).map(|k| 20.0 * f64::from(k)).collect();
    let spectrum = |y: &[f32]| -> Vec<f32> {
        let second = &y[settle..];
        bins.iter().map(|&f| level_db(second, f)).collect()
    };
    csv(
        out,
        "spectrum",
        &["hz", "input", "defaults", "low"],
        &rows(&[
            bins.iter().map(|&f| f as f32).collect(),
            spectrum(&x),
            spectrum(&defaults),
            spectrum(&low),
        ]),
    );
}

/// LOW's DECAY on a 55 Hz bass note at 0.5, LOW on and soloed at 3 bits,
/// at four DECAYs: 40 ms of it after a second to settle. A short DECAY
/// lets auto gain swell between the wave's peaks, which bends its shape.
fn decay(out: &Path) {
    let settle = RATE as usize;
    let span = (0.04 * RATE) as usize;
    let x: Vec<f32> = (0..settle + span)
        .map(|i| (0.5 * (TAU * 55.0 * i as f64 / f64::from(RATE)).sin()) as f32)
        .collect();
    let mut columns = vec![
        (0..span).map(|i| 1000.0 * i as f32 / RATE).collect(),
        x[settle..].to_vec(),
    ];
    for ms in [5.0, 40.0, 300.0] {
        let y = run(
            &[
                (band_param(LOW, BandParam::On), 1.0),
                (band_param(LOW, BandParam::BitDepth), 3.0),
                (band_param(LOW, BandParam::Decay), ms),
                (SOLO, 1.0),
            ],
            &x,
        );
        columns.push(y[settle..].to_vec());
    }
    csv(
        out,
        "decay",
        &["ms", "input", "d5", "d40", "d300"],
        &rows(&columns),
    );
}

/// Each knob's value at five points of its travel, as its taper maps it.
fn tapers(out: &Path) {
    let b = |p| band_param(MID, p);
    let knobs = [
        b(BandParam::Gain),
        b(BandParam::Gamma),
        b(BandParam::BitDepth),
        b(BandParam::Smoothing),
        b(BandParam::Decay),
        b(BandParam::Level),
        LOW_XOVER,
        HIGH_XOVER,
        MIX,
    ];
    let gamma = b(BandParam::Gamma);
    let mut rows = Vec::new();
    for pos in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
        let mut row = vec![pos];
        for k in knobs {
            let spec = &PARAMS[k.0 as usize];
            let plain = spec.denormalize(pos);
            // GAMMA's knob sets the power ten to it; show the power.
            row.push(if k == gamma { 10f32.powf(plain) } else { plain });
        }
        rows.push(row);
    }
    csv(
        out,
        "tapers",
        &[
            "pos",
            "gain",
            "gamma",
            "bits",
            "smoothing",
            "decay",
            "level",
            "lowx",
            "highx",
            "mix",
        ],
        &rows,
    );
}

/// The param table: each param's name, range and default, as the editor
/// shows them.
fn params(out: &Path) {
    let mut text = String::from("name,min,max,default\n");
    for spec in &PARAMS {
        let _ = writeln!(
            text,
            "{},{},{},{}",
            spec.name,
            spec.format(spec.min),
            spec.format(spec.max),
            spec.format(spec.default)
        );
    }
    std::fs::write(out.join("params.csv"), text).expect("write params.csv");
}
