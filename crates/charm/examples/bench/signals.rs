//! The bench's synthetic test set, generated in code so no licensed audio
//! goes in the repo. Every signal is deterministic.

use std::f32::consts::TAU;

/// Names, in the order `all` runs them.
pub const NAMES: [&str; 6] = ["tone", "chord", "dist-chord", "pink", "clicks", "drums"];

/// The signal called `name`, `seconds` long at `rate`, as stereo (both
/// channels equal, except pink noise, whose channels are independent).
pub fn make(name: &str, rate: f32, seconds: f32) -> Option<[Vec<f32>; 2]> {
    let len = (rate * seconds) as usize;
    let mono = match name {
        // A sustained harmonic tone: 220 Hz, twelve harmonics at 1/k.
        "tone" => harmonic(&[220.0], 12, rate, len, 0.5),
        // A clean four-note chord: C4 E4 G4 B4, eight harmonics each.
        "chord" => harmonic(&[261.63, 329.63, 392.0, 493.88], 8, rate, len, 0.25),
        // A distorted power chord, E2 B2 E3, through tanh.
        "dist-chord" => harmonic(&[82.41, 123.47, 164.81], 6, rate, len, 0.4)
            .into_iter()
            .map(|x| 0.5 * (8.0 * x).tanh())
            .collect(),
        "pink" => {
            return Some([pink(len, 0x1234_5678, 0.25), pink(len, 0x8765_4321, 0.25)]);
        }
        // A click train at 4 Hz: short decaying bursts.
        "clicks" => clicks(rate, len),
        // A loop at 120 bpm: kick on 1 and 3, snare on 2 and 4, hats on
        // the eighths.
        "drums" => drums(rate, len),
        _ => return None,
    };
    Some([mono.clone(), mono])
}

fn harmonic(fundamentals: &[f32], harmonics: usize, rate: f32, len: usize, level: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; len];
    for &f0 in fundamentals {
        for k in 1..=harmonics {
            let hz = f0 * k as f32;
            if hz > 0.45 * rate {
                break;
            }
            let w = TAU * hz / rate;
            let a = level / k as f32;
            // Phases spread so the sum's crest stays sensible.
            let phase = k as f32 * 1.3 + f0 * 0.01;
            for (n, s) in out.iter_mut().enumerate() {
                *s += a * (w * n as f32 + phase).sin();
            }
        }
    }
    out
}

/// A xorshift32 step: uniform on −1..1.
fn uniform(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 * (2.0 / (1u32 << 24) as f32) - 1.0
}

/// Pink noise by Paul Kellet's economy filter.
fn pink(len: usize, seed: u32, level: f32) -> Vec<f32> {
    let mut state = seed;
    let (mut b0, mut b1, mut b2) = (0.0f32, 0.0f32, 0.0f32);
    (0..len)
        .map(|_| {
            let white = uniform(&mut state);
            b0 = 0.99765 * b0 + white * 0.099_046;
            b1 = 0.96300 * b1 + white * 0.296_516_4;
            b2 = 0.57000 * b2 + white * 1.052_691_3;
            level * (b0 + b1 + b2 + white * 0.1848) * 0.5
        })
        .collect()
}

fn clicks(rate: f32, len: usize) -> Vec<f32> {
    let period = (rate / 4.0) as usize;
    let mut state = 99;
    (0..len)
        .map(|n| {
            let t = (n % period) as f32 / rate;
            0.8 * (-t / 0.002).exp() * uniform(&mut state)
        })
        .collect()
}

fn drums(rate: f32, len: usize) -> Vec<f32> {
    let beat = (rate * 0.5) as usize;
    let eighth = beat / 2;
    let mut state = 7;
    let mut hat_hp = 0.0f32;
    let mut hat_prev = 0.0f32;
    (0..len)
        .map(|n| {
            let in_beat = n % beat;
            let which = (n / beat) % 4;
            let t = in_beat as f32 / rate;
            let mut s = 0.0;
            if which.is_multiple_of(2) {
                // Kick: a sine falling from 120 Hz to 45 Hz, its phase the
                // integral of 45 + 75 e^(−t/30 ms).
                let phase = TAU * (45.0 * t + 75.0 * 0.03 * (1.0 - (-t / 0.03).exp()));
                s += 0.9 * (-t / 0.25).exp() * phase.sin();
            } else {
                // Snare: noise and a 180 Hz body.
                s += 0.5 * (-t / 0.12).exp() * uniform(&mut state);
                s += 0.3 * (-t / 0.06).exp() * (TAU * 180.0 * t).sin();
            }
            // Hats: high-passed noise, short.
            let te = (n % eighth) as f32 / rate;
            let white = uniform(&mut state);
            hat_hp = 0.6 * (hat_hp + white - hat_prev);
            hat_prev = white;
            s += 0.25 * (-te / 0.02).exp() * hat_hp;
            s
        })
        .collect()
}
