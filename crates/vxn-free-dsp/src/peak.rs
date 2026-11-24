//! Envelope followers: a peak follower on a signal's magnitude, and a
//! range follower on its maximum and minimum.
//!
//! Both let go at a rate given as a *fall*: the share of the distance to
//! where they are falling to that is left after one sample. [`fall`]
//! works it out from the time to fall 20 dB.

/// The fall per sample that takes `seconds` to drop 20 dB (to a tenth) at
/// `sample_rate`.
pub fn fall(seconds: f32, sample_rate: f32) -> f32 {
    let samples = (seconds * sample_rate).max(1.0);
    0.1f32.powf(1.0 / samples)
}

/// The coefficient of a one-pole filter with a time constant of `ms` at
/// `sample_rate`: the share of the old value kept each sample.
pub fn one_pole_coeff(ms: f32, sample_rate: f32) -> f32 {
    let samples = (ms * 0.001 * sample_rate).max(f32::MIN_POSITIVE);
    (-1.0 / samples).exp()
}

/// Follows a signal's peak magnitude: it rises towards a louder input
/// with the attack time constant, and falls towards zero at its fall
/// rate when the input is below it.
#[derive(Clone, Copy, Debug)]
pub struct PeakFollower {
    attack: f32,
    fall: f32,
    level: f32,
}

impl PeakFollower {
    /// At rest (level 0), attacking with a time constant of `attack_ms`
    /// at `sample_rate`, falling by `fall` per sample.
    pub fn new(attack_ms: f32, fall: f32, sample_rate: f32) -> Self {
        Self {
            attack: one_pole_coeff(attack_ms, sample_rate),
            fall,
            level: 0.0,
        }
    }

    /// Set how fast it falls (see [`fall`]).
    pub fn set_fall(&mut self, fall: f32) {
        self.fall = fall;
    }

    /// Feed one sample's magnitude; the new level.
    #[inline]
    pub fn next(&mut self, magnitude: f32) -> f32 {
        self.level = if magnitude > self.level {
            self.attack * self.level + (1.0 - self.attack) * magnitude
        } else {
            self.fall * self.level
        };
        self.level
    }

    /// The current level.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Back to rest.
    pub fn reset(&mut self) {
        self.level = 0.0;
    }
}

/// Follows a signal's maximum and minimum. A sample beyond either takes
/// its place at once; otherwise both fall towards their midpoint at the
/// fall rate. The half-range, (max − min) / 2, is the signal's swing
/// whatever its offset; the midpoint is that offset.
#[derive(Clone, Copy, Debug, Default)]
pub struct RangeFollower {
    max: f32,
    min: f32,
    fall: f32,
}

impl RangeFollower {
    /// At rest (max and min at 0), falling by `fall` per sample.
    pub fn new(fall: f32) -> Self {
        Self {
            max: 0.0,
            min: 0.0,
            fall,
        }
    }

    /// Set how fast it falls (see [`fall`]).
    pub fn set_fall(&mut self, fall: f32) {
        self.fall = fall;
    }

    /// One sample's fall towards the midpoint.
    #[inline]
    pub fn decay(&mut self) {
        let mid = self.mid();
        self.max = mid + (self.max - mid) * self.fall;
        self.min = mid + (self.min - mid) * self.fall;
    }

    /// Take in a sample, after this sample's [`decay`](Self::decay).
    /// Several may be pushed per decay, to follow several channels as one.
    #[inline]
    pub fn push(&mut self, x: f32) {
        self.max = self.max.max(x);
        self.min = self.min.min(x);
    }

    /// Decay, then push `x`: one sample of one signal.
    #[inline]
    pub fn next(&mut self, x: f32) {
        self.decay();
        self.push(x);
    }

    /// The midpoint of max and min.
    #[inline]
    pub fn mid(&self) -> f32 {
        0.5 * (self.max + self.min)
    }

    /// Half the distance from min to max.
    #[inline]
    pub fn half_range(&self) -> f32 {
        0.5 * (self.max - self.min)
    }

    /// Back to rest.
    pub fn reset(&mut self) {
        self.max = 0.0;
        self.min = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fall_takes_its_time_to_drop_20_db() {
        let f = fall(0.01, 48_000.0);
        assert!((f.powi(480) - 0.1).abs() < 1e-4);
    }

    #[test]
    fn the_attack_reaches_63_percent_in_one_time_constant() {
        // 1 ms at 48 kHz is 48 samples.
        let mut p = PeakFollower::new(1.0, 0.0, 48_000.0);
        let v = (0..48).map(|_| p.next(1.0)).last().unwrap();
        assert!((v - (1.0 - (-1.0f32).exp())).abs() < 1e-3, "{v}");
    }

    #[test]
    fn a_peak_falls_20_db_in_its_fall_time() {
        let mut p = PeakFollower::new(0.0, fall(0.01, 48_000.0), 48_000.0);
        p.next(1.0);
        assert!((p.level() - 1.0).abs() < 1e-6);
        let v = (0..480).map(|_| p.next(0.0)).last().unwrap();
        assert!((v - 0.1).abs() < 1e-3, "{v}");
    }

    #[test]
    fn the_attack_follows_the_sample_rate() {
        let mut a = PeakFollower::new(1.0, 0.0, 44_100.0);
        let mut b = PeakFollower::new(1.0, 0.0, 96_000.0);
        let va = (0..441).map(|_| a.next(1.0)).last().unwrap();
        let vb = (0..960).map(|_| b.next(1.0)).last().unwrap();
        assert!((va - vb).abs() < 1e-4, "{va} {vb}");
    }

    #[test]
    fn a_range_takes_a_new_extreme_at_once() {
        let mut r = RangeFollower::new(fall(0.1, 48_000.0));
        r.next(0.8);
        assert_eq!((r.mid(), r.half_range()), (0.4, 0.4));
        r.next(-0.8);
        assert!(r.mid().abs() < 1e-3 && (r.half_range() - 0.8).abs() < 1e-3);
    }

    #[test]
    fn a_range_falls_20_db_towards_its_midpoint() {
        let mut r = RangeFollower::new(fall(0.01, 48_000.0));
        // From rest (0, 0): max 0.7, min −0.3, so midpoint 0.2, half 0.5.
        r.push(0.7);
        r.push(-0.3);
        for _ in 0..480 {
            r.next(0.2);
        }
        assert!((r.mid() - 0.2).abs() < 1e-6);
        assert!((r.half_range() - 0.05).abs() < 1e-4, "{}", r.half_range());
    }

    #[test]
    fn a_range_sees_an_offset_signal_by_its_swing() {
        let mut r = RangeFollower::new(fall(0.1, 48_000.0));
        for i in 0..4800 {
            r.next(0.5 + 0.1 * (i as f32 * 0.05).sin());
        }
        assert!((r.mid() - 0.5).abs() < 0.01, "{}", r.mid());
        assert!((r.half_range() - 0.1).abs() < 0.01, "{}", r.half_range());
    }
}
