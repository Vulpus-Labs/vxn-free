//! Where the resonances sit at the GRID end of FOCUS (ADR 0005, point 6):
//! each grid ascending, from [`LOWEST`] to [`HIGHEST`] Hz.
//!
//! - **LIN**: the analysis bins' spacing, `fs / N`: sparse low down, dense
//!   up high.
//! - **IRR**: like a room's modes. [`IRREGULAR_POINTS`] points drawn from a
//!   density on log frequency that is a floor plus a bump about 1 kHz, 1.3
//!   octaves either side, so densest through the midrange; each point is
//!   jittered within its share by a fixed seed.
//! - **TET**: twelve-tone equal temperament at A = 440 Hz.
//!
//! Built when the engine is made, never on the audio thread.

pub const LOWEST: f64 = 40.0;
pub const HIGHEST: f64 = 18_000.0;
pub const IRREGULAR_POINTS: usize = 120;

/// LIN at `window` samples and `sample_rate`.
pub fn linear(sample_rate: f64, window: usize) -> Vec<f64> {
    let step = sample_rate / window as f64;
    let first = (LOWEST / step).ceil() as usize;
    let last = (HIGHEST / step).floor() as usize;
    (first..=last).map(|k| k as f64 * step).collect()
}

/// TET.
pub fn equal_tempered() -> Vec<f64> {
    (-60..80)
        .map(|n| 440.0 * 2f64.powf(f64::from(n) / 12.0))
        .filter(|&f| (LOWEST..=HIGHEST).contains(&f))
        .collect()
}

/// IRR.
pub fn irregular() -> Vec<f64> {
    const STEPS: usize = 4096;
    let (u0, u1) = (LOWEST.ln(), HIGHEST.ln());
    let mu = 1000f64.ln();
    let sigma = 1.3 * 2f64.ln();
    let mut cdf = vec![0.0; STEPS + 1];
    for i in 1..=STEPS {
        let u = u0 + (u1 - u0) * (i as f64 - 0.5) / STEPS as f64;
        let z = (u - mu) / sigma;
        cdf[i] = cdf[i - 1] + 0.25 + (-0.5 * z * z).exp();
    }
    let total = cdf[STEPS];
    let mut seed: u32 = 0x6c0f;
    let mut unit = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(seed >> 8) / f64::from(1u32 << 24)
    };
    let mut pts: Vec<f64> = (0..IRREGULAR_POINTS)
        .map(|k| {
            let target = (k as f64 + 0.15 + 0.7 * unit()) / IRREGULAR_POINTS as f64 * total;
            let i = cdf.partition_point(|&c| c < target).clamp(1, STEPS);
            let frac = (target - cdf[i - 1]) / (cdf[i] - cdf[i - 1]);
            (u0 + (u1 - u0) * ((i - 1) as f64 + frac) / STEPS as f64).exp()
        })
        .collect();
    pts.sort_by(f64::total_cmp);
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ascending_in_range(g: &[f64]) {
        assert!(g.windows(2).all(|w| w[0] < w[1]));
        assert!(g.iter().all(|f| (LOWEST..=HIGHEST).contains(f)));
    }

    #[test]
    fn the_grids_ascend_within_their_range() {
        let lin = linear(48_000.0, 2048);
        ascending_in_range(&lin);
        assert_eq!(lin.len(), 767);
        let tet = equal_tempered();
        ascending_in_range(&tet);
        assert!(tet.iter().any(|&f| (f - 440.0).abs() < 1e-9));
        let irr = irregular();
        ascending_in_range(&irr);
        assert_eq!(irr.len(), IRREGULAR_POINTS);
    }

    #[test]
    fn irr_is_densest_through_the_midrange() {
        let irr = irregular();
        let per_octave = |lo: f64| irr.iter().filter(|&&f| f >= lo && f < 2.0 * lo).count();
        assert!(per_octave(700.0) > 2 * per_octave(50.0));
        assert!(per_octave(700.0) > 2 * per_octave(8000.0));
    }
}
