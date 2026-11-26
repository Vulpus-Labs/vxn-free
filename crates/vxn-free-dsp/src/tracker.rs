//! Follows one partial's frequency at audio rate.
//!
//! A band-pass ([`Resonator`]) on the input is centred on the partial, and
//! the frequency is measured over [`SPAN`] periods between the band-pass's
//! upward zero crossings, interpolated between samples. Every control block
//! the band re-centres on what was measured. An FFT seeds the tracker and
//! checks it. Its lag is a cycle or two of the partial, where an estimate
//! from frames lags by the window.
//!
//! What makes it hold:
//!
//! - **Settle first.** It steers nothing until [`SETTLE`] crossings: its
//!   first crossings measure its own ring-in, which beats against the
//!   partial.
//! - **Accept only within half its band** of its centre, so it never hops
//!   to a neighbouring partial.
//! - **Presence.** When its level falls under half its slow average (a
//!   note ending) it isn't hearing its partial: it accepts nothing and
//!   steers nothing until the FFT says otherwise.
//!
//! The rest is the caller's, since it depends on what the partials are to
//! it: the band's width, the FFT's view of the tracker's own peak (its
//! [anchor](PartialTracker::anchor_to)), and when to
//! [snap](PartialTracker::snap) back to the FFT.

use crate::resonator::Resonator;

/// Periods measured between the crossings.
pub const SPAN: usize = 4;
/// Crossings before it steers anything.
pub const SETTLE: u32 = 6;

/// One partial's tracker. Nothing allocates.
#[derive(Clone, Copy, Debug)]
pub struct PartialTracker {
    rate: f64,
    filter: Resonator,
    /// The last `SPAN + 1` crossings, in samples, as a ring.
    cross: [f64; SPAN + 1],
    active: bool,
    hz: f64,
    prev: f64,
    anchor: f64,
    slow: f64,
    slow_coeff: f64,
    present: bool,
    count: u32,
}

impl PartialTracker {
    /// Stopped, at `sample_rate`.
    pub fn new(sample_rate: f64) -> Self {
        Self {
            rate: sample_rate,
            filter: Resonator::new(sample_rate, 1000.0),
            cross: [0.0; SPAN + 1],
            active: false,
            hz: 1000.0,
            prev: 0.0,
            anchor: 1000.0,
            slow: 0.0,
            // About 50 ms.
            slow_coeff: 1.0 - (-1.0 / (0.05 * sample_rate)).exp(),
            present: false,
            count: 0,
        }
    }

    /// Start on a partial at `hz`, with a band of damping `k`.
    pub fn start(&mut self, hz: f64, k: f64) {
        self.active = true;
        self.hz = hz;
        self.anchor = hz;
        self.slow = 0.0;
        self.present = false;
        self.filter.reset();
        self.filter.set_frequency(hz);
        self.filter.set_damping(k);
        self.prev = 0.0;
        self.count = 0;
    }

    /// Stop: it ticks no more until started again.
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Whether it is started.
    pub fn active(&self) -> bool {
        self.active
    }

    /// What it measures, Hz; the seed until it has measured anything.
    pub fn frequency(&self) -> f64 {
        self.hz
    }

    /// Whether it hears its partial.
    pub fn present(&self) -> bool {
        self.present
    }

    /// Started, through its ring-in and hearing its partial: fit to steer.
    pub fn settled(&self) -> bool {
        self.active && self.count >= SETTLE && self.present
    }

    /// The FFT's view of its own peak, Hz.
    pub fn anchor(&self) -> f64 {
        self.anchor
    }

    /// Set the FFT's view of its own peak, Hz.
    pub fn anchor_to(&mut self, hz: f64) {
        self.anchor = hz;
    }

    /// Set its band's damping.
    pub fn set_damping(&mut self, k: f64) {
        self.filter.set_damping(k);
    }

    /// Back to `hz`, from the FFT, to settle again.
    pub fn snap(&mut self, hz: f64) {
        self.hz = hz;
        self.filter.set_frequency(hz);
        self.count = 0;
    }

    /// One sample, `x`, at the caller's sample count `now`. On a control
    /// block (`retune`) the band re-centres on what it measures.
    #[inline]
    pub fn tick(&mut self, x: f64, now: u64, retune: bool) {
        if !self.active {
            return;
        }
        let bp = self.filter.next(x);
        let level = self.filter.level();
        self.slow += self.slow_coeff * (level - self.slow);
        self.present = level > 0.5 * self.slow;
        if self.prev < 0.0 && bp >= 0.0 && level > 1e-5 && self.present {
            let c = now as f64 - 1.0 + (-self.prev) / (bp - self.prev);
            let i = self.count as usize;
            self.count += 1;
            self.cross[i % (SPAN + 1)] = c;
            if i >= SPAN {
                let oldest = self.cross[(i - SPAN) % (SPAN + 1)];
                let measured = SPAN as f64 * self.rate / (c - oldest);
                if (measured - self.hz).abs() < 0.5 * self.filter.damping() * self.hz {
                    self.hz = measured;
                }
            }
        }
        self.prev = bp;
        if retune {
            self.filter.set_frequency(self.hz);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    const RATE: f64 = 48_000.0;

    /// Track a sine whose frequency is `f(t)`, seeded `seed_cents` off;
    /// returns the worst error in cents over `from..to` seconds.
    fn track(f: impl Fn(f64) -> f64, seed_cents: f64, from: f64, to: f64) -> f64 {
        let mut t = PartialTracker::new(RATE);
        t.start(f(0.0) * 2f64.powf(seed_cents / 1200.0), 0.1);
        let mut phase = 0.0;
        let mut worst: f64 = 0.0;
        for n in 0..(to * RATE) as u64 {
            let s = n as f64 / RATE;
            phase += TAU * f(s) / RATE;
            t.tick(0.5 * phase.sin(), n, n % 32 == 0);
            if s >= from {
                assert!(t.settled(), "unsettled at {s}");
                worst = worst.max((1200.0 * (t.frequency() / f(s)).log2()).abs());
            }
        }
        worst
    }

    #[test]
    fn it_settles_on_a_steady_partial_from_a_rough_seed() {
        // A low partial takes longer: the band re-centres once a block, and
        // its crossings come a cycle apart.
        for (hz, from) in [(82.0, 0.6), (440.0, 0.2), (2500.0, 0.2)] {
            let worst = track(|_| hz, 30.0, from, from + 0.3);
            assert!(worst < 0.5, "{hz}: {worst} cents");
        }
    }

    #[test]
    fn it_follows_a_glide() {
        // 300 to 450 Hz over half a second, after settling: 1400 cents a
        // second, a fast bend.
        let f = |t: f64| 300.0 * 1.5f64.powf(((t - 0.2) / 0.5).clamp(0.0, 1.0));
        let worst = track(f, 0.0, 0.2, 1.0);
        assert!(worst < 25.0, "{worst} cents");
        // And it lands on the note.
        let landed = track(f, 0.0, 0.9, 1.2);
        assert!(landed < 1.0, "{landed} cents");
    }

    #[test]
    fn it_follows_a_vibrato() {
        // 5.5 Hz, ±30 cents.
        let f = |t: f64| 440.0 * 2f64.powf(30.0 * (TAU * 5.5 * t).sin() / 1200.0);
        let worst = track(f, 0.0, 0.2, 1.5);
        assert!(worst < 15.0, "{worst} cents");
    }

    #[test]
    fn it_lets_go_when_its_partial_ends() {
        let mut t = PartialTracker::new(RATE);
        t.start(440.0, 0.1);
        let w = TAU * 440.0 / RATE;
        for n in 0..24_000u64 {
            t.tick(0.5 * (w * n as f64).sin(), n, n % 32 == 0);
        }
        assert!(t.settled());
        for n in 24_000..26_000u64 {
            t.tick(0.0, n, n % 32 == 0);
        }
        assert!(!t.present() && !t.settled());
    }
}
