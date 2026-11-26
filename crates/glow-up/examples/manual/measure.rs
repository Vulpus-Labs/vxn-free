//! Measurements for the manual, taken by running Glow-Up's engine as a
//! host does: params set, the engine reset so they hold from the first
//! sample, a signal processed. Each table is a CSV with a header row.

use std::fmt::Write as _;
use std::path::Path;

use glow_up::bank::State;
use glow_up::engine::GlowUpEngine;
use glow_up::{
    A_AMOUNT, A_DIV, A_MULT, B_AMOUNT, DRY_CUT, FOCUS, PARAMS, RESONANCE, SPEED, THRESHOLD, TONE,
    WIDTH,
};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;

const RATE: f32 = 48_000.0;
const TAU: f64 = std::f64::consts::TAU;
/// The test note: a saw whose period is a whole number of samples, so a
/// second holds whole cycles of every partial.
const ROOT: f64 = 200.0;
/// Partials shown.
const PARTIALS: usize = 16;

pub fn write(out: &Path) {
    ratios(out);
    knobs(out);
    glide(out);
    speed(out);
    params(out);
}

/// Boost A alone at full AMOUNT, at `mult`/`div` (the knobs' numbers).
fn boost_a(mult: u8, div: u8) -> Vec<(ParamIndex, f32)> {
    vec![
        (A_AMOUNT, 100.0),
        (A_MULT, f32::from(mult - 1)),
        (A_DIV, f32::from(div - 1)),
        (B_AMOUNT, 0.0),
    ]
}

fn engine(values: &[(ParamIndex, f32)]) -> GlowUpEngine {
    let mut e = GlowUpEngine::new(RATE);
    for &(p, v) in values {
        e.set_param(p, v);
    }
    e.reset();
    e
}

/// `input` through an engine with `values`, one channel.
fn run(values: &[(ParamIndex, f32)], input: &[f32]) -> Vec<f32> {
    let mut e = engine(values);
    let mut x = input.to_vec();
    for block in x.chunks_mut(256) {
        e.process(&mut [block]);
    }
    x
}

fn csv(out: &Path, name: &str, header: &[String], rows: &[Vec<Option<f32>>]) {
    let mut text = header.join(",");
    text.push('\n');
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .map(|v| v.map_or_else(String::new, |v| format!("{v:.4}")))
            .collect();
        text.push_str(&cells.join(","));
        text.push('\n');
    }
    std::fs::write(out.join(format!("{name}.csv")), text).expect("write a CSV");
}

/// Columns of equal length into rows.
fn rows(columns: &[Vec<f32>]) -> Vec<Vec<Option<f32>>> {
    (0..columns[0].len())
        .map(|i| columns.iter().map(|c| Some(c[i])).collect())
        .collect()
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|&s| s.to_owned()).collect()
}

/// A saw at `hz(t)`: partials at 0.3 over their number, below 16 kHz.
fn saw(seconds: f64, hz: impl Fn(f64) -> f64) -> Vec<f32> {
    let n = (seconds * f64::from(RATE)) as usize;
    let mut phase = 0.0f64;
    (0..n)
        .map(|i| {
            let f = hz(i as f64 / f64::from(RATE));
            let mut v = 0.0;
            let mut k = 1.0;
            while k * f < 16_000.0 {
                v += 0.3 / k * (k * phase).sin();
                k += 1.0;
            }
            phase = (phase + TAU * f / f64::from(RATE)) % (TAU * 1000.0);
            v as f32
        })
        .collect()
}

/// The level of `signal` at `cycles` whole cycles over its length, dB
/// against full scale.
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
    (20.0 * amp.max(1e-7).log10()) as f32
}

/// Each partial's level over the second after the first.
fn partials(y: &[f32]) -> Vec<f32> {
    let second = &y[RATE as usize..2 * RATE as usize];
    (1..=PARTIALS)
        .map(|k| level_db(second, ROOT * k as f64))
        .collect()
}

/// How much each partial of a held 200 Hz saw is raised, in dB, under
/// `values`.
fn raised(values: &[(ParamIndex, f32)], x: &[f32], input: &[f32]) -> Vec<f32> {
    let y = run(values, x);
    partials(&y).iter().zip(input).map(|(o, i)| o - i).collect()
}

/// Boost A alone at five ratios, at full AMOUNT and the other defaults:
/// how much each partial of a 200 Hz saw is raised. Columns `k`, `input`
/// (its level), then `r{mult}{div}`.
fn ratios(out: &Path) {
    let x = saw(2.0, |_| ROOT);
    let input = partials(&x);
    let mut header = names(&["k", "input"]);
    let mut columns = vec![(1..=PARTIALS).map(|k| k as f32).collect(), input.clone()];
    for (mult, div) in [(2, 1), (3, 2), (3, 1), (5, 4), (1, 2)] {
        header.push(format!("r{mult}{div}"));
        columns.push(raised(&boost_a(mult, div), &x, &input));
    }
    csv(out, "ratios", &header, &rows(&columns));
}

/// WIDTH, TONE, THRESHOLD and RESONANCE, each at three settings, with boost A at
/// 2/1 and full AMOUNT, at TRACK: how much each partial of a 200 Hz saw
/// is raised. Columns `k`, then `{knob}_{value}`.
fn knobs(out: &Path) {
    let x = saw(2.0, |_| ROOT);
    let input = partials(&x);
    let mut header = names(&["k"]);
    let mut columns = vec![(1..=PARTIALS).map(|k| k as f32).collect()];
    let settings: [(&str, ParamIndex, [f32; 3]); 4] = [
        ("width", WIDTH, [10.0, 50.0, 200.0]),
        ("tone", TONE, [-6.0, 0.0, 6.0]),
        ("threshold", THRESHOLD, [0.0, 6.0, 30.0]),
        ("resonance", RESONANCE, [0.0, 50.0, 100.0]),
    ];
    for (name, p, values) in settings {
        for value in values {
            let mut v = boost_a(2, 1);
            v.extend([(FOCUS, 100.0), (p, value)]);
            header.push(format!("{name}_{value}"));
            columns.push(raised(&v, &x, &input));
        }
    }
    csv(out, "knobs", &header, &rows(&columns));
}

/// A saw held at 200 Hz, gliding up to 300 Hz over a second, then held:
/// every 25 ms, the centres of the filters boosting between 300 and
/// 700 Hz, at FOCUS 0, 50 and 100, with boost A at 2/1. Columns `ms`,
/// `target` (twice the note), then `{focus}_{slot}`, empty where a slot
/// has no filter there.
fn glide(out: &Path) {
    let start = 0.5;
    let hz = |t: f64| ROOT * 1.5f64.powf(((t - start) / 1.0).clamp(0.0, 1.0));
    let x = saw(2.0, hz);
    let step = (0.025 * RATE) as usize;
    let slots = 6;
    let mut header = names(&["ms", "target"]);
    let times: Vec<usize> = (0..x.len() / step).map(|i| i * step).collect();
    let mut table: Vec<Vec<Option<f32>>> = times
        .iter()
        .map(|&n| {
            let t = n as f64 / f64::from(RATE);
            vec![Some((1000.0 * t) as f32), Some((2.0 * hz(t)) as f32)]
        })
        .collect();
    for focus in [0.0f32, 50.0, 100.0] {
        for s in 0..slots {
            header.push(format!("{focus}_{s}"));
        }
        let mut v = boost_a(2, 1);
        v.push((FOCUS, focus));
        let mut e = engine(&v);
        let mut y = x.clone();
        for (row, block) in table.iter_mut().zip(y.chunks_mut(step)) {
            e.process(&mut [block]);
            let mut found: Vec<f32> = e
                .bank()
                .filters()
                .iter()
                .filter(|f| f.state == State::Active && f.hz > 300.0 && f.hz < 700.0)
                .map(|f| f.hz as f32)
                .collect();
            found.sort_by(f32::total_cmp);
            found.resize(slots, f32::NAN);
            row.extend(found.iter().map(|&h| (!h.is_nan()).then_some(h)));
        }
    }
    csv(out, "glide", &header, &table);
}

/// The filters alone (DRY CUT on), every 5 ms, as a 200 Hz saw starts
/// after 50 ms of silence, with boost A at 2/1: their level in dB against
/// where it settles, at four SPEEDs.
fn speed(out: &Path) {
    let lead = (0.05 * RATE) as usize;
    let mut x = vec![0.0f32; lead];
    x.extend(saw(0.55, |_| ROOT));
    let step = (0.005 * RATE) as usize;
    let rms = |s: &[f32]| (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt();
    let mut header = names(&["ms"]);
    let mut columns = vec![
        (0..x.len() / step)
            .map(|i| 1000.0 * (i * step) as f32 / RATE)
            .collect::<Vec<_>>(),
    ];
    for speed in [0.0f32, 50.0, 80.0, 100.0] {
        let mut v = boost_a(2, 1);
        v.extend([(FOCUS, 100.0), (DRY_CUT, 1.0), (SPEED, speed)]);
        let y = run(&v, &x);
        let settled = rms(&y[y.len() - step..]);
        header.push(format!("s{speed}"));
        columns.push(
            y.chunks(step)
                .take(x.len() / step)
                .map(|c| 20.0 * (rms(c) / settled).max(1e-4).log10())
                .collect(),
        );
    }
    csv(out, "speed", &header, &rows(&columns));
}

/// The param table: each param's name, range and default, as the host
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
