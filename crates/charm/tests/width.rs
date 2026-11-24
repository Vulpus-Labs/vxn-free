//! AUTO WIDTH and the mono guard against ticket 0018's criteria.

use charm::engine::{CharmEngine, MID_HIGH_XOVER, PAN_GUARD, TICK};
use charm::observe::Observation;
use charm::{AUTO_WIDTH, LOW_XOVER, MIX, WIDTH};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::Crossover;

fn noise(len: usize, mut seed: u32) -> Vec<f32> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            0.25 * ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5)
        })
        .collect()
}

/// Render, keeping each tick's observation and the output.
fn run(
    params: &[(ParamIndex, f32)],
    l: &[f32],
    r: &[f32],
) -> (Vec<Observation>, Vec<f32>, Vec<f32>) {
    let mut e = CharmEngine::new(48_000.0);
    for &(p, v) in params {
        e.set_param(p, v);
    }
    e.reset();
    let (mut l, mut r) = (l.to_vec(), r.to_vec());
    let mut obs = Vec::new();
    for (cl, cr) in l.chunks_mut(TICK).zip(r.chunks_mut(TICK)) {
        e.process(&mut [cl, cr]);
        obs.push(*e.observation());
    }
    (obs, l, r)
}

/// The widest any voice is panned, after the first second.
fn widest(obs: &[Observation]) -> f32 {
    obs.iter()
        .skip(1500)
        .flat_map(|o| {
            o.bands
                .iter()
                .flat_map(|b| b.voices[..b.active as usize].iter())
        })
        .fold(0.0f32, |m, v| m.max(v.pan.abs()))
}

const LEN: usize = 48_000 * 4;

#[test]
fn mono_input_gets_the_full_width() {
    let x = noise(LEN, 1);
    let on = run(&[(WIDTH, 60.0), (AUTO_WIDTH, 1.0)], &x, &x).0;
    let off = run(&[(WIDTH, 60.0), (AUTO_WIDTH, 0.0)], &x, &x).0;
    let (a, b) = (widest(&on), widest(&off));
    assert!((a - b).abs() < 0.02, "auto {a}, fixed {b}");
}

#[test]
fn wide_input_gets_less_spread() {
    let (l, r) = (noise(LEN, 1), noise(LEN, 2));
    let on = widest(&run(&[(WIDTH, 60.0), (AUTO_WIDTH, 1.0)], &l, &r).0);
    let off = widest(&run(&[(WIDTH, 60.0), (AUTO_WIDTH, 0.0)], &l, &r).0);
    assert!(on < 0.4 * off, "auto {on}, fixed {off}");
}

#[test]
fn with_auto_width_off_the_input_does_not_move_the_image() {
    let x = noise(LEN, 1);
    let (l, r) = (noise(LEN, 3), noise(LEN, 4));
    let mono = run(&[(AUTO_WIDTH, 0.0)], &x, &x).0;
    let wide = run(&[(AUTO_WIDTH, 0.0)], &l, &r).0;
    for (a, b) in mono.iter().zip(&wide) {
        for (ba, bb) in a.bands.iter().zip(&b.bands) {
            for (va, vb) in ba.voices.iter().zip(&bb.voices) {
                assert_eq!(va.pan, vb.pan);
            }
        }
    }
}

#[test]
fn no_voice_is_panned_past_the_guard() {
    let x = noise(LEN, 1);
    let w = widest(&run(&[(WIDTH, 100.0), (AUTO_WIDTH, 0.0)], &x, &x).0);
    assert!(w <= PAN_GUARD + 1.0e-6, "{w}");
}

/// Level lost summing `l`/`r` to mono, per band (MID, HIGH), dB, after the
/// first second.
fn mono_loss(l: &[f32], r: &[f32]) -> [f64; 2] {
    let split = |x: &[f32]| {
        let mut low = Crossover::new(150.0, 48_000.0);
        let mut high = Crossover::new(MID_HIGH_XOVER, 48_000.0);
        x.iter()
            .map(|&s| {
                let (_, rest) = low.split(s);
                let (m, h) = high.split(rest);
                [m, h]
            })
            .collect::<Vec<_>>()
    };
    let (bl, br) = (split(l), split(r));
    std::array::from_fn(|b| {
        let (mut stereo, mut mono) = (0.0f64, 0.0f64);
        for (x, y) in bl.iter().zip(&br).skip(48_000) {
            let (x, y) = (f64::from(x[b]), f64::from(y[b]));
            stereo += 0.5 * (x * x + y * y);
            mono += (0.5 * (x + y)).powi(2);
        }
        10.0 * (mono / stereo).log10()
    })
}

#[test]
fn the_ensemble_never_costs_more_than_the_guard_in_mono() {
    let chord: Vec<f32> = (0..LEN)
        .map(|n| {
            let t = n as f32 / 48_000.0;
            [261.63f32, 329.63, 392.0, 493.88]
                .iter()
                .flat_map(|&f| {
                    (1..=8).map(move |k| {
                        0.25 / k as f32 * (std::f32::consts::TAU * f * k as f32 * t).sin()
                    })
                })
                .sum()
        })
        .collect();
    let (pl, pr) = (noise(LEN, 5), noise(LEN, 6));
    for (l, r) in [(&chord, &chord), (&pl, &pr)] {
        let before = mono_loss(l, r);
        let (_, ol, or) = run(&[(WIDTH, 100.0), (MIX, 100.0), (LOW_XOVER, 150.0)], l, r);
        let after = mono_loss(&ol, &or);
        for b in 0..2 {
            // The ensemble may cost at most the guard's 2 dB in mono.
            assert!(
                after[b] - before[b] > -2.0,
                "band {b}: {} → {}",
                before[b],
                after[b]
            );
        }
    }
}
