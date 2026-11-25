//! Beverley's engine: one crush chain ([`Chain`]) over its input, with a
//! dry/wet mix. It sees only the bridge (`silkscreen::bridge`) and
//! `vxn-free-dsp`: it runs on the audio thread, so it never allocates or
//! locks.
//!
//! Settings are worked out once per control tick of [`TICK`] samples. The
//! tick count carries across `process` calls, so how the host splits its
//! blocks doesn't change how settings move. The output lags the input by
//! nothing: the chain is memoryless but for auto gain's detectors, and
//! isn't oversampled (aliasing was measured, and oversampling bought too
//! little for its cost: ticket 0003).

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;

use crate::chain::{Chain, Setting};
pub use crate::chain::{Detect, Stereo, TICK};
use crate::crush::Mode;
use crate::{AUTO_GAIN, BIT_DEPTH, CRUSH_MODE, DECAY, DETECT, GAIN, GAMMA, MIX, SMOOTHING, STEREO};

/// Beverley. Build it with [`BeverleyEngine::new`] at the host's sample
/// rate.
#[derive(Debug)]
pub struct BeverleyEngine {
    chain: Chain,
    /// Samples of the current tick already rendered; a new tick starts
    /// when it reaches `TICK`.
    into_tick: usize,
}

impl BeverleyEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            chain: Chain::new(sample_rate),
            into_tick: TICK,
        }
    }

    /// Samples the output lags the input by, at any setting and rate:
    /// none.
    pub const LATENCY: u32 = 0;
}

/// Beverley's param `index` at plain `value`, as a chain setting.
pub fn setting(index: ParamIndex, value: f32) -> Option<Setting> {
    Some(match index {
        GAIN => Setting::Gain(value),
        AUTO_GAIN => Setting::AutoGain(value >= 0.5),
        GAMMA => Setting::Gamma(value),
        BIT_DEPTH => Setting::BitDepth(value),
        SMOOTHING => Setting::Smoothing(value),
        CRUSH_MODE => Setting::CrushMode(if value >= 0.5 {
            Mode::Symmetric
        } else {
            Mode::Asymmetric
        }),
        MIX => Setting::Mix(value),
        STEREO => Setting::Stereo(match value.round() as i32 {
            0 => Stereo::Mono,
            1 => Stereo::Separate,
            _ => Stereo::Linked,
        }),
        DECAY => Setting::Decay(value),
        DETECT => Setting::Detect(if value >= 0.5 {
            Detect::Range
        } else {
            Detect::Peak
        }),
        _ => return None,
    })
}

impl Engine for BeverleyEngine {
    fn set_param(&mut self, index: ParamIndex, value: f32) {
        if let Some(s) = setting(index, value) {
            self.chain.set(s);
        }
    }

    fn process(&mut self, channels: &mut [&mut [f32]]) {
        let frames = channels.iter().map(|c| c.len()).min().unwrap_or(0);
        let mut start = 0;
        while start < frames {
            if self.into_tick == TICK {
                self.chain.tick();
                self.into_tick = 0;
            }
            let n = (TICK - self.into_tick).min(frames - start);
            let offset = self.into_tick;
            match channels {
                [] => {}
                [one] => self.chain.run_one(&mut one[start..start + n], offset),
                [l, r, ..] => {
                    self.chain
                        .run_two(&mut l[start..start + n], &mut r[start..start + n], offset);
                }
            }
            self.into_tick += n;
            start += n;
        }
    }

    fn reset(&mut self) {
        self.chain.reset();
        self.into_tick = TICK;
    }
}
