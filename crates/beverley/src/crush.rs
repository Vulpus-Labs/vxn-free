//! The crush chain, one sample at a time.
//!
//! Each sample goes through: gain (fixed, or the inverse of the input's
//! peak envelope), a hard clip of its magnitude to 1, a power-law curve
//! (gamma), quantisation, the inverse curve and the inverse gain. The sign
//! is set aside first and put back at the end, so the curve works on
//! magnitudes.
//!
//! Quantisation snaps to `2^bits − 1` steps. A fractional depth blends
//! the result at the whole depths either side of it. Within each step,
//! smoothing shapes the move from one level to the next: a hard step at
//! one end, a smoothstep in the middle, a straight line (the input
//! itself) at the other.
//!
//! The two crush modes differ in what is quantised. Symmetric quantises
//! each polarity's magnitude on its own grid from 0 to 1, so zero is a
//! level and small signals fall silent. Asymmetric quantises the whole
//! signal on one grid from −1 to 1, which has no level at zero: every
//! output sits at least half a step from it. Near zero (curved magnitude
//! below 0.05) the asymmetric result fades into the unquantised signal,
//! so very quiet input doesn't buzz between the two levels nearest zero.

/// Below this curved magnitude the asymmetric mode fades towards the
/// unquantised signal.
const FADE_BELOW: f32 = 0.05;
/// Auto gain treats a peak below this as silence and holds its gain.
const GATE: f32 = 0.005;

/// The gain auto gain applies at a peak envelope `level`: enough to bring
/// the peak to full scale, at most `1 / GATE`.
#[inline]
pub fn auto_gain(level: f32) -> f32 {
    1.0 / level.max(GATE)
}
/// The peak detector's attack time constant.
pub const ATTACK_MS: f32 = 0.5;

/// Which grid quantisation snaps to (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Asymmetric,
    Symmetric,
}

/// The crush chain's settings, recomputed once per control tick.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Linear gain into the chain. The engine ramps it, or follows the
    /// input with [`auto_gain`], and passes each sample's to
    /// [`Prepared::crush`].
    pub gain: f32,
    /// The power the inverse curve raises to: from 0.1 to 10.
    pub gamma: f32,
    /// Bit depth, from 1 to 16; fractional depths blend.
    pub bits: f32,
    /// 0 for hard steps, 0.5 for a smoothstep, 1 for no steps at all.
    pub smoothing: f32,
    pub mode: Mode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gain: 1.0,
            gamma: 1.0,
            bits: 4.0,
            smoothing: 0.0,
            mode: Mode::Asymmetric,
        }
    }
}

/// [`Settings`] with what the per-sample work needs worked out.
#[derive(Clone, Copy, Debug)]
pub struct Prepared {
    gamma: f32,
    gamma_recip: f32,
    /// Steps at the whole depths below and above, as floats.
    low_levels: f32,
    high_levels: f32,
    /// How far the depth is from the lower whole depth to the upper.
    fraction: f32,
    /// 1 − smoothing: 1 is a hard step.
    steepness: f32,
    mode: Mode,
}

impl Prepared {
    pub fn new(s: &Settings) -> Self {
        let bits = s.bits.clamp(1.0, 16.0);
        let low = bits.floor();
        let high = bits.ceil();
        let gamma = s.gamma.clamp(0.1, 10.0);
        Self {
            gamma,
            gamma_recip: 1.0 / gamma,
            low_levels: levels(low),
            high_levels: levels(high),
            fraction: bits - low,
            steepness: 1.0 - s.smoothing.clamp(0.0, 1.0),
            mode: s.mode,
        }
    }

    /// One sample through the chain, at linear `gain`.
    #[inline]
    pub fn crush(&self, input: f32, gain: f32) -> f32 {
        let magnitude = input.abs();
        let sign = if input > 0.0 {
            1.0
        } else if input < 0.0 {
            -1.0
        } else {
            return 0.0;
        };
        let gain = gain.max(1.0e-6);
        let clipped = (gain * magnitude).min(1.0);
        let curved = clipped.powf(self.gamma_recip);
        let quantised = match self.mode {
            Mode::Symmetric => self.quantise(curved),
            Mode::Asymmetric => self.quantise_asymmetric(sign, curved),
        };
        // Quantisation keeps the sign, so `quantised` is never negative
        // but for rounding.
        sign / gain * quantised.max(0.0).powf(self.gamma)
    }

    /// `x` in `[0, 1]` quantised at the fractional depth.
    #[inline]
    fn quantise(&self, x: f32) -> f32 {
        let low = step(x, self.low_levels, self.steepness);
        if self.fraction == 0.0 {
            return low;
        }
        let high = step(x, self.high_levels, self.steepness);
        low + self.fraction * (high - low)
    }

    /// The signed signal on one grid over `[−1, 1]`; returns the
    /// magnitude.
    #[inline]
    fn quantise_asymmetric(&self, sign: f32, curved: f32) -> f32 {
        // [−1, 1] onto [0, 1].
        let unit = (sign * curved + 1.0) * 0.5;
        let mut quantised = self.quantise(unit);
        if curved < FADE_BELOW {
            let fade = curved / FADE_BELOW;
            quantised = quantised * fade + unit * (1.0 - fade);
        }
        sign * (quantised * 2.0 - 1.0)
    }
}

/// Steps at a whole bit depth: `2^bits − 1`.
fn levels(bits: f32) -> f32 {
    ((1u32 << bits as u32) - 1) as f32
}

/// `x` in `[0, 1]` on a grid of `levels` steps, each step's rise shaped by
/// `steepness`.
#[inline]
fn step(x: f32, levels: f32, steepness: f32) -> f32 {
    let scaled = x * levels;
    let floor = scaled.floor();
    let within = scaled - floor;
    if within == 0.0 {
        return floor / levels;
    }
    (floor + shape(within, steepness)) / levels
}

/// The rise across one step: `x` itself at steepness 0, a smoothstep at
/// 0.5, a hard step at its middle at 1, blending linearly between.
#[inline]
fn shape(x: f32, steepness: f32) -> f32 {
    if steepness == 0.0 {
        return x;
    }
    let smooth = x * x * (3.0 - 2.0 * x);
    if steepness < 0.5 {
        let blend = steepness * 2.0;
        return x + blend * (smooth - x);
    }
    let hard = if x >= 0.5 { 1.0 } else { 0.0 };
    let blend = (steepness - 0.5) * 2.0;
    smooth + blend * (hard - smooth)
}

#[cfg(test)]
mod tests {
    use super::*;

    use vxn_free_dsp::PeakFollower;

    fn crush(s: Settings, x: f32) -> f32 {
        Prepared::new(&s).crush(x, s.gain)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-4
    }

    #[test]
    fn the_step_shapes_are_hard_smoothstep_and_straight() {
        assert_eq!(shape(0.49, 1.0), 0.0);
        assert_eq!(shape(0.5, 1.0), 1.0);
        assert!(close(shape(0.25, 0.5), 0.15625)); // 3/16 − 2/64
        assert_eq!(shape(0.3, 0.0), 0.3);
    }

    #[test]
    fn symmetric_two_bits_snaps_magnitudes_to_thirds() {
        // 2 bits: 3 steps, levels at 0, 1/3, 2/3 and 1.
        let s = Settings {
            bits: 2.0,
            mode: Mode::Symmetric,
            ..Settings::default()
        };
        assert!(close(crush(s, 0.2), 1.0 / 3.0)); // 0.6 steps: up
        assert!(close(crush(s, 0.1), 0.0)); // 0.3 steps: down
        assert!(close(crush(s, -0.6), -2.0 / 3.0)); // 1.8 steps: up
        assert!(close(crush(s, 0.95), 1.0));
    }

    #[test]
    fn asymmetric_one_bit_is_a_square_wave() {
        // One grid over [−1, 1] with one step: only ±1.
        let s = Settings {
            bits: 1.0,
            ..Settings::default()
        };
        assert!(close(crush(s, 0.3), 1.0));
        assert!(close(crush(s, -0.3), -1.0));
        // Symmetric one bit has 0 as a level as well.
        let sym = Settings {
            mode: Mode::Symmetric,
            ..s
        };
        assert!(close(crush(sym, 0.3), 0.0));
        assert!(close(crush(sym, 0.7), 1.0));
    }

    #[test]
    fn asymmetric_two_bits_has_no_level_at_zero() {
        // 2 bits on [0, 1]: 0, 1/3, 2/3, 1, which over [−1, 1] are −1,
        // −1/3, 1/3, 1.
        let s = Settings {
            bits: 2.0,
            ..Settings::default()
        };
        assert!(close(crush(s, 0.2), 1.0 / 3.0));
        assert!(close(crush(s, 0.6), 1.0 / 3.0)); // 0.8 on [0,1]: 2.4 steps
        assert!(close(crush(s, 0.7), 1.0)); // 0.85: 2.55 steps
        // Below the fade it leans towards the input. 0.025 is halfway in:
        // on [0, 1] it is 0.5125, which snaps to 2/3; half each.
        let blended = 0.5 * (2.0 / 3.0) + 0.5 * 0.5125;
        assert!(close(crush(s, 0.025), 2.0 * blended - 1.0));
    }

    #[test]
    fn a_fractional_depth_blends_the_whole_depths_beside_it() {
        let at = |bits| {
            crush(
                Settings {
                    bits,
                    mode: Mode::Symmetric,
                    ..Settings::default()
                },
                0.3,
            )
        };
        // 0.3 at 2 bits: 0.9 steps of 3, up to 1/3. At 3 bits: 2.1 of 7,
        // down to 2/7.
        assert!(close(at(2.0), 1.0 / 3.0));
        assert!(close(at(3.0), 2.0 / 7.0));
        assert!(close(at(2.25), 0.75 / 3.0 + 0.25 * 2.0 / 7.0));
    }

    #[test]
    fn full_smoothing_is_transparent() {
        let s = Settings {
            bits: 1.0,
            smoothing: 1.0,
            gamma: 3.0,
            gain: 4.0,
            ..Settings::default()
        };
        for x in [-0.2f32, -0.01, 0.05, 0.2] {
            assert!(close(crush(s, x), x), "{x}");
        }
    }

    #[test]
    fn gamma_curves_before_and_uncurves_after() {
        // Gamma 2: 0.25 becomes 0.5 for the quantiser (at 2 bits,
        // symmetric: 1.5 steps, up to 2/3), and 2/3 comes back as 4/9.
        let s = Settings {
            bits: 2.0,
            gamma: 2.0,
            mode: Mode::Symmetric,
            ..Settings::default()
        };
        assert!(close(crush(s, 0.25), 4.0 / 9.0));
    }

    #[test]
    fn gain_clips_and_is_undone() {
        // 20 dB: 0.06 becomes 0.6, which at 2 bits symmetric goes to 2/3
        // (1.8 steps), and comes back as 2/30.
        let s = Settings {
            bits: 2.0,
            gain: 10.0,
            mode: Mode::Symmetric,
            ..Settings::default()
        };
        assert!(close(crush(s, 0.06), 2.0 / 30.0));
        // 0.5 clips to 1 and comes back as 0.1.
        assert!(close(crush(s, 0.5), 0.1));
    }

    #[test]
    fn auto_gain_follows_the_peak() {
        let s = Settings {
            bits: 2.0,
            mode: Mode::Symmetric,
            ..Settings::default()
        };
        let p = Prepared::new(&s);
        let mut peak = PeakFollower::new(
            ATTACK_MS,
            vxn_free_dsp::peak::fall(0.115, 48_000.0),
            48_000.0,
        );
        // A steady 0.1 drives the peak to 0.1, so the gain to 10: the
        // quantiser sees 1, and the output is 0.1 again.
        let mut out = 0.0;
        for _ in 0..4800 {
            out = p.crush(0.1, auto_gain(peak.next(0.1)));
        }
        assert!((peak.level() - 0.1).abs() < 1e-4);
        assert!(close(out, 0.1), "{out}");
        // Silence holds the gain at its ceiling rather than dividing by 0.
        assert_eq!(auto_gain(0.0), 200.0);
    }

    #[test]
    fn zero_stays_zero() {
        for mode in [Mode::Asymmetric, Mode::Symmetric] {
            let s = Settings {
                mode,
                ..Settings::default()
            };
            assert_eq!(crush(s, 0.0), 0.0);
        }
    }
}
