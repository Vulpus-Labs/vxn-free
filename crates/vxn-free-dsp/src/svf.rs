//! A state-variable filter in topology-preserving (trapezoidal) form.
//!
//! Its coefficients can change every sample without the state blowing up
//! or clicking, which makes it the base for filters whose frequency is
//! automated, such as Charm's crossovers.

use std::f32::consts::PI;

/// One sample's outputs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SvfOut {
    /// Low-pass.
    pub lp: f32,
    /// Band-pass, with gain Q at its centre.
    pub bp: f32,
    /// High-pass.
    pub hp: f32,
}

/// A second-order state-variable filter, low-, band- and high-pass at once.
#[derive(Clone, Copy, Debug)]
pub struct Svf {
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    ic1: f32,
    ic2: f32,
}

impl Svf {
    /// Butterworth damping, Q = 1/√2.
    pub const BUTTERWORTH_Q: f32 = std::f32::consts::FRAC_1_SQRT_2;

    /// Tuned to `hz` with quality `q` at `sample_rate`.
    pub fn new(hz: f32, q: f32, sample_rate: f32) -> Self {
        let mut f = Self {
            k: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            ic1: 0.0,
            ic2: 0.0,
        };
        f.set(hz, q, sample_rate);
        f
    }

    /// Retune, keeping the state. `hz` is kept below Nyquist.
    pub fn set(&mut self, hz: f32, q: f32, sample_rate: f32) {
        let hz = hz.clamp(1.0, 0.49 * sample_rate);
        let g = (PI * hz / sample_rate).tan();
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// Forget the past.
    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }

    /// Filter one sample through the allpass response, x − 2·bp/Q: unity
    /// gain at every frequency, its phase turning a full cycle about the
    /// centre, faster the higher the Q.
    #[inline]
    pub fn allpass(&mut self, x: f32) -> f32 {
        let bp = self.tick(x).bp;
        x - 2.0 * self.k * bp
    }

    /// Filter one sample.
    #[inline]
    pub fn tick(&mut self, x: f32) -> SvfOut {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        SvfOut {
            lp: v2,
            bp: v1,
            hp: x - self.k * v1 - v2,
        }
    }
}

/// Allpass [`Svf`]s side by side, one a lane, as structure-of-arrays: the
/// same sums as [`Svf::allpass`] on each lane, laid out so the compiler
/// runs the lanes together in SIMD. For per-voice decorrelation, where
/// every voice has its own, fixed, filter.
#[derive(Clone, Copy, Debug)]
pub struct AllpassBank<const N: usize> {
    k2: [f32; N],
    a1: [f32; N],
    a2: [f32; N],
    a3: [f32; N],
    ic1: [f32; N],
    ic2: [f32; N],
}

impl<const N: usize> AllpassBank<N> {
    /// Lane `l` filters as `filters[l]` does, from its state.
    pub fn new(filters: &[Svf; N]) -> Self {
        Self {
            k2: filters.map(|f| 2.0 * f.k),
            a1: filters.map(|f| f.a1),
            a2: filters.map(|f| f.a2),
            a3: filters.map(|f| f.a3),
            ic1: filters.map(|f| f.ic1),
            ic2: filters.map(|f| f.ic2),
        }
    }

    /// Each lane's next sample, in place.
    // Lanes index several parallel arrays at once: the plain form vectorises.
    #[allow(clippy::needless_range_loop)]
    #[inline]
    pub fn process(&mut self, x: &mut [f32; N]) {
        for l in 0..N {
            let v3 = x[l] - self.ic2[l];
            let v1 = self.a1[l] * self.ic1[l] + self.a2[l] * v3;
            let v2 = self.ic2[l] + self.a2[l] * self.ic1[l] + self.a3[l] * v3;
            self.ic1[l] = 2.0 * v1 - self.ic1[l];
            self.ic2[l] = 2.0 * v2 - self.ic2[l];
            x[l] -= self.k2[l] * v1;
        }
    }

    /// Forget the past.
    pub fn reset(&mut self) {
        self.ic1 = [0.0; N];
        self.ic2 = [0.0; N];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn the_allpass_is_flat() {
        let fs = 48_000.0;
        for q in [0.5f32, 1.0, 3.0] {
            for hz in [50.0f64, 300.0, 1000.0, 5000.0, 15_000.0] {
                let mut f = Svf::new(1000.0, q, fs);
                let w = TAU * hz / f64::from(fs);
                let (mut si, mut so) = (0.0f64, 0.0f64);
                for n in 0..96_000 {
                    let x = (w * f64::from(n)).sin() as f32;
                    let y = f.allpass(x);
                    if n >= 48_000 {
                        si += f64::from(x * x);
                        so += f64::from(y * y);
                    }
                }
                let db = 10.0 * (so / si).log10();
                assert!(db.abs() < 0.01, "Q {q}, {hz} Hz: {db} dB");
            }
        }
    }

    #[test]
    fn a_bank_filters_each_lane_as_its_own_allpass() {
        let rate = 48_000.0;
        let mut one: [Svf; 4] =
            std::array::from_fn(|l| Svf::new(300.0 * (l + 1) as f32, 0.7 + l as f32, rate));
        let mut bank = AllpassBank::new(&one);
        for n in 0..500 {
            let mut x: [f32; 4] = std::array::from_fn(|l| ((n * (l + 3)) as f32 * 0.01).sin());
            let want: [f32; 4] = std::array::from_fn(|l| one[l].allpass(x[l]));
            bank.process(&mut x);
            for l in 0..4 {
                assert!(
                    (x[l] - want[l]).abs() <= 4.0 * f32::EPSILON * want[l].abs().max(1.0),
                    "lane {l} sample {n}"
                );
            }
        }
    }
}
