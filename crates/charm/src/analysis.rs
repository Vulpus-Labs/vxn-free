//! The sidechain that listens to the dry input (ADR 0002, points 6 and 7).
//!
//! It runs beside the audio and never in it: the audio path doesn't wait
//! for it, and it lags the input by at most one window and one hop. Every
//! voice reads at least [`FLOOR`](crate::flock::FLOOR) back, which is
//! longer than that lag, so whatever the analysis says about a sample
//! reaches the voices before they play it.
//!
//! - One streaming STFT on the mid signal, windows of the next power of two
//!   at or above 5 ms, hop a quarter-window. MID's features come from the
//!   bins between LOW XOVER and 2.5 kHz, HIGH's from those above.
//! - Onsets per band by spectral flux: each bin's rise in magnitude over a
//!   reference, summed, over the reference's sum. The reference for a bin
//!   is the largest magnitude it or either neighbour had over the last
//!   [`REFERENCE_SECONDS`] (as in the `SuperFlux` method). Comparing with only the
//!   last frame fired constantly on sustained tones: neighbouring partials
//!   leak into shared bins and beat as frames slide, and that wobble read
//!   as rises. A reference only a window long still fired 54 times a
//!   second on a distorted power chord, whose 41 Hz common period makes a
//!   burst of top end every 24 ms: a buzz, not onsets. A frame whose rise passes [`ONSET_RISE`] (and isn't near
//!   silence) is an onset; its window's start is recorded so a later
//!   ticket can duck from there.
//!   A band fires once per onset: it re-arms when its rise falls below
//!   half the threshold, or it goes quiet. (A click enters one frame at
//!   the window's tapered edge and the next frame still sees it rise.)
//! - The strongest [`PEAKS`] peaks per band.
//! - Coherence per band, from the band's mid and side signals smoothed
//!   over about 300 ms: (M² − S²)/(M² + S²). That is 1 when the channels
//!   are equal, 0 when they are uncorrelated at equal level and −1 when
//!   one is the other inverted.

use vxn_free_dsp::{Crossover, Frame, Peak, Stft, peaks_between};

use crate::engine::MID_HIGH_XOVER;
use crate::observe::{BANDS, BandObs, PEAKS, PeakObs};

/// The shortest analysis window, seconds.
pub const MIN_WINDOW_SECONDS: f32 = 0.005;
/// Frames per window.
pub const OVERLAP: usize = 4;
/// How far back a bin's flux reference reaches, seconds: longer than the
/// period of any buzz above about 33 Hz, shorter than sixteenth notes.
pub const REFERENCE_SECONDS: f32 = 0.03;
/// A frame's relative rise in magnitude that counts as an onset.
pub const ONSET_RISE: f32 = 0.6;
/// Below this mean magnitude a band is silent, and nothing in it is an
/// onset (a quiet hiss rising from nothing would otherwise be one).
const SILENCE: f32 = 1.0e-4;
/// A peak stands this far above its band's mean power. MID spans only a
/// dozen bins at a 5 ms window, so one sine's main lobe lifts the band's
/// mean: a lone sine stands about 8 dB above it.
const PEAK_THRESHOLD_DB: f32 = 6.0;
/// Coherence's smoothing time constant, seconds.
const COHERENCE_SECONDS: f32 = 0.3;

/// The window for `sample_rate`: the next power of two at or above 5 ms.
pub fn window_for(sample_rate: f32) -> usize {
    ((MIN_WINDOW_SECONDS * sample_rate).ceil() as usize).next_power_of_two()
}

/// The latest onset in one band.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Onset {
    /// The relative rise that made it one.
    pub rise: f32,
    /// The stream position its frame's window starts at.
    pub frame_start: u64,
    /// The stream position its frame completed at.
    pub frame_end: u64,
}

/// One band's running analysis.
#[derive(Clone, Copy, Debug, Default)]
struct BandState {
    peaks: [Peak; PEAKS],
    found: usize,
    /// The strongest onset since the last [`Analysis::take`], if any.
    onset: Option<Onset>,
    /// Whether a rise can fire an onset (it can't again until the last
    /// one has passed).
    armed: bool,
    /// Smoothed M² and S².
    mid_power: f32,
    side_power: f32,
}

/// Charm's analysis sidechain. Build it with [`Analysis::new`] at the
/// host's rate.
#[derive(Debug)]
pub struct Analysis {
    sample_rate: f32,
    stft: Stft,
    flux: Flux,
    bands: [BandState; BANDS],
    /// LOW XOVER, Hz: MID starts here.
    low_xover: f32,
    /// Band-splitters for the side signal (the mid signal's are the
    /// engine's).
    side_low: Crossover,
    side_split: Crossover,
    smooth: f32,
}

impl Analysis {
    /// Allocates; call it off the audio thread.
    pub fn new(sample_rate: f32, low_xover: f32) -> Self {
        let window = window_for(sample_rate);
        Self {
            sample_rate,
            stft: Stft::new(window, OVERLAP, window, window / OVERLAP),
            flux: Flux::new(
                window / 2 + 1,
                (REFERENCE_SECONDS * sample_rate / (window / OVERLAP) as f32).ceil() as usize,
            ),
            bands: [BandState {
                armed: true,
                ..BandState::default()
            }; BANDS],
            low_xover,
            side_low: Crossover::new(low_xover, sample_rate),
            side_split: Crossover::new(MID_HIGH_XOVER, sample_rate),
            smooth: 1.0 - (-1.0 / (COHERENCE_SECONDS * sample_rate)).exp(),
        }
    }

    /// Samples the analysis can lag the input by: a window and a hop.
    pub fn lag(&self) -> usize {
        self.stft.window() + self.stft.hop()
    }

    /// The window, samples.
    pub fn window(&self) -> usize {
        self.stft.window()
    }

    /// Samples since the last restart: the stream position of the next
    /// sample.
    pub fn position(&self) -> u64 {
        self.stft.position()
    }

    /// Move MID's lower edge.
    pub fn set_low_xover(&mut self, hz: f32) {
        self.low_xover = hz;
        self.side_low.set_frequency(hz);
    }

    /// Back to silence.
    pub fn restart(&mut self) {
        self.stft.restart();
        self.flux.clear();
        self.bands = [BandState {
            armed: true,
            ..BandState::default()
        }; BANDS];
        self.side_low.reset();
        self.side_split.reset();
    }

    /// Feed one stretch of input: `mid` is (L + R)/2, and `bands` the MID
    /// and HIGH parts of it, sample for sample; `side` is (L − R)/2.
    pub fn push(&mut self, mid: &[f32], bands: &[[f32; BANDS]], side: &[f32]) {
        for (m, &s) in bands.iter().zip(side) {
            let (_, rest) = self.side_low.split(s);
            let (side_mid, side_high) = self.side_split.split(rest);
            for (b, sb) in [side_mid, side_high].into_iter().enumerate() {
                let band = &mut self.bands[b];
                band.mid_power += self.smooth * (m[b] * m[b] - band.mid_power);
                band.side_power += self.smooth * (sb * sb - band.side_power);
            }
        }
        let (rate, low_xover) = (self.sample_rate, self.low_xover);
        let flux = &mut self.flux;
        let states = &mut self.bands;
        self.stft.push(mid, |frame| {
            frame_done(&frame, rate, low_xover, flux, states);
        });
    }

    /// The analysis for the latest tick into `obs`, clearing the onsets
    /// seen since the last call. Returns each band's onset, if any.
    pub fn take(&mut self, obs: &mut [BandObs; BANDS]) -> [Option<Onset>; BANDS] {
        let mut onsets = [None; BANDS];
        for ((state, o), onset) in self.bands.iter_mut().zip(obs).zip(&mut onsets) {
            o.peaks = [PeakObs::default(); PEAKS];
            for (dst, src) in o.peaks.iter_mut().zip(&state.peaks[..state.found]) {
                *dst = PeakObs {
                    hz: src.hz,
                    strength: src.amplitude,
                };
            }
            let total = state.mid_power + state.side_power;
            o.coherence = if total > 1.0e-12 {
                (state.mid_power - state.side_power) / total
            } else {
                1.0
            };
            *onset = state.onset.take();
            o.onset = onset.map_or(0.0, |x| x.rise);
            o.onset_start = onset.map_or(0, |x| x.frame_start);
        }
        onsets
    }
}

/// The last [`REFERENCE_SECONDS`] of frames' magnitudes, for flux.
#[derive(Debug)]
struct Flux {
    /// `frames` frames of `bins` magnitudes.
    history: Box<[f32]>,
    frames: usize,
    /// Each bin's reference for the frame in hand.
    reference: Box<[f32]>,
    bins: usize,
    /// The history slot the next frame goes in.
    slot: usize,
}

impl Flux {
    fn new(bins: usize, frames: usize) -> Self {
        let frames = frames.max(1);
        Self {
            history: vec![0.0; frames * bins].into_boxed_slice(),
            frames,
            reference: vec![0.0; bins].into_boxed_slice(),
            bins,
            slot: 0,
        }
    }

    fn clear(&mut self) {
        self.history.fill(0.0);
        self.slot = 0;
    }

    /// Work out each bin's reference from the history, then take `frame`
    /// into it. Frames go into the history already spread across each
    /// bin's neighbours, so the reference is a plain bin-wise maximum over
    /// the history, which vectorises.
    fn update(&mut self, frame: &Frame<'_>) {
        let n = self.bins;
        self.reference.fill(0.0);
        for past in self.history.chunks_exact(n) {
            for (r, &m) in self.reference.iter_mut().zip(past) {
                *r = r.max(m);
            }
        }
        let slot = &mut self.history[self.slot * n..(self.slot + 1) * n];
        let magnitude = |k: usize| frame.power(k).sqrt();
        let mut before = 0.0f32;
        let mut here = magnitude(0);
        for (k, out) in slot.iter_mut().enumerate() {
            let after = if k + 1 < n { magnitude(k + 1) } else { 0.0 };
            *out = before.max(here).max(after);
            (before, here) = (here, after);
        }
        self.slot = (self.slot + 1) % self.frames;
    }
}

/// The bin range of each band, for a frame of `bins` bins.
fn band_bins(bins: usize, rate: f32, low_xover: f32) -> [std::ops::Range<usize>; BANDS] {
    let per_bin = rate / (2 * (bins - 1)) as f32;
    let at = |hz: f32| ((hz / per_bin).ceil() as usize).clamp(1, bins);
    [at(low_xover)..at(MID_HIGH_XOVER), at(MID_HIGH_XOVER)..bins]
}

/// One frame's work: peaks and flux per band.
fn frame_done(
    frame: &Frame<'_>,
    rate: f32,
    low_xover: f32,
    flux: &mut Flux,
    states: &mut [BandState; BANDS],
) {
    flux.update(frame);
    let ranges = band_bins(frame.len(), rate, low_xover);
    let edges = [(low_xover, MID_HIGH_XOVER), (MID_HIGH_XOVER, rate)];
    for ((state, range), (lo, hi)) in states.iter_mut().zip(ranges).zip(edges) {
        state.found = peaks_between(frame, rate, lo, hi, PEAK_THRESHOLD_DB, &mut state.peaks);
        let (mut rise, mut before) = (0.0f32, 0.0f32);
        let count = range.len().max(1) as f32;
        let mut now_sum = 0.0f32;
        for k in range {
            let now = frame.power(k).sqrt();
            rise += (now - flux.reference[k]).max(0.0);
            before += flux.reference[k];
            now_sum += now;
        }
        // Each bin's magnitude is scaled by the window: compare against a
        // sine-normalised silence level.
        let scale = 4.0 / (2 * (frame.len() - 1)) as f32;
        let loud = now_sum / count * scale > SILENCE;
        let relative = rise / before.max(now_sum * 1.0e-3).max(1.0e-20);
        if !loud || relative < 0.5 * ONSET_RISE {
            state.armed = true;
        } else if state.armed && relative > ONSET_RISE {
            state.armed = false;
            let window = 2 * (frame.len() - 1);
            let onset = Onset {
                rise: relative,
                frame_start: frame.end.saturating_sub(window as u64),
                frame_end: frame.end,
            };
            if state.onset.is_none_or(|o| onset.rise > o.rise) {
                state.onset = Some(onset);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_are_at_least_5_ms() {
        for (rate, window) in [
            (44_100.0, 256),
            (48_000.0, 256),
            (96_000.0, 512),
            (192_000.0, 1024),
        ] {
            assert_eq!(window_for(rate), window);
            assert!(window as f32 / rate >= MIN_WINDOW_SECONDS);
        }
    }
}
