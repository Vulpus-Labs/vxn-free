//! Wesley's engine (ADR 0003): the input split into three bands, each
//! through its own crush chain, summed. It sees only the bridge
//! (`silkscreen::bridge`) and `vxn-free-dsp`: it runs on the audio thread,
//! so it never allocates or locks.
//!
//! Each control tick of [`TICK`] samples, the crossovers glide a step and
//! every band's chain ticks. Within a tick, each channel is split into
//! bands a sample at a time, each band's stretch runs through its chain,
//! and the bands are summed:
//!
//! - a band's ON crossfades between its uncrushed and crushed signal,
//!   and SOLO fades bands in and out, so switching either doesn't click;
//! - LEVEL scales the band's output;
//! - MIX blends the sum against the uncrushed bands' sum, not the raw
//!   input: the split turns the phase, and mixing with the raw input
//!   would comb about the crossovers (point 5).
//!
//! Each band's chain has its own auto gain DETECT and DECAY; STEREO goes
//! to all three.
//!
//! Nothing is delayed, so the latency is zero; the crossovers turn the
//! phase (ticket 0031).

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::crush::Mode;
use vxn_free_dsp::crush_chain::{Chain, Detect, Setting, Stereo, TICK};
use vxn_free_dsp::{Glide, Ramp, ThreeBand};

use crate::{
    BANDS, BandParam, HIGH_XOVER, LOW_XOVER, MIX, PARAMS, SOLO, STEREO, band_param, soloed,
    which_band_param,
};

/// Time constant of the glides for LEVEL, ON, SOLO, MIX and the
/// crossovers.
const GLIDE_MS: f32 = 10.0;

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

fn default_of(index: ParamIndex) -> f32 {
    PARAMS[index.0 as usize].default
}

/// One band's level, on/off, solo and their ramps across the tick.
#[derive(Clone, Copy, Debug)]
struct BandGains {
    /// Linear.
    level: Glide,
    /// 0 (uncrushed) to 1 (crushed).
    on: Glide,
    /// 0 (muted by another band's solo) to 1 (heard).
    heard: Glide,
    level_ramp: Ramp,
    on_ramp: Ramp,
    heard_ramp: Ramp,
}

/// Wesley. Build it with [`WesleyEngine::new`] at the host's sample rate.
#[derive(Debug)]
pub struct WesleyEngine {
    /// Left and right.
    split: [ThreeBand; 2],
    /// Hz.
    low_xover: Glide,
    high_xover: Glide,
    chains: [Chain; BANDS],
    gains: [BandGains; BANDS],
    /// 0 (uncrushed) to 1 (crushed).
    mix: Glide,
    mix_ramp: Ramp,
    /// The soloed band, if any.
    solo: Option<usize>,
    /// Samples of the current tick already rendered; a new tick starts
    /// when it reaches `TICK`.
    into_tick: usize,
    /// Each band's uncrushed and crushed stretch, by channel.
    dry: [[[f32; TICK]; 2]; BANDS],
    wet: [[[f32; TICK]; 2]; BANDS],
}

impl WesleyEngine {
    pub fn new(sample_rate: f32) -> Self {
        let tick_rate = sample_rate / TICK as f32;
        let glide = |v| Glide::new(v, GLIDE_MS, tick_rate);
        let (lo, hi) = (default_of(LOW_XOVER), default_of(HIGH_XOVER));
        let mut e = Self {
            split: [ThreeBand::new(lo, hi, sample_rate); 2],
            low_xover: glide(lo),
            high_xover: glide(hi),
            chains: std::array::from_fn(|_| Chain::new(sample_rate)),
            gains: std::array::from_fn(|band| BandGains {
                level: glide(db_to_linear(default_of(band_param(band, BandParam::Level)))),
                on: glide(default_of(band_param(band, BandParam::On))),
                heard: glide(1.0),
                level_ramp: Ramp::default(),
                on_ramp: Ramp::default(),
                heard_ramp: Ramp::default(),
            }),
            mix: glide(default_of(MIX) * 0.01),
            mix_ramp: Ramp::default(),
            solo: None,
            into_tick: TICK,
            dry: [[[0.0; TICK]; 2]; BANDS],
            wet: [[[0.0; TICK]; 2]; BANDS],
        };
        // Every chain starts at its band's defaults.
        for (i, spec) in PARAMS.iter().enumerate() {
            e.set_param(ParamIndex(i as u32), spec.default);
        }
        e
    }

    /// Samples the output lags the input by, at any setting and rate:
    /// none.
    pub const LATENCY: u32 = 0;

    /// Start a control tick: glide each setting a step.
    fn tick(&mut self) {
        let (lo, hi) = (self.low_xover.value(), self.high_xover.value());
        let (lo_to, hi_to) = (self.low_xover.tick(), self.high_xover.tick());
        if (lo_to, hi_to) != (lo, hi) {
            for split in &mut self.split {
                split.set_frequencies(lo_to, hi_to);
            }
        }
        for chain in &mut self.chains {
            chain.tick();
        }
        for (band, g) in self.gains.iter_mut().enumerate() {
            g.heard.set_target(heard(self.solo, band));
            let ramp = |glide: &mut Glide| {
                let from = glide.value();
                Ramp::new(from, glide.tick(), TICK)
            };
            g.level_ramp = ramp(&mut g.level);
            g.on_ramp = ramp(&mut g.on);
            g.heard_ramp = ramp(&mut g.heard);
        }
        let from = self.mix.value();
        self.mix_ramp = Ramp::new(from, self.mix.tick(), TICK);
        self.into_tick = 0;
    }

    /// Render `channels` (one or two, all `n` long) from `offset` into the
    /// tick.
    fn run(&mut self, channels: &mut [&mut [f32]], offset: usize, n: usize) {
        let stereo = channels.len() > 1;
        for (c, channel) in channels.iter().enumerate().take(2) {
            for (j, &x) in channel[..n].iter().enumerate() {
                let bands = self.split[c].split(x);
                for (b, &y) in bands.iter().enumerate() {
                    self.dry[b][c][j] = y;
                    self.wet[b][c][j] = y;
                }
            }
        }
        for (b, chain) in self.chains.iter_mut().enumerate() {
            let [l, r] = &mut self.wet[b];
            if stereo {
                chain.run_two(&mut l[..n], &mut r[..n], offset);
            } else {
                chain.run_one(&mut l[..n], offset);
            }
        }
        for (c, channel) in channels.iter_mut().enumerate().take(2) {
            for (j, out) in channel[..n].iter_mut().enumerate() {
                let i = offset + j;
                let (mut dry_sum, mut wet_sum) = (0.0, 0.0);
                for (b, g) in self.gains.iter().enumerate() {
                    let (dry, wet) = (self.dry[b][c][j], self.wet[b][c][j]);
                    let heard = g.heard_ramp.at(i);
                    let band = dry + g.on_ramp.at(i) * (wet - dry);
                    dry_sum += heard * dry;
                    wet_sum += heard * g.level_ramp.at(i) * band;
                }
                *out = dry_sum + self.mix_ramp.at(i) * (wet_sum - dry_sum);
            }
        }
    }
}

/// How much of `band` is heard with `solo` soloed: all of it with none
/// soloed or itself soloed, none otherwise.
fn heard(solo: Option<usize>, band: usize) -> f32 {
    if solo.is_none_or(|s| s == band) {
        1.0
    } else {
        0.0
    }
}

/// A band's chain setting for its param `p` at plain `value`, if it is
/// one.
fn chain_setting(p: BandParam, value: f32) -> Option<Setting> {
    Some(match p {
        BandParam::Gain => Setting::Gain(value),
        BandParam::AutoGain => Setting::AutoGain(value >= 0.5),
        BandParam::Gamma => Setting::Gamma(value),
        BandParam::BitDepth => Setting::BitDepth(value),
        BandParam::Smoothing => Setting::Smoothing(value),
        BandParam::CrushMode => Setting::CrushMode(if value >= 0.5 {
            Mode::Symmetric
        } else {
            Mode::Asymmetric
        }),
        BandParam::Mix => Setting::Mix(value),
        BandParam::Detect => Setting::Detect(if value >= 0.5 {
            Detect::Range
        } else {
            Detect::Peak
        }),
        BandParam::Decay => Setting::Decay(value),
        BandParam::Level | BandParam::On => return None,
    })
}

impl Engine for WesleyEngine {
    fn set_param(&mut self, index: ParamIndex, value: f32) {
        if let Some((band, p)) = which_band_param(index) {
            if let Some(s) = chain_setting(p, value) {
                self.chains[band].set(s);
            }
            let g = &mut self.gains[band];
            match p {
                BandParam::Level => g.level.set_target(db_to_linear(value)),
                BandParam::On => g.on.set_target(if value >= 0.5 { 1.0 } else { 0.0 }),
                _ => {}
            }
            return;
        }
        let shared = match index {
            LOW_XOVER => {
                self.low_xover.set_target(value);
                return;
            }
            HIGH_XOVER => {
                self.high_xover.set_target(value);
                return;
            }
            MIX => {
                self.mix.set_target(value * 0.01);
                return;
            }
            SOLO => {
                self.solo = soloed(value);
                return;
            }
            STEREO => Setting::Stereo(match value.round() as i32 {
                0 => Stereo::Mono,
                1 => Stereo::Separate,
                _ => Stereo::Linked,
            }),
            _ => return,
        };
        for chain in &mut self.chains {
            chain.set(shared);
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
                [one] => self.run(&mut [&mut one[start..start + n]], offset, n),
                [l, r, ..] => self.run(
                    &mut [&mut l[start..start + n], &mut r[start..start + n]],
                    offset,
                    n,
                ),
            }
            self.into_tick += n;
            start += n;
        }
    }

    fn reset(&mut self) {
        for (band, g) in self.gains.iter_mut().enumerate() {
            g.heard.set_target(heard(self.solo, band));
            for glide in [&mut g.level, &mut g.on, &mut g.heard] {
                glide.snap();
            }
        }
        for glide in [&mut self.low_xover, &mut self.high_xover, &mut self.mix] {
            glide.snap();
        }
        for split in &mut self.split {
            split.set_frequencies(self.low_xover.value(), self.high_xover.value());
            split.reset();
        }
        for chain in &mut self.chains {
            chain.reset();
        }
        self.into_tick = TICK;
    }
}
