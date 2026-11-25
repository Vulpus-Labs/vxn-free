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
    /// Duck and Bloom's gain where the voice reads (1 between onsets):
    /// an onset reaches each voice at its own delay.
    pub shape: f32,
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
    /// The furthest a voice may be panned: WIDTH times AUTO WIDTH's
    /// share, before the mono guard. AUTO WIDTH's walls.
    pub width: f32,
}

/// Everything the engine reports, as of the end of one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Observation {
    /// Ticks since the engine was reset.
    pub tick: u64,
    /// SPREAD as the flocks used it this tick, seconds: delays run from
    /// the floor to the floor plus this.
    pub spread: f32,
    /// DETUNE as a speed limit on the delay axis, seconds per second:
    /// the largest |velocity|.
    pub max_velocity: f32,
    /// MID, then HIGH.
    pub bands: [BandObs; BANDS],
}

/// Words per voice in a frame: delay, pan, velocity, gain, shape.
const VOICE_WORDS: usize = 5;
/// Words per band in a frame: active, onset, coherence, width, then the
/// voices.
const BAND_WORDS: usize = 4 + MAX_VOICES * VOICE_WORDS;
/// Words before the bands: tick, spread, max velocity.
const HEAD_WORDS: usize = 3;

/// Exact for whole numbers below this, so a frame's tick is exact.
const TICK_WRAP: u64 = 1 << 24;

/// Words in Charm's telemetry frame: the tick (modulo 2^24), spread and
/// max velocity, then each band's active count, onset, coherence, width
/// and voices (delay, pan, velocity, gain, shape). `onset_start` stays on
/// the audio side.
pub const FRAME_WORDS: usize = HEAD_WORDS + BANDS * BAND_WORDS;

impl Observation {
    /// This observation as a telemetry frame.
    pub fn to_frame(&self, out: &mut [f32; FRAME_WORDS]) {
        out[0] = (self.tick % TICK_WRAP) as f32;
        out[1] = self.spread;
        out[2] = self.max_velocity;
        let bands = out[HEAD_WORDS..].chunks_exact_mut(BAND_WORDS);
        for (band, words) in self.bands.iter().zip(bands) {
            words[..4].copy_from_slice(&[
                f32::from(band.active),
                band.onset,
                band.coherence,
                band.width,
            ]);
            let voices = words[4..].chunks_exact_mut(VOICE_WORDS);
            for (v, w) in band.voices.iter().zip(voices) {
                w.copy_from_slice(&[v.delay, v.pan, v.velocity, v.gain, v.shape]);
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
            spread: frame[1],
            max_velocity: frame[2],
            ..Self::default()
        };
        let bands = frame[HEAD_WORDS..].chunks_exact(BAND_WORDS);
        for (band, words) in obs.bands.iter_mut().zip(bands) {
            band.active = words[0] as u8;
            band.onset = words[1];
            band.coherence = words[2];
            band.width = words[3];
            let voices = words[4..].chunks_exact(VOICE_WORDS);
            for (v, w) in band.voices.iter_mut().zip(voices) {
                *v = VoiceObs {
                    delay: w[0],
                    pan: w[1],
                    velocity: w[2],
                    gain: w[3],
                    shape: w[4],
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
            spread: 0.01,
            max_velocity: 0.007,
            ..Observation::default()
        };
        for (b, band) in obs.bands.iter_mut().enumerate() {
            band.active = 3 + b as u8;
            band.onset = 0.25 * b as f32;
            band.onset_start = 99;
            band.coherence = -0.5;
            band.width = 0.6;
            for (i, v) in band.voices.iter_mut().enumerate() {
                let x = (b * 100 + i) as f32;
                *v = VoiceObs {
                    delay: x,
                    pan: -x,
                    velocity: x * 0.5,
                    gain: x * 2.0,
                    shape: x * 0.25,
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
