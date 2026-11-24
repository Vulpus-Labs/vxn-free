//! Charm's engine. It sees only the bridge (`silkscreen::bridge`) and
//! `vxn-free-dsp`: it runs on the audio thread, so it never allocates or
//! locks.
//!
//! Signal path, per sample:
//!
//! 1. The input's mid signal, (L + R) / 2, is split by the LOW crossover;
//!    what is above it is split again at a fixed 2.5 kHz into MID and
//!    HIGH. LOW is never chorused.
//! 2. MID and HIGH each go into their own delay line, read by their own
//!    flock of voices (`flock`). Each voice is panned with equal power,
//!    scaled by its band's depth and by 1/√VOICES so the ensemble's
//!    level holds as voices come and go.
//! 3. The output is the unfiltered input, at unity, plus MIX times the
//!    voices. 100 % puts the ensemble level with the dry signal.
//!
//! Settings and the flocks step once per control tick of [`TICK`] samples.
//! Within a tick each voice's delay slides in a straight line, so its rate
//! of change, and so its pitch, is the voice's velocity exactly. Gains,
//! pans, depths and the mix ramp across the tick so nothing steps. The
//! tick count carries across `process` calls, so how the host splits its
//! blocks doesn't change the output.
//!
//! The output lags the input by nothing: the dry path is direct, and every
//! voice reads at least [`FLOOR`] back.

use std::f32::consts::FRAC_PI_4;

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::{Crossover, DelayLine, Glide, Ramp, SideBuffer};

use crate::analysis::Analysis;
use crate::decorrelate::Chain;
use crate::flock::{FLOOR, Flock, FlockSettings, max_velocity};
use crate::observe::{BANDS, Observation};
use crate::wet::{Shape, WetGain};
use crate::{
    AUTO_WIDTH, BLOOM, COHESION, DETUNE, DUCK, HIGH_DEPTH, LOW_XOVER, MAX_VOICES, MID_DEPTH,
    MIN_VOICES, MIX, PARAMS, SEPARATION, SPREAD, TURBULENCE, VOICES, WIDTH,
};

/// Samples per control tick.
pub const TICK: usize = 32;
/// Time constant of every setting's glide.
const GLIDE_MS: f32 = 20.0;
/// Time constant of a voice fading in or out as VOICES changes.
const FADE_MS: f32 = 30.0;
/// Where MID ends and HIGH begins (provisional: ticket 0024 settles it).
pub const MID_HIGH_XOVER: f32 = 2500.0;
/// The longest delay a voice can have, seconds: the floor plus SPREAD's
/// top.
const MAX_DELAY: f64 = FLOOR + 0.025;
/// Each band's flock has its own turbulence.
const SEEDS: [u32; BANDS] = [0x9E37_79B9, 0x85EB_CA6B];

/// A voice's delay across one tick, in samples, worked out in f64 so the
/// rate of change is the flock's velocity exactly, whatever the delay's
/// size. Each sample's value is rounded to f32 for the read, which jitters
/// it by at most half an f32 step but never drifts.
#[derive(Clone, Copy, Debug, Default)]
struct DelayRamp {
    from: f64,
    step: f64,
}

impl DelayRamp {
    fn new(from: f64, to: f64) -> Self {
        Self {
            from,
            step: (to - from) / TICK as f64,
        }
    }

    #[inline]
    fn at(&self, i: usize) -> f32 {
        (self.from + self.step * (i + 1) as f64) as f32
    }
}

/// One chorused band: its delay line and flock, and each voice's ramps.
#[derive(Debug)]
struct Band {
    line: DelayLine,
    flock: Flock,
    /// Each voice's level: 1/√VOICES in use, 0 otherwise.
    level: [Glide; MAX_VOICES],
    /// This tick's delay (samples) and left and right gains, per voice.
    delay: [DelayRamp; MAX_VOICES],
    left: [Ramp; MAX_VOICES],
    right: [Ramp; MAX_VOICES],
    /// Whether a voice is heard at all this tick.
    sounding: [bool; MAX_VOICES],
    depth: Glide,
    depth_ramp: Ramp,
    /// Each voice's allpass chain (`decorrelate`).
    chains: [Chain; MAX_VOICES],
    /// The wet gain after onsets (Duck, Bloom), and the side-buffer it is
    /// written into, read by each voice at its own delay.
    wet: WetGain,
    side: SideBuffer,
    /// AUTO WIDTH's share of WIDTH for this band, glided, and the width
    /// (WIDTH times it) the last tick used.
    auto: Glide,
    width: f32,
}

impl Band {
    fn new(
        band: usize,
        seed: u32,
        sample_rate: f32,
        spread: f32,
        voices: usize,
        max_lag: usize,
    ) -> Self {
        let tick_rate = sample_rate / TICK as f32;
        let mut level = [Glide::new(0.0, FADE_MS, tick_rate); MAX_VOICES];
        for g in level.iter_mut().take(voices) {
            *g = Glide::new(voice_level(voices), FADE_MS, tick_rate);
        }
        Self {
            line: DelayLine::new(max_delay(sample_rate)),
            flock: Flock::new(seed, f64::from(spread)),
            level,
            delay: [DelayRamp::default(); MAX_VOICES],
            left: [Ramp::default(); MAX_VOICES],
            right: [Ramp::default(); MAX_VOICES],
            sounding: [false; MAX_VOICES],
            depth: Glide::new(1.0, GLIDE_MS, tick_rate),
            depth_ramp: Ramp::new(1.0, 1.0, TICK),
            chains: std::array::from_fn(|voice| Chain::new(band, voice, sample_rate)),
            wet: WetGain::new(sample_rate),
            auto: Glide::new(1.0, AUTO_WIDTH_MS, sample_rate / TICK as f32),
            width: 0.0,
            side: {
                let mut s = SideBuffer::new(max_delay(sample_rate), max_lag);
                s.fill(1.0);
                s
            },
        }
    }
}

/// The dry input of one stretch (at most a tick), for the analysis.
struct Heard {
    side: [f32; TICK],
    bands: [[f32; BANDS]; TICK],
}

impl Default for Heard {
    fn default() -> Self {
        Self {
            side: [0.0; TICK],
            bands: [[0.0; BANDS]; TICK],
        }
    }
}

/// AUTO WIDTH's glide, ms: the image follows coherence slowly.
const AUTO_WIDTH_MS: f32 = 200.0;
/// AUTO WIDTH's share of WIDTH for a band with no correlation between its
/// channels (or worse): it is already wide.
const AUTO_WIDTH_FLOOR: f32 = 0.25;
/// The mono guard: no voice is panned further than this. A voice panned p
/// with equal power loses 10·log10((1 + cos(p·π/2))/2) dB when summed to
/// mono; at this pan, 2 dB (hard left or right, 3 dB).
pub const PAN_GUARD: f32 = 0.83;

/// AUTO WIDTH's share of WIDTH for a band whose input has `coherence`.
pub fn auto_width(coherence: f32) -> f32 {
    AUTO_WIDTH_FLOOR + (1.0 - AUTO_WIDTH_FLOOR) * coherence.clamp(0.0, 1.0)
}

/// The delay lines' length, samples.
fn max_delay(sample_rate: f32) -> usize {
    (MAX_DELAY * f64::from(sample_rate)).ceil() as usize + TICK
}

/// Each voice's level with `voices` in use.
fn voice_level(voices: usize) -> f32 {
    1.0 / (voices as f32).sqrt()
}

/// `pan` within the mono guard.
fn guarded(pan: f32) -> f32 {
    pan.clamp(-PAN_GUARD, PAN_GUARD)
}

/// Equal-power gains for `pan`, −1 to 1.
fn pan_gains(pan: f32) -> (f32, f32) {
    let theta = (pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (theta.cos(), theta.sin())
}

/// Charm. Build it with [`CharmEngine::new`] at the host's sample rate.
#[derive(Debug)]
pub struct CharmEngine {
    sample_rate: f32,
    low: Crossover,
    split: Crossover,
    bands: [Band; BANDS],
    voices: usize,
    /// Cents.
    detune: Glide,
    /// Seconds.
    spread: Glide,
    cohesion: Glide,
    separation: Glide,
    turbulence: Glide,
    /// 0 to 1.
    width: Glide,
    auto_width: bool,
    /// Hz.
    low_xover: Glide,
    low_xover_for: f32,
    /// 0 to 1.
    mix: Glide,
    mix_ramp: Ramp,
    into_tick: usize,
    /// Samples since reset.
    position: u64,
    /// 0 to 1, and samples.
    duck: f32,
    bloom: f32,
    /// Whether voices pass through their allpass chains (0028).
    decorrelate: bool,
    analysis: Analysis,
    obs: Observation,
}

fn default_of(index: ParamIndex) -> f32 {
    PARAMS[index.0 as usize].default
}

impl CharmEngine {
    /// Samples the output lags the input by, at any setting and rate:
    /// none.
    pub const LATENCY: u32 = 0;

    pub fn new(sample_rate: f32) -> Self {
        let tick_rate = sample_rate / TICK as f32;
        let glide = |v| Glide::new(v, GLIDE_MS, tick_rate);
        let voices = default_of(VOICES).round() as usize + MIN_VOICES;
        let spread = default_of(SPREAD) * 0.001;
        let low_xover = default_of(LOW_XOVER);
        let analysis = Analysis::new(sample_rate, low_xover);
        // An onset reaches the side-buffer at the next tick: up to a tick
        // after the analysis found it, which is up to its lag after it
        // starts.
        let max_lag = analysis.lag() + TICK + 1;
        // The analysis must have seen each sample before any voice reads
        // it: its lag, and a sample for the interpolator's reach.
        assert!(
            ((FLOOR * f64::from(sample_rate)) as usize) > max_lag + 1,
            "the voices' floor ({FLOOR} s) is inside the analysis's lag and a tick ({max_lag} samples) at {sample_rate} Hz"
        );
        assert!(
            max_lag < max_delay(sample_rate),
            "the side-buffer's lag ({max_lag}) must fit its delay line"
        );
        Self {
            sample_rate,
            low: Crossover::new(low_xover, sample_rate),
            split: Crossover::new(MID_HIGH_XOVER, sample_rate),
            bands: [
                Band::new(0, SEEDS[0], sample_rate, spread, voices, max_lag),
                Band::new(1, SEEDS[1], sample_rate, spread, voices, max_lag),
            ],
            voices,
            detune: glide(default_of(DETUNE)),
            spread: glide(spread),
            cohesion: glide(default_of(COHESION) * 0.01),
            separation: glide(default_of(SEPARATION) * 0.01),
            turbulence: glide(default_of(TURBULENCE) * 0.01),
            width: glide(default_of(WIDTH) * 0.01),
            auto_width: default_of(AUTO_WIDTH) >= 0.5,
            low_xover: glide(low_xover),
            low_xover_for: low_xover,
            mix: glide(default_of(MIX) * 0.01),
            mix_ramp: Ramp::default(),
            into_tick: TICK,
            position: 0,
            duck: default_of(DUCK) * 0.01,
            bloom: default_of(BLOOM) * 0.001 * sample_rate,
            decorrelate: true,
            analysis,
            obs: Observation::default(),
        }
    }

    /// Turn the voices' allpass decorrelation on or off (for measuring
    /// it: ticket 0028).
    pub fn set_decorrelation(&mut self, on: bool) {
        self.decorrelate = on;
    }

    /// What the engine is doing, as of the latest tick.
    pub fn observation(&self) -> &Observation {
        &self.obs
    }

    /// Start a control tick: glide the settings, step the flocks, and set
    /// up each voice's ramps across the tick.
    fn tick(&mut self) {
        let dt = TICK as f64 / f64::from(self.sample_rate);
        let settings = FlockSettings {
            voices: self.voices,
            max_velocity: max_velocity(f64::from(self.detune.tick())),
            spread: f64::from(self.spread.tick()),
            cohesion: f64::from(self.cohesion.tick()),
            separation: f64::from(self.separation.tick()),
            turbulence: f64::from(self.turbulence.tick()),
        };
        let width = self.width.tick();
        let xover = self.low_xover.tick();
        if xover != self.low_xover_for {
            self.low.set_frequency(xover);
            self.analysis.set_low_xover(xover);
            self.low_xover_for = xover;
        }
        let onsets = self.analysis.take(&mut self.obs.bands);
        let shape = Shape {
            duck: self.duck,
            bloom: self.bloom,
        };
        for (band, onset) in self.bands.iter_mut().zip(onsets) {
            band.wet.set_shape(shape);
            if let Some(o) = onset {
                // Restart the shape at the onset's start and rewrite what
                // has been pushed since: no voice has read that far back.
                let ago = self.position.saturating_sub(o.start);
                band.wet.onset(ago);
                let reach = ago.min(band.side.max_lag() as u64 + 1);
                for age in (0..reach).rev() {
                    let g = band.wet.at(ago - 1 - age);
                    band.side.set_past(age as usize, g);
                }
            }
        }
        let mix_from = self.mix.value();
        self.mix_ramp = Ramp::new(mix_from, self.mix.tick(), TICK);

        let level = voice_level(self.voices);
        let fs = f64::from(self.sample_rate);
        self.obs.tick += 1;
        for (band, obs) in self.bands.iter_mut().zip(&mut self.obs.bands) {
            band.auto.set_target(if self.auto_width {
                auto_width(obs.coherence)
            } else {
                1.0
            });
            let width_from = band.width;
            band.width = width * band.auto.tick();
            let width_to = band.width;
            let depth_from = band.depth.value();
            band.depth_ramp = Ramp::new(depth_from, band.depth.tick(), TICK);
            let before = *band.flock.voices();
            band.flock.step(&settings, dt);
            let after = band.flock.voices();
            obs.active = self.voices as u8;
            for i in 0..MAX_VOICES {
                let g_from = band.level[i].value();
                band.level[i].set_target(if i < self.voices { level } else { 0.0 });
                let g_to = band.level[i].tick();
                band.sounding[i] = g_from > 0.0 || g_to > 0.0;
                band.delay[i] = DelayRamp::new(before[i].delay * fs, after[i].delay * fs);
                let (l0, r0) = pan_gains(guarded(before[i].pan as f32 * width_from));
                let (l1, r1) = pan_gains(guarded(after[i].pan as f32 * width_to));
                band.left[i] = Ramp::new(l0 * g_from, l1 * g_to, TICK);
                band.right[i] = Ramp::new(r0 * g_from, r1 * g_to, TICK);
                let o = &mut obs.voices[i];
                o.delay = after[i].delay as f32;
                o.pan = guarded(after[i].pan as f32 * width_to);
                o.velocity = after[i].delay_velocity as f32;
                o.gain = g_to;
            }
        }
        self.into_tick = 0;
    }

    /// Push one sample of the mid signal into the bands and return the
    /// voices' left and right sum, `i` samples into the tick, and the
    /// sample's MID and HIGH parts.
    #[inline]
    fn voices_at(&mut self, i: usize, mid: f32) -> (f32, f32, [f32; BANDS]) {
        let (_, rest) = self.low.split(mid);
        let (m, h) = self.split.split(rest);
        let (mut wl, mut wr) = (0.0, 0.0);
        self.position += 1;
        for (band, x) in self.bands.iter_mut().zip([m, h]) {
            band.line.push(x);
            band.side.push(band.wet.step());
            let (mut bl, mut br) = (0.0, 0.0);
            for v in 0..MAX_VOICES {
                if !band.sounding[v] {
                    continue;
                }
                let delay = band.delay[v].at(i);
                let mut y = band.line.read(delay) * band.side.read(delay);
                if self.decorrelate {
                    y = band.chains[v].process(y);
                }
                bl += y * band.left[v].at(i);
                br += y * band.right[v].at(i);
            }
            let depth = band.depth_ramp.at(i);
            wl += bl * depth;
            wr += br * depth;
        }
        (wl, wr, [m, h])
    }

    /// Render from `offset` into the tick: one channel.
    fn run_one(&mut self, x: &mut [f32], offset: usize) {
        let mut heard = Heard::default();
        for (n, s) in x.iter_mut().enumerate() {
            let i = offset + n;
            let dry = *s;
            let (wl, wr, bands) = self.voices_at(i, dry);
            heard.bands[n] = bands;
            // Summed to one channel, equal-power pans would favour the
            // centre; each voice's left and right add to at most √2, so
            // scale the sum back to its stereo level.
            *s = dry + self.mix_ramp.at(i) * (wl + wr) * std::f32::consts::FRAC_1_SQRT_2;
        }
        let n = x.len();
        self.analysis.push(&heard.bands[..n], &heard.side[..n]);
    }

    /// Render from `offset` into the tick: two channels.
    fn run_two(&mut self, left: &mut [f32], right: &mut [f32], offset: usize) {
        let mut heard = Heard::default();
        for (n, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let i = offset + n;
            let (dl, dr) = (*l, *r);
            let mid = 0.5 * (dl + dr);
            let (wl, wr, bands) = self.voices_at(i, mid);
            heard.side[n] = 0.5 * (dl - dr);
            heard.bands[n] = bands;
            let mix = self.mix_ramp.at(i);
            *l = dl + mix * wl;
            *r = dr + mix * wr;
        }
        let n = left.len().min(right.len());
        self.analysis.push(&heard.bands[..n], &heard.side[..n]);
    }
}

impl Engine for CharmEngine {
    fn set_param(&mut self, index: ParamIndex, value: f32) {
        match index {
            VOICES => {
                self.voices = (value.round().max(0.0) as usize + MIN_VOICES).min(MAX_VOICES);
            }
            DETUNE => self.detune.set_target(value),
            SPREAD => self.spread.set_target(value * 0.001),
            COHESION => self.cohesion.set_target(value * 0.01),
            SEPARATION => self.separation.set_target(value * 0.01),
            TURBULENCE => self.turbulence.set_target(value * 0.01),
            WIDTH => self.width.set_target(value * 0.01),
            AUTO_WIDTH => self.auto_width = value >= 0.5,
            LOW_XOVER => self.low_xover.set_target(value),
            MID_DEPTH => self.bands[0].depth.set_target(value * 0.01),
            HIGH_DEPTH => self.bands[1].depth.set_target(value * 0.01),
            MIX => self.mix.set_target(value * 0.01),
            DUCK => self.duck = value * 0.01,
            BLOOM => self.bloom = value * 0.001 * self.sample_rate,
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
            &mut self.detune,
            &mut self.spread,
            &mut self.cohesion,
            &mut self.separation,
            &mut self.turbulence,
            &mut self.width,
            &mut self.low_xover,
            &mut self.mix,
        ] {
            glide.snap();
        }
        self.low.set_frequency(self.low_xover.value());
        self.low_xover_for = self.low_xover.value();
        self.low.reset();
        self.split.reset();
        self.analysis.set_low_xover(self.low_xover.value());
        self.analysis.restart();
        let spread = f64::from(self.spread.value());
        let level = voice_level(self.voices);
        for band in &mut self.bands {
            band.line.clear();
            band.side.fill(1.0);
            band.wet.reset();
            band.auto.set_target(1.0);
            band.auto.snap();
            band.width = self.width.value();
            for c in &mut band.chains {
                c.reset();
            }
            band.flock.reset(spread);
            band.depth.snap();
            for (i, g) in band.level.iter_mut().enumerate() {
                g.set_target(if i < self.voices { level } else { 0.0 });
                g.snap();
            }
        }
        self.obs = Observation::default();
        self.position = 0;
        self.into_tick = TICK;
    }
}
