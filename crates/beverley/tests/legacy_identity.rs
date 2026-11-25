//! Ticket 0032 moves Beverley's per-channel work into `beverley::chain`.
//! Here the engine before the move (`legacy`) and after run the same
//! input, settings and automation, and must agree bit for bit. Removed
//! once the ticket closes.

mod legacy;

use beverley::engine::BeverleyEngine;
use beverley::{
    AUTO_GAIN, BIT_DEPTH, CRUSH_MODE, DECAY, DETECT, GAIN, GAMMA, MIX, SMOOTHING, STEREO,
};
use legacy::LegacyEngine;
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;

/// A drum-ish test signal: a decaying tone with an offset, and noise.
fn signal(len: usize, seed: u32) -> Vec<f32> {
    let mut s = seed;
    (0..len)
        .map(|n| {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (s >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
            let t = (n % 12_000) as f32 / 48_000.0;
            0.8 * (-t * 8.0).exp() * (n as f32 * 0.031).sin() + 0.1 * noise + 0.05
        })
        .collect()
}

fn settings() -> Vec<Vec<(ParamIndex, f32)>> {
    vec![
        vec![],
        vec![
            (GAIN, 12.0),
            (BIT_DEPTH, 2.5),
            (GAMMA, 0.4),
            (SMOOTHING, 30.0),
            (MIX, 70.0),
        ],
        vec![
            (AUTO_GAIN, 1.0),
            (DETECT, 0.0),
            (DECAY, 40.0),
            (BIT_DEPTH, 3.0),
        ],
        vec![
            (AUTO_GAIN, 1.0),
            (DETECT, 1.0),
            (DECAY, 900.0),
            (CRUSH_MODE, 1.0),
        ],
        vec![(STEREO, 0.0), (BIT_DEPTH, 6.0), (GAMMA, -0.7)],
        vec![(STEREO, 1.0), (AUTO_GAIN, 1.0), (DETECT, 1.0)],
        vec![
            (STEREO, 2.0),
            (AUTO_GAIN, 1.0),
            (DETECT, 0.0),
            (SMOOTHING, 100.0),
        ],
    ]
}

/// Automation part way through: every continuous param moves, and the
/// switches flip.
fn automation() -> Vec<(ParamIndex, f32)> {
    vec![
        (GAIN, 18.0),
        (BIT_DEPTH, 1.3),
        (GAMMA, -0.2),
        (SMOOTHING, 60.0),
        (MIX, 40.0),
        (DECAY, 2000.0),
        (CRUSH_MODE, 0.0),
        (AUTO_GAIN, 1.0),
        (DETECT, 1.0),
        (STEREO, 1.0),
    ]
}

fn render<E: Engine>(
    mut e: E,
    set: &[(ParamIndex, f32)],
    channels: usize,
    block: usize,
) -> Vec<Vec<f32>> {
    for &(p, v) in set {
        e.set_param(p, v);
    }
    e.reset();
    let len = 48_000;
    let mut out: Vec<Vec<f32>> = (0..channels).map(|c| signal(len, 3 + c as u32)).collect();
    let mut at = 0;
    while at < len {
        if at >= len / 2 && at < len / 2 + block {
            for (p, v) in automation() {
                e.set_param(p, v);
            }
        }
        let end = (at + block).min(len);
        let mut views: Vec<&mut [f32]> = out.iter_mut().map(|c| &mut c[at..end]).collect();
        e.process(&mut views);
        at = end;
    }
    out
}

#[test]
fn the_chain_changes_no_output() {
    for rate in [44_100.0, 96_000.0] {
        for set in settings() {
            for channels in [1, 2] {
                for block in [1, 7, 32, 64, 500] {
                    let old = render(LegacyEngine::new(rate), &set, channels, block);
                    let new = render(BeverleyEngine::new(rate), &set, channels, block);
                    for (c, (o, n)) in old.iter().zip(&new).enumerate() {
                        let same = o.iter().zip(n).all(|(a, b)| a.to_bits() == b.to_bits());
                        assert!(
                            same,
                            "{rate} Hz, {set:?}, {channels} ch (ch {c}), block {block}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "timing; run with --release -- --ignored --nocapture"]
fn time_old_and_new() {
    use std::time::Instant;
    let time = |new: bool, set: &[(ParamIndex, f32)]| {
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = Instant::now();
            let out = if new {
                render(BeverleyEngine::new(96_000.0), set, 2, 64)
            } else {
                render(LegacyEngine::new(96_000.0), set, 2, 64)
            };
            std::hint::black_box(out);
            best = best.min(t.elapsed().as_secs_f64());
        }
        best * 1e9 / 48_000.0
    };
    for set in [settings()[1].clone(), settings()[6].clone()] {
        println!(
            "old {:.1} ns/frame, new {:.1} ns/frame ({set:?})",
            time(false, &set),
            time(true, &set)
        );
        println!(
            "old {:.1} ns/frame, new {:.1} ns/frame (again)",
            time(false, &set),
            time(true, &set)
        );
    }
}
