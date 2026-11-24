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
