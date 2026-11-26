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
/// The share of each new second difference mixed into the roughness.
const ROUGH_SMOOTHING: f64 = 0.1;
/// No one reading adds more roughness than this, cents/s².
const ROUGH_CAP: f64 = 1.0e8;

/// One partial's tracker. Nothing allocates.
#[derive(Clone, Copy, Debug)]
pub struct PartialTracker {
    rate: f64,
    filter: Resonator,
    /// The last `SPAN + 1` crossings, in samples, as a ring.
    cross: [f64; SPAN + 1],
    active: bool,
    hz: f64,
    /// The band-pass's last three outputs, oldest first.
    prev: [f64; 3],
    anchor: f64,
    slow: f64,
    slow_coeff: f64,
    present: bool,
    count: u32,
    /// The last two readings, Hz, newest first; 0 before there are any.
    readings: [f64; 2],
    /// Smoothed second difference of the readings, cents.
    rough: f64,
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
            prev: [0.0; 3],
            anchor: 1000.0,
            slow: 0.0,
            // About 50 ms.
            slow_coeff: 1.0 - (-1.0 / (0.05 * sample_rate)).exp(),
            present: false,
            count: 0,
            readings: [0.0; 2],
            rough: 0.0,
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
        self.prev = [0.0; 3];
        self.count = 0;
        self.readings = [0.0; 2];
        self.rough = 0.0;
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

    /// How erratic its readings are, in cents a second squared: the
    /// smoothed second difference of successive readings, a period apart.
    /// A bend or a vibrato moves its readings smoothly: a ±30 cent vibrato
    /// at 5.5 Hz curves at about 36 000 cents/s², and reads 34 000 to
    /// 65 000 from 110 to 880 Hz. Above that the crossings' own error
    /// grows with pitch: it reads 128 000 at 1760 Hz and 1.3 million at
    /// 3520. A neighbouring partial in its band makes the readings jump
    /// about: 570 000 to 6.5 million at 440 to 660 Hz.
    pub fn roughness(&self) -> f64 {
        self.rough
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
        // A crossing between the middle two of the last four outputs,
        // placed by the cubic through all four: a straight line between two
        // samples misplaces it by a pattern that repeats, which high up
        // reads as a jittering frequency.
        let [y0, y1, y2] = self.prev;
        if y1 < 0.0 && y2 >= 0.0 && level > 1e-5 && self.present {
            let c = now as f64 - 2.0 + cubic_root(y0, y1, y2, bp);
            let i = self.count as usize;
            self.count += 1;
            self.cross[i % (SPAN + 1)] = c;
            if i >= SPAN {
                let oldest = self.cross[(i - SPAN) % (SPAN + 1)];
                let measured = SPAN as f64 * self.rate / (c - oldest);
                if self.readings[1] > 0.0 {
                    let second = measured - 2.0 * self.readings[0] + self.readings[1];
                    // In cents a second squared: the readings come a
                    // period apart.
                    let cents = (1200.0 / std::f64::consts::LN_2 * second / measured).abs();
                    let curvature = (cents * measured * measured).min(ROUGH_CAP);
                    self.rough += ROUGH_SMOOTHING * (curvature - self.rough);
                }
                self.readings = [measured, self.readings[0]];
                if (measured - self.hz).abs() < 0.5 * self.filter.damping() * self.hz {
                    self.hz = measured;
                }
            }
        }
        self.prev = [y1, y2, bp];
        if retune {
            self.filter.set_frequency(self.hz);
        }
    }
}

/// Where, from 0 to 1, the cubic through `(−1, before)`, `(0, below)`,
/// `(1, above)`, `(2, after)` crosses zero, given `below < 0 ≤ above`:
/// Newton's method from the straight line's estimate.
fn cubic_root(before: f64, below: f64, above: f64, after: f64) -> f64 {
    let c1 = -before / 3.0 - below / 2.0 + above - after / 6.0;
    let c2 = before / 2.0 - below + above / 2.0;
    let c3 = -before / 6.0 + below / 2.0 - above / 2.0 + after / 6.0;
    let mut at = -below / (above - below);
    for _ in 0..3 {
        let value = below + at * (c1 + at * (c2 + at * c3));
        let slope = c1 + at * (2.0 * c2 + at * 3.0 * c3);
        if slope.abs() < 1e-30 {
            break;
        }
        at = (at - value / slope).clamp(0.0, 1.0);
    }
    at
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

    /// The roughness of a tracker on `parts`, the first `vib` cents of
    /// 5.5 Hz vibrato, over the last 0.7 of a second.
    fn roughness(parts: &[(f64, f64)], k: f64, vib: f64) -> f64 {
        let mut t = PartialTracker::new(RATE);
        t.start(parts[0].0, k);
        let mut phase = vec![0.0; parts.len()];
        let mut most: f64 = 0.0;
        for n in 0..48_000u64 {
            let s = n as f64 / RATE;
            let mut x = 0.0;
            for (j, &(hz, a)) in parts.iter().enumerate() {
                let wobble = if j == 0 {
                    vib * (TAU * 5.5 * s).sin()
                } else {
                    0.0
                };
                phase[j] += TAU * hz * 2f64.powf(wobble / 1200.0) / RATE;
                x += a * phase[j].sin();
            }
            t.tick(x, n, n % 32 == 0);
            if s > 0.3 {
                most = most.max(t.roughness());
            }
        }
        most
    }

    #[test]
    fn roughness_tells_a_vibrato_from_a_neighbour() {
        for (hz, most) in [
            (110.0, 1e5),
            (440.0, 1e5),
            (880.0, 1e5),
            (1760.0, 2e5),
            (3520.0, 2e6),
        ] {
            let r = roughness(&[(hz, 0.5)], 0.1, 30.0);
            assert!(r < most, "vibrato at {hz}: {r}");
        }
        for (parts, k) in [
            (&[(440.0, 0.3), (466.2, 0.3)][..], 0.02),
            (&[(440.0, 0.2), (523.0, 0.5)][..], 0.057),
            (&[(659.3, 0.1), (654.1, 0.1), (784.0, 0.1)][..], 0.05),
        ] {
            let r = roughness(parts, k, 0.0);
            assert!(r > 500_000.0, "{parts:?}: {r}");
        }
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
