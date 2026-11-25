//! A Linkwitz-Riley (fourth-order) crossover.
//!
//! The low band is a second-order Butterworth low-pass applied twice, the
//! high band a second-order Butterworth high-pass applied twice. Each is
//! 6 dB down at the crossover, and the two sum to an allpass: flat in
//! magnitude, with a phase turn of 360° about the crossover.
//!
//! The first stage's state-variable filter gives both the first low-pass
//! and the first high-pass, so a crossover is three filters, not four.
//! Its frequency can be moved while it runs: the filters are in
//! topology-preserving form, so retuning them doesn't click. Move it in
//! small steps (a glide per control tick) to avoid zipper noise.
//!
//! [`ThreeBand`] splits into three bands that sum flat. A crossover's two
//! bands sum to a second-order allpass at its frequency (Q = 1/√2):
//! LP² + HP² = (s² − √2·s + 1) / (s² + √2·s + 1). The input splits at
//! the lower frequency and the high side again at the upper, so the mid
//! and high bands together carry the upper crossover's allpass. Passing
//! the low band through that same allpass makes all three sum to the two
//! allpasses in series: flat in magnitude, with nothing delayed. The
//! allpass is the same state-variable filter as the crossovers', so the
//! match holds sample for sample, not just in the analogue prototype.

use crate::svf::Svf;

/// Splits a signal into a low and a high band about one frequency.
#[derive(Clone, Copy, Debug)]
pub struct Crossover {
    first: Svf,
    low: Svf,
    high: Svf,
    sample_rate: f32,
}

impl Crossover {
    /// Crossing at `hz`, at `sample_rate`.
    pub fn new(hz: f32, sample_rate: f32) -> Self {
        let f = Svf::new(hz, Svf::BUTTERWORTH_Q, sample_rate);
        Self {
            first: f,
            low: f,
            high: f,
            sample_rate,
        }
    }

    /// Move the crossover to `hz`, keeping the filters' state.
    pub fn set_frequency(&mut self, hz: f32) {
        for f in [&mut self.first, &mut self.low, &mut self.high] {
            f.set(hz, Svf::BUTTERWORTH_Q, self.sample_rate);
        }
    }

    /// Forget the past.
    pub fn reset(&mut self) {
        self.first.reset();
        self.low.reset();
        self.high.reset();
    }

    /// One sample, split: `(low, high)`.
    #[inline]
    pub fn split(&mut self, x: f32) -> (f32, f32) {
        let s = self.first.tick(x);
        (self.low.tick(s.lp).lp, self.high.tick(s.hp).hp)
    }
}

/// The least ratio [`ThreeBand`] keeps between its upper and lower
/// frequencies: an octave, so the mid band never vanishes and the
/// filters never cross.
pub const MIN_BAND_RATIO: f32 = 2.0;

/// Splits a signal into low, mid and high bands that sum to an allpass of
/// it (see the module docs).
#[derive(Clone, Copy, Debug)]
pub struct ThreeBand {
    lower: Crossover,
    upper: Crossover,
    /// The upper crossover's allpass, for the low band.
    align: Svf,
    sample_rate: f32,
}

impl ThreeBand {
    /// Crossing at `low_hz` and `high_hz`, at `sample_rate`; `high_hz` is
    /// held at least [`MIN_BAND_RATIO`] above `low_hz`.
    pub fn new(low_hz: f32, high_hz: f32, sample_rate: f32) -> Self {
        let high_hz = high_hz.max(low_hz * MIN_BAND_RATIO);
        Self {
            lower: Crossover::new(low_hz, sample_rate),
            upper: Crossover::new(high_hz, sample_rate),
            align: Svf::new(high_hz, Svf::BUTTERWORTH_Q, sample_rate),
            sample_rate,
        }
    }

    /// Move both crossovers, keeping the filters' state. Move them in
    /// small steps, as a [`Crossover`]'s.
    pub fn set_frequencies(&mut self, low_hz: f32, high_hz: f32) {
        let high_hz = high_hz.max(low_hz * MIN_BAND_RATIO);
        self.lower.set_frequency(low_hz);
        self.upper.set_frequency(high_hz);
        self.align
            .set(high_hz, Svf::BUTTERWORTH_Q, self.sample_rate);
    }

    /// Forget the past.
    pub fn reset(&mut self) {
        self.lower.reset();
        self.upper.reset();
        self.align.reset();
    }

    /// One sample, split: `[low, mid, high]`.
    #[inline]
    pub fn split(&mut self, x: f32) -> [f32; 3] {
        let (low, rest) = self.lower.split(x);
        let (mid, high) = self.upper.split(rest);
        [self.align.allpass(low), mid, high]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smooth::Glide;
    use std::f64::consts::TAU;

    /// Steady-state gain of `band` (0 low, 1 high, 2 sum) at `hz`.
    fn gain(hz: f64, fs: f32, xover: f32, band: usize) -> f64 {
        let mut c = Crossover::new(xover, fs);
        let w = TAU * hz / f64::from(fs);
        // Settle for a long while at low frequencies, then measure whole
        // cycles' RMS.
        let settle = (f64::from(fs) * 0.5) as usize;
        let cycles = (hz * 0.5).max(20.0);
        let len = (cycles * f64::from(fs) / hz) as usize;
        let (mut si, mut so) = (0.0f64, 0.0f64);
        for n in 0..settle + len {
            let x = (w * n as f64).sin() as f32;
            let (lo, hi) = c.split(x);
            let y = [lo, hi, lo + hi][band];
            if n >= settle {
                si += f64::from(x) * f64::from(x);
                so += f64::from(y) * f64::from(y);
            }
        }
        10.0 * (so / si).log10()
    }

    #[test]
    fn the_bands_sum_flat() {
        for fs in [44_100.0, 48_000.0, 96_000.0] {
            for xover in [150.0, 2500.0] {
                for hz in [
                    20.0, 60.0, 150.0, 400.0, 1000.0, 2500.0, 6000.0, 12_000.0, 18_000.0,
                ] {
                    let g = gain(hz, fs, xover, 2);
                    assert!(
                        g.abs() < 0.01,
                        "{fs} Hz, crossing {xover}: {hz} Hz sums to {g} dB"
                    );
                }
            }
        }
    }

    #[test]
    fn each_band_is_6_db_down_at_the_crossover() {
        for band in [0, 1] {
            let g = gain(1000.0, 48_000.0, 1000.0, band);
            assert!((g + 6.02).abs() < 0.05, "band {band}: {g} dB");
        }
    }

    #[test]
    fn the_bands_fall_at_24_db_per_octave() {
        // Two octaves past the crossover a fourth-order slope is ~48 dB
        // down, less the shoulder.
        let lo = gain(4000.0, 48_000.0, 1000.0, 0);
        let hi = gain(250.0, 48_000.0, 1000.0, 1);
        assert!(lo < -45.0, "{lo}");
        assert!(hi < -45.0, "{hi}");
    }

    #[test]
    fn a_gliding_crossover_does_not_click() {
        // A sine through a crossover moving 150 Hz → 600 Hz in control-tick
        // steps, against the same sine through a still one. A click shows
        // as a jump in the second difference.
        let fs = 48_000.0f32;
        let w = TAU * 300.0 / f64::from(fs);
        let tick = 32;
        let mut glide = Glide::new(150.0, 50.0, fs / tick as f32);
        glide.set_target(600.0);
        let mut moving = Crossover::new(150.0, fs);
        let mut still = Crossover::new(150.0, fs);
        let worst = |ys: &[f32]| {
            ys.windows(3)
                .map(|v| (v[2] - 2.0 * v[1] + v[0]).abs())
                .fold(0.0f32, f32::max)
        };
        let (mut moved, mut held) = (Vec::new(), Vec::new());
        for n in 0..48_000 {
            if n % tick == 0 {
                moving.set_frequency(glide.tick());
            }
            let x = (w * f64::from(n)).sin() as f32;
            let (lo, hi) = moving.split(x);
            moved.push(lo);
            moved.push(hi);
            let (lo, hi) = still.split(x);
            held.push(lo);
            held.push(hi);
        }
        // De-interleave the bands, skipping the start-up transient.
        for band in 0..2 {
            let m: Vec<f32> = moved
                .iter()
                .skip(band)
                .step_by(2)
                .skip(4800)
                .copied()
                .collect();
            let s: Vec<f32> = held
                .iter()
                .skip(band)
                .step_by(2)
                .skip(4800)
                .copied()
                .collect();
            // A pure 300 Hz sine of amplitude 1 has second differences up
            // to w² ≈ 1.5e-3; a click would be far larger.
            let limit = 1.2 * (w * w) as f32;
            assert!(
                worst(&m) < limit,
                "band {band}: {} vs {}",
                worst(&m),
                worst(&s)
            );
        }
    }

    /// The impulse response of `band` (0 low, 1 mid, 2 high, 3 the sum)
    /// of a three-band split.
    fn three_band_ir(lo: f32, hi: f32, fs: f32, band: usize, len: usize) -> Vec<f64> {
        let mut t = ThreeBand::new(lo, hi, fs);
        (0..len)
            .map(|n| {
                let b = t.split(if n == 0 { 1.0 } else { 0.0 });
                let y = if band == 3 {
                    b[0] + b[1] + b[2]
                } else {
                    b[band]
                };
                f64::from(y)
            })
            .collect()
    }

    /// The gain in dB of an impulse response at `hz`.
    fn ir_gain(ir: &[f64], hz: f64, fs: f32) -> f64 {
        let w = TAU * hz / f64::from(fs);
        let (re, im) = ir.iter().enumerate().fold((0.0, 0.0), |(re, im), (n, h)| {
            let a = w * n as f64;
            (re + h * a.cos(), im - h * a.sin())
        });
        10.0 * (re * re + im * im).log10()
    }

    const HZ: [f64; 12] = [
        20.0, 40.0, 100.0, 200.0, 500.0, 1000.0, 2500.0, 5000.0, 8000.0, 12_000.0, 16_000.0,
        20_000.0,
    ];

    #[test]
    fn three_bands_sum_flat() {
        for fs in [44_100.0, 48_000.0, 96_000.0] {
            for (lo, hi) in [
                (200.0, 2500.0),
                (40.0, 500.0),
                (1000.0, 12_000.0),
                (300.0, 600.0),
            ] {
                let ir = three_band_ir(lo, hi, fs, 3, 1 << 16);
                for hz in HZ {
                    let g = ir_gain(&ir, hz, fs);
                    assert!(
                        g.abs() < 0.01,
                        "{fs} Hz, crossing {lo}/{hi}: {hz} Hz sums to {g} dB"
                    );
                }
            }
        }
    }

    #[test]
    fn three_bands_are_6_db_down_at_their_crossovers_and_fall_at_24_db() {
        let fs = 48_000.0;
        let (lo, hi) = (200.0, 3200.0);
        let band = |b: usize, hz: f64| ir_gain(&three_band_ir(lo, hi, fs, b, 1 << 16), hz, fs);
        // The far crossover adds a little to the mid band's loss at each.
        assert!((band(0, 200.0) + 6.02).abs() < 0.05, "{}", band(0, 200.0));
        assert!((band(1, 200.0) + 6.02).abs() < 0.1, "{}", band(1, 200.0));
        assert!((band(1, 3200.0) + 6.02).abs() < 0.1, "{}", band(1, 3200.0));
        assert!((band(2, 3200.0) + 6.02).abs() < 0.05, "{}", band(2, 3200.0));
        // An octave further out, each falls 24 dB more (within the
        // slope's shoulder).
        let octave = |b: usize, near: f64, far: f64| band(b, near) - band(b, far);
        assert!((octave(0, 1600.0, 3200.0) - 24.0).abs() < 1.0);
        // Well above the lower crossover, so only the upper one shapes it.
        assert!(
            (octave(2, 1600.0, 800.0) - 24.0).abs() < 1.0,
            "{}",
            octave(2, 1600.0, 800.0)
        );
        assert!((octave(1, 50.0, 25.0) - 24.0).abs() < 1.0);
        // Towards Nyquist the bilinear transform steepens the slope.
        assert!(
            octave(1, 6400.0, 12_800.0) > 23.0,
            "{}",
            octave(1, 6400.0, 12_800.0)
        );
    }

    #[test]
    fn three_bands_add_no_delay() {
        // No fixed delay: the sum's impulse response starts at once, with
        // much of its energy. The allpasses turn the phase, so the
        // response rings over the next few samples, more with the upper
        // crossover near a quarter of the rate. First samples at 48 kHz
        // (ticket 0031): 200/2500 Hz 0.61, −0.60; 40/500 0.91, −0.18;
        // 1000/12000 0.14, −0.05, 0.76.
        for (lo, hi) in [
            (40.0, 500.0),
            (200.0, 2500.0),
            (500.0, 4000.0),
            (1000.0, 6000.0),
            (1000.0, 12_000.0),
        ] {
            let ir = three_band_ir(lo, hi, 48_000.0, 3, 4096);
            assert!(ir[0] > 0.1, "{lo}/{hi}: starts at {}", ir[0]);
        }
    }

    #[test]
    fn the_upper_crossover_stays_an_octave_above_the_lower() {
        let fs = 48_000.0;
        let held = three_band_ir(1000.0, 1200.0, fs, 3, 1 << 15);
        let octave = three_band_ir(1000.0, 2000.0, fs, 3, 1 << 15);
        assert_eq!(held, octave);
        let mut t = ThreeBand::new(200.0, 2500.0, fs);
        t.set_frequencies(1000.0, 1200.0);
        let mut u = ThreeBand::new(1000.0, 2000.0, fs);
        for n in 0..1000 {
            let x = (n as f32 * 0.05).sin();
            assert_eq!(t.split(x), u.split(x));
        }
    }

    #[test]
    fn moving_three_band_crossovers_do_not_click() {
        // Both crossovers glide (100 → 400 Hz, 2 → 6 kHz) under a sine;
        // the sum's second difference stays as small as with them still.
        let fs = 48_000.0f32;
        let w = TAU * 700.0 / f64::from(fs);
        let tick = 32;
        let mut lo = Glide::new(100.0, 50.0, fs / tick as f32);
        let mut hi = Glide::new(2000.0, 50.0, fs / tick as f32);
        lo.set_target(400.0);
        hi.set_target(6000.0);
        let mut moving = ThreeBand::new(100.0, 2000.0, fs);
        let mut still = ThreeBand::new(100.0, 2000.0, fs);
        let worst = |ys: &[f32]| {
            ys.windows(3)
                .map(|v| (v[2] - 2.0 * v[1] + v[0]).abs())
                .fold(0.0f32, f32::max)
        };
        let (mut moved, mut held) = (Vec::new(), Vec::new());
        for n in 0..48_000 {
            if n % tick == 0 {
                moving.set_frequencies(lo.tick(), hi.tick());
            }
            let x = (w * f64::from(n)).sin() as f32;
            moved.push(moving.split(x).iter().sum::<f32>());
            held.push(still.split(x).iter().sum::<f32>());
        }
        let (m, h) = (worst(&moved[4800..]), worst(&held[4800..]));
        assert!(m < h * 1.5, "moving {m} against still {h}");
    }
}
