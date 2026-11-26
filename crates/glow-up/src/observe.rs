//! What the engine is doing, for the display (ticket 0009), as of the end
//! of a host block: the input's spectrum, the latest analysis frame's
//! peaks, and every filter's centre, level, damping and state, with what
//! the output makes of them (WIDTH's damping, MIX and DRY CUT). From these
//! [`Observation::response`] gives the output's gain over the input at any
//! frequency.
//!
//! The editor gets it as a telemetry frame of [`FRAME_WORDS`] floats
//! ([`Observation::to_frame`], [`Observation::from_frame`]), published
//! whole by silkscreen, so the display never draws half of one block and
//! half of the next. It is filled in place on the audio thread and never
//! allocates.

use crate::bank::{BOOST_MAX, Bank, POOL, State};
use vxn_free_dsp::PeakDetector;

/// The most peaks a frame carries: the strongest, by prominence.
pub const PEAKS: usize = 32;
/// Points in the input's spectrum, log-spaced from [`LOWEST`] to
/// [`HIGHEST`]: a point every 65 cents.
pub const SPECTRUM: usize = 160;
/// The spectrum's span, Hz.
pub const LOWEST: f32 = 40.0;
pub const HIGHEST: f32 = 16_000.0;
/// The spectrum's floor, dB under full scale.
pub const FLOOR_DB: f32 = -120.0;

/// Exact for whole numbers below this, so a frame's time is exact.
const TIME_WRAP: u64 = 1 << 24;

/// What a filter is doing, for the display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shown {
    #[default]
    Free,
    /// Boosting its target.
    Boosting,
    /// Self-oscillating.
    Singing,
    /// Ringing down, or fading out before it moves.
    Fading,
}

/// One filter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FilterObs {
    /// Hz.
    pub hz: f32,
    /// Its gain as a share of the most it can have, 0 to 1.
    pub level: f32,
    /// Its band-pass's damping now: WIDTH's, lowered by RESONANCE, at or
    /// below zero while it sings.
    pub damping: f32,
    pub shown: Shown,
}

/// One peak.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PeakObs {
    /// Hz.
    pub hz: f32,
    /// 0 to 1.
    pub prominence: f32,
}

/// Everything the display draws, as of the end of one host block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    /// Samples since the engine was reset, modulo 2^24 in a frame.
    pub time: u64,
    pub sample_rate: f32,
    /// WIDTH's damping, `k0`: each filter's output is `gain · k0 · bp`.
    pub k0: f32,
    /// The output is `direct · x + wet · Σ gain · k0 · bp`: MIX's dry
    /// gain, plus its wet gain unless DRY CUT; and its wet gain.
    pub direct: f32,
    pub wet: f32,
    /// Peaks in use; the rest of `peaks` are empty.
    pub peak_count: u8,
    pub peaks: [PeakObs; PEAKS],
    pub filters: [FilterObs; POOL],
    /// The input's level at each of [`spectrum_hz`]'s points, dB under
    /// full scale for a sine: the analysis's smoothed power.
    pub spectrum: [f32; SPECTRUM],
}

/// Where spectrum point `i` lies, Hz.
pub fn spectrum_hz(i: usize) -> f32 {
    LOWEST * (HIGHEST / LOWEST).powf(i as f32 / (SPECTRUM - 1) as f32)
}

impl Default for Observation {
    fn default() -> Self {
        Self {
            time: 0,
            sample_rate: 48_000.0,
            k0: 0.0289,
            direct: 1.0,
            wet: 1.0,
            peak_count: 0,
            peaks: [PeakObs::default(); PEAKS],
            filters: [FilterObs::default(); POOL],
            spectrum: [FLOOR_DB; SPECTRUM],
        }
    }
}

const HEAD_WORDS: usize = 6;
const PEAK_WORDS: usize = 2;
const FILTER_WORDS: usize = 4;

/// Words in Glow-Up's telemetry frame: the time (modulo 2^24), the sample
/// rate, the peak count, `k0`, `direct` and `wet`; then each peak (Hz,
/// prominence); each filter (Hz, level, damping, state); and the
/// spectrum.
pub const FRAME_WORDS: usize = HEAD_WORDS + PEAKS * PEAK_WORDS + POOL * FILTER_WORDS + SPECTRUM;

impl Observation {
    /// Fill in from `bank`'s filters and `detector`: its latest frame's
    /// peaks (the most prominent [`PEAKS`], in order) and its smoothed
    /// power; with MIX's `dry` and `wet` gains and whether DRY CUT is on.
    pub fn observe(
        &mut self,
        time: u64,
        sample_rate: f32,
        bank: &Bank,
        detector: &PeakDetector,
        (dry, wet, dry_cut): (f64, f64, bool),
    ) {
        self.time = time;
        self.sample_rate = sample_rate;
        self.k0 = bank.nominal_damping() as f32;
        self.direct = (dry + if dry_cut { 0.0 } else { wet }) as f32;
        self.wet = wet as f32;
        self.spectrum(detector);
        let peaks = detector.peaks();
        // The cut-off prominence that keeps at most PEAKS: the PEAKS-th
        // largest, found without sorting (or allocating).
        let mut floor = -1.0f32;
        if peaks.len() > PEAKS {
            let above = |f: f32| peaks.iter().filter(|p| p.prominence > f).count();
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..20 {
                let mid = 0.5 * (lo + hi);
                if above(mid) >= PEAKS {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            floor = lo;
        }
        let mut n = 0;
        for p in peaks.iter().filter(|p| p.prominence > floor).take(PEAKS) {
            self.peaks[n] = PeakObs {
                hz: p.hz,
                prominence: p.prominence,
            };
            n += 1;
        }
        self.peaks[n..].fill(PeakObs::default());
        self.peak_count = n as u8;
        for (obs, f) in self.filters.iter_mut().zip(bank.filters()) {
            *obs = FilterObs {
                hz: f.hz as f32,
                level: (f.gain / BOOST_MAX) as f32,
                damping: f.damping() as f32,
                shown: match f.state {
                    State::Free => Shown::Free,
                    State::Active if f.oscillating => Shown::Singing,
                    State::Active => Shown::Boosting,
                    State::Releasing | State::Choking => Shown::Fading,
                },
            };
        }
    }

    /// The input's level at each spectrum point from the detector's
    /// smoothed power: the loudest bin in the point's share of the axis,
    /// or, where bins are sparser than points (low down), interpolated
    /// between the two either side.
    fn spectrum(&mut self, detector: &PeakDetector) {
        let power = detector.smoothed_power();
        let window = detector.window() as f32;
        let per_bin = self.sample_rate / window;
        // A sine of amplitude A on a bin reads (A · N / 4)².
        let scale = 4.0 / window;
        let half_step = (HIGHEST / LOWEST).powf(0.5 / (SPECTRUM - 1) as f32);
        let last = power.len() - 1;
        for (i, out) in self.spectrum.iter_mut().enumerate() {
            let hz = spectrum_hz(i);
            let (lo, hi) = (hz / half_step / per_bin, hz * half_step / per_bin);
            let (first, end) = (lo.ceil() as usize, (hi.floor() as usize).min(last));
            let p = if first <= end {
                power[first..=end].iter().fold(0.0f32, |a, &b| a.max(b))
            } else {
                let b = hz / per_bin;
                let k = (b.floor() as usize).min(last - 1);
                let t = b - k as f32;
                power[k] + t * (power[k + 1] - power[k])
            };
            let amplitude = p.max(0.0).sqrt() * scale;
            *out = (20.0 * amplitude.max(1e-9).log10()).max(FLOOR_DB);
        }
    }

    /// The output's gain over the input at `hz`, as a magnitude:
    /// `|direct + wet · Σ gain · k0 · bp(hz)|`, each band-pass the TPT
    /// form's own response (prewarped, so exact at every frequency). A
    /// singing filter's damping is taken at RESONANCE's floor, `k0 / 4`,
    /// so its peak is finite.
    pub fn response(&self, hz: f32) -> f32 {
        let fs = f64::from(self.sample_rate.max(1.0));
        let k0 = f64::from(self.k0);
        let warp = |f: f64| (std::f64::consts::PI * (f / fs).min(0.499)).tan();
        let w = warp(f64::from(hz));
        let (mut re, mut im) = (f64::from(self.direct), 0.0f64);
        for f in &self.filters {
            if f.shown == Shown::Free || f.level <= 0.0 || f.hz <= 0.0 {
                continue;
            }
            let gain = f64::from(f.level) * BOOST_MAX * k0 * f64::from(self.wet);
            let damping = f64::from(f.damping).max(0.25 * k0);
            // bp(jx) = jx / (1 − x² + j k x), x the prewarped ratio.
            let ratio = w / warp(f64::from(f.hz));
            let (dr, di) = (1.0 - ratio * ratio, damping * ratio);
            let norm = dr * dr + di * di;
            re += gain * (ratio * di) / norm;
            im += gain * (ratio * dr) / norm;
        }
        (re * re + im * im).sqrt() as f32
    }

    /// This observation as a telemetry frame.
    pub fn to_frame(&self, out: &mut [f32; FRAME_WORDS]) {
        out[0] = (self.time % TIME_WRAP) as f32;
        out[1] = self.sample_rate;
        out[2] = f32::from(self.peak_count);
        out[3] = self.k0;
        out[4] = self.direct;
        out[5] = self.wet;
        let (peaks, rest) = out[HEAD_WORDS..].split_at_mut(PEAKS * PEAK_WORDS);
        let (filters, spectrum) = rest.split_at_mut(POOL * FILTER_WORDS);
        spectrum.copy_from_slice(&self.spectrum);
        for (p, w) in self.peaks.iter().zip(peaks.chunks_exact_mut(PEAK_WORDS)) {
            w.copy_from_slice(&[p.hz, p.prominence]);
        }
        for (f, w) in self
            .filters
            .iter()
            .zip(filters.chunks_exact_mut(FILTER_WORDS))
        {
            let state = match f.shown {
                Shown::Free => 0.0,
                Shown::Boosting => 1.0,
                Shown::Singing => 2.0,
                Shown::Fading => 3.0,
            };
            w.copy_from_slice(&[f.hz, f.level, f.damping, state]);
        }
    }

    /// An observation back from a frame, its time modulo 2^24; `None` if
    /// `frame` isn't [`FRAME_WORDS`] long.
    pub fn from_frame(frame: &[f32]) -> Option<Self> {
        if frame.len() != FRAME_WORDS {
            return None;
        }
        let mut obs = Self {
            time: frame[0] as u64,
            sample_rate: frame[1],
            peak_count: (frame[2] as usize).min(PEAKS) as u8,
            k0: frame[3],
            direct: frame[4],
            wet: frame[5],
            ..Self::default()
        };
        let (peaks, rest) = frame[HEAD_WORDS..].split_at(PEAKS * PEAK_WORDS);
        let (filters, spectrum) = rest.split_at(POOL * FILTER_WORDS);
        obs.spectrum.copy_from_slice(spectrum);
        for (p, w) in obs.peaks.iter_mut().zip(peaks.chunks_exact(PEAK_WORDS)) {
            *p = PeakObs {
                hz: w[0],
                prominence: w[1],
            };
        }
        for (f, w) in obs
            .filters
            .iter_mut()
            .zip(filters.chunks_exact(FILTER_WORDS))
        {
            *f = FilterObs {
                hz: w[0],
                level: w[1],
                damping: w[2],
                shown: match w[3] as u8 {
                    1 => Shown::Boosting,
                    2 => Shown::Singing,
                    3 => Shown::Fading,
                    _ => Shown::Free,
                },
            };
        }
        Some(obs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips() {
        let mut obs = Observation {
            time: TIME_WRAP + 1234,
            sample_rate: 44_100.0,
            k0: 0.05,
            direct: 0.7,
            wet: 0.7,
            peak_count: 3,
            ..Observation::default()
        };
        for (i, v) in obs.spectrum.iter_mut().enumerate() {
            *v = -(i as f32) * 0.5;
        }
        for (i, p) in obs.peaks.iter_mut().take(3).enumerate() {
            *p = PeakObs {
                hz: 100.0 * (i + 1) as f32,
                prominence: 0.25 * i as f32,
            };
        }
        for (i, f) in obs.filters.iter_mut().enumerate() {
            *f = FilterObs {
                hz: 50.0 * i as f32,
                level: i as f32 / POOL as f32,
                damping: 0.01 * i as f32,
                shown: [Shown::Free, Shown::Boosting, Shown::Singing, Shown::Fading][i % 4],
            };
        }
        let mut frame = [0.0; FRAME_WORDS];
        obs.to_frame(&mut frame);
        assert_eq!(
            Observation::from_frame(&frame),
            Some(Observation { time: 1234, ..obs })
        );
        assert_eq!(Observation::from_frame(&frame[1..]), None);
    }

    #[test]
    fn the_response_is_flat_without_filters_and_lifted_at_a_centre() {
        let mut obs = Observation::default();
        assert!((obs.response(1000.0) - 1.0).abs() < 1e-6);
        obs.filters[0] = FilterObs {
            hz: 880.0,
            level: 1.0,
            damping: obs.k0,
            shown: Shown::Boosting,
        };
        // 1 + 4 at the centre: +14 dB.
        assert!(
            (obs.response(880.0) - 5.0).abs() < 1e-3,
            "{}",
            obs.response(880.0)
        );
        // Half a bandwidth off, the band-pass is 3 dB down and 45° round.
        let edge = 880.0 * (1.0 + 0.5 * obs.k0);
        let r = obs.response(edge);
        assert!(r > 3.5 && r < 4.0, "{r}");
        // Far off, nothing.
        assert!((obs.response(100.0) - 1.0).abs() < 0.01);
        // DRY CUT at full MIX: only the filter.
        obs.direct = 0.0;
        assert!((obs.response(880.0) - 4.0).abs() < 1e-3);
    }

    #[test]
    fn the_spectrum_points_span_the_axis() {
        assert!((spectrum_hz(0) - LOWEST).abs() < 1e-3);
        assert!((spectrum_hz(SPECTRUM - 1) - HIGHEST).abs() < 0.1);
    }
}
