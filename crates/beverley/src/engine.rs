//! Beverley's engine: the crush chain on each channel, with a dry/wet
//! mix. It sees only the bridge (`silkscreen::bridge`) and
//! `vxn-free-dsp`: it runs on the audio thread, so it never allocates or
//! locks.
//!
//! Settings are worked out once per control tick of [`TICK`] samples.
//! Each glides towards its target over a few milliseconds so automation
//! doesn't zipper; gain and mix, where a step would click, also ramp in
//! a straight line across each tick's samples. The tick count carries
//! across `process` calls, so how the host splits its blocks doesn't
//! change how settings move.
//!
//! The output lags the input by nothing: the chain is memoryless but for
//! auto gain's envelope, and isn't oversampled (aliasing was measured,
//! and oversampling bought too little for its cost: ticket 0003).

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::{Glide, PeakFollower, Ramp};

use crate::crush::{ATTACK_MS, Mode, Prepared, RELEASE_MS, Settings, auto_gain};
use crate::{AUTO_GAIN, BIT_DEPTH, CRUSH_MODE, GAIN, GAMMA, MIX, SMOOTHING, STEREO};

/// Samples per control tick.
pub const TICK: usize = 32;
/// Time constant of every setting's glide.
const GLIDE_MS: f32 = 10.0;

/// How the two channels are crushed. With one channel, all three crush
/// it alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stereo {
    /// Left and right summed to one signal, crushed once and sent to both.
    Mono,
    /// Each channel crushed on its own, auto gain following each.
    Separate,
    /// Each channel crushed on its own, with one auto gain for both,
    /// following the mid signal (the average of the two), so the image
    /// holds still.
    Linked,
}

/// Beverley. Build it with [`BeverleyEngine::new`] at the host's sample
/// rate.
#[derive(Debug)]
pub struct BeverleyEngine {
    /// Linear gain.
    gain: Glide,
    /// GAMMA's knob, −1 to 1; the power is ten to it.
    gamma: Glide,
    bits: Glide,
    /// 0 to 1.
    smoothing: Glide,
    /// 0 (dry) to 1 (wet).
    mix: Glide,
    auto_gain: bool,
    mode: Mode,
    stereo: Stereo,
    /// Auto gain's envelopes: left (or mono, or mid) and right.
    peaks: [PeakFollower; 2],
    prepared: Prepared,
    gain_ramp: Ramp,
    mix_ramp: Ramp,
    /// Samples of the current tick already rendered; a new tick starts
    /// when it reaches `TICK`.
    into_tick: usize,
}

/// `mix` of `wet` and the rest `dry`: exactly one or the other at the
/// ends.
#[inline]
fn blend(dry: f32, wet: f32, mix: f32) -> f32 {
    wet * mix + dry * (1.0 - mix)
}

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

impl BeverleyEngine {
    pub fn new(sample_rate: f32) -> Self {
        let tick_rate = sample_rate / TICK as f32;
        let glide = |v| Glide::new(v, GLIDE_MS, tick_rate);
        let settings = Settings::default();
        Self {
            gain: glide(1.0),
            gamma: glide(0.0),
            bits: glide(settings.bits),
            smoothing: glide(0.0),
            mix: glide(1.0),
            auto_gain: false,
            mode: Mode::Asymmetric,
            stereo: Stereo::Linked,
            peaks: [PeakFollower::new(ATTACK_MS, RELEASE_MS, sample_rate); 2],
            prepared: Prepared::new(&settings),
            gain_ramp: Ramp::new(1.0, 1.0, TICK),
            mix_ramp: Ramp::new(1.0, 1.0, TICK),
            into_tick: TICK,
        }
    }

    /// Samples the output lags the input by, at any setting and rate:
    /// none.
    pub const LATENCY: u32 = 0;

    /// Start a control tick: glide each setting one step and prepare the
    /// chain for it.
    fn tick(&mut self) {
        let gain_from = self.gain.value();
        let mix_from = self.mix.value();
        let settings = Settings {
            gain: self.gain.tick(),
            gamma: 10f32.powf(self.gamma.tick()),
            bits: self.bits.tick(),
            smoothing: self.smoothing.tick(),
            mode: self.mode,
        };
        self.prepared = Prepared::new(&settings);
        self.gain_ramp = Ramp::new(gain_from, settings.gain, TICK);
        self.mix_ramp = Ramp::new(mix_from, self.mix.tick(), TICK);
        self.into_tick = 0;
    }

    /// The gain for sample `i` of the tick: the ramped fixed gain, or the
    /// envelope of `magnitude` through `peak`.
    #[inline]
    fn gain_at(&self, i: usize, peak: &mut PeakFollower, magnitude: f32) -> f32 {
        if self.auto_gain {
            auto_gain(peak.next(magnitude))
        } else {
            self.gain_ramp.at(i)
        }
    }

    /// Render `n` samples of one channel from `offset` into the tick.
    fn run_one(&mut self, x: &mut [f32], offset: usize) {
        let mut peak = self.peaks[0];
        for (i, s) in x.iter_mut().enumerate() {
            let i = offset + i;
            let dry = *s;
            let wet = self
                .prepared
                .crush(dry, self.gain_at(i, &mut peak, dry.abs()));
            let mix = self.mix_ramp.at(i);
            *s = blend(dry, wet, mix);
        }
        self.peaks[0] = peak;
    }

    /// Render two channels from `offset` into the tick.
    fn run_two(&mut self, left: &mut [f32], right: &mut [f32], offset: usize) {
        let [mut pl, mut pr] = self.peaks;
        for (n, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let i = offset + n;
            let (dl, dr) = (*l, *r);
            let (wl, wr) = match self.stereo {
                Stereo::Mono => {
                    let mono = 0.5 * (dl + dr);
                    let wet = self
                        .prepared
                        .crush(mono, self.gain_at(i, &mut pl, mono.abs()));
                    (wet, wet)
                }
                Stereo::Separate => (
                    self.prepared.crush(dl, self.gain_at(i, &mut pl, dl.abs())),
                    self.prepared.crush(dr, self.gain_at(i, &mut pr, dr.abs())),
                ),
                Stereo::Linked => {
                    let gain = self.gain_at(i, &mut pl, (0.5 * (dl + dr)).abs());
                    (self.prepared.crush(dl, gain), self.prepared.crush(dr, gain))
                }
            };
            let mix = self.mix_ramp.at(i);
            *l = blend(dl, wl, mix);
            *r = blend(dr, wr, mix);
        }
        self.peaks = [pl, pr];
    }
}

impl Engine for BeverleyEngine {
    fn set_param(&mut self, index: ParamIndex, value: f32) {
        match index {
            GAIN => self.gain.set_target(db_to_linear(value)),
            AUTO_GAIN => self.auto_gain = value >= 0.5,
            GAMMA => self.gamma.set_target(value),
            BIT_DEPTH => self.bits.set_target(value),
            SMOOTHING => self.smoothing.set_target(value * 0.01),
            CRUSH_MODE => {
                self.mode = if value >= 0.5 {
                    Mode::Symmetric
                } else {
                    Mode::Asymmetric
                };
            }
            MIX => self.mix.set_target(value * 0.01),
            STEREO => {
                self.stereo = match value.round() as i32 {
                    0 => Stereo::Mono,
                    1 => Stereo::Separate,
                    _ => Stereo::Linked,
                };
            }
            _ => {}
        }
    }

    fn process(&mut self, channels: &mut [&mut [f32]]) {
        let frames = channels.iter().map(|c| c.len()).min().unwrap_or(0);
        let mut start = 0;
        while start < frames {
            if self.into_tick == TICK {
                self.tick();
            }
            let n = (TICK - self.into_tick).min(frames - start);
            let offset = self.into_tick;
            match channels {
                [] => {}
                [one] => self.run_one(&mut one[start..start + n], offset),
                [l, r, ..] => {
                    self.run_two(&mut l[start..start + n], &mut r[start..start + n], offset);
                }
            }
            self.into_tick += n;
            start += n;
        }
    }

    fn reset(&mut self) {
        for glide in [
            &mut self.gain,
            &mut self.gamma,
            &mut self.bits,
            &mut self.smoothing,
            &mut self.mix,
        ] {
            glide.snap();
        }
        for peak in &mut self.peaks {
            peak.reset();
        }
        // The next tick starts from the snapped values.
        self.into_tick = TICK;
    }
}
