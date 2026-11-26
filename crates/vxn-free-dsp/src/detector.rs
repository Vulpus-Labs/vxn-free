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
//! - Its centre comes from its bin's phase, as a phase vocoder's does: over
//!   a hop `H` a partial at `k + d` bins advances the bin's phase by
//!   `2π (k + d) H / N`, and the advance beyond `2π k H / N`, wrapped to
//!   ±π, gives `d`. With a hop of a quarter window that reads offsets up to
//!   ±2 bins. The advance is the angle of the bin times the conjugate of
//!   the last frame's, so it takes one `atan2` a peak, not one a bin: a
//!   third of the detector's cost at 2048. On the first frame after a
//!   restart, with no phase before it, and where the bin has risen more
//!   than 12 dB since the last frame (an onset, whose last phase was
//!   something else's), the centre comes from the magnitudes instead.
//! - From the magnitudes: with `a` the larger neighbour's magnitude over
//!   the peak's, a steady sine through a Hann window is centred
//!   `(2a − 1) / (a + 1)` bins towards that neighbour. This offset also
//!   gives the amplitude, `4 |X| / N` divided by Hann's response that far
//!   off a bin's centre, and the shape test below.
//! - It must have a lone sine's shape. Given the centre, Hann fixes the
//!   smaller neighbour's share of the peak's magnitude too; partials closer
//!   than the window resolves merge into one maximum that doesn't fit, and
//!   whose centre wanders as they beat. A peak whose smaller neighbour is
//!   off by up to [`SHAPE_FROM_DB`] keeps its prominence; beyond that it
//!   fades, to nothing at [`SHAPE_TO_DB`]. Measured at 2048: a lone sine is
//!   off by 0.00 dB, a 110 Hz saw's partials by at most 0.7 dB, and
//!   unresolved partials of a chord by 5 to 7 dB.
//!
//! Silence is safe: a peak needs a magnitude of its own, so smoothed power
//! that outlasts the signal finds nothing.

use std::f32::consts::{PI, TAU};

use crate::stft::Frame;

/// The share of each new frame's power mixed into the smoothed power.
pub const SMOOTHING: f32 = 0.8;
/// dB above the threshold over which a peak's prominence reaches 1.
pub const PROMINENCE_SPAN_DB: f32 = 12.0;
/// No peak is quieter than this amplitude: 60 dB under full scale.
pub const FLOOR: f32 = 1.0e-3;
/// dB a peak's smaller neighbour may be off a lone sine's before its
/// prominence fades.
pub const SHAPE_FROM_DB: f32 = 1.5;
/// dB off at which a peak counts for nothing.
pub const SHAPE_TO_DB: f32 = 3.0;
/// The least power, as a share of this frame's, that a peak's bin had in
/// the last frame for its phase advance to be read: −12 dB.
const PHASE_LEAST: f32 = 1.0 / 16.0;

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
    /// The last frame's bins, packed as the frame's are.
    last: Box<[f32]>,
    /// Where the last frame ended, if there was one since the restart.
    last_end: Option<u64>,
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
            last: vec![0.0; max_window].into_boxed_slice(),
            last_end: None,
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
        self.last.fill(0.0);
        self.last_end = None;
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

    /// The window it is set to.
    pub fn window(&self) -> usize {
        self.window
    }

    /// Each bin's smoothed power, from DC to just under Nyquist: a steady
    /// sine of amplitude `A` on a bin reads `(A · window / 4)²`.
    pub fn smoothed_power(&self) -> &[f32] {
        &self.pow[..self.window / 2]
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
        // The hop since the last frame, if the phases before it are this
        // stream's and the advance is unambiguous over a peak's ±2 bins.
        let hop = self
            .last_end
            .map(|e| frame.end.wrapping_sub(e) as usize)
            .filter(|&h| h > 0 && 4 * h <= self.window);
        self.last_end = Some(frame.end);
        let mean = self.mean_prev;
        self.mean_prev = sum / (half - 1) as f32;
        if mean.is_nan() || mean <= 0.0 {
            self.store(frame);
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
            let (near, far) = if mag[k + 1] > mag[k - 1] {
                (mag[k + 1], mag[k - 1])
            } else {
                (mag[k - 1], mag[k + 1])
            };
            let a = near / m;
            let off = (2.0 * a - 1.0) / (a + 1.0);
            let d = if mag[k + 1] > mag[k - 1] { off } else { -off };
            let shape = shape(off, far / m);
            if shape <= 0.0 {
                continue;
            }
            if self.count == self.found.len() {
                break;
            }
            let (re, im) = (frame.bins[2 * k], frame.bins[2 * k + 1]);
            let (lr, li) = (self.last[2 * k], self.last[2 * k + 1]);
            let centre = match hop {
                Some(h) if lr * lr + li * li > PHASE_LEAST * m * m => {
                    // The angle of this bin times the conjugate of the last
                    // frame's: the phase advance, one atan2.
                    let advance = (im * lr - re * li).atan2(re * lr + im * li);
                    let expected = TAU * k as f32 * h as f32 / self.window as f32;
                    k as f32 + wrap(advance - expected) * self.window as f32 / (TAU * h as f32)
                }
                _ => k as f32 + d,
            };
            self.found[self.count] = DetectedPeak {
                hz: centre * per_bin,
                amplitude: 4.0 * m / self.window as f32 / hann_response(d),
                prominence: ((db - self.threshold_db) / PROMINENCE_SPAN_DB).min(1.0) * shape,
            };
            self.count += 1;
        }
        self.store(frame);
    }

    fn store(&mut self, frame: &Frame<'_>) {
        let n = self.window;
        self.last[..n].copy_from_slice(&frame.bins[..n]);
    }
}

/// `x` wrapped to −π..π.
fn wrap(x: f32) -> f32 {
    x - TAU * ((x + PI) / TAU).floor()
}

/// A Hann window's magnitude response `d` bins off a sine (|d| < 2),
/// relative to its response on it: `sinc(d) / (1 − d²)`.
fn hann_response(d: f32) -> f32 {
    if d.abs() < 1.0e-6 {
        return 1.0;
    }
    if (d.abs() - 1.0).abs() < 1.0e-4 {
        return 0.5;
    }
    let x = std::f32::consts::PI * d;
    x.sin() / x / (1.0 - d * d)
}

/// How much a peak counts, 0 to 1, by how well it fits a lone sine: its
/// centre `off` bins (0 to 1/2) towards its larger neighbour, and `far` the
/// smaller neighbour's share of its magnitude.
fn shape(off: f32, far: f32) -> f32 {
    let off = off.clamp(0.0, 0.5);
    let want = hann_response(1.0 + off) / hann_response(off);
    let db = (20.0 * (far.max(1.0e-12) / want).log10()).abs();
    (1.0 - (db - SHAPE_FROM_DB) / (SHAPE_TO_DB - SHAPE_FROM_DB)).clamp(0.0, 1.0)
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
    fn unresolved_partials_make_no_peak() {
        // Two sines a bin and a half apart at 2048 merge into one maximum
        // whose shape is no lone sine's.
        let bin = f64::from(RATE) / 2048.0;
        let x = sines(&[(1000.0, 0.3), (1000.0 + 1.5 * bin, 0.3)], 8192);
        let found = detect(&x, 2048, 6.0);
        assert!(
            found
                .iter()
                .all(|p| (p.hz - 1017.0).abs() > 40.0 || p.prominence == 0.0),
            "{found:?}"
        );
        // A saw's resolved partials are all found, the strong ones at full
        // prominence.
        let saw: Vec<(f64, f32)> = (1..40)
            .map(|k| (110.0 * f64::from(k), 0.3 / k as f32))
            .collect();
        let found = detect(&sines(&saw, 8192), 2048, 6.0);
        assert!(found.len() >= 10, "{found:?}");
        for (k, p) in found.iter().enumerate().take(10) {
            assert!(
                (p.hz / (110.0 * (k + 1) as f32) - 1.0).abs() < 0.002,
                "{found:?}"
            );
            assert!(p.prominence > 0.0, "{found:?}");
        }
        assert!(found[0].prominence == 1.0 && found[1].prominence == 1.0);
    }

    /// Each frame's peaks for `x`; with `magnitude`, the phases are
    /// forgotten before every frame, so centres come from the magnitudes.
    fn every_frame(x: &[f32], magnitude: bool) -> Vec<Vec<DetectedPeak>> {
        let mut stft = Stft::new(2048, 4, 2048, 512);
        let mut d = PeakDetector::new(2048);
        d.set_threshold_db(6.0);
        let mut out = vec![];
        stft.push(x, |f| {
            if magnitude {
                d.last_end = None;
            }
            d.detect(&f, RATE);
            out.push(d.peaks().to_vec());
        });
        out
    }

    /// The mean move in cents of each peak to its nearest in the next
    /// frame, after the first eight.
    fn jitter(frames: &[Vec<DetectedPeak>]) -> f64 {
        let (mut sum, mut n) = (0.0, 0);
        for w in frames[8..].windows(2) {
            for p in &w[1] {
                let near = w[0]
                    .iter()
                    .map(|q| cents(p.hz, f64::from(q.hz)).abs())
                    .fold(f64::MAX, f64::min);
                if near < 50.0 {
                    sum += near;
                    n += 1;
                }
            }
        }
        sum / f64::from(n)
    }

    #[test]
    fn phase_centres_are_steadier_than_magnitude_centres() {
        // A saw's partials: measured 0.18 cents a frame from the phase,
        // 0.43 from the magnitudes.
        let saw: Vec<(f64, f32)> = (1..40)
            .map(|k| (110.0 * f64::from(k), 0.3 / k as f32))
            .collect();
        let x = sines(&saw, 48_000);
        let (phase, magnitude) = (
            jitter(&every_frame(&x, false)),
            jitter(&every_frame(&x, true)),
        );
        assert!(
            phase < 0.25 && phase < 0.6 * magnitude,
            "{phase} against {magnitude}"
        );
    }

    /// The packed bins of the last 2048-sample frame of `x`.
    fn bins_of(x: &[f32]) -> Vec<f32> {
        let mut stft = Stft::new(2048, 4, 2048, 512);
        let mut bins = vec![];
        stft.push(x, |f| bins = f.bins.to_vec());
        bins
    }

    #[test]
    fn an_onset_takes_its_centre_from_the_magnitudes() {
        // A weak tone, then a frame where the same bin is 40 dB louder with
        // an unrelated phase: the advance would be the two phases'
        // difference, not the partial's. The magnitudes give the centre.
        let bin = f64::from(RATE) / 2048.0;
        let hz = 100.3 * bin;
        let tone = |a: f32, phase: f64| -> Vec<f32> {
            (0..2048)
                .map(|n| a * (TAU * hz * f64::from(n) / f64::from(RATE) + phase).sin() as f32)
                .collect()
        };
        let (weak, loud) = (bins_of(&tone(0.005, 0.0)), bins_of(&tone(0.5, 2.0)));
        let mut d = PeakDetector::new(2048);
        d.set_threshold_db(6.0);
        for (i, bins) in [&weak, &weak, &loud].into_iter().enumerate() {
            let end = 2048 + 512 * i as u64;
            d.detect(
                &Frame {
                    index: i as u64,
                    end,
                    bins,
                },
                RATE,
            );
        }
        let p = d.peaks();
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(cents(p[0].hz, hz).abs() < 0.5, "{p:?} against {hz}");
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
