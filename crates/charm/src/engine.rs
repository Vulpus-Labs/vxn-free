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
use vxn_free_dsp::{
    AllpassBank, Crossover, DelayLine, Glide, LANES, Ramp, SideBuffer, lagrange4_weights,
};

use crate::analysis::Analysis;
use crate::decorrelate::{STAGES, chain_filters};
use crate::flock::{FLOOR, Flock, FlockSettings, max_velocity};
use crate::observe::{BANDS, FRAME_WORDS, Observation};
use crate::wet::{Shape, WetGain};
use crate::{
    AUTO_WIDTH, BLOOM, COHESION, DECORRELATE, DETUNE, DUCK, HIGH_DEPTH, LOW_XOVER, MAX_VOICES,
    MID_DEPTH, MIN_VOICES, MIX, PARAMS, SEPARATION, SPREAD, TURBULENCE, VOICES, WIDTH,
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
const MAX_DELAY: f64 = FLOOR + 2.0;
/// Each band's flock has its own turbulence.
const SEEDS: [u32; BANDS] = [0x9E37_79B9, 0x85EB_CA6B];

/// Whether this tick's voices pass through their allpass chains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Allpass {
    Off,
    On,
    /// DECORRELATE was switched: the chains crossfade in or out.
    Fading,
}

/// Voices in groups of [`LANES`], run side by side.
const CHUNKS: usize = MAX_VOICES / LANES;
const _: () = assert!(MAX_VOICES.is_multiple_of(LANES), "voices fill whole lanes");

/// [`LANES`] voices' state for one tick, as structure-of-arrays so each
/// sample's sums run on every lane at once (SIMD).
///
/// A voice's delay across the tick is split into a whole number of
/// samples and a fraction, so each sample's delay, `whole + frac + step ×
/// (i + 1)`, is exact to well under a hundredth of an f32 step of the
/// whole delay: the rate of change is the flock's velocity, and it never
/// drifts, since each tick starts afresh from the flock's f64 delays.
#[derive(Clone, Copy, Debug)]
struct Lanes {
    whole: [f32; LANES],
    frac: [f32; LANES],
    step: [f32; LANES],
    /// Left and right gains, from and per sample.
    left: [f32; LANES],
    left_step: [f32; LANES],
    right: [f32; LANES],
    right_step: [f32; LANES],
    /// Each voice's allpass chain (`decorrelate`), a stage at a time.
    chain: [AllpassBank<LANES>; STAGES],
}

impl Lanes {
    fn new(band: usize, chunk: usize, sample_rate: f32) -> Self {
        let filters: [_; LANES] =
            std::array::from_fn(|l| chain_filters(band, chunk * LANES + l, sample_rate));
        Self {
            whole: [0.0; LANES],
            frac: [0.0; LANES],
            step: [0.0; LANES],
            left: [0.0; LANES],
            left_step: [0.0; LANES],
            right: [0.0; LANES],
            right_step: [0.0; LANES],
            chain: std::array::from_fn(|stage| {
                AllpassBank::new(&std::array::from_fn(|l| filters[l][stage]))
            }),
        }
    }

    /// Lane `l`'s delay ramp across the tick, from `from` to `to`
    /// samples.
    fn set_delay(&mut self, l: usize, from: f64, to: f64) {
        let whole = from.floor();
        self.whole[l] = whole as f32;
        self.frac[l] = (from - whole) as f32;
        self.step[l] = ((to - from) / TICK as f64) as f32;
    }

    /// Lane `l`'s gains across the tick.
    fn set_gains(&mut self, l: usize, left: Ramp, right: Ramp) {
        (self.left[l], self.left_step[l]) = (left.at(0) - left.step(), left.step());
        (self.right[l], self.right_step[l]) = (right.at(0) - right.step(), right.step());
    }

    /// Silence lane `l` for the tick.
    fn mute(&mut self, l: usize) {
        self.left[l] = 0.0;
        self.left_step[l] = 0.0;
        self.right[l] = 0.0;
        self.right_step[l] = 0.0;
    }
}

/// One chorused band: its delay line and flock, and each voice's ramps.
#[derive(Debug)]
struct Band {
    line: DelayLine,
    flock: Flock,
    /// Each voice's level: 1/√VOICES in use, 0 otherwise.
    level: [Glide; MAX_VOICES],
    /// This tick's delays, gains and allpass chains, [`LANES`] voices
    /// to a group.
    lanes: Box<[Lanes; CHUNKS]>,
    /// One past the highest voice sounding this tick: the per-sample loop
    /// stops at the group holding it, so unused voices cost nothing.
    reach: usize,
    depth: Glide,
    depth_ramp: Ramp,
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
            // Boxed: 64 voices' chains are too big for the stack.
            lanes: (0..CHUNKS)
                .map(|chunk| Lanes::new(band, chunk, sample_rate))
                .collect::<Box<[Lanes]>>()
                .try_into()
                .expect("CHUNKS groups"),
            reach: 0,
            depth: Glide::new(1.0, GLIDE_MS, tick_rate),
            depth_ramp: Ramp::new(1.0, 1.0, TICK),
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

/// The ensemble's make-up gain: each voice is equal-power panned, so each
/// channel carries half its power; this restores it, so MIX at 100 % puts
/// the ensemble level with the dry signal in each channel (measured within
/// 2 dB on noise and a chord: ticket 0024).
const WET_GAIN: f32 = std::f32::consts::SQRT_2;

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
    /// How much of each voice comes through its allpass chain (0028), 0
    /// to 1: DECORRELATE, crossfaded so switching it doesn't click.
    allpass: Glide,
    allpass_ramp: Ramp,
    allpass_mode: Allpass,
    analysis: Analysis,
    obs: Observation,
    /// The tick last published as a telemetry frame, and room to encode
    /// the next.
    published: Option<u64>,
    frame: [f32; FRAME_WORDS],
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
            allpass: glide(if default_of(DECORRELATE) >= 0.5 {
                1.0
            } else {
                0.0
            }),
            allpass_ramp: Ramp::default(),
            allpass_mode: Allpass::On,
            analysis,
            obs: Observation::default(),
            published: None,
            frame: [0.0; FRAME_WORDS],
        }
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
        let allpass_from = self.allpass.value();
        let allpass_to = self.allpass.tick();
        self.allpass_ramp = Ramp::new(allpass_from, allpass_to, TICK);
        self.allpass_mode = match (allpass_from, allpass_to) {
            (0.0, 0.0) => Allpass::Off,
            (1.0, 1.0) => Allpass::On,
            _ => Allpass::Fading,
        };

        let level = voice_level(self.voices);
        let fs = f64::from(self.sample_rate);
        self.obs.tick += 1;
        self.obs.spread = settings.spread as f32;
        self.obs.max_velocity = settings.max_velocity as f32;
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
            let mut reach = 0;
            obs.width = width_to;
            for i in 0..MAX_VOICES {
                let g_from = band.level[i].value();
                band.level[i].set_target(if i < self.voices { level } else { 0.0 });
                let g_to = band.level[i].tick();
                let sounding = g_from > 0.0 || g_to > 0.0;
                let o = &mut obs.voices[i];
                o.delay = after[i].delay as f32;
                o.pan = guarded(after[i].pan as f32 * width_to);
                o.velocity = after[i].delay_velocity as f32;
                o.gain = g_to;
                let lanes = &mut band.lanes[i / LANES];
                let l = i % LANES;
                lanes.set_delay(l, before[i].delay * fs, after[i].delay * fs);
                if !sounding {
                    // Silent and staying so: nothing to ramp or read.
                    lanes.mute(l);
                    o.shape = 1.0;
                    continue;
                }
                reach = i + 1;
                o.shape = band.side.read(o.delay * self.sample_rate);
                let (l0, r0) = pan_gains(guarded(before[i].pan as f32 * width_from));
                let (l1, r1) = pan_gains(guarded(after[i].pan as f32 * width_to));
                lanes.set_gains(
                    l,
                    Ramp::new(l0 * g_from, l1 * g_to, TICK),
                    Ramp::new(r0 * g_from, r1 * g_to, TICK),
                );
            }
            band.reach = reach;
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
        let (allpass, allpass_mix) = (self.allpass_mode, self.allpass_ramp.at(i));
        for (band, x) in self.bands.iter_mut().zip([m, h]) {
            band.line.push(x);
            band.side.push(band.wet.step());
            let at = (i + 1) as f32;
            let (mut bl, mut br) = ([0.0f32; LANES], [0.0f32; LANES]);
            for lanes in &mut band.lanes[..band.reach.div_ceil(LANES)] {
                let mut whole = [0usize; LANES];
                let mut frac = [0.0f32; LANES];
                for lane in 0..LANES {
                    let pos = lanes.frac[lane] + lanes.step[lane] * at;
                    let down = pos.floor();
                    whole[lane] = (lanes.whole[lane] + down) as usize;
                    frac[lane] = pos - down;
                }
                // One set of weights for the audio and its side-buffer.
                let weights = lagrange4_weights(&frac);
                let audio = band.line.gather(&whole, &weights);
                let shape = band.side.gather(&whole, &weights);
                let mut voice: [f32; LANES] = std::array::from_fn(|lane| audio[lane] * shape[lane]);
                if allpass != Allpass::Off {
                    let dry = voice;
                    for stage in &mut lanes.chain {
                        stage.process(&mut voice);
                    }
                    if allpass == Allpass::Fading {
                        for lane in 0..LANES {
                            voice[lane] = dry[lane] + allpass_mix * (voice[lane] - dry[lane]);
                        }
                    }
                }
                for lane in 0..LANES {
                    bl[lane] += voice[lane] * (lanes.left[lane] + lanes.left_step[lane] * at);
                    br[lane] += voice[lane] * (lanes.right[lane] + lanes.right_step[lane] * at);
                }
            }
            let (bl, br): (f32, f32) = (bl.iter().sum(), br.iter().sum());
            let depth = band.depth_ramp.at(i);
            wl += bl * depth;
            wr += br * depth;
        }
        (wl * WET_GAIN, wr * WET_GAIN, [m, h])
    }

    /// Render from `offset` into the tick: one channel.
    fn run_one(&mut self, x: &mut [f32], offset: usize) {
        let mut heard = Heard::default();
        for (n, s) in x.iter_mut().enumerate() {
            let i = offset + n;
            let dry = *s;
            let (wl, wr, bands) = self.voices_at(i, dry);
            heard.bands[n] = bands;
            // Summed to one channel: the mean of left and right, so a
            // centred voice is at the level it has in each channel in
            // stereo.
            *s = dry + self.mix_ramp.at(i) * 0.5 * (wl + wr);
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
            DECORRELATE => self
                .allpass
                .set_target(if value >= 0.5 { 1.0 } else { 0.0 }),
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

    /// The latest tick's observation, once per tick: the flock display
    /// reads the newest whole.
    fn frame(&mut self, write: &mut dyn FnMut(&[f32])) {
        if self.published != Some(self.obs.tick) {
            self.obs.to_frame(&mut self.frame);
            write(&self.frame);
            self.published = Some(self.obs.tick);
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
            &mut self.allpass,
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
            for lanes in band.lanes.iter_mut() {
                for stage in &mut lanes.chain {
                    stage.reset();
                }
            }
            band.flock.reset(spread);
            band.depth.snap();
            for (i, g) in band.level.iter_mut().enumerate() {
                g.set_target(if i < self.voices { level } else { 0.0 });
                g.snap();
            }
        }
        self.obs = Observation::default();
        self.published = None;
        self.position = 0;
        self.into_tick = TICK;
    }
}
