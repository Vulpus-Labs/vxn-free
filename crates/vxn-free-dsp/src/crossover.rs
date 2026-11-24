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
}
