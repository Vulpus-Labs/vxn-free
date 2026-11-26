//! Scratch: what the bank does on a held chord. Per FOCUS, over the held
//! part: filter starts, how far sounding filters' centres move, and how
//! much their gains wobble block to block.

// A measuring harness: short names, as in the maths.
#![allow(clippy::many_single_char_names, clippy::cast_lossless)]

use std::f64::consts::TAU;

use glow_up::FOCUS;
use glow_up::bank::{BLOCK, POOL, State};
use glow_up::engine::GlowUpEngine;
use silkscreen::bridge::Engine;

const RATE: f64 = 48_000.0;

fn chord(seconds: f64) -> Vec<f32> {
    // C3 E3 G3 C4 in equal temperament, saws.
    let notes = [130.81, 164.81, 196.0, 261.63];
    (0..(seconds * RATE) as usize)
        .map(|n| {
            let t = n as f64 / RATE;
            let mut s = 0.0;
            for hz in notes {
                for k in 1..60 {
                    let f = hz * f64::from(k);
                    if f < 16_000.0 {
                        s += (TAU * f * t).sin() / f64::from(k);
                    }
                }
            }
            (0.08 * s) as f32
        })
        .collect()
}

fn main() {
    let x = chord(4.0);
    for focus in [0.0, 50.0, 100.0] {
        let mut e = GlowUpEngine::new(RATE as f32);
        e.set_param(FOCUS, focus);
        e.reset();
        let mut prev: [(State, f64, f64); POOL] = [(State::Free, 0.0, 0.0); POOL];
        let (mut starts, mut moves, mut max_move, mut blocks) = (0, 0.0f64, 0.0f64, 0);
        let mut big = 0;
        let mut wobble = 0.0f64;
        let mut wob_n = 0;
        let mut gain_sum = 0.0;
        let mut y = x.clone();
        let mut tprev = [0.0f64; POOL];
        let (mut tmoves, mut tn, mut tmax, mut releases) = (0.0f64, 0, 0.0f64, 0);
        for (b, chunk) in y.chunks_mut(BLOCK).enumerate() {
            e.process(&mut [chunk]);
            let t = (b * BLOCK) as f64 / RATE;
            let now: Vec<(State, f64, f64)> = e
                .bank()
                .filters()
                .iter()
                .map(|f| (f.state, f.hz, f.gain))
                .collect();
            if t > 1.5 {
                blocks += 1;
                for (k, t) in e.bank().trackers().iter().enumerate() {
                    if t.settled()
                        && t.roughness() < 300_000.0 * (t.frequency() / 880.0).max(1.0).powi(2)
                        && tprev[k] > 0.0
                    {
                        let c = (1200.0 * (t.frequency() / tprev[k]).log2()).abs();
                        tmoves += c;
                        tmax = tmax.max(c);
                        tn += 1;
                    }
                }
                for (i, &(s, _, _)) in now.iter().enumerate() {
                    if s == State::Releasing && prev[i].0 == State::Active {
                        releases += 1;
                    }
                }
                for (i, &(s, hz, g)) in now.iter().enumerate() {
                    let (ps, phz, pg) = prev[i];
                    if s == State::Active && ps != State::Active {
                        starts += 1;
                    }
                    if s == State::Active && ps == State::Active {
                        let c = (1200.0 * (hz / phz).log2()).abs();
                        moves += c;
                        max_move = max_move.max(c);
                        if c > 2.0 {
                            big += 1;
                        }
                        wobble += (g - pg).abs();
                        wob_n += 1;
                        gain_sum += g;
                    }
                }
            }
            for (k, t) in e.bank().trackers().iter().enumerate() {
                tprev[k] = if t.settled()
                    && t.roughness() < 300_000.0 * (t.frequency() / 880.0).max(1.0).powi(2)
                {
                    t.frequency()
                } else {
                    0.0
                };
            }
            for (i, v) in now.into_iter().enumerate() {
                prev[i] = v;
            }
        }
        let secs = blocks as f64 * BLOCK as f64 / RATE;
        println!(
            "FOCUS {focus:5}: starts {:.1}/s, centre moves {:.2} cents/block mean, {max_move:.2} max; \
             gain {:.3} mean, |Δgain| {:.4}/block ({:.1} % of mean)",
            starts as f64 / secs,
            moves / wob_n.max(1) as f64,
            gain_sum / wob_n.max(1) as f64,
            wobble / wob_n.max(1) as f64,
            100.0 * wobble / gain_sum.max(1e-9)
        );
        println!(
            "             moves over 2 cents: {:.1}/s; steering trackers move {:.2} cents/block mean, {tmax:.1} max; releases {:.1}/s",
            f64::from(big) / secs,
            tmoves / f64::from(tn.max(1)),
            f64::from(releases) / secs
        );
    }
}
