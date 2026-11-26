//! A band-pass resonator that can sing: a state-variable filter whose
//! damping may reach zero and go below it, with the level of its
//! oscillation held by the damping rather than by anything that saturates.
//!
//! # The filter
//!
//! Two trapezoidal integrators in a loop, solved without a delay
//! (topology-preserving form), prewarped by `g = tan(π f / fs)`, in `f64`.
//! The band-pass's gain at its centre is `1 / k`, so `k · bp` is unity
//! there, and its bandwidth is `k · f`. With `k` at zero the poles sit on
//! the unit circle at exactly the prewarped frequency: a lossless
//! oscillator whose band-pass and low-pass outputs are a sine and a cosine
//! of the same amplitude.
//!
//! # Its level
//!
//! [`Resonator::level`] reads the integrators' energy,
//! `(ic1² + ic2²) / (1 + g²)`. With no damping the loop conserves
//! `ic1² + ic2²` exactly however `g` moves, so the reading is the
//! oscillation's amplitude, steady over a cycle and through a glide.
//!
//! # Holding a level
//!
//! A limiter inside the loop would pull the pitch with the level. Instead,
//! once a control block, [`Resonator::holding_damping`] gives
//! `growth · (A² / target² − 1)`: negative while the oscillation is small,
//! so it grows; zero at the target; positive above it. Its slope is capped
//! so one block corrects at most half the level's error, and the damping
//! can't swing and drag the pitch. Damping moves the resonant frequency by
//! about `k² / 8` of itself, which at a held oscillation is negligible.

use std::f64::consts::PI;

/// The highest centre, as a share of the sample rate: below the cramp at
/// Nyquist.
pub const MAX_FREQUENCY_SHARE: f64 = 0.45;

/// The share of the level's error the growth may correct in one block.
const MOST_PER_BLOCK: f64 = 0.5;

/// A TPT state-variable band-pass with a damping that may go negative.
#[derive(Clone, Copy, Debug)]
pub struct Resonator {
    rate: f64,
    ic1: f64,
    ic2: f64,
    hz: f64,
    g: f64,
    k: f64,
    a1: f64,
    a2: f64,
    a3: f64,
}

impl Resonator {
    /// Silent, at `hz` with damping 1, at `sample_rate`.
    pub fn new(sample_rate: f64, hz: f64) -> Self {
        let mut r = Self {
            rate: sample_rate,
            ic1: 0.0,
            ic2: 0.0,
            hz: 0.0,
            g: 0.0,
            k: 1.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
        };
        r.set_frequency(hz);
        r
    }

    /// The centre in Hz, kept between 1 Hz and [`MAX_FREQUENCY_SHARE`] of
    /// the rate. Keeps the state.
    pub fn set_frequency(&mut self, hz: f64) {
        self.hz = hz.clamp(1.0, MAX_FREQUENCY_SHARE * self.rate);
        self.g = (PI * self.hz / self.rate).tan();
        self.coefficients();
    }

    /// The damping `k`: `1 / Q` above zero, lossless at zero, growing
    /// below it.
    pub fn set_damping(&mut self, k: f64) {
        self.k = k;
        self.coefficients();
    }

    fn coefficients(&mut self) {
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }

    /// One sample in; the band-pass out.
    #[inline]
    pub fn next(&mut self, x: f64) -> f64 {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        v1
    }

    /// The centre, Hz.
    pub fn frequency(&self) -> f64 {
        self.hz
    }

    /// The damping.
    pub fn damping(&self) -> f64 {
        self.k
    }

    /// The oscillation's amplitude squared, from the integrators' energy.
    pub fn level_squared(&self) -> f64 {
        (self.ic1 * self.ic1 + self.ic2 * self.ic2) / (1.0 + self.g * self.g)
    }

    /// The oscillation's amplitude, in the input's units.
    pub fn level(&self) -> f64 {
        self.level_squared().sqrt()
    }

    /// The damping that steers the level to `target`, for a block of
    /// `block` samples whose highest centre is `peak_hz` (the damping acts
    /// fastest there), kept within `−growth ..= max_damping`.
    pub fn holding_damping(
        &self,
        target: f64,
        growth: f64,
        block: usize,
        peak_hz: f64,
        max_damping: f64,
    ) -> f64 {
        // The level moves by k·π·f/fs a sample, and the law's slope is
        // 2·growth: cap it so one block corrects at most MOST_PER_BLOCK.
        let most = MOST_PER_BLOCK / (2.0 * PI * peak_hz.max(self.hz) / self.rate * block as f64);
        let rate = growth.min(most);
        let d = rate * (self.level_squared() / (target * target) - 1.0);
        d.clamp(-rate, max_damping)
    }

    /// Silence.
    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }

    /// Start an oscillation of `amplitude`: a blow to the low-pass state,
    /// so the band-pass begins as a sine. Oscillation from exact silence
    /// needs a seed.
    pub fn seed(&mut self, amplitude: f64) {
        self.ic2 += amplitude * (1.0 + self.g * self.g).sqrt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    const RATE: f64 = 48_000.0;

    /// The steady gain of `k · bp` for a sine at `hz`.
    #[allow(clippy::many_single_char_names)]
    fn gain(centre: f64, k: f64, hz: f64) -> f64 {
        let mut r = Resonator::new(RATE, centre);
        r.set_damping(k);
        let w = TAU * hz / RATE;
        let settle = (40.0 / (k * centre) * RATE) as usize + 4800;
        let (mut si, mut so) = (0.0, 0.0);
        for n in 0..settle + 48_000 {
            let x = (w * n as f64).sin();
            let y = k * r.next(x);
            if n >= settle {
                si += x * x;
                so += y * y;
            }
        }
        (so / si).sqrt()
    }

    #[test]
    fn unity_at_the_centre() {
        for centre in [100.0, 1000.0, 5000.0, 15_000.0] {
            let g = gain(centre, 0.05, centre);
            assert!((g - 1.0).abs() < 1e-3, "{centre}: {g}");
        }
    }

    #[test]
    fn the_band_edges_are_where_the_damping_puts_them() {
        // The −3 dB edges of a band-pass with bandwidth k·f sit at
        // f·(√(1 + k²/4) ± k/2).
        for k in [0.03, 0.1, 0.3] {
            let f = 1000.0;
            let s = (1.0 + k * k / 4.0f64).sqrt();
            for edge in [f * (s - k / 2.0), f * (s + k / 2.0)] {
                let db = 20.0 * gain(f, k, edge).log10();
                // Prewarping keeps the centre exact; the edges move a
                // little at 1 kHz.
                assert!((db + 3.01).abs() < 0.05, "k {k} edge {edge}: {db} dB");
            }
        }
    }

    /// Hold an oscillation at `target` for `seconds`, the centre gliding
    /// by `glide(t)`; returns the zero-crossing frequency over the last
    /// second and the level's worst error over it.
    fn oscillate(hz: f64, target: f64, glide: impl Fn(f64) -> f64, seconds: f64) -> (f64, f64) {
        let mut r = Resonator::new(RATE, hz);
        r.set_damping(0.0);
        r.seed(1e-3 * target);
        let total = (seconds * RATE) as usize;
        let block = 32;
        let (mut prev, mut first, mut last, mut count) = (0.0, None, 0.0, 0);
        let mut worst: f64 = 0.0;
        for n in 0..total {
            if n % block == 0 {
                r.set_frequency(hz * glide(n as f64 / RATE));
                let k = r.holding_damping(target, 0.05, block, r.frequency(), 0.03);
                r.set_damping(k);
            }
            let y = r.next(0.0);
            if n >= total - RATE as usize {
                worst = worst.max((r.level() / target - 1.0).abs());
                if prev < 0.0 && y >= 0.0 {
                    let c = n as f64 - 1.0 + (-prev) / (y - prev);
                    first.get_or_insert(c);
                    last = c;
                    count += 1;
                }
            }
            prev = y;
        }
        let f = f64::from(count - 1) * RATE / (last - first.unwrap());
        (f, worst)
    }

    #[test]
    fn self_oscillation_sits_on_its_tuning_and_holds_its_level() {
        for hz in [55.0, 440.0, 3000.0, 19_000.0] {
            let (f, worst) = oscillate(hz, 0.25, |_| 1.0, 3.0);
            let cents = 1200.0 * (f / hz).log2();
            assert!(cents.abs() < 0.01, "{hz}: {f} Hz, {cents} cents");
            assert!(worst < 1e-3, "{hz}: level off by {worst}");
        }
    }

    #[test]
    fn the_level_holds_through_a_glide() {
        // An octave up and back over the last second.
        let glide = |t: f64| {
            if t < 2.0 {
                1.0
            } else {
                2f64.powf(1.0 - (2.0 * (t - 2.0) - 1.0).abs())
            }
        };
        let (_, worst) = oscillate(300.0, 0.25, glide, 3.0);
        assert!(worst < 0.01, "level off by {worst}");
    }

    #[test]
    fn a_ring_dies_once_the_damping_is_back() {
        let mut r = Resonator::new(RATE, 1000.0);
        r.set_damping(0.0);
        r.seed(0.5);
        for _ in 0..4800 {
            r.next(0.0);
        }
        r.set_damping(0.05);
        for _ in 0..48_000 {
            r.next(0.0);
        }
        assert!(r.level() < 1e-6, "{}", r.level());
    }
}
