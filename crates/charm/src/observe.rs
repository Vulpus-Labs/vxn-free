//! What the engine is doing, once per flock tick.
//!
//! One plain, fixed-size struct that the bench (ticket 0023), telemetry
//! frames (0022) and the flock display (0019) all read. It is filled in
//! place on the audio thread and never allocates. The analysis fields are
//! reserved here and filled from ticket 0015 on.

use crate::MAX_VOICES;

/// The two chorused bands, each with its own flock.
pub const BANDS: usize = 2;
/// Index of the MID band.
pub const MID: usize = 0;
/// Index of the HIGH band.
pub const HIGH: usize = 1;

/// One voice at the end of a tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VoiceObs {
    /// Delay, seconds.
    pub delay: f32,
    /// Pan, −1 (left) to 1 (right), with WIDTH applied.
    pub pan: f32,
    /// Rate of change of the delay, seconds per second. The voice plays
    /// at ratio 1 − velocity.
    pub velocity: f32,
    /// The voice's gain, 0 while silent (beyond VOICES, or fading).
    pub gain: f32,
}

impl VoiceObs {
    /// The voice's detune in cents: negative when its delay grows.
    pub fn cents(&self) -> f32 {
        1200.0 * (1.0 - self.velocity).log2()
    }
}

/// One band's flock and analysis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BandObs {
    /// Voices in use (VOICES); the rest of `voices` are silent.
    pub active: u8,
    pub voices: [VoiceObs; MAX_VOICES],
    /// Onset strength this tick (the frame's relative rise in
    /// magnitude), 0 when none (0015, 0016).
    pub onset: f32,
    /// Where the onset's frame starts, in samples since reset; 0 when
    /// none.
    pub onset_start: u64,
    /// Inter-channel coherence of the input, −1 to 1 (0015, 0018).
    pub coherence: f32,
}

/// Everything the engine reports, as of the end of one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Observation {
    /// Ticks since the engine was reset.
    pub tick: u64,
    /// MID, then HIGH.
    pub bands: [BandObs; BANDS],
}
