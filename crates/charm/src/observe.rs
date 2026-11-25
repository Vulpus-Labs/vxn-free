//! What the engine is doing, once per flock tick.
//!
//! One plain, fixed-size struct that the bench (ticket 0023), telemetry
//! frames (0022) and the flock display (0019) all read. It is filled in
//! place on the audio thread and never allocates. The analysis fields are
//! reserved here and filled from ticket 0015 on.
//!
//! The editor gets it as a telemetry frame of [`FRAME_WORDS`] floats
//! ([`Observation::to_frame`], [`Observation::from_frame`]), published whole
//! by silkscreen so the display never draws half of one tick and half of the
//! next.

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

/// Words per voice in a frame: delay, pan, velocity, gain.
const VOICE_WORDS: usize = 4;
/// Words per band in a frame: active, onset, coherence, then the voices.
const BAND_WORDS: usize = 3 + MAX_VOICES * VOICE_WORDS;

/// Exact for whole numbers below this, so a frame's tick is exact.
const TICK_WRAP: u64 = 1 << 24;

/// Words in Charm's telemetry frame: the tick (modulo 2^24), then each
/// band's active count, onset, coherence and voices (delay, pan, velocity,
/// gain). `onset_start` stays on the audio side.
pub const FRAME_WORDS: usize = 1 + BANDS * BAND_WORDS;

impl Observation {
    /// This observation as a telemetry frame.
    pub fn to_frame(&self, out: &mut [f32; FRAME_WORDS]) {
        out[0] = (self.tick % TICK_WRAP) as f32;
        for (band, words) in self.bands.iter().zip(out[1..].chunks_exact_mut(BAND_WORDS)) {
            words[0] = f32::from(band.active);
            words[1] = band.onset;
            words[2] = band.coherence;
            for (v, w) in band
                .voices
                .iter()
                .zip(words[3..].chunks_exact_mut(VOICE_WORDS))
            {
                w.copy_from_slice(&[v.delay, v.pan, v.velocity, v.gain]);
            }
        }
    }

    /// An observation back from a frame, its tick modulo 2^24 and without
    /// `onset_start`; `None` if `frame` isn't [`FRAME_WORDS`] long.
    pub fn from_frame(frame: &[f32]) -> Option<Self> {
        if frame.len() != FRAME_WORDS {
            return None;
        }
        let mut obs = Self {
            tick: frame[0] as u64,
            ..Self::default()
        };
        for (band, words) in obs
            .bands
            .iter_mut()
            .zip(frame[1..].chunks_exact(BAND_WORDS))
        {
            band.active = words[0] as u8;
            band.onset = words[1];
            band.coherence = words[2];
            for (v, w) in band
                .voices
                .iter_mut()
                .zip(words[3..].chunks_exact(VOICE_WORDS))
            {
                *v = VoiceObs {
                    delay: w[0],
                    pan: w[1],
                    velocity: w[2],
                    gain: w[3],
                };
            }
        }
        Some(obs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_all_but_the_onset_start() {
        let mut obs = Observation {
            tick: TICK_WRAP + 7,
            ..Observation::default()
        };
        for (b, band) in obs.bands.iter_mut().enumerate() {
            band.active = 3 + b as u8;
            band.onset = 0.25 * b as f32;
            band.onset_start = 99;
            band.coherence = -0.5;
            for (i, v) in band.voices.iter_mut().enumerate() {
                let x = (b * 100 + i) as f32;
                *v = VoiceObs {
                    delay: x,
                    pan: -x,
                    velocity: x * 0.5,
                    gain: x * 2.0,
                };
            }
        }
        let mut frame = [0.0; FRAME_WORDS];
        obs.to_frame(&mut frame);
        let back = Observation::from_frame(&frame).unwrap();
        for band in &mut obs.bands {
            band.onset_start = 0;
        }
        assert_eq!(back, Observation { tick: 7, ..obs });
        assert_eq!(Observation::from_frame(&frame[1..]), None);
    }
}
