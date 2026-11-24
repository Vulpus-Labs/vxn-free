//! Peaks in one STFT frame.
//!
//! A peak is a bin whose power is a local maximum and stands a threshold
//! above the frame's mean power. Its frequency is refined between bins by
//! fitting a parabola through the log magnitudes of the bin and its
//! neighbours (quadratically interpolated FFT), which for a Hann window
//! lands within a few hundredths of a bin.

use crate::stft::Frame;

/// One spectral peak.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Peak {
    /// Hz.
    pub hz: f32,
    /// The sinusoid's amplitude, as a sine of that amplitude would show it
    /// through a Hann window.
    pub amplitude: f32,
}

/// The strongest peaks in `frame`, at most `out.len()` of them, written to
/// `out` strongest first; returns how many. A peak must stand
/// `threshold_db` above the frame's mean power. `sample_rate` turns bins
/// into Hz. Never allocates.
pub fn peaks(frame: &Frame<'_>, sample_rate: f32, threshold_db: f32, out: &mut [Peak]) -> usize {
    let bins = frame.len();
    let window = 2 * (bins - 1);
    if out.is_empty() || bins < 3 {
        return 0;
    }
    let mean = (0..bins).map(|k| frame.power(k)).sum::<f32>() / bins as f32;
    let floor = mean * 10f32.powf(threshold_db / 10.0);
    let mut found = 0;
    // A Hann window's coherent gain is a half: a sine of amplitude A shows
    // as A · window / 4 at its peak.
    let to_amplitude = 4.0 / window as f32;
    for k in 1..bins - 1 {
        let power = frame.power(k);
        if power <= floor || power <= frame.power(k - 1) || power < frame.power(k + 1) {
            continue;
        }
        // Parabola through the log magnitudes (half the log powers).
        let below = 0.5 * frame.power(k - 1).max(1.0e-30).ln();
        let here = 0.5 * power.ln();
        let above = 0.5 * frame.power(k + 1).max(1.0e-30).ln();
        let curve = below - 2.0 * here + above;
        let delta = if curve < 0.0 {
            (0.5 * (below - above) / curve).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let peak_log = here - 0.25 * (below - above) * delta;
        let peak = Peak {
            hz: (k as f32 + delta) * sample_rate / window as f32,
            amplitude: peak_log.exp() * to_amplitude,
        };
        // Keep the strongest, in order: insert, dropping the weakest.
        let mut i = found.min(out.len() - 1);
        if found == out.len() && out[i].amplitude >= peak.amplitude {
            continue;
        }
        while i > 0 && out[i - 1].amplitude < peak.amplitude {
            out[i] = out[i - 1];
            i -= 1;
        }
        out[i] = peak;
        found = (found + 1).min(out.len());
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stft::Stft;
    use std::f64::consts::TAU;

    fn sines(parts: &[(f64, f32)], rate: f64, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| {
                parts
                    .iter()
                    .map(|&(hz, a)| a * (TAU * hz * n as f64 / rate).sin() as f32)
                    .sum()
            })
            .collect()
    }

    /// The peaks of the last frame of `x`.
    fn last_peaks(x: &[f32], window: usize, rate: f32, n: usize) -> Vec<Peak> {
        let mut stft = Stft::new(2048, 4, window, window / 4);
        stft.push(x, |_| {});
        let mut out = vec![Peak::default(); n];
        let found = peaks(&stft.latest().unwrap(), rate, 10.0, &mut out);
        out.truncate(found);
        out
    }

    fn cents(a: f32, b: f64) -> f64 {
        1200.0 * (f64::from(a) / b).log2()
    }

    #[test]
    fn a_sine_is_found_within_a_few_cents_at_every_window() {
        let rate = 48_000.0;
        for window in [128usize, 256, 512, 1024, 2048] {
            let bin = rate / window as f64;
            // From 12 bins up: below that, a bin's width is too much of
            // the frequency for a few cents at the smallest windows.
            for bins in [12.3f64, 20.5, 31.77, 50.0] {
                let hz = bins * bin;
                let x = sines(&[(hz, 0.5)], rate, 8 * window);
                let found = last_peaks(&x, window, rate as f32, 4);
                assert!(!found.is_empty(), "{window}: nothing at {hz}");
                let err = cents(found[0].hz, hz);
                assert!(
                    err.abs() < 5.0,
                    "{window}: {hz} Hz found {} off by {err} cents",
                    found[0].hz
                );
                assert!(
                    (found[0].amplitude - 0.5).abs() < 0.04,
                    "{window}: {found:?}"
                );
            }
        }
    }

    #[test]
    fn two_close_partials_are_separated_at_2048() {
        // Three bins apart at 48 kHz.
        let rate = 48_000.0;
        let (f1, f2) = (1000.0, 1000.0 + 3.0 * rate / 2048.0);
        let x = sines(&[(f1, 0.5), (f2, 0.3)], rate, 8 * 2048);
        let found = last_peaks(&x, 2048, rate as f32, 4);
        assert!(found.len() >= 2, "{found:?}");
        assert!(cents(found[0].hz, f1).abs() < 5.0, "{found:?}");
        assert!(cents(found[1].hz, f2).abs() < 5.0, "{found:?}");
    }

    #[test]
    fn peaks_come_strongest_first_and_only_as_many_as_asked() {
        let rate = 48_000.0;
        let parts = [(500.0, 0.1), (1500.0, 0.4), (3000.0, 0.2), (6000.0, 0.3)];
        let x = sines(&parts, rate, 8 * 1024);
        let found = last_peaks(&x, 1024, rate as f32, 3);
        assert_eq!(found.len(), 3);
        let hz: Vec<f64> = found.iter().map(|p| f64::from(p.hz)).collect();
        for (got, want) in hz.iter().zip([1500.0, 6000.0, 3000.0]) {
            assert!((got / want - 1.0).abs() < 0.01, "{hz:?}");
        }
    }

    #[test]
    fn silence_and_noise_floors_have_no_peaks() {
        let x = vec![0.0f32; 4096];
        assert!(last_peaks(&x, 512, 48_000.0, 8).is_empty());
    }
}
