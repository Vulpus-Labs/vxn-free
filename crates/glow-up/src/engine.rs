//! Glow-Up's engine (ADR 0005). It sees only the bridge
//! (`silkscreen::bridge`) and `vxn-free-dsp`: it runs on the audio thread,
//! so it never allocates or locks.
//!
//! The analysis signal (the left input in MONO, the mid `(L + R) / 2` in
//! STEREO) feeds an STFT of [`WINDOW`] samples with a hop of a quarter of it,
//! and the peak detector. A frame completes on a hop boundary, which is
//! always a control-block boundary; the bank takes it at the start of the
//! next block, so the output is the same however the host splits its
//! blocks.
//!
//! The audio never passes through the analysis: each channel's output is
//! the input crossfaded by MIX (equal power) with the input plus the
//! filters' sum (or the sum alone, with DRY CUT), through a soft limit
//! that is transparent to ±1 and never passes ±2. The latency is zero.

use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use vxn_free_dsp::{Glide, PeakDetector, Ramp, Stft};

use crate::bank::{BLOCK, Bank};
use crate::observe::{FRAME_WORDS, Observation};
use crate::{
    A_AMOUNT, A_DEN, A_NUM, B_AMOUNT, B_DEN, B_NUM, DRY_CUT, FOCUS, GRID, GRID_LIN, GRID_TET, MIX,
    MODE, PARAMS, RESONANCE, SPEED, THRESHOLD, TONE, WIDTH, WINDOW, grids,
};

/// Frames overlap this many times.
const OVERLAP: usize = 4;
/// RESONANCE's top, as the bank reads it.
const RESONANCE_MOST: f64 = 2.0;
/// MIX's glide, ms.
const MIX_GLIDE_MS: f32 = 10.0;

/// Glow-Up. Build it with [`GlowUpEngine::new`] at the host's sample rate.
#[derive(Debug)]
pub struct GlowUpEngine {
    rate: f32,
    values: [f32; PARAMS.len()],
    stft: Stft,
    detector: PeakDetector,
    bank: Bank,
    linear: Box<[f64]>,
    irregular: Box<[f64]>,
    tempered: Box<[f64]>,
    /// A frame completed and the bank hasn't taken it.
    frame_ready: bool,
    mix: Glide,
    mix_ramp: Ramp,
    dry_cut: bool,
    stereo: bool,
    /// Samples of the current block already rendered.
    into_block: usize,
    /// Samples since the last reset, for the display's time axis.
    time: u64,
    /// What the display is sent, and room to encode it.
    obs: Observation,
    frame: [f32; FRAME_WORDS],
}

impl GlowUpEngine {
    /// Samples the output lags the input by, at any setting and rate:
    /// none.
    pub const LATENCY: u32 = 0;

    pub fn new(sample_rate: f32) -> Self {
        let rate = f64::from(sample_rate);
        let linear = grids::linear(rate, WINDOW).into_boxed_slice();
        let irregular = grids::irregular().into_boxed_slice();
        let tempered = grids::equal_tempered().into_boxed_slice();
        let max_grid = linear.len().max(irregular.len()).max(tempered.len());
        let mut e = Self {
            rate: sample_rate,
            values: PARAMS.each_ref().map(|p| p.default),
            stft: Stft::new(WINDOW, OVERLAP, WINDOW, WINDOW / OVERLAP),
            detector: PeakDetector::new(WINDOW),
            bank: Bank::new(rate, WINDOW / 4, max_grid),
            linear,
            irregular,
            tempered,
            frame_ready: false,
            mix: Glide::new(1.0, MIX_GLIDE_MS, sample_rate / BLOCK as f32),
            mix_ramp: Ramp::default(),
            dry_cut: false,
            stereo: false,
            into_block: BLOCK,
            time: 0,
            obs: Observation::default(),
            frame: [0.0; FRAME_WORDS],
        };
        e.bank.set_hop(WINDOW / OVERLAP);
        e.controls();
        e.mix.snap();
        e
    }

    /// The bank, for tests and the display.
    pub fn bank(&self) -> &Bank {
        &self.bank
    }

    fn value(&self, p: ParamIndex) -> f32 {
        self.values[p.0 as usize]
    }

    fn step(&self, p: ParamIndex) -> usize {
        self.value(p).round().max(0.0) as usize
    }

    /// A boost's ratio from its NUM and DEN positions.
    fn ratio(&self, num: ParamIndex, den: ParamIndex) -> f64 {
        (self.step(num) + 1) as f64 / (self.step(den) + 1) as f64
    }

    /// Read every control, once a block.
    fn controls(&mut self) {
        self.detector.set_threshold_db(self.value(THRESHOLD));
        let s = &mut self.bank.settings;
        s.amount = [
            f64::from(self.values[A_AMOUNT.0 as usize]) * 0.01,
            f64::from(self.values[B_AMOUNT.0 as usize]) * 0.01,
        ];
        let ratios = [self.ratio(A_NUM, A_DEN), self.ratio(B_NUM, B_DEN)];
        let s = &mut self.bank.settings;
        s.ratio = ratios;
        s.resonance = RESONANCE_MOST * f64::from(self.values[RESONANCE.0 as usize]) * 0.01;
        s.focus = (f64::from(self.values[FOCUS.0 as usize]) * 0.01).clamp(0.0, 1.0);
        s.tone_db_per_oct = f64::from(self.values[TONE.0 as usize]);
        self.bank.set_width_cents(f64::from(self.value(WIDTH)));
        self.bank
            .set_speed((f64::from(self.value(SPEED)) * 0.01).clamp(0.0, 1.0));
        self.dry_cut = self.value(DRY_CUT) >= 0.5;
        self.stereo = self.step(MODE) == 1;
        self.mix
            .set_target((self.value(MIX) * 0.01).clamp(0.0, 1.0));
    }

    /// Start a control block.
    fn block(&mut self) {
        self.controls();
        if self.frame_ready {
            self.frame_ready = false;
            let grid: &[f64] = match self.step(GRID) {
                GRID_LIN => &self.linear,
                GRID_TET => &self.tempered,
                _ => &self.irregular,
            };
            self.bank.frame(self.detector.peaks(), grid);
        }
        let from = self.mix.value();
        self.mix_ramp = Ramp::new(from, self.mix.tick(), BLOCK);
        self.into_block = 0;
    }

    /// Render `n` samples of `left` (and `right`, in STEREO) from `offset`
    /// into the block.
    fn run(&mut self, left: &mut [f32], mut right: Option<&mut [f32]>, offset: usize, n: usize) {
        let stereo = self.stereo && right.is_some();
        let channels = if stereo { 2 } else { 1 };
        // The analysis signal, a block at most.
        let mut mid = [0.0f32; BLOCK];
        for j in 0..n {
            let l = left[j];
            mid[j] = match (&right, stereo) {
                (Some(r), true) => 0.5 * (l + r[j]),
                _ => l,
            };
        }
        for j in 0..n {
            let r = match (&right, stereo) {
                (Some(r), true) => f64::from(r[j]),
                _ => 0.0,
            };
            let x = [f64::from(left[j]), r];
            let sum = self.bank.tick(f64::from(mid[j]), x, channels);
            let mix = f64::from(self.mix_ramp.at(offset + j));
            let (dry, wet) = equal_power(mix);
            let out = |c: usize| {
                let wet_signal = if self.dry_cut { sum[c] } else { x[c] + sum[c] };
                limit(dry * x[c] + wet * wet_signal) as f32
            };
            left[j] = out(0);
            if let Some(r) = right.as_deref_mut() {
                r[j] = if stereo { out(1) } else { left[j] };
            }
        }
        let detector = &mut self.detector;
        let rate = self.rate;
        let mut ready = false;
        self.stft.push(&mid[..n], |f| {
            detector.detect(&f, rate);
            ready = true;
        });
        self.frame_ready |= ready;
    }
}

/// MIX's dry and wet gains, equal power, exact at the ends.
fn equal_power(mix: f64) -> (f64, f64) {
    if mix >= 1.0 {
        (0.0, 1.0)
    } else if mix <= 0.0 {
        (1.0, 0.0)
    } else {
        let a = 0.5 * std::f64::consts::PI * mix;
        (a.cos(), a.sin())
    }
}

/// Transparent to ±1, never past ±2, never folding.
pub fn limit(x: f64) -> f64 {
    let a = x.abs();
    if a <= 1.0 {
        x
    } else {
        (1.0 + (a - 1.0).tanh()).copysign(x)
    }
}

impl Engine for GlowUpEngine {
    fn set_param(&mut self, index: ParamIndex, value: f32) {
        if let Some(v) = self.values.get_mut(index.0 as usize) {
            *v = value;
        }
    }

    fn process(&mut self, channels: &mut [&mut [f32]]) {
        let frames = channels.iter().map(|c| c.len()).min().unwrap_or(0);
        let mut start = 0;
        while start < frames {
            if self.into_block == BLOCK {
                self.block();
            }
            let n = (BLOCK - self.into_block).min(frames - start);
            let offset = self.into_block;
            match channels {
                [] => {}
                [one] => self.run(&mut one[start..start + n], None, offset, n),
                [l, r, ..] => self.run(
                    &mut l[start..start + n],
                    Some(&mut r[start..start + n]),
                    offset,
                    n,
                ),
            }
            self.into_block += n;
            start += n;
        }
        self.time += frames as u64;
    }

    /// The display's frame: the input's spectrum, the latest analysis
    /// frame's peaks and every filter, as of the end of the block.
    fn frame(&mut self, write: &mut dyn FnMut(&[f32])) {
        let (dry, wet) = equal_power(f64::from(self.mix.value()));
        self.obs.observe(
            self.time,
            self.rate,
            &self.bank,
            &self.detector,
            (dry, wet, self.dry_cut),
        );
        self.obs.to_frame(&mut self.frame);
        write(&self.frame);
    }

    fn reset(&mut self) {
        self.controls();
        self.mix.snap();
        self.bank.clear();
        self.stft.restart();
        self.detector.restart();
        self.frame_ready = false;
        self.into_block = BLOCK;
        self.time = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_is_transparent_to_one_and_bounded_by_two() {
        for x in [-1.0, -0.3, 0.0, 0.7, 1.0] {
            assert_eq!(limit(x), x);
        }
        for x in [1.5f64, 4.0, 100.0] {
            let y = limit(x);
            assert!(y > 1.0 && y <= 2.0 && limit(-x) == -y, "{x}: {y}");
        }
        assert!(limit(1.2) < limit(1.3));
    }
}
