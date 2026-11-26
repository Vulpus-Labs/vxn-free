//! Glow-Up's peak detector (ADR 0005, point 4): a frame's peaks, ascending
//! in frequency, each with a centre, an amplitude and a prominence.
//!
//! - A peak is a maximum of this frame's magnitudes over two bins either
//!   side, found in the frame alone: a smoothed maximum lags a moving peak,
//!   and while a glide crosses between bins the two would disagree.
//! - Its smoothed power ([`SMOOTHING`] of each new frame mixed in, the
//!   largest of it and its neighbours') must stand the threshold above the
//!   previous frame's mean power, and above a floor [`FLOOR`] of full
//!   scale.
//! - Its prominence rises from 0 to 1 over the [`PROMINENCE_SPAN_DB`]
//!   above the threshold.
//! - Its centre comes from the larger neighbour's share of its magnitude,
//!   by the closed form that is exact for a steady sine through a Hann
//!   window: with `a` the neighbour's magnitude over the peak's, the true
//!   centre is `(2a − 1) / (a + 1)` bins towards that neighbour. Its
//!   amplitude is `4 |X| / N`, divided by Hann's response that far off a
//!   bin's centre.
//!
//! Silence is safe: a peak needs a magnitude of its own, so smoothed power
//! that outlasts the signal finds nothing.

use crate::stft::Frame;

/// The share of each new frame's power mixed into the smoothed power.
pub const SMOOTHING: f32 = 0.8;
/// dB above the threshold over which a peak's prominence reaches 1.
pub const PROMINENCE_SPAN_DB: f32 = 12.0;
/// No peak is quieter than this amplitude: 60 dB under full scale.
pub const FLOOR: f32 = 1.0e-3;

/// One peak.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DetectedPeak {
    /// Hz.
    pub hz: f32,
    /// A steady sine's amplitude at that centre.
    pub amplitude: f32,
    /// How far it stands above the threshold, 0 to 1.
    pub prominence: f32,
}

/// The detector. Build it with [`PeakDetector::new`] off the audio thread;
/// nothing allocates after.
#[derive(Debug)]
pub struct PeakDetector {
    mag: Box<[f32]>,
    pow: Box<[f32]>,
    found: Box<[DetectedPeak]>,
    count: usize,
    window: usize,
    mean_prev: f32,
    threshold_db: f32,
}

impl PeakDetector {
    /// For windows up to `max_window`, set to `max_window`.
    pub fn new(max_window: usize) -> Self {
        let mut d = Self {
            mag: vec![0.0; max_window / 2].into_boxed_slice(),
            pow: vec![0.0; max_window / 2].into_boxed_slice(),
            found: vec![DetectedPeak::default(); max_window / 4].into_boxed_slice(),
            count: 0,
            window: max_window,
            mean_prev: 0.0,
            threshold_db: 0.0,
        };
        d.set_window(max_window);
        d
    }

    /// A new window size: the smoothing starts again.
    pub fn set_window(&mut self, window: usize) {
        assert!(
            window / 2 <= self.mag.len(),
            "window {window} is over the detector's"
        );
        self.window = window;
        self.restart();
    }

    /// Forget the past.
    pub fn restart(&mut self) {
        self.mag.fill(0.0);
        self.pow.fill(0.0);
        self.mean_prev = 0.0;
        self.count = 0;
    }

    /// dB a peak must stand above the mean power.
    pub fn set_threshold_db(&mut self, db: f32) {
        self.threshold_db = db;
    }

    /// The last frame's peaks, ascending in frequency.
    pub fn peaks(&self) -> &[DetectedPeak] {
        &self.found[..self.count]
    }

    /// Find `frame`'s peaks, at `sample_rate`. The frame's window must be
    /// the detector's.
    pub fn detect(&mut self, frame: &Frame<'_>, sample_rate: f32) {
        let half = self.window / 2;
        debug_assert_eq!(
            frame.len(),
            half + 1,
            "the frame's window isn't the detector's"
        );
        let mut sum = 0.0f32;
        for k in 0..half {
            let p = frame.power(k);
            self.mag[k] = p.sqrt();
            self.pow[k] += SMOOTHING * (p - self.pow[k]);
            if k > 0 {
                sum += self.pow[k];
            }
        }
        self.count = 0;
        let mean = self.mean_prev;
        self.mean_prev = sum / (half - 1) as f32;
        if mean.is_nan() || mean <= 0.0 {
            return;
        }
        let floor = FLOOR * self.window as f32 / 4.0;
        let floor_power = floor * floor;
        let per_bin = sample_rate / self.window as f32;
        let (mag, pow) = (&self.mag, &self.pow);
        for k in 2..half - 2 {
            let m = mag[k];
            if m.is_nan()
                || m <= 0.0
                || m <= mag[k - 1]
                || m < mag[k + 1]
                || m <= mag[k - 2]
                || m < mag[k + 2]
            {
                continue;
            }
            let power = pow[k].max(pow[k - 1]).max(pow[k + 1]);
            let db = 10.0 * (power / mean).log10();
            if db < self.threshold_db || power < floor_power {
                continue;
            }
            let d = if mag[k + 1] > mag[k - 1] {
                let a = mag[k + 1] / m;
                (2.0 * a - 1.0) / (a + 1.0)
            } else {
                let a = mag[k - 1] / m;
                -(2.0 * a - 1.0) / (a + 1.0)
            };
            if self.count == self.found.len() {
                break;
            }
            self.found[self.count] = DetectedPeak {
                hz: (k as f32 + d) * per_bin,
                amplitude: 4.0 * m / self.window as f32 / hann_response(d),
                prominence: ((db - self.threshold_db) / PROMINENCE_SPAN_DB).min(1.0),
            };
            self.count += 1;
        }
    }
}

/// A Hann window's magnitude response `d` bins off a sine (|d| ≤ 1/2),
/// relative to its response on it: `sinc(d) / (1 − d²)`.
fn hann_response(d: f32) -> f32 {
    if d.abs() < 1.0e-6 {
        return 1.0;
    }
    let x = std::f32::consts::PI * d;
    x.sin() / x / (1.0 - d * d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stft::Stft;
    use std::f64::consts::TAU;

    const RATE: f32 = 48_000.0;

    fn sines(parts: &[(f64, f32)], len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| {
                parts
                    .iter()
                    .map(|&(hz, a)| a * (TAU * hz * n as f64 / f64::from(RATE)).sin() as f32)
                    .sum()
            })
            .collect()
    }

    /// The peaks after feeding `x` at `window`, threshold `db`.
    fn detect(x: &[f32], window: usize, db: f32) -> Vec<DetectedPeak> {
        let mut stft = Stft::new(2048, 4, window, window / 4);
        let mut d = PeakDetector::new(2048);
        d.set_window(window);
        d.set_threshold_db(db);
        stft.push(x, |f| d.detect(&f, RATE));
        d.peaks().to_vec()
    }

    fn cents(a: f32, b: f64) -> f64 {
        1200.0 * (f64::from(a) / b).log2()
    }

    #[test]
    fn sines_are_found_with_their_amplitude_in_order() {
        for window in [512usize, 1024, 2048] {
            let bin = f64::from(RATE) / window as f64;
            let parts = [(10.3 * bin, 0.3f32), (23.7 * bin, 0.2), (41.5 * bin, 0.1)];
            let found = detect(&sines(&parts, 8 * window), window, 6.0);
            assert_eq!(found.len(), 3, "{window}: {found:?}");
            for (p, &(hz, a)) in found.iter().zip(&parts) {
                assert!(cents(p.hz, hz).abs() < 2.0, "{window}: {p:?} for {hz}");
                assert!(
                    (p.amplitude / a - 1.0).abs() < 0.03,
                    "{window}: {p:?} for {a}"
                );
                assert!(p.prominence > 0.0, "{p:?}");
            }
        }
    }

    #[test]
    fn silence_and_sines_under_the_floor_make_no_peaks() {
        assert!(detect(&vec![0.0; 8192], 1024, 6.0).is_empty());
        assert!(detect(&sines(&[(1000.0, 5e-4)], 8192), 1024, 0.0).is_empty());
        assert!(!detect(&sines(&[(1000.0, 5e-3)], 8192), 1024, 0.0).is_empty());
    }

    #[test]
    fn prominence_rises_with_the_peak_above_the_threshold() {
        // A quieter sine beside a loud one: its standing over the mean
        // grows with its level.
        let mut last = -1.0;
        for a in [0.04f32, 0.08, 0.16, 0.32] {
            let x = sines(&[(500.0, 0.5), (3000.0, a)], 8192);
            let found = detect(&x, 1024, 6.0);
            let p = found
                .iter()
                .find(|p| cents(p.hz, 3000.0).abs() < 20.0)
                .map_or(0.0, |p| p.prominence);
            assert!(p >= last, "{a}: {p} after {last}");
            last = p;
        }
        assert!(last > 0.5, "{last}");
    }

    #[test]
    fn a_window_change_restarts() {
        let mut d = PeakDetector::new(2048);
        let mut stft = Stft::new(2048, 4, 2048, 512);
        stft.push(&sines(&[(1000.0, 0.5)], 8192), |f| d.detect(&f, RATE));
        assert!(!d.peaks().is_empty());
        d.set_window(512);
        assert!(d.peaks().is_empty());
    }
}
