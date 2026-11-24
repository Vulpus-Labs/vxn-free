//! Delay lines read at fractional, moving delays.
//!
//! A chorus voice reads its delay line at a delay that changes every
//! sample; the rate of change is its pitch shift (a delay growing by d′
//! samples per sample plays at ratio 1 − d′). Reads interpolate between
//! samples with a 4-point, third-order Lagrange interpolator. Ticket 0026
//! measured it against linear, cubic Hermite and first-order allpass
//! interpolation: it has the least error of the non-recursive ones, and a
//! recursive (allpass) read can't serve the [`SideBuffer`], which must read
//! exactly as the audio does.

/// A power-of-two ring of samples, read at fractional delays.
#[derive(Clone, Debug)]
pub struct DelayLine {
    buf: Box<[f32]>,
    mask: usize,
    /// Where the latest sample is.
    write: usize,
}

impl DelayLine {
    /// Room for delays up to at least `max_delay` samples. Allocates; call
    /// it off the audio thread.
    pub fn new(max_delay: usize) -> Self {
        // Three more for the interpolator's reach either side.
        let len = (max_delay + 4).next_power_of_two();
        Self {
            buf: vec![0.0; len].into_boxed_slice(),
            mask: len - 1,
            write: 0,
        }
    }

    /// The longest delay [`read`](Self::read) honours.
    pub fn max_delay(&self) -> f32 {
        (self.buf.len() - 3) as f32
    }

    /// The shortest delay [`read`](Self::read) honours: the interpolator
    /// needs a sample either side of the one it starts from.
    pub const MIN_DELAY: f32 = 1.0;

    /// Silence the line.
    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }

    /// Append the next sample. Delay 0 is now this sample.
    #[inline]
    pub fn push(&mut self, x: f32) {
        self.write = (self.write + 1) & self.mask;
        self.buf[self.write] = x;
    }

    /// The sample `age` samples ago; 0 is the latest.
    #[inline]
    fn at(&self, age: usize) -> f32 {
        self.buf[self.write.wrapping_sub(age) & self.mask]
    }

    /// Overwrite the sample `age` samples ago; 0 is the latest.
    #[inline]
    fn set_at(&mut self, age: usize, value: f32) {
        self.buf[self.write.wrapping_sub(age) & self.mask] = value;
    }

    /// The signal `delay` samples ago, interpolated, with `delay` clamped
    /// to [`MIN_DELAY`](Self::MIN_DELAY)..=[`max_delay`](Self::max_delay).
    #[inline]
    pub fn read(&self, delay: f32) -> f32 {
        let d = delay.clamp(Self::MIN_DELAY, self.max_delay());
        let whole = d.floor();
        let i = whole as usize;
        lagrange4(
            self.at(i - 1),
            self.at(i),
            self.at(i + 1),
            self.at(i + 2),
            d - whole,
        )
    }
}

/// Third-order Lagrange through `y0..y3` at ages i − 1, i, i + 1, i + 2,
/// evaluated `f` (0..1) of the way from `y1` to `y2`.
#[inline]
fn lagrange4(y0: f32, y1: f32, y2: f32, y3: f32, f: f32) -> f32 {
    let fp1 = f + 1.0;
    let fm1 = f - 1.0;
    let fm2 = f - 2.0;
    let a = fp1 * f;
    let b = fm1 * fm2;
    (-f * b * y0 + a * fm1 * y3) * (1.0 / 6.0) + (fp1 * b * y1 - a * fm2 * y2) * 0.5
}

/// Values that run alongside a [`DelayLine`], one per audio sample, read at
/// the same delays with the same interpolation, and that may be written
/// after the fact.
///
/// An analysis that runs a little behind the audio writes its verdict (a
/// gain, say) back at the positions of the samples it judged. A read at a
/// delay of at least `max_lag + 1` touches only values that are final
/// (the interpolator reaches one sample newer than its delay), so a voice
/// that reads that far back meets each verdict before the sample it
/// belongs to. Keep pushing in step with the audio line.
#[derive(Clone, Debug)]
pub struct SideBuffer {
    line: DelayLine,
    max_lag: usize,
}

impl SideBuffer {
    /// Room for the same delays as `DelayLine::new(max_delay)`, with
    /// writes allowed up to `max_lag` samples back. Allocates.
    pub fn new(max_delay: usize, max_lag: usize) -> Self {
        let line = DelayLine::new(max_delay);
        assert!(
            (max_lag as f32) < line.max_delay(),
            "a side-buffer's lag ({max_lag}) must be shorter than its delay line"
        );
        Self { line, max_lag }
    }

    /// The furthest back [`set_past`](Self::set_past) may write.
    pub fn max_lag(&self) -> usize {
        self.max_lag
    }

    /// Fill with `value`.
    pub fn fill(&mut self, value: f32) {
        self.line.buf.fill(value);
    }

    /// Append the value for the audio sample just pushed.
    #[inline]
    pub fn push(&mut self, value: f32) {
        self.line.push(value);
    }

    /// Replace the value `age` samples ago (0 is the latest), for
    /// `age <= max_lag`. Ages past the lag are clamped to it.
    #[inline]
    pub fn set_past(&mut self, age: usize, value: f32) {
        self.line.set_at(age.min(self.max_lag), value);
    }

    /// The value `delay` samples ago, interpolated exactly as
    /// [`DelayLine::read`] interpolates audio.
    #[inline]
    pub fn read(&self, delay: f32) -> f32 {
        self.line.read(delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    /// Least-squares amplitude and phase of a sinusoid at `w` (radians per
    /// sample) in `y`, whose first sample is at time `n0`.
    fn fit(samples: &[f32], omega: f64, n0: usize) -> (f64, f64) {
        let (mut cc, mut ss, mut cs, mut yc, mut ys) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (k, &v) in samples.iter().enumerate() {
            let t = omega * (n0 + k) as f64;
            let (s, c) = t.sin_cos();
            cc += c * c;
            ss += s * s;
            cs += c * s;
            yc += f64::from(v) * c;
            ys += f64::from(v) * s;
        }
        let det = cc * ss - cs * cs;
        let a = (yc * ss - ys * cs) / det; // cos weight
        let b = (ys * cc - yc * cs) / det; // sin weight
        // y ≈ A sin(t + φ) = A cos φ sin t + A sin φ cos t.
        ((a * a + b * b).sqrt(), a.atan2(b))
    }

    /// Phase (radians) of a sine at `hz` delayed by `delay` samples.
    fn delayed_phase(hz: f64, fs: f64, delay: f32) -> (f64, f64) {
        let w = TAU * hz / fs;
        let mut line = DelayLine::new(512);
        let mut out = Vec::new();
        for n in 0..20_000 {
            line.push((w * f64::from(n)).sin() as f32);
            if n >= 1000 {
                out.push(line.read(delay));
            }
        }
        fit(&out, w, 1000)
    }

    #[test]
    fn a_static_fractional_delay_has_its_phase_and_group_delay() {
        let fs = 48_000.0;
        for delay in [10.0f32, 100.3, 100.5, 257.81] {
            let (amp, phase) = delayed_phase(1000.0, fs, delay);
            let w = TAU * 1000.0 / fs;
            // The phase lag, unwrapped near the expected one.
            let expect = -w * f64::from(delay);
            let k = ((expect - phase) / TAU).round();
            let phase_delay = -(phase + k * TAU) / w;
            assert!(
                (phase_delay - f64::from(delay)).abs() < 0.002,
                "{delay}: {phase_delay}"
            );
            assert!((20.0 * amp.log10()).abs() < 0.01, "{delay}: {amp}");

            let (_, p2) = delayed_phase(1010.0, fs, delay);
            let w2 = TAU * 1010.0 / fs;
            let mut dp = p2 - phase;
            dp -= TAU * ((dp + (w2 - w) * f64::from(delay)) / TAU).round();
            let group = -dp / (w2 - w);
            assert!(
                (group - f64::from(delay)).abs() < 0.01,
                "{delay}: group {group}"
            );
        }
    }

    /// Frequency of `y` from its rising zero crossings.
    fn frequency(y: &[f32], fs: f64) -> f64 {
        let mut crossings = Vec::new();
        for n in 1..y.len() {
            if y[n - 1] < 0.0 && y[n] >= 0.0 {
                let t = f64::from(y[n - 1]) / f64::from(y[n - 1] - y[n]);
                crossings.push((n - 1) as f64 + t);
            }
        }
        let first = crossings[0];
        let last = *crossings.last().unwrap();
        (crossings.len() - 1) as f64 / (last - first) * fs
    }

    #[test]
    fn a_moving_delay_shifts_pitch_by_its_rate_of_change() {
        let fs = 48_000.0;
        let f0 = 1000.0;
        for cents in [30.0f64, 5.0, -30.0] {
            // Ratio 1 − d′ = 2^(−cents/1200): the delay grows to go down.
            let dprime = 1.0 - 2f64.powf(-cents / 1200.0);
            let w = TAU * f0 / fs;
            let mut line = DelayLine::new(4096);
            let mut d = 2000.0f64;
            let mut out = Vec::new();
            for n in 0..48_000 {
                line.push((w * f64::from(n)).sin() as f32);
                if n >= 2100 {
                    out.push(line.read(d as f32));
                    d += dprime;
                }
            }
            let got = frequency(&out, fs);
            let err = 1200.0 * (got / (f0 * (1.0 - dprime))).log2();
            assert!(err.abs() < 1.0, "{cents} cents: off by {err} cents");
        }
    }

    #[test]
    fn integer_delays_are_exact() {
        let mut line = DelayLine::new(64);
        for n in 0..100 {
            line.push(n as f32);
        }
        assert_eq!(line.read(1.0), 98.0);
        assert_eq!(line.read(10.0), 89.0);
    }

    #[test]
    fn delays_out_of_range_are_clamped() {
        let mut line = DelayLine::new(64);
        for n in 0..200 {
            line.push(n as f32);
        }
        assert_eq!(line.read(0.0), line.read(DelayLine::MIN_DELAY));
        assert_eq!(line.read(1.0e6), line.read(line.max_delay()));
    }

    #[test]
    fn a_side_buffer_written_late_reads_with_its_audio() {
        let lag = 300;
        let mut audio = DelayLine::new(2048);
        let mut side = SideBuffer::new(2048, lag);
        let mut held = std::collections::VecDeque::new();
        let mut seed = 1u32;
        for n in 0..20_000 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let x = (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
            audio.push(x);
            // The side-buffer learns each value `lag` samples late.
            side.push(0.0);
            held.push_back(x);
            if held.len() > lag {
                side.set_past(lag, held.pop_front().unwrap());
            }
            if n > 2000 {
                for d in [lag as f32 + 1.0, lag as f32 + 1.5, 400.25, 1000.0, 1500.9] {
                    assert_eq!(audio.read(d).to_bits(), side.read(d).to_bits(), "{n} {d}");
                }
            }
        }
    }
}
