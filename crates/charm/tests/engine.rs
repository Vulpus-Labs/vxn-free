//! Charm's engine against ticket 0014's acceptance criteria.

use charm::engine::CharmEngine;
use charm::flock::{FLOOR, max_velocity};
use charm::{COHESION, DETUNE, LOW_XOVER, MIX, SEPARATION, SPREAD, TURBULENCE, VOICES, WIDTH};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;

/// White noise, repeatable.
fn noise(len: usize, mut seed: u32) -> Vec<f32> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect()
}

fn engine(rate: f32, params: &[(ParamIndex, f32)]) -> CharmEngine {
    let mut e = CharmEngine::new(rate);
    for &(p, v) in params {
        e.set_param(p, v);
    }
    e.reset();
    e
}

/// Settings that exercise the flock hard, and gently.
fn settings() -> Vec<Vec<(ParamIndex, f32)>> {
    vec![
        vec![],
        vec![
            (VOICES, 6.0),
            (DETUNE, 40.0),
            (SPREAD, 1.0),
            (COHESION, 100.0),
            (SEPARATION, 100.0),
            (TURBULENCE, 100.0),
        ],
        vec![
            (VOICES, 0.0),
            (DETUNE, 1.0),
            (SPREAD, 25.0),
            (COHESION, 0.0),
            (SEPARATION, 0.0),
            (TURBULENCE, 100.0),
        ],
        vec![
            (VOICES, 3.0),
            (DETUNE, 25.0),
            (SPREAD, 4.0),
            (TURBULENCE, 0.0),
        ],
        // The most voices (ticket 0030).
        vec![
            (VOICES, 62.0),
            (DETUNE, 40.0),
            (SPREAD, 25.0),
            (COHESION, 100.0),
            (SEPARATION, 100.0),
            (TURBULENCE, 100.0),
        ],
    ]
}

fn get(params: &[(ParamIndex, f32)], p: ParamIndex) -> f32 {
    params
        .iter()
        .find(|(q, _)| *q == p)
        .map_or(charm::PARAMS[p.0 as usize].default, |(_, v)| *v)
}

#[test]
fn no_voice_exceeds_detune_or_leaves_its_delay_range() {
    for params in settings() {
        let cents = get(&params, DETUNE);
        let spread = get(&params, SPREAD) * 0.001;
        let vmax = max_velocity(f64::from(cents)) as f32;
        let floor = FLOOR as f32;
        let mut e = engine(48_000.0, &params);
        let input = noise(32, 5);
        for _ in 0..20_000 {
            let mut l = input.clone();
            let mut r = input.clone();
            e.process(&mut [&mut l, &mut r]);
            let obs = e.observation();
            for band in &obs.bands {
                for v in &band.voices[..band.active as usize] {
                    assert!(v.velocity.abs() <= vmax * 1.000_01, "{params:?}: {v:?}");
                    assert!(v.cents().abs() <= cents * 1.000_1, "{params:?}: {v:?}");
                    assert!(
                        v.delay >= floor - 1.0e-7 && v.delay <= floor + spread + 1.0e-7,
                        "{params:?}: {v:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn crowded_voices_only_pass_each_other_briefly() {
    // Eight voices, cohesion pulling them in at full against separation at
    // full. Under the acceleration limit (pitch must glide), a pair closing
    // at speed can't always be stopped in time, so voices may pass each
    // other; what matters is that none sits on another for long.
    let params = [
        (VOICES, 6.0),
        (COHESION, 100.0),
        (SEPARATION, 100.0),
        (TURBULENCE, 40.0),
        (WIDTH, 100.0),
    ];
    let spread = get(&params, SPREAD) * 0.001;
    let mut e = engine(48_000.0, &params);
    let input = vec![0.0f32; 32];
    let (mut close, mut ticks, mut spell, mut longest) = (0usize, 0usize, 0usize, 0usize);
    for t in 0..30_000 {
        let mut l = input.clone();
        let mut r = input.clone();
        e.process(&mut [&mut l, &mut r]);
        if t < 3000 {
            continue; // settling, two seconds
        }
        let mut nearest = f32::MAX;
        for band in &e.observation().bands {
            let v = &band.voices[..band.active as usize];
            for i in 0..v.len() {
                for j in i + 1..v.len() {
                    let du = (v[i].delay - v[j].delay) / spread;
                    let dp = 0.5 * (v[i].pan - v[j].pan);
                    nearest = nearest.min((du * du + dp * dp).sqrt());
                }
            }
        }
        ticks += 1;
        if nearest < NEAR {
            close += 1;
            spell += 1;
            longest = longest.max(spell);
        } else {
            spell = 0;
        }
    }
    let share = close as f32 / ticks as f32;
    let longest_ms = longest as f32 * 32.0 / 48.0;
    assert!(
        share < 0.10,
        "some pair near {:.1} % of the time",
        share * 100.0
    );
    assert!(longest_ms < 250.0, "a pair stayed near for {longest_ms} ms");
}

/// Two voices this close in the unit square are passing each other.
/// Measured with the flock's acceleration limit (ticket 0024): near 3.9 %
/// of the time at most, for at most 143 ms at a time.
const NEAR: f32 = 0.05;

#[test]
fn output_is_the_same_however_the_host_splits_blocks() {
    let input_l = noise(48_000, 1);
    let input_r = noise(48_000, 2);
    let render = |sizes: &[usize]| {
        let mut e = engine(48_000.0, &settings()[1]);
        let (mut l, mut r) = (input_l.clone(), input_r.clone());
        let mut at = 0;
        let mut block = 0;
        while at < l.len() {
            let n = sizes[block % sizes.len()].min(l.len() - at);
            e.process(&mut [&mut l[at..at + n], &mut r[at..at + n]]);
            at += n;
            block += 1;
        }
        (l, r)
    };
    let whole = render(&[512]);
    for sizes in [&[1][..], &[64], &[7, 300, 1, 33, 512]] {
        let other = render(sizes);
        assert!(
            whole
                .0
                .iter()
                .zip(&other.0)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "{sizes:?}"
        );
        assert!(
            whole
                .1
                .iter()
                .zip(&other.1)
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "{sizes:?}"
        );
    }
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / x.len() as f64).sqrt()
}

/// The wet part (output − input) of a sine at `hz`, relative to the
/// sine, in dB.
fn wet_db(hz: f32) -> f64 {
    let rate = 48_000.0;
    let mut e = engine(rate, &[(LOW_XOVER, 150.0), (MIX, 100.0)]);
    let input: Vec<f32> = (0..96_000)
        .map(|n| (std::f32::consts::TAU * hz * n as f32 / rate).sin())
        .collect();
    let (mut l, mut r) = (input.clone(), input.clone());
    e.process(&mut [&mut l, &mut r]);
    let wet: Vec<f32> = l
        .iter()
        .zip(&input)
        .map(|(o, i)| o - i)
        .skip(48_000)
        .collect();
    20.0 * (rms(&wet) / rms(&input[48_000..])).log10()
}

#[test]
fn the_low_band_is_never_chorused() {
    let low = wet_db(40.0);
    assert!(low < -40.0, "40 Hz has wet at {low} dB");
    let mid = wet_db(1000.0);
    assert!(mid > -12.0, "1 kHz has wet at only {mid} dB");
}

#[test]
fn latency_is_zero_at_every_setting_and_rate() {
    assert_eq!(CharmEngine::LATENCY, 0);
    for rate in [44_100.0f32, 48_000.0, 96_000.0] {
        for params in settings() {
            let mut p = params.clone();
            p.push((MIX, 100.0));
            let mut e = engine(rate, &p);
            let mut l = vec![0.0f32; 4096];
            l[0] = 1.0;
            let mut r = l.clone();
            e.process(&mut [&mut l, &mut r]);
            // The impulse is out at once, and nothing else is until the
            // voices' floor.
            assert_eq!(l[0], 1.0);
            assert_eq!(r[0], 1.0);
            let floor = (FLOOR * f64::from(rate)) as usize - 2;
            assert!(l[1..floor].iter().all(|&s| s == 0.0), "{rate} {p:?}");
        }
    }
}

#[test]
fn one_channel_is_chorused_too() {
    let mut e = engine(48_000.0, &[(MIX, 100.0)]);
    let input = noise(48_000, 9);
    let mut x = input.clone();
    e.process(&mut [&mut x]);
    let wet: Vec<f32> = x.iter().zip(&input).map(|(o, i)| o - i).collect();
    assert!(rms(&wet[24_000..]) > 0.1 * rms(&input));
}

/// Sweeping SPREAD up and down landed voices on the floor at rest, where
/// the edge brake held them for good (seen in a DAW, ticket 0019). After
/// the sweeps every voice must keep moving through its range.
#[test]
fn voices_keep_moving_after_spread_is_swept() {
    let mut e = engine(48_000.0, &[]);
    let x = noise(480, 5);
    let block = |e: &mut CharmEngine| {
        let (mut l, mut r) = (x.clone(), x.clone());
        e.process(&mut [&mut l, &mut r]);
    };
    for k in 0..8 {
        e.set_param(SPREAD, if k % 2 == 0 { 25.0 } else { 1.0 });
        for _ in 0..50 {
            block(&mut e);
        }
    }
    e.set_param(SPREAD, 10.0);
    let mut highest = [[0.0f32; charm::MAX_VOICES]; 2];
    for _ in 0..500 {
        block(&mut e);
        let o = e.observation();
        for (b, band) in o.bands.iter().enumerate() {
            for (v, top) in band.voices.iter().zip(&mut highest[b]) {
                *top = top.max((v.delay - FLOOR as f32) / o.spread);
            }
        }
    }
    let active = usize::from(e.observation().bands[0].active);
    for (b, band) in highest.iter().enumerate() {
        for (i, top) in band[..active].iter().enumerate() {
            assert!(*top > 0.3, "band {b} voice {i} stayed low: {top}");
        }
    }
}
