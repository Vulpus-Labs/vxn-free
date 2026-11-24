//! Measurements for the manual, taken by running Beverley's engine as a
//! host does: params set, the engine reset so they hold from the first
//! sample, a signal processed. Each table is a CSV with a header row.

use std::fmt::Write as _;
use std::path::Path;

use beverley::crush::{ATTACK_MS, auto_gain};
use beverley::engine::BeverleyEngine;
use beverley::{
    AUTO_GAIN, BIT_DEPTH, CRUSH_MODE, DECAY, DETECT, GAIN, GAMMA, MIX, PARAMS, SMOOTHING, STEREO,
};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::peak::fall;
use vxn_free_dsp::{PeakFollower, RangeFollower};

const RATE: f32 = 48_000.0;
const TAU: f32 = std::f32::consts::TAU;
/// STEREO's SEPARATE position: one channel is crushed on its own.
const SEPARATE: f32 = 1.0;

pub fn write(out: &Path) {
    grids(out);
    spectra(out);
    aliasing(out);
    auto_gain_traces(out);
    offset(out);
    tapers(out);
    params(out);
}

/// An engine at `rate` with `values` set and settled, running one channel.
fn engine(rate: f32, values: &[(ParamIndex, f32)]) -> BeverleyEngine {
    let mut e = BeverleyEngine::new(rate);
    e.set_param(STEREO, SEPARATE);
    for &(p, v) in values {
        e.set_param(p, v);
    }
    e.reset();
    e
}

/// `input` through an engine with `values`, at `rate`.
fn run(rate: f32, values: &[(ParamIndex, f32)], input: &[f32]) -> Vec<f32> {
    let mut e = engine(rate, values);
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

/// Samples in each test signal.
const SIGNAL: usize = 480;

/// The three test signals, each `SIGNAL` samples: a linear ramp from −1
/// to 1 (output against it is the transfer curve), one cycle of a sine at
/// 0.8, and a complex wave (a fundamental with its second and third
/// harmonics, two cycles) fading from loud to a sixth of that.
fn signals() -> [Vec<f32>; 3] {
    let n = SIGNAL as f32;
    let ramp = (0..SIGNAL)
        .map(|i| -1.0 + 2.0 * i as f32 / (n - 1.0))
        .collect();
    let sine = (0..SIGNAL)
        .map(|i| 0.8 * (TAU * i as f32 / n).sin())
        .collect();
    let complex = (0..SIGNAL)
        .map(|i| {
            let phase = 2.0 * TAU * i as f32 / n;
            let fade = 1.0 - 0.85 * i as f32 / n;
            fade * (0.6 * phase.sin() + 0.3 * (2.0 * phase).sin() + 0.2 * (3.0 * phase + 1.0).sin())
        })
        .collect();
    [ramp, sine, complex]
}

/// A grid for one control: each test signal through each of `settings`.
/// Columns: `t`, the three inputs, then for setting `i`,
/// `{i}_ramp`, `{i}_sine` and `{i}_complex`. With AUTO off the chain is
/// memoryless, so each signal runs through a fresh engine.
fn grid(out: &Path, name: &str, settings: &[Vec<(ParamIndex, f32)>]) {
    let inputs = signals();
    let mut header = vec![
        "t".to_owned(),
        "ramp".into(),
        "sine".into(),
        "complex".into(),
    ];
    let mut columns = vec![
        (0..SIGNAL)
            .map(|i| i as f32 / (SIGNAL - 1) as f32)
            .collect(),
    ];
    columns.extend(inputs.iter().cloned());
    for (i, values) in settings.iter().enumerate() {
        for (signal, x) in ["ramp", "sine", "complex"].iter().zip(&inputs) {
            header.push(format!("{i}_{signal}"));
            columns.push(run(RATE, values, x));
        }
    }
    let header: Vec<&str> = header.iter().map(String::as_str).collect();
    csv(out, name, &header, &rows(&columns));
}

fn grids(out: &Path) {
    let sym = (CRUSH_MODE, 1.0);
    grid(
        out,
        "grid-bits",
        &[1.0, 2.0, 4.0, 8.0].map(|b| vec![(BIT_DEPTH, b)]),
    );
    grid(
        out,
        "grid-fraction",
        &[2.0, 2.5, 3.0].map(|b| vec![(BIT_DEPTH, b), sym]),
    );
    grid(
        out,
        "grid-modes",
        &[0.0, 1.0].map(|m| vec![(BIT_DEPTH, 2.0), (CRUSH_MODE, m)]),
    );
    grid(
        out,
        "grid-smoothing",
        &[0.0, 25.0, 50.0, 100.0].map(|v| vec![(BIT_DEPTH, 2.0), sym, (SMOOTHING, v)]),
    );
    grid(
        out,
        "grid-gamma",
        &[-0.5, 0.0, 0.5].map(|g| vec![(BIT_DEPTH, 3.0), sym, (GAMMA, g)]),
    );
    grid(
        out,
        "grid-gain",
        &[0.0, 6.0, 12.0, 20.0].map(|g| vec![(BIT_DEPTH, 3.0), sym, (GAIN, g)]),
    );
}

/// The power of `y` at whole cycles-per-length `k` (Goertzel): its
/// amplitude squared over two.
fn power_at(signal: &[f32], cycles: f64) -> f64 {
    let len = signal.len() as f64;
    let omega = std::f64::consts::TAU * cycles / len;
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
    amp * amp / 2.0
}

/// A one-second, 997 Hz sine at 0.8 through `values` at `rate`: whole
/// cycles, so each harmonic falls on one frequency bin, and 997 Hz so its
/// aliases fall between them.
fn tone(rate: f32, values: &[(ParamIndex, f32)]) -> Vec<f32> {
    let n = rate as usize;
    let x: Vec<f32> = (0..n)
        .map(|i| 0.8 * (f64::from(TAU) * 997.0 * i as f64 / f64::from(rate)).sin() as f32)
        .collect();
    run(rate, values, &x)
}

/// The levels of the first 20 harmonics against the fundamental, in dB.
fn harmonics(y: &[f32]) -> Vec<f32> {
    let fundamental = power_at(y, 997.0);
    (1..=20)
        .map(|k| {
            let p = power_at(y, 997.0 * f64::from(k));
            (10.0 * (p / fundamental).max(1e-12).log10()).max(-120.0) as f32
        })
        .collect()
}

fn spectra(out: &Path) {
    let settings: [&[(ParamIndex, f32)]; 4] = [
        &[(BIT_DEPTH, 2.0)],
        &[(BIT_DEPTH, 4.0)],
        &[(BIT_DEPTH, 8.0)],
        &[(BIT_DEPTH, 4.0), (SMOOTHING, 50.0)],
    ];
    let mut columns = vec![(1..=20).map(|k| k as f32).collect::<Vec<_>>()];
    for values in settings {
        columns.push(harmonics(&tone(RATE, values)));
    }
    csv(
        out,
        "spectra",
        &["harmonic", "b2", "b4", "b8", "b4s50"],
        &rows(&columns),
    );
}

/// Aliased power against the fundamental, in dB: everything in the
/// output that is neither DC nor a harmonic below Nyquist (the output's
/// power less those, by Parseval).
fn alias_db(rate: f32, values: &[(ParamIndex, f32)]) -> f32 {
    let y = tone(rate, values);
    let n = y.len() as f64;
    let total = y.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / n;
    let mean = y.iter().map(|&v| f64::from(v)).sum::<f64>() / n;
    let mut harmonic = 0.0;
    let mut k = 1.0;
    while 997.0 * k < f64::from(rate) / 2.0 {
        harmonic += power_at(&y, 997.0 * k);
        k += 1.0;
    }
    let alias = (total - mean * mean - harmonic).max(1e-14);
    (10.0 * (alias / power_at(&y, 997.0)).log10()) as f32
}

fn aliasing(out: &Path) {
    let mut rows = Vec::new();
    for bits in [2.0, 4.0, 8.0] {
        for smoothing in [0.0, 25.0, 50.0, 100.0] {
            let values = [(BIT_DEPTH, bits), (SMOOTHING, smoothing)];
            rows.push(vec![
                bits,
                smoothing,
                alias_db(44_100.0, &values),
                alias_db(48_000.0, &values),
            ]);
        }
    }
    csv(
        out,
        "aliasing",
        &["bits", "smoothing", "alias441", "alias48"],
        &rows,
    );
}

/// Auto gain's gain, in dB, over a burst then a quiet passage: a 200 Hz
/// sine at 0.8 for 60 ms, then at 0.08. The detectors as the engine runs
/// them (`vxn-free-dsp`'s followers, `crush::auto_gain`), at three DECAYs.
fn auto_gain_traces(out: &Path) {
    let n = (0.4 * RATE) as usize;
    let x: Vec<f32> = (0..n)
        .map(|i| {
            let a = if i < (0.06 * RATE) as usize {
                0.8
            } else {
                0.08
            };
            a * (TAU * 200.0 * i as f32 / RATE).sin()
        })
        .collect();
    let db = |g: f32| 20.0 * g.log10();
    let mut columns = vec![(0..n).map(|i| 1000.0 * i as f32 / RATE).collect::<Vec<_>>()];
    for decay_ms in [20.0, 115.0, 500.0] {
        let mut p = PeakFollower::new(ATTACK_MS, fall(decay_ms * 0.001, RATE), RATE);
        columns.push(x.iter().map(|&s| db(auto_gain(p.next(s.abs())))).collect());
    }
    let mut r = RangeFollower::new(fall(0.115, RATE));
    columns.push(
        x.iter()
            .map(|&s| {
                r.next(s);
                db(auto_gain(r.half_range()))
            })
            .collect(),
    );
    // Every 24th sample (0.5 ms) is plenty for a plot.
    let columns: Vec<Vec<f32>> = columns
        .into_iter()
        .map(|c| c.into_iter().step_by(24).collect())
        .collect();
    csv(
        out,
        "autogain",
        &["ms", "peak20", "peak115", "peak500", "range115"],
        &rows(&columns),
    );
}

/// An offset signal, 0.5 + 0.1 sin at 300 Hz, crushed to one bit with
/// auto gain by PEAK and by RANGE (DECAY 100 ms): one cycle, after a
/// second to settle.
fn offset(out: &Path) {
    let n = RATE as usize + 160;
    let x: Vec<f32> = (0..n)
        .map(|i| 0.5 + 0.1 * (TAU * 300.0 * i as f32 / RATE).sin())
        .collect();
    let base = [(AUTO_GAIN, 1.0), (BIT_DEPTH, 1.0), (DECAY, 100.0)];
    let peak = run(RATE, &[base[0], base[1], base[2], (DETECT, 0.0)], &x);
    let range = run(RATE, &[base[0], base[1], base[2], (DETECT, 1.0)], &x);
    let from = RATE as usize;
    let t: Vec<f32> = (0..160).map(|i| 1000.0 * i as f32 / RATE).collect();
    csv(
        out,
        "offset",
        &["ms", "input", "peak", "range"],
        &rows(&[
            t,
            x[from..].to_vec(),
            peak[from..].to_vec(),
            range[from..].to_vec(),
        ]),
    );
}

/// Each knob's value at five points of its travel, as its taper maps it.
fn tapers(out: &Path) {
    let knobs = [GAIN, GAMMA, BIT_DEPTH, SMOOTHING, DECAY, MIX];
    let mut rows = Vec::new();
    for pos in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
        let mut row = vec![pos];
        for k in knobs {
            let spec = &PARAMS[k.0 as usize];
            let plain = spec.denormalize(pos);
            // GAMMA's knob sets the power ten to it; show the power.
            row.push(if k == GAMMA { 10f32.powf(plain) } else { plain });
        }
        rows.push(row);
    }
    csv(
        out,
        "tapers",
        &["pos", "gain", "gamma", "bits", "smoothing", "decay", "mix"],
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
