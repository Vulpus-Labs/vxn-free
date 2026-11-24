//! The sidechain that listens to the dry input (ADR 0002, points 6 and 7).
//!
//! It runs beside the audio and never in it: the audio path doesn't wait
//! for it, and it lags the input by a few milliseconds ([`Analysis::lag`]).
//! Every voice reads at least [`FLOOR`](crate::flock::FLOOR) back, which is
//! longer than that lag, so whatever the analysis says about a sample
//! reaches the voices before they play it.
//!
//! - **Onsets**, per band (MID, HIGH), with no FFT. Each band's energy is
//!   followed fast (a 1 ms one-pole) and judged once per block of
//!   [`ENVELOPE_BLOCK`] samples against a reference: the largest the band
//!   has been over the last [`REFERENCE_SECONDS`]. A block that rises about
//!   4 dB over the reference, and isn't near silence, is an onset. A band
//!   re-arms when its rise falls back below 2 dB, or it goes quiet, and
//!   then waits out the rest of the reference time before firing again (a
//!   click's energy can rise in two steps a millisecond apart). The
//!   30 ms reference keeps sustained tones, whose partials beat, and
//!   buzzes, whose common period makes a burst every few tens of
//!   milliseconds, from reading as onsets. Each onset records where it
//!   starts, a block and 3 ms before it fired, so a duck can begin there.
//! - **Coherence** per band, from the band's mid and side signals smoothed
//!   over about 300 ms: (M² − S²)/(M² + S²). That is 1 when the channels
//!   are equal, 0 when they are uncorrelated at equal level and −1 when one
//!   is the other inverted.
//!
//! Ticket 0029 chose this detector over spectral flux from a 5 ms STFT: it
//! found more hits in a drum loop's MID band (28 of 40 against 10), as
//! many elsewhere, sooner (1.3 ms against 2), for less CPU, and neither
//! saw a legato line's note changes.

use vxn_free_dsp::Crossover;

use crate::engine::MID_HIGH_XOVER;
use crate::observe::{BANDS, BandObs};

/// How far back an onset's reference reaches, seconds: longer than the
/// period of any buzz above about 33 Hz, shorter than sixteenth notes.
pub const REFERENCE_SECONDS: f32 = 0.03;
/// The fast level's time constant, seconds.
const ENVELOPE_SECONDS: f32 = 0.001;
/// Onsets are judged once per block of this many samples.
pub const ENVELOPE_BLOCK: usize = 16;
/// A block's peak energy over the reference that counts as an onset:
/// about 4 dB.
const ENERGY_RISE: f32 = 2.5;
/// An onset is marked as starting this long before the block it fired in,
/// seconds: the fast level's rise takes a moment.
const LOOKBACK_SECONDS: f32 = 0.003;
/// Below this amplitude (as a sine's) a band is silent: nothing in it is
/// an onset, and a reference below it counts as it.
const SILENCE: f32 = 1.0e-4;
/// Coherence's smoothing time constant, seconds.
const COHERENCE_SECONDS: f32 = 0.3;

/// The latest onset in one band.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Onset {
    /// The block's energy over the reference that made it one.
    pub rise: f32,
    /// The stream position the onset is taken to start at.
    pub start: u64,
    /// The stream position it was found at.
    pub found: u64,
}

/// One band's running analysis.
#[derive(Clone, Copy, Debug)]
struct BandState {
    /// The strongest onset since the last [`Analysis::take`], if any.
    onset: Option<Onset>,
    /// Whether a rise can fire an onset (it can't again until the last
    /// one has passed).
    armed: bool,
    /// Blocks to wait before it can fire again.
    hold: usize,
    /// The fast energy, and the peak of it in the block in hand.
    level: f32,
    block_peak: f32,
    /// Smoothed M² and S².
    mid_power: f32,
    side_power: f32,
}

impl Default for BandState {
    fn default() -> Self {
        Self {
            onset: None,
            armed: true,
            hold: 0,
            level: 0.0,
            block_peak: 0.0,
            mid_power: 0.0,
            side_power: 0.0,
        }
    }
}

/// Charm's analysis sidechain. Build it with [`Analysis::new`] at the
/// host's rate.
#[derive(Debug)]
pub struct Analysis {
    bands: [BandState; BANDS],
    /// Each band's block peaks over the reference time, a ring of `count`
    /// per band.
    blocks: Box<[f32]>,
    count: usize,
    slot: usize,
    into_block: usize,
    /// Samples since the last restart.
    position: u64,
    level_coeff: f32,
    coherence_coeff: f32,
    lookback: u64,
    /// Band-splitters for the side signal (the mid signal's are the
    /// engine's).
    side_low: Crossover,
    side_split: Crossover,
}

impl Analysis {
    /// Allocates; call it off the audio thread.
    pub fn new(sample_rate: f32, low_xover: f32) -> Self {
        let count = ((REFERENCE_SECONDS * sample_rate) / ENVELOPE_BLOCK as f32).ceil() as usize;
        Self {
            bands: [BandState::default(); BANDS],
            blocks: vec![0.0; BANDS * count].into_boxed_slice(),
            count,
            slot: 0,
            into_block: 0,
            position: 0,
            level_coeff: 1.0 - (-1.0 / (ENVELOPE_SECONDS * sample_rate)).exp(),
            coherence_coeff: 1.0 - (-1.0 / (COHERENCE_SECONDS * sample_rate)).exp(),
            lookback: (LOOKBACK_SECONDS * sample_rate) as u64 + ENVELOPE_BLOCK as u64,
            side_low: Crossover::new(low_xover, sample_rate),
            side_split: Crossover::new(MID_HIGH_XOVER, sample_rate),
        }
    }

    /// Samples the analysis can lag the input by: an onset is judged at
    /// the end of its block and taken to start `lookback` before it.
    pub fn lag(&self) -> usize {
        self.lookback as usize + ENVELOPE_BLOCK
    }

    /// Samples since the last restart: the stream position of the next
    /// sample.
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Move MID's lower edge.
    pub fn set_low_xover(&mut self, hz: f32) {
        self.side_low.set_frequency(hz);
    }

    /// Back to silence.
    pub fn restart(&mut self) {
        self.bands = [BandState::default(); BANDS];
        self.blocks.fill(0.0);
        self.slot = 0;
        self.into_block = 0;
        self.position = 0;
        self.side_low.reset();
        self.side_split.reset();
    }

    /// Feed one stretch of input: `bands` is the MID and HIGH parts of the
    /// mid signal (L + R)/2, sample for sample, and `side` is (L − R)/2.
    pub fn push(&mut self, bands: &[[f32; BANDS]], side: &[f32]) {
        // A sine at the silence level has this much energy.
        let quiet = 0.5 * SILENCE * SILENCE;
        for (x, &s) in bands.iter().zip(side) {
            let (_, rest) = self.side_low.split(s);
            let (side_mid, side_high) = self.side_split.split(rest);
            for ((band, &m), sb) in self.bands.iter_mut().zip(x).zip([side_mid, side_high]) {
                band.level += self.level_coeff * (m * m - band.level);
                band.block_peak = band.block_peak.max(band.level);
                band.mid_power += self.coherence_coeff * (m * m - band.mid_power);
                band.side_power += self.coherence_coeff * (sb * sb - band.side_power);
            }
            self.position += 1;
            self.into_block += 1;
            if self.into_block < ENVELOPE_BLOCK {
                continue;
            }
            self.into_block = 0;
            for (b, band) in self.bands.iter_mut().enumerate() {
                let ring = &mut self.blocks[b * self.count..(b + 1) * self.count];
                let reference = ring.iter().fold(0.0f32, |m, &v| m.max(v));
                let peak = band.block_peak;
                // Anything under the silence level counts as silence, so a
                // tail wavering just above it isn't a rise from nothing.
                let ratio = peak / reference.max(quiet);
                band.hold = band.hold.saturating_sub(1);
                if peak < quiet || ratio < ENERGY_RISE.sqrt() {
                    band.armed = true;
                } else if band.armed && band.hold == 0 && ratio > ENERGY_RISE {
                    band.armed = false;
                    band.hold = self.count;
                    let onset = Onset {
                        rise: ratio,
                        start: self.position.saturating_sub(self.lookback),
                        found: self.position,
                    };
                    if band.onset.is_none_or(|o| onset.rise > o.rise) {
                        band.onset = Some(onset);
                    }
                }
                ring[self.slot] = peak;
                band.block_peak = 0.0;
            }
            self.slot = (self.slot + 1) % self.count;
        }
    }

    /// The analysis for the latest tick into `obs`, clearing the onsets
    /// seen since the last call. Returns each band's onset, if any.
    pub fn take(&mut self, obs: &mut [BandObs; BANDS]) -> [Option<Onset>; BANDS] {
        let mut onsets = [None; BANDS];
        for ((state, o), onset) in self.bands.iter_mut().zip(obs).zip(&mut onsets) {
            let total = state.mid_power + state.side_power;
            o.coherence = if total > 1.0e-12 {
                (state.mid_power - state.side_power) / total
            } else {
                1.0
            };
            *onset = state.onset.take();
            o.onset = onset.map_or(0.0, |x| x.rise);
            o.onset_start = onset.map_or(0, |x| x.start);
        }
        onsets
    }
}
