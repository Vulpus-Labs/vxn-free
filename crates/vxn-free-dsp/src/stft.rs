//! A streaming short-time Fourier transform that never copies its input.
//!
//! Frames are `window` samples long and start every `hop` samples, so
//! `O = window / hop` of them overlap at any moment. The STFT keeps `O + 1`
//! frame buffers: `O` filling, and one holding the latest spectrum.
//!
//! - Each incoming sample is written, multiplied by the window, straight
//!   into every filling buffer it belongs to. Write and window are one
//!   pass; there is no ring buffer and no copy out of one.
//! - Frame `m` covers stream samples `m·hop − window .. m·hop` and lives in
//!   buffer `m mod (O + 1)`. When it fills, at stream position `m·hop`, it
//!   is transformed in place and becomes the latest spectrum. The frame
//!   that starts then takes the buffer of frame `m − 1`, whose spectrum has
//!   just been superseded, so nothing is written to a buffer that is still
//!   being read.
//! - Frame timing depends only on the stream position, so splitting the
//!   input into blocks differently gives the same frames, bit for bit.
//! - At stream start the history is zeros: early frames are partly
//!   silent, as if the stream had been running.
//!
//! Spectra are in [`RealPackedFft`]'s packed layout. Everything is
//! allocated by [`Stft::new`] for the largest window and overlap it will be
//! given; [`Stft::configure`] then changes window and hop without
//! allocating, restarting the analysis from silence.

use std::f32::consts::TAU;

use crate::fft::RealPackedFft;

/// The smallest window.
pub const MIN_WINDOW: usize = 128;

/// One completed frame.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// Frames completed before this one since the last restart.
    pub index: u64,
    /// The stream position (samples since the last restart) at which it
    /// completed: it covers `end − window .. end`.
    pub end: u64,
    /// The spectrum, packed: `[0]` DC, `[1]` Nyquist, then `re, im` for
    /// bins 1 to `window/2 − 1`.
    pub bins: &'a [f32],
}

impl Frame<'_> {
    /// Bins from DC to Nyquist.
    pub fn len(&self) -> usize {
        self.bins.len() / 2 + 1
    }

    /// Whether it has no bins (never).
    pub fn is_empty(&self) -> bool {
        self.bins.is_empty()
    }

    /// Power (|X|²) of bin `k`, 0 to `window/2`.
    #[inline]
    pub fn power(&self, k: usize) -> f32 {
        let half = self.bins.len() / 2;
        if k == 0 {
            self.bins[0] * self.bins[0]
        } else if k == half {
            self.bins[1] * self.bins[1]
        } else {
            let (re, im) = (self.bins[2 * k], self.bins[2 * k + 1]);
            re * re + im * im
        }
    }
}

/// A streaming STFT. See the module docs.
pub struct Stft {
    /// A transform per window size, `MIN_WINDOW` up to the largest.
    ffts: Box<[RealPackedFft]>,
    /// Hann windows per size, concatenated: size `MIN_WINDOW << i` starts
    /// at `MIN_WINDOW · (2^i − 1)`.
    windows: Box<[f32]>,
    /// Frame buffers, `max_frames` of the largest window.
    store: Box<[f32]>,
    max_window: usize,
    max_frames: usize,
    window: usize,
    hop: usize,
    /// Samples since the last restart.
    pos: u64,
    /// Frames completed since the last restart.
    done: u64,
}

impl std::fmt::Debug for Stft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stft")
            .field("window", &self.window)
            .field("hop", &self.hop)
            .field("pos", &self.pos)
            .field("done", &self.done)
            .finish_non_exhaustive()
    }
}

impl Stft {
    /// Room for windows up to `max_window` (a power of two, at least
    /// [`MIN_WINDOW`]) and overlaps up to `max_overlap`, configured to
    /// `window` and `hop`. Allocates; call it off the audio thread.
    pub fn new(max_window: usize, max_overlap: usize, window: usize, hop: usize) -> Self {
        assert!(
            max_window.is_power_of_two() && max_window >= MIN_WINDOW,
            "an STFT's largest window must be a power of two of at least {MIN_WINDOW}, not {max_window}"
        );
        let sizes = (max_window / MIN_WINDOW).trailing_zeros() as usize + 1;
        let ffts: Vec<RealPackedFft> = (0..sizes)
            .map(|i| RealPackedFft::new(MIN_WINDOW << i))
            .collect();
        let mut windows = Vec::with_capacity(2 * max_window);
        for i in 0..sizes {
            let n = MIN_WINDOW << i;
            // Periodic Hann: overlapped at a quarter-window hop, it sums
            // to a constant.
            windows.extend((0..n).map(|k| 0.5 - 0.5 * (TAU * k as f32 / n as f32).cos()));
        }
        let max_frames = max_overlap + 1;
        let mut stft = Self {
            ffts: ffts.into_boxed_slice(),
            windows: windows.into_boxed_slice(),
            store: vec![0.0; max_frames * max_window].into_boxed_slice(),
            max_window,
            max_frames,
            window: max_window,
            hop: max_window,
            pos: 0,
            done: 0,
        };
        stft.configure(window, hop);
        stft
    }

    /// Use frames of `window` samples every `hop` samples, and restart
    /// from silence. `window` must be a power of two from [`MIN_WINDOW`] to
    /// the largest given to [`new`](Self::new); `hop` must divide it, with
    /// `window / hop` no more than the largest overlap. Never allocates.
    pub fn configure(&mut self, window: usize, hop: usize) {
        assert!(
            window.is_power_of_two() && (MIN_WINDOW..=self.max_window).contains(&window),
            "window {window} must be a power of two from {MIN_WINDOW} to {}",
            self.max_window
        );
        assert!(
            hop > 0 && window.is_multiple_of(hop) && window / hop < self.max_frames,
            "hop {hop} must divide window {window}, at most {} times",
            self.max_frames - 1
        );
        self.window = window;
        self.hop = hop;
        self.restart();
    }

    /// Restart from silence, keeping window and hop.
    pub fn restart(&mut self) {
        self.store.fill(0.0);
        self.pos = 0;
        self.done = 0;
    }

    /// Samples per frame.
    pub fn window(&self) -> usize {
        self.window
    }

    /// Samples between frames.
    pub fn hop(&self) -> usize {
        self.hop
    }

    /// Samples since the last restart.
    pub fn position(&self) -> u64 {
        self.pos
    }

    fn frames(&self) -> usize {
        self.window / self.hop + 1
    }

    fn size_index(&self) -> usize {
        (self.window / MIN_WINDOW).trailing_zeros() as usize
    }

    fn buffer(&self, frame: u64) -> std::ops::Range<usize> {
        let b = (frame % self.frames() as u64) as usize;
        b * self.window..(b + 1) * self.window
    }

    /// The latest completed frame, if any.
    pub fn latest(&self) -> Option<Frame<'_>> {
        let index = self.done.checked_sub(1)?;
        // Frame numbers count from 1 (frame m ends at m·hop).
        let m = index + 1;
        Some(Frame {
            index,
            end: m * self.hop as u64,
            bins: &self.store[self.buffer(m)],
        })
    }

    /// Feed `input`, calling `on_frame` for each frame completed within it,
    /// in order. A frame's `end` minus the position before this call is
    /// where in `input` it completed.
    pub fn push(&mut self, input: &[f32], mut on_frame: impl FnMut(Frame<'_>)) {
        let (window, hop) = (self.window as u64, self.hop as u64);
        let overlap = self.window / self.hop;
        let wstart = MIN_WINDOW * ((1 << self.size_index()) - 1);
        let mut at = 0;
        while at < input.len() {
            let next_boundary = (self.pos / hop + 1) * hop;
            let n = ((next_boundary - self.pos) as usize).min(input.len() - at);
            let seg = &input[at..at + n];
            // Every frame whose window holds the current samples: frames
            // m with m·hop in (pos, pos + window].
            let first = self.pos / hop + 1;
            for m in first..first + overlap as u64 {
                // Where this segment starts within frame m's window.
                let offset = (self.pos + window - m * hop) as usize;
                let buf = self.buffer(m);
                let dst = &mut self.store[buf.start + offset..buf.start + offset + n];
                let win = &self.windows[wstart + offset..wstart + offset + n];
                for ((d, &s), &c) in dst.iter_mut().zip(seg).zip(win) {
                    *d = s * c;
                }
            }
            self.pos += n as u64;
            at += n;
            if self.pos == next_boundary {
                let m = self.pos / hop;
                let buf = self.buffer(m);
                self.ffts[self.size_index()].forward(&mut self.store[buf.clone()]);
                on_frame(Frame {
                    index: self.done,
                    end: self.pos,
                    bins: &self.store[buf],
                });
                self.done += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(len: usize, mut seed: u32) -> Vec<f32> {
        (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
            })
            .collect()
    }

    /// Every frame's (end, bins) for `x` fed in blocks cycling `sizes`.
    fn frames(x: &[f32], window: usize, hop: usize, sizes: &[usize]) -> Vec<(u64, Vec<u32>)> {
        let mut stft = Stft::new(2048, 8, window, hop);
        let mut out = Vec::new();
        let mut at = 0;
        let mut k = 0;
        while at < x.len() {
            let n = sizes[k % sizes.len()].min(x.len() - at);
            stft.push(&x[at..at + n], |f| {
                out.push((f.end, f.bins.iter().map(|v| v.to_bits()).collect()));
            });
            at += n;
            k += 1;
        }
        out
    }

    #[test]
    fn frames_are_the_same_however_the_input_is_split() {
        let x = noise(20_000, 3);
        for (window, hop) in [(256, 64), (128, 128), (2048, 512), (512, 64)] {
            let whole = frames(&x, window, hop, &[20_000]);
            assert_eq!(whole.len(), 20_000 / hop);
            let mut rng = 17u32;
            let random: Vec<usize> = (0..200)
                .map(|_| {
                    rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    1 + (rng >> 22) as usize
                })
                .collect();
            for sizes in [&[1][..], &[7], &[64], &[512], &random] {
                assert_eq!(frames(&x, window, hop, sizes), whole, "{window}/{hop}");
            }
        }
    }

    #[test]
    fn frame_timing_follows_the_stream_from_the_first_frame() {
        // An impulse at sample t is in frames m with m·hop − window ≤ t <
        // m·hop, and in no others.
        let (window, hop) = (256usize, 64usize);
        let t = 300usize;
        let mut x = vec![0.0f32; 2000];
        x[t] = 1.0;
        let mut stft = Stft::new(256, 4, window, hop);
        let mut seen = Vec::new();
        stft.push(&x, |f| {
            let energy: f32 = (0..f.len()).map(|k| f.power(k)).sum();
            seen.push((f.end, energy > 1.0e-12));
        });
        for (end, lit) in seen {
            let start = end as i64 - window as i64;
            let t = t as i64;
            let holds = end as i64 > t && start <= t;
            // Hann is zero at its first sample: the frame starting on the
            // impulse doesn't see it.
            let at_edge = start == t;
            assert_eq!(lit, holds && !at_edge, "frame ending at {end}");
        }
        assert_eq!(stft.latest().map(|f| f.end), Some(2000 / 64 * 64));
    }

    #[test]
    fn early_frames_see_zeros_before_the_stream() {
        // The first frame (ending at hop) holds hop samples of a DC input
        // under the last hop of the window, zeros before.
        let (window, hop) = (256usize, 64usize);
        let mut stft = Stft::new(256, 4, window, hop);
        let mut dc = Vec::new();
        stft.push(&vec![1.0; 64], |f| dc.push(f.bins[0]));
        let w: f32 = (192..256)
            .map(|k| 0.5 - 0.5 * (TAU * k as f32 / 256.0).cos())
            .sum();
        assert!((dc[0] - w).abs() < 1.0e-4, "{} vs {w}", dc[0]);
    }

    #[test]
    fn reconfiguring_restarts_from_silence() {
        let x = noise(4096, 5);
        let mut stft = Stft::new(1024, 4, 256, 64);
        stft.push(&x, |_| {});
        stft.configure(512, 128);
        assert_eq!(stft.position(), 0);
        assert!(stft.latest().is_none());
        let mut a = Vec::new();
        stft.push(&x, |f| a.push(f.bins.to_vec()));
        let mut fresh = Stft::new(1024, 4, 512, 128);
        let mut b = Vec::new();
        fresh.push(&x, |f| b.push(f.bins.to_vec()));
        assert_eq!(a, b);
    }
}
