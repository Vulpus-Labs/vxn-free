//! A peak envelope follower.

/// Follows a signal's peak level: it rises towards a louder input with
/// the attack time constant, and decays towards zero with the release
/// time constant when the input is below it.
#[derive(Clone, Copy, Debug)]
pub struct PeakFollower {
    attack: f32,
    release: f32,
    level: f32,
}

/// The coefficient of a one-pole filter with a time constant of `ms` at
/// `sample_rate`: the share of the old value kept each sample.
pub fn one_pole_coeff(ms: f32, sample_rate: f32) -> f32 {
    let samples = (ms * 0.001 * sample_rate).max(f32::MIN_POSITIVE);
    (-1.0 / samples).exp()
}

impl PeakFollower {
    /// At rest (level 0), with attack and release time constants in
    /// milliseconds at `sample_rate`.
    pub fn new(attack_ms: f32, release_ms: f32, sample_rate: f32) -> Self {
        Self {
            attack: one_pole_coeff(attack_ms, sample_rate),
            release: one_pole_coeff(release_ms, sample_rate),
            level: 0.0,
        }
    }

    /// Feed one sample's magnitude; the new level.
    #[inline]
    pub fn next(&mut self, magnitude: f32) -> f32 {
        self.level = if magnitude > self.level {
            self.attack * self.level + (1.0 - self.attack) * magnitude
        } else {
            self.release * self.level
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attack_reaches_63_percent_in_one_time_constant() {
        // 1 ms at 48 kHz is 48 samples.
        let mut p = PeakFollower::new(1.0, 50.0, 48_000.0);
        let v = (0..48).map(|_| p.next(1.0)).last().unwrap();
        assert!((v - (1.0 - (-1.0f32).exp())).abs() < 1e-3, "{v}");
    }

    #[test]
    fn the_release_decays_to_37_percent_in_one_time_constant() {
        let mut p = PeakFollower::new(0.0, 10.0, 48_000.0);
        p.next(1.0);
        assert!((p.level() - 1.0).abs() < 1e-6);
        let v = (0..480).map(|_| p.next(0.0)).last().unwrap();
        assert!((v - (-1.0f32).exp()).abs() < 1e-3, "{v}");
    }

    #[test]
    fn time_constants_follow_the_sample_rate() {
        let mut a = PeakFollower::new(1.0, 50.0, 44_100.0);
        let mut b = PeakFollower::new(1.0, 50.0, 96_000.0);
        let va = (0..441).map(|_| a.next(1.0)).last().unwrap();
        let vb = (0..960).map(|_| b.next(1.0)).last().unwrap();
        assert!((va - vb).abs() < 1e-4, "{va} {vb}");
    }
}
