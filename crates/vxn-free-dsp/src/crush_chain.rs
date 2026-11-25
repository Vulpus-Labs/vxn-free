//! One crush chain ([`crate::crush`]) with its settings: what Beverley
//! runs on its input and Wesley on each of its bands (ADR 0004, tickets
//! 0032 and 0033). It runs on the audio thread, so it never allocates or
//! locks.
//!
//! Settings are worked out once per control tick of [`TICK`] samples: the
//! owner calls [`Chain::tick`] at the start of each, then runs the tick's
//! samples through [`Chain::run_one`] or [`Chain::run_two`].
//! Each glides towards its target over a few milliseconds so automation
//! doesn't zipper; gain and mix, where a step would click, also ramp in
//! a straight line across each tick's samples. The tick count carries
//! across `process` calls, so how the host splits its blocks doesn't
//! change how settings move.
//!
//! Auto gain follows the input one of two ways ([`Detect`]), letting go
//! at DECAY's rate. The output lags the input by nothing: the chain is
//! memoryless but for auto gain's detectors, and isn't oversampled
//! (aliasing was measured, and oversampling bought too little for its
//! cost: ticket 0003).

use crate::peak::fall;
use crate::{Glide, PeakFollower, Ramp, RangeFollower};

use crate::crush::{ATTACK_MS, Mode, Prepared, Settings, auto_gain};

/// Samples per control tick.
pub const TICK: usize = 32;
/// Time constant of every setting's glide.
const GLIDE_MS: f32 = 10.0;
/// DECAY's default, seconds.
const DEFAULT_DECAY: f32 = 0.115;

/// How the two channels are crushed. With one channel, all three crush
/// it alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stereo {
    /// Left and right summed to one signal, crushed once and sent to both.
    Mono,
    /// Each channel crushed on its own, auto gain following each.
    Separate,
    /// Each channel crushed on its own, with one auto gain for both, so
    /// the image holds still. PEAK follows the mid signal (the average of
    /// the two); RANGE follows both channels with one maximum and minimum.
    Linked,
}

/// How auto gain follows the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detect {
    /// The magnitude's peak, attacking in 0.5 ms and falling towards zero
    /// at DECAY's rate. The chain crushes about zero.
    Peak,
    /// The maximum and minimum, taking a new extreme at once and falling
    /// towards their midpoint at DECAY's rate. The gain brings the
    /// half-range to full scale, and the chain crushes about the
    /// midpoint, so an offset signal is crushed on its swing.
    Range,
}

/// One channel's (or one linked pair's) auto gain detectors. Only the
/// one in use is fed.
#[derive(Clone, Copy, Debug)]
struct Follow {
    peak: PeakFollower,
    range: RangeFollower,
}

/// One setting of a [`Chain`], in the units Beverley's params use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Setting {
    /// dB, 0 to 20.
    Gain(f32),
    AutoGain(bool),
    /// GAMMA's knob, −1 to 1: the power is ten to it.
    Gamma(f32),
    /// Bits, 1 to 16.
    BitDepth(f32),
    /// %, 0 to 100.
    Smoothing(f32),
    CrushMode(Mode),
    /// %, 0 (dry) to 100 (crushed).
    Mix(f32),
    Stereo(Stereo),
    /// Auto gain's DECAY, ms.
    Decay(f32),
    Detect(Detect),
}

/// One crush chain (a channel, or a linked or mono pair) with its
/// settings. Build it with [`Chain::new`] at the host's sample rate.
#[derive(Debug)]
pub struct Chain {
    sample_rate: f32,
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
    detect: Detect,
    /// DECAY in seconds, and the fall per sample worked out from it.
    decay: f32,
    fall_for: f32,
    /// Left (or mono, or linked) and right.
    follow: [Follow; 2],
    prepared: Prepared,
    gain_ramp: Ramp,
    mix_ramp: Ramp,
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

impl Chain {
    pub fn new(sample_rate: f32) -> Self {
        let tick_rate = sample_rate / TICK as f32;
        let glide = |v| Glide::new(v, GLIDE_MS, tick_rate);
        let settings = Settings::default();
        let f = fall(DEFAULT_DECAY, sample_rate);
        Self {
            sample_rate,
            gain: glide(1.0),
            gamma: glide(0.0),
            bits: glide(settings.bits),
            smoothing: glide(0.0),
            mix: glide(1.0),
            auto_gain: false,
            mode: Mode::Asymmetric,
            stereo: Stereo::Linked,
            detect: Detect::Range,
            decay: DEFAULT_DECAY,
            fall_for: DEFAULT_DECAY,
            follow: [Follow {
                peak: PeakFollower::new(ATTACK_MS, f, sample_rate),
                range: RangeFollower::new(f),
            }; 2],
            prepared: Prepared::new(&settings),
            gain_ramp: Ramp::new(1.0, 1.0, TICK),
            mix_ramp: Ramp::new(1.0, 1.0, TICK),
        }
    }

    /// Change a setting. Gain, gamma, depth, smoothing and mix glide to
    /// it from the next tick; the rest apply at once.
    pub fn set(&mut self, setting: Setting) {
        match setting {
            Setting::Gain(db) => self.gain.set_target(db_to_linear(db)),
            Setting::AutoGain(on) => self.auto_gain = on,
            Setting::Gamma(knob) => self.gamma.set_target(knob),
            Setting::BitDepth(bits) => self.bits.set_target(bits),
            Setting::Smoothing(pct) => self.smoothing.set_target(pct * 0.01),
            Setting::CrushMode(mode) => self.mode = mode,
            Setting::Mix(pct) => self.mix.set_target(pct * 0.01),
            Setting::Stereo(stereo) => self.stereo = stereo,
            Setting::Decay(ms) => self.decay = ms * 0.001,
            Setting::Detect(detect) => self.detect = detect,
        }
    }

    /// Start a control tick: glide each setting one step and prepare the
    /// chain for it.
    pub fn tick(&mut self) {
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
        if self.decay != self.fall_for {
            let f = fall(self.decay, self.sample_rate);
            for follow in &mut self.follow {
                follow.peak.set_fall(f);
                follow.range.set_fall(f);
            }
            self.fall_for = self.decay;
        }
    }

    /// One signal's wet sample, `i` samples into the tick, its auto gain
    /// following it alone.
    #[inline]
    fn wet(&self, i: usize, x: f32, follow: &mut Follow) -> f32 {
        if !self.auto_gain {
            return self.prepared.crush(x, self.gain_ramp.at(i));
        }
        match self.detect {
            Detect::Peak => self.prepared.crush(x, auto_gain(follow.peak.next(x.abs()))),
            Detect::Range => {
                follow.range.next(x);
                let mid = follow.range.mid();
                let gain = auto_gain(follow.range.half_range());
                self.prepared.crush(x - mid, gain) + mid
            }
        }
    }

    /// Two channels' wet samples with one auto gain (LINKED).
    #[inline]
    fn wet_linked(&self, i: usize, l: f32, r: f32, follow: &mut Follow) -> (f32, f32) {
        if !self.auto_gain {
            let gain = self.gain_ramp.at(i);
            return (self.prepared.crush(l, gain), self.prepared.crush(r, gain));
        }
        match self.detect {
            Detect::Peak => {
                let gain = auto_gain(follow.peak.next((0.5 * (l + r)).abs()));
                (self.prepared.crush(l, gain), self.prepared.crush(r, gain))
            }
            Detect::Range => {
                follow.range.decay();
                follow.range.push(l);
                follow.range.push(r);
                let mid = follow.range.mid();
                let gain = auto_gain(follow.range.half_range());
                (
                    self.prepared.crush(l - mid, gain) + mid,
                    self.prepared.crush(r - mid, gain) + mid,
                )
            }
        }
    }

    /// Render one channel from `offset` into the tick.
    pub fn run_one(&mut self, x: &mut [f32], offset: usize) {
        let mut follow = self.follow[0];
        for (n, s) in x.iter_mut().enumerate() {
            let i = offset + n;
            let dry = *s;
            let wet = self.wet(i, dry, &mut follow);
            *s = blend(dry, wet, self.mix_ramp.at(i));
        }
        self.follow[0] = follow;
    }

    /// Render two channels from `offset` into the tick.
    pub fn run_two(&mut self, left: &mut [f32], right: &mut [f32], offset: usize) {
        let [mut fl, mut fr] = self.follow;
        for (n, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let i = offset + n;
            let (dl, dr) = (*l, *r);
            let (wl, wr) = match self.stereo {
                Stereo::Mono => {
                    let wet = self.wet(i, 0.5 * (dl + dr), &mut fl);
                    (wet, wet)
                }
                Stereo::Separate => (self.wet(i, dl, &mut fl), self.wet(i, dr, &mut fr)),
                Stereo::Linked => self.wet_linked(i, dl, dr, &mut fl),
            };
            let mix = self.mix_ramp.at(i);
            *l = blend(dl, wl, mix);
            *r = blend(dr, wr, mix);
        }
        self.follow = [fl, fr];
    }

    /// Snap every setting to its target and empty the detectors.
    pub fn reset(&mut self) {
        for glide in [
            &mut self.gain,
            &mut self.gamma,
            &mut self.bits,
            &mut self.smoothing,
            &mut self.mix,
        ] {
            glide.snap();
        }
        for follow in &mut self.follow {
            follow.peak.reset();
            follow.range.reset();
        }
        // The next tick starts from the snapped values, at DECAY's fall.
        self.fall_for = f32::NAN;
    }
}
