//! Per-voice allpass chains, so voices don't comb alike (ticket 0028).
//!
//! A voice is its band's signal, delayed, so with the dry signal it forms
//! a comb whose notches are evenly spaced, heard as hollowness. An allpass
//! chain leaves every frequency's level alone but turns each frequency's
//! phase by a different amount, so the voice's notches are no longer
//! evenly spaced. Each voice in each band has its own chain, so voices
//! don't comb alike with each other either.
//!
//! Each chain is [`STAGES`] second-order allpasses, centred at
//! log-spaced frequencies across the band. Each voice's centres are offset
//! by a different golden-ratio fraction of a step, and their Q varies a
//! little, so no two chains match. Nothing here moves: the chains are
//! fixed when built.

use vxn_free_dsp::Svf;

use crate::engine::MID_HIGH_XOVER;

/// Allpass stages per voice.
pub const STAGES: usize = 4;
/// The lowest centre in MID, Hz.
const MID_LOW: f32 = 200.0;
/// The highest centre in HIGH, Hz (or just below Nyquist).
const HIGH_TOP: f32 = 14_000.0;
/// The golden ratio's fractional part: successive multiples spread evenly.
const GOLDEN: f32 = 0.618_034;
/// Each stage's Q is from this to this plus `Q_SPAN`.
const Q_LOW: f32 = 1.5;
const Q_SPAN: f32 = 1.5;

/// One voice's chain.
#[derive(Clone, Copy, Debug)]
pub struct Chain {
    stages: [Svf; STAGES],
}

/// The fractional part.
fn frac(x: f32) -> f32 {
    x - x.floor()
}

/// The filters of voice `voice`'s chain in band `band` (0 MID, 1 HIGH),
/// fresh: what [`Chain::new`] runs, for the engine to lay out side by side.
pub fn chain_filters(band: usize, voice: usize, sample_rate: f32) -> [Svf; STAGES] {
    let (lo, hi) = if band == 0 {
        (MID_LOW, MID_HIGH_XOVER)
    } else {
        (MID_HIGH_XOVER, HIGH_TOP.min(0.4 * sample_rate))
    };
    let offset = frac(GOLDEN * (voice + 1) as f32 + 0.37 * band as f32);
    let mut stages = [Svf::new(1000.0, 1.0, sample_rate); STAGES];
    for (j, s) in stages.iter_mut().enumerate() {
        let at = (j as f32 + offset) / STAGES as f32;
        let hz = lo * (hi / lo).powf(at);
        let q = Q_LOW + Q_SPAN * frac(GOLDEN * (voice * STAGES + j + 1) as f32 * 1.7);
        *s = Svf::new(hz, q, sample_rate);
    }
    stages
}

impl Chain {
    /// The chain for voice `voice` of band `band` (0 MID, 1 HIGH).
    pub fn new(band: usize, voice: usize, sample_rate: f32) -> Self {
        Self {
            stages: chain_filters(band, voice, sample_rate),
        }
    }

    /// One sample through the chain.
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.stages.iter_mut().fold(x, |y, s| s.allpass(y))
    }

    /// Forget the past.
    pub fn reset(&mut self) {
        for s in &mut self.stages {
            s.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_VOICES;
    use crate::observe::BANDS;
    use std::f64::consts::TAU;

    #[test]
    fn every_chain_is_allpass() {
        let fs = 48_000.0;
        for band in 0..BANDS {
            for voice in [0, 3, 7] {
                for hz in [100.0f64, 700.0, 2500.0, 9000.0] {
                    let mut c = Chain::new(band, voice, fs);
                    let w = TAU * hz / f64::from(fs);
                    let (mut si, mut so) = (0.0f64, 0.0f64);
                    for n in 0..96_000 {
                        let x = (w * f64::from(n)).sin() as f32;
                        let y = c.process(x);
                        if n >= 48_000 {
                            si += f64::from(x * x);
                            so += f64::from(y * y);
                        }
                    }
                    let db = 10.0 * (so / si).log10();
                    assert!(
                        db.abs() < 0.01,
                        "band {band} voice {voice} {hz} Hz: {db} dB"
                    );
                }
            }
        }
    }

    #[test]
    fn no_two_chains_are_the_same() {
        let fs = 48_000.0;
        let mut seen: Vec<Vec<u32>> = Vec::new();
        for band in 0..BANDS {
            for voice in 0..MAX_VOICES {
                let mut c = Chain::new(band, voice, fs);
                let ir: Vec<u32> = (0..64)
                    .map(|n| c.process(if n == 0 { 1.0 } else { 0.0 }).to_bits())
                    .collect();
                assert!(!seen.contains(&ir), "band {band} voice {voice}");
                seen.push(ir);
            }
        }
    }
}
