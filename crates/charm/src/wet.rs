//! The wet gain after an onset: Duck and Bloom (ADR 0002, point 7).
//!
//! τ is the time since the onset's marked start (a few milliseconds before
//! the attack itself, since the analysis looks back). The gain is
//!
//! - Duck: 1 − DUCK, held for [`HOLD_SECONDS`] (past the attack), then
//!   recovering towards 1 with time constant [`RECOVER_SECONDS`];
//! - times Bloom: a raised cosine from 0 to 1 over BLOOM (1 throughout when
//!   BLOOM is 0), which passes −3 dB at [`BLOOM_HALF_POWER`] × BLOOM;
//! - crossfaded from wherever the gain was over the first
//!   [`ATTACK_SECONDS`], so a new onset never steps the wet signal.
//!
//! Before any onset the gain is 1.

use std::f32::consts::PI;

/// How long the duck holds full depth after the onset's marked start,
/// seconds: past the analysis's lookback (3.3 ms) and the attack's first
/// milliseconds.
pub const HOLD_SECONDS: f32 = 0.006;
/// The duck's recovery time constant, seconds (draft: settled by ear).
pub const RECOVER_SECONDS: f32 = 0.03;
/// The crossfade from the previous gain, seconds.
pub const ATTACK_SECONDS: f32 = 0.001;
/// Bloom's raised cosine is at −3 dB this far through BLOOM:
/// acos(1 − √2)/π.
pub const BLOOM_HALF_POWER: f32 = 0.636;

/// The shape's settings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shape {
    /// 0 to 1.
    pub duck: f32,
    /// Samples; 0 is off.
    pub bloom: f32,
}

/// One band's wet gain, a sample at a time.
#[derive(Clone, Copy, Debug)]
pub struct WetGain {
    shape: Shape,
    /// Samples since the onset's start; `None` before any onset.
    since: Option<u64>,
    /// The gain when the onset arrived.
    from: f32,
    hold: f32,
    recover: f32,
    attack: f32,
    /// The gain last given.
    current: f32,
}

impl WetGain {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            shape: Shape::default(),
            since: None,
            from: 1.0,
            hold: HOLD_SECONDS * sample_rate,
            recover: RECOVER_SECONDS * sample_rate,
            attack: ATTACK_SECONDS * sample_rate,
            current: 1.0,
        }
    }

    /// New settings, for what follows.
    pub fn set_shape(&mut self, shape: Shape) {
        self.shape = shape;
    }

    /// Back to no onset, gain 1.
    pub fn reset(&mut self) {
        self.since = None;
        self.from = 1.0;
        self.current = 1.0;
    }

    /// The gain `tau` samples after an onset's start.
    pub fn at(&self, tau: u64) -> f32 {
        let t = tau as f32;
        let duck = if t < self.hold {
            1.0 - self.shape.duck
        } else {
            1.0 - self.shape.duck * (-(t - self.hold) / self.recover).exp()
        };
        let bloom = if self.shape.bloom > 0.0 && t < self.shape.bloom {
            0.5 - 0.5 * (PI * t / self.shape.bloom).cos()
        } else {
            1.0
        };
        let a = (t / self.attack).min(1.0);
        self.from * (1.0 - a) + duck * bloom * a
    }

    /// An onset started `ago` samples before the next sample: from it the
    /// gain follows the shape, crossfading from where it is now.
    pub fn onset(&mut self, ago: u64) {
        self.from = self.current;
        self.since = Some(ago);
    }

    /// The gain for the next sample.
    #[inline]
    pub fn step(&mut self) -> f32 {
        let g = match self.since {
            None => 1.0,
            Some(tau) => {
                self.since = Some(tau + 1);
                self.at(tau)
            }
        };
        self.current = g;
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f32 = 48_000.0;

    fn after_onset(shape: Shape) -> WetGain {
        let mut w = WetGain::new(FS);
        w.set_shape(shape);
        w.onset(0);
        w
    }

    #[test]
    fn no_onset_no_change() {
        let mut w = WetGain::new(FS);
        w.set_shape(Shape {
            duck: 1.0,
            bloom: 9600.0,
        });
        assert!((0..10_000).all(|_| w.step() == 1.0));
    }

    #[test]
    fn the_duck_holds_then_recovers() {
        let mut w = after_onset(Shape {
            duck: 1.0,
            bloom: 0.0,
        });
        let g: Vec<f32> = (0..9600).map(|_| w.step()).collect();
        let at = |s: f32| g[(s * FS) as usize];
        // Fully ducked from the end of the crossfade to the end of the hold.
        assert!(at(0.0015) < 1.0e-6 && at(0.0059) < 1.0e-6);
        // One time constant into the recovery: 1 − 1/e.
        let one = at(HOLD_SECONDS + RECOVER_SECONDS);
        assert!((one - (1.0 - (-1.0f32).exp())).abs() < 1.0e-3, "{one}");
    }

    #[test]
    fn bloom_passes_minus_3_db_where_it_should() {
        let bloom = FS; // a second
        let mut w = after_onset(Shape { duck: 0.0, bloom });
        let g: Vec<f32> = (0..FS as usize).map(|_| w.step()).collect();
        let at = g[(BLOOM_HALF_POWER * bloom) as usize];
        assert!((20.0 * at.log10() + 3.01).abs() < 0.05, "{at}");
        assert!(g[(bloom * 0.01) as usize] < 0.01);
    }

    #[test]
    fn a_new_onset_never_steps_the_gain() {
        let mut w = after_onset(Shape {
            duck: 0.0,
            bloom: FS,
        });
        // Before the onset the gain was 1.
        let mut last = 1.0;
        for n in 0..96_000 {
            if n == 30_000 {
                w.onset(0);
            }
            let g = w.step();
            assert!((g - last).abs() < 0.03, "{n}: {last} → {g}");
            last = g;
        }
    }
}
