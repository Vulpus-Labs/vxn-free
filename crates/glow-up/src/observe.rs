//! What the engine is doing, for the tracking display (ticket 0009): the
//! latest analysis frame's peaks and every filter's centre, level and
//! state, as of the end of a host block.
//!
//! The editor gets it as a telemetry frame of [`FRAME_WORDS`] floats
//! ([`Observation::to_frame`], [`Observation::from_frame`]), published
//! whole by silkscreen, so the display never draws half of one block and
//! half of the next. It is filled in place on the audio thread and never
//! allocates.

use crate::bank::{BOOST_MAX, Bank, POOL, State};
use vxn_free_dsp::DetectedPeak;

/// The most peaks a frame carries: the strongest, by prominence.
pub const PEAKS: usize = 32;

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
    /// Peaks in use; the rest of `peaks` are empty.
    pub peak_count: u8,
    pub peaks: [PeakObs; PEAKS],
    pub filters: [FilterObs; POOL],
}

impl Default for Observation {
    fn default() -> Self {
        Self {
            time: 0,
            sample_rate: 48_000.0,
            peak_count: 0,
            peaks: [PeakObs::default(); PEAKS],
            filters: [FilterObs::default(); POOL],
        }
    }
}

const HEAD_WORDS: usize = 3;
const PEAK_WORDS: usize = 2;
const FILTER_WORDS: usize = 3;

/// Words in Glow-Up's telemetry frame: the time (modulo 2^24), the sample
/// rate and the peak count, then each peak (Hz, prominence), then each
/// filter (Hz, level, state).
pub const FRAME_WORDS: usize = HEAD_WORDS + PEAKS * PEAK_WORDS + POOL * FILTER_WORDS;

impl Observation {
    /// Fill in from `bank`'s filters and the latest frame's `peaks`
    /// (ascending): the most prominent [`PEAKS`] of them, in order.
    pub fn observe(&mut self, time: u64, sample_rate: f32, bank: &Bank, peaks: &[DetectedPeak]) {
        self.time = time;
        self.sample_rate = sample_rate;
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
                shown: match f.state {
                    State::Free => Shown::Free,
                    State::Active if f.oscillating => Shown::Singing,
                    State::Active => Shown::Boosting,
                    State::Releasing | State::Choking => Shown::Fading,
                },
            };
        }
    }

    /// This observation as a telemetry frame.
    pub fn to_frame(&self, out: &mut [f32; FRAME_WORDS]) {
        out[0] = (self.time % TIME_WRAP) as f32;
        out[1] = self.sample_rate;
        out[2] = f32::from(self.peak_count);
        let (peaks, filters) = out[HEAD_WORDS..].split_at_mut(PEAKS * PEAK_WORDS);
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
            w.copy_from_slice(&[f.hz, f.level, state]);
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
            ..Self::default()
        };
        let (peaks, filters) = frame[HEAD_WORDS..].split_at(PEAKS * PEAK_WORDS);
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
                shown: match w[2] as u8 {
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
            peak_count: 3,
            ..Observation::default()
        };
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
}
