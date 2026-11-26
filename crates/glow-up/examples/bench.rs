//! CPU per sample, by MODE and FOCUS, on five sawtooths (a
//! chord) with RESONANCE part way: `cargo run --release -p glow-up
//! --example bench`.

use std::f64::consts::TAU;
use std::time::Instant;

use glow_up::engine::GlowUpEngine;
use glow_up::{FOCUS, MODE, RESONANCE};
use silkscreen::bridge::Engine;

const RATE: f64 = 48_000.0;
const SECONDS: f64 = 10.0;

fn chord() -> Vec<f32> {
    let notes = [110.0, 164.8, 220.0, 277.2, 329.6];
    (0..(SECONDS * RATE) as usize)
        .map(|n| {
            let t = n as f64 / RATE;
            let mut s = 0.0;
            for hz in notes {
                for k in 1..40 {
                    let f = hz * f64::from(k);
                    if f < 16_000.0 {
                        s += (TAU * f * t).sin() / f64::from(k);
                    }
                }
            }
            (0.05 * s) as f32
        })
        .collect()
}

fn main() {
    let x = chord();
    println!("mode    focus  ns/sample  % of a core at 48 kHz");
    for mode in [0.0, 1.0] {
        for focus in [0.0, 100.0] {
            let mut e = GlowUpEngine::new(RATE as f32);
            for (p, v) in [(MODE, mode), (FOCUS, focus), (RESONANCE, 40.0)] {
                e.set_param(p, v);
            }
            e.reset();
            let (mut l, mut r) = (x.clone(), x.clone());
            let start = Instant::now();
            for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
                e.process(&mut [a, b]);
            }
            let ns = start.elapsed().as_nanos() as f64 / x.len() as f64;
            std::hint::black_box(&l);
            println!(
                "{:6}  {:5}  {:9.1}  {:6.2}",
                if mode > 0.0 { "STEREO" } else { "MONO" },
                focus,
                ns,
                ns * RATE / 1e9 * 100.0
            );
        }
    }
}
