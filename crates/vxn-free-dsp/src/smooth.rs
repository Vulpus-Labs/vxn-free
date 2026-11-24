//! Parameter smoothing at control rate.
//!
//! The plugins update their settings once per control tick (a few dozen
//! samples), not every sample. A [`Glide`] moves a setting towards its
//! target once per tick, and a [`Ramp`] spreads one tick's change across
//! its samples where a step would be heard (gains, mixes).

/// Closer than this, a glide lands on its target rather than creeping
/// towards it for ever.
const SNAP: f32 = 1.0e-6;

/// A value that glides towards its target, one control tick at a time, as
/// a one-pole low-pass.
#[derive(Clone, Copy, Debug)]
pub struct Glide {
    current: f32,
    target: f32,
    /// The share of the remaining distance covered each tick.
    coeff: f32,
}

impl Glide {
    /// Settled at `initial`, with a time constant of `ms` when ticked
    /// `tick_rate` times a second.
    pub fn new(initial: f32, ms: f32, tick_rate: f32) -> Self {
        let ticks = (ms * 0.001 * tick_rate).max(1.0);
        Self {
            current: initial,
            target: initial,
            coeff: 1.0 - (-1.0 / ticks).exp(),
        }
    }

    /// Where to glide to.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Where it is gliding to.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Where it is now.
    pub fn value(&self) -> f32 {
        self.current
    }

    /// Jump to the target at once.
    pub fn snap(&mut self) {
        self.current = self.target;
    }

    /// One tick's step; the new value.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        self.current += self.coeff * (self.target - self.current);
        if (self.target - self.current).abs() < SNAP {
            self.current = self.target;
        }
        self.current
    }
}

/// A straight line from one tick's value to the next, one sample at a
/// time.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ramp {
    from: f32,
    step: f32,
}

impl Ramp {
    /// From `from`, reaching `to` at sample `samples − 1`.
    #[inline]
    pub fn new(from: f32, to: f32, samples: usize) -> Self {
        let step = if samples == 0 {
            0.0
        } else {
            (to - from) / samples as f32
        };
        Self { from, step }
    }

    /// The value at sample `i` of the tick.
    #[inline]
    pub fn at(&self, i: usize) -> f32 {
        self.from + self.step * (i + 1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glide_settles_within_five_time_constants() {
        // 10 ms at 1500 ticks a second: 15 ticks per time constant.
        let mut g = Glide::new(0.0, 10.0, 1500.0);
        g.set_target(1.0);
        let v = (0..75).map(|_| g.tick()).last().unwrap();
        assert!((v - 1.0).abs() < 0.01, "{v}");
    }

    #[test]
    fn a_glide_lands_exactly() {
        let mut g = Glide::new(1.0, 20.0, 1500.0);
        g.set_target(0.0);
        assert!((0..10_000).any(|_| g.tick() == 0.0));
    }

    #[test]
    fn a_ramp_ends_on_its_target() {
        let r = Ramp::new(1.0, 2.0, 4);
        let v: Vec<f32> = (0..4).map(|i| r.at(i)).collect();
        assert_eq!(v, [1.25, 1.5, 1.75, 2.0]);
    }
}
