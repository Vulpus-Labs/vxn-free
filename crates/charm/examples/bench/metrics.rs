//! What the bench measures about one render. Later tickets add their own
//! fields here (0016–0018).

use charm::engine::MID_HIGH_XOVER;
use charm::flock::max_velocity;
use charm::observe::{BANDS, Observation};
use vxn_free_dsp::Crossover;

/// The settings in force at one tick, for metrics that depend on them.
#[derive(Clone, Copy, Debug)]
pub struct TickSettings {
    pub detune: f32,
    /// Seconds.
    pub spread: f32,
    /// 0 to 1.
    pub width: f32,
    pub low_xover: f32,
}

/// Per band: MID, HIGH.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlockMetrics {
    /// The largest detune any voice reached, cents (absolute).
    pub max_cents: f32,
    /// DETUNE's largest value over the run.
    pub detune_limit: f32,
    /// RMS of each voice's delay velocity, as a share of the speed limit.
    pub rms_speed: f32,
    /// RMS detune, cents.
    pub rms_cents: f32,
    /// The closest two voices came, in the unit square (delay over
    /// SPREAD, pan over WIDTH), after the first second.
    pub min_spacing: f32,
    /// The closest two voices came in delay alone, ms.
    pub min_delay_gap_ms: f32,
}

/// Level lost when summed to mono, per band (LOW, MID, HIGH), dB: 0 when
/// both channels are equal, −3 when uncorrelated.
#[derive(Clone, Copy, Debug, Default)]
pub struct MonoLoss {
    pub input: [f32; 3],
    pub output: [f32; 3],
}

/// What the analysis saw, per band.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnalysisMetrics {
    pub onsets: usize,
    pub onsets_per_second: f32,
    pub mean_coherence: f32,
    /// Peaks found per tick, on average.
    pub mean_peaks: f32,
}

/// How much the known partials of a harmonic signal dip in the output's
/// mono sum, against the input's: the time-averaged loss in dB (gains
/// count as 0), per partial above LOW XOVER.
#[derive(Clone, Copy, Debug, Default)]
pub struct PartialLoss {
    pub partials: usize,
    /// The partial that lost most, and its loss (≤ 0).
    pub worst_hz: f32,
    pub worst_db: f32,
    /// The mean loss over the partials.
    pub mean_db: f32,
}

/// How regular a comb the output makes with its input (0028).
#[derive(Clone, Copy, Debug, Default)]
pub struct Comb {
    /// The largest cepstral magnitude at quefrencies within the voices'
    /// delay range: a regular comb at delay d puts a peak at d.
    pub cepstral_peak: f32,
    pub peak_ms: f32,
    /// The standard deviation of the transfer's level, dB, 300 Hz–10 kHz.
    pub ripple_db: f32,
}

/// Everything about one render.
#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub comb: Comb,
    pub partial_loss: Option<PartialLoss>,
    pub flocks: [FlockMetrics; BANDS],
    pub analysis: [AnalysisMetrics; BANDS],
    pub mono: MonoLoss,
    /// Whether rendering in other block sizes gave bit-identical output.
    pub split_identical: bool,
}

pub fn flocks(obs: &[Observation], settings: &[TickSettings], rate: f32) -> [FlockMetrics; BANDS] {
    let settle = (rate / charm::engine::TICK as f32) as usize; // one second
    let mut out = [FlockMetrics::default(); BANDS];
    for (b, m) in out.iter_mut().enumerate() {
        m.min_spacing = f32::INFINITY;
        m.min_delay_gap_ms = f32::INFINITY;
        let (mut sum_speed, mut sum_cents, mut count) = (0.0f64, 0.0f64, 0usize);
        for (t, (o, s)) in obs.iter().zip(settings).enumerate() {
            let band = &o.bands[b];
            let vmax = max_velocity(f64::from(s.detune)) as f32;
            m.detune_limit = m.detune_limit.max(s.detune);
            let voices = &band.voices[..band.active as usize];
            for v in voices {
                let cents = v.cents().abs();
                m.max_cents = m.max_cents.max(cents);
                sum_speed += f64::from(v.velocity / vmax).powi(2);
                sum_cents += f64::from(cents).powi(2);
                count += 1;
            }
            if t < settle {
                continue;
            }
            for i in 0..voices.len() {
                for j in i + 1..voices.len() {
                    let dd = voices[i].delay - voices[j].delay;
                    m.min_delay_gap_ms = m.min_delay_gap_ms.min(dd.abs() * 1000.0);
                    let du = dd / s.spread;
                    let dp = if s.width > 0.0 {
                        0.5 * (voices[i].pan - voices[j].pan) / s.width
                    } else {
                        0.0
                    };
                    m.min_spacing = m.min_spacing.min((du * du + dp * dp).sqrt());
                }
            }
        }
        if count > 0 {
            m.rms_speed = (sum_speed / count as f64).sqrt() as f32;
            m.rms_cents = (sum_cents / count as f64).sqrt() as f32;
        }
    }
    out
}

/// Mono-sum loss per band of `l`/`r`, skipping the first half second.
pub fn mono_loss(l: &[f32], r: &[f32], rate: f32, low_xover: f32) -> [f32; 3] {
    let split = |x: &[f32]| {
        let mut low = Crossover::new(low_xover, rate);
        let mut high = Crossover::new(MID_HIGH_XOVER, rate);
        let mut bands = [Vec::new(), Vec::new(), Vec::new()];
        for &s in x {
            let (lo, rest) = low.split(s);
            let (m, h) = high.split(rest);
            bands[0].push(lo);
            bands[1].push(m);
            bands[2].push(h);
        }
        bands
    };
    let (bl, br) = (split(l), split(r));
    let skip = (rate * 0.5) as usize;
    let mut out = [0.0f32; 3];
    for b in 0..3 {
        let (mut stereo, mut mono) = (0.0f64, 0.0f64);
        for (&x, &y) in bl[b].iter().zip(&br[b]).skip(skip) {
            let (x, y) = (f64::from(x), f64::from(y));
            stereo += 0.5 * (x * x + y * y);
            mono += (0.5 * (x + y)).powi(2);
        }
        out[b] = if stereo > 1.0e-20 {
            (10.0 * (mono / stereo).log10()) as f32
        } else {
            0.0
        };
    }
    out
}

pub fn analysis(obs: &[Observation], rate: f32) -> [AnalysisMetrics; BANDS] {
    let mut out = [AnalysisMetrics::default(); BANDS];
    let ticks = obs.len().max(1) as f32;
    let samples = obs.last().map_or(1, |o| o.tick) as f32 * charm::engine::TICK as f32;
    for (b, a) in out.iter_mut().enumerate() {
        let mut coherence = 0.0f32;
        let mut peaks = 0usize;
        for o in obs {
            let band = &o.bands[b];
            if band.onset > 0.0 {
                a.onsets += 1;
            }
            coherence += band.coherence;
            peaks += band.peaks.iter().filter(|p| p.strength > 0.0).count();
        }
        a.mean_coherence = coherence / ticks;
        a.mean_peaks = peaks as f32 / ticks;
        a.onsets_per_second = a.onsets as f32 / (samples / rate);
    }
    out
}

/// Each partial's level in `input` and `output` (mono), in Hann-windowed
/// stretches of 4096 samples every 2048, from half a second in.
pub fn partial_loss(
    input: &[f32],
    output: &[f32],
    partials: &[f32],
    rate: f32,
    low_xover: f32,
) -> PartialLoss {
    const N: usize = 4096;
    let window: Vec<f32> = (0..N)
        .map(|k| 0.5 - 0.5 * (std::f32::consts::TAU * k as f32 / N as f32).cos())
        .collect();
    let level = |x: &[f32], hz: f32| {
        let w = std::f64::consts::TAU * f64::from(hz) / f64::from(rate);
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (n, (&s, &h)) in x.iter().zip(&window).enumerate() {
            let v = f64::from(s * h);
            re += v * (w * n as f64).cos();
            im -= v * (w * n as f64).sin();
        }
        (re * re + im * im).sqrt()
    };
    let start = (rate * 0.5) as usize;
    let mut out = PartialLoss {
        worst_db: 0.0,
        ..PartialLoss::default()
    };
    let mut sum = 0.0f32;
    for &hz in partials.iter().filter(|&&hz| hz > low_xover * 1.5) {
        let (mut loss, mut count) = (0.0f64, 0usize);
        let mut at = start;
        while at + N <= input.len().min(output.len()) {
            let x = level(&input[at..at + N], hz);
            if x > 1.0e-9 {
                let gain = 20.0 * (level(&output[at..at + N], hz) / x).log10();
                loss += gain.min(0.0);
                count += 1;
            }
            at += N / 2;
        }
        if count == 0 {
            continue;
        }
        let mean = (loss / count as f64) as f32;
        out.partials += 1;
        sum += mean;
        if mean < out.worst_db {
            out.worst_db = mean;
            out.worst_hz = hz;
        }
    }
    if out.partials > 0 {
        out.mean_db = sum / out.partials as f32;
    }
    out
}

/// The comb `output` makes of `input`: their transfer, averaged over
/// Hann-windowed frames of 4096 every 2048 from one second in, then the
/// cepstrum of its log level, searched over the voices' delays.
pub fn comb(input: &[f32], output: &[f32], rate: f32, spread: f32) -> Comb {
    use vxn_free_dsp::RealPackedFft;
    const N: usize = 4096;
    let fft = RealPackedFft::new(N);
    let window: Vec<f32> = (0..N)
        .map(|k| 0.5 - 0.5 * (std::f32::consts::TAU * k as f32 / N as f32).cos())
        .collect();
    let bins = N / 2;
    let (mut cross_re, mut cross_im, mut auto) =
        (vec![0.0f64; bins], vec![0.0f64; bins], vec![0.0f64; bins]);
    let (mut x, mut y) = (vec![0.0f32; N], vec![0.0f32; N]);
    let mut at = rate as usize;
    while at + N <= input.len().min(output.len()) {
        for k in 0..N {
            x[k] = input[at + k] * window[k];
            y[k] = output[at + k] * window[k];
        }
        fft.forward(&mut x);
        fft.forward(&mut y);
        for k in 1..bins {
            let (xr, xi) = (f64::from(x[2 * k]), f64::from(x[2 * k + 1]));
            let (yr, yi) = (f64::from(y[2 * k]), f64::from(y[2 * k + 1]));
            // conj(X)·Y
            cross_re[k] += xr * yr + xi * yi;
            cross_im[k] += xr * yi - xi * yr;
            auto[k] += xr * xr + xi * xi;
        }
        at += N / 2;
    }
    // Log level of the transfer, as a real spectrum for the inverse FFT.
    let mut log = vec![0.0f32; N];
    let (mut sum, mut sum2, mut count) = (0.0f64, 0.0f64, 0usize);
    let per_bin = f64::from(rate) / N as f64;
    for k in 1..bins {
        let h = (cross_re[k].hypot(cross_im[k]) / auto[k].max(1.0e-30)).max(1.0e-6);
        log[2 * k] = h.ln() as f32;
        let hz = k as f64 * per_bin;
        if (300.0..10_000.0).contains(&hz) {
            let db = 20.0 * h.log10();
            sum += db;
            sum2 += db * db;
            count += 1;
        }
    }
    log[0] = log[2];
    log[1] = log[2 * bins - 2];
    fft.inverse(&mut log);
    let first = (charm::flock::FLOOR * f64::from(rate)) as usize;
    let last = ((charm::flock::FLOOR + f64::from(spread)) * f64::from(rate)) as usize + 1;
    let mut out = Comb::default();
    for (n, &c) in log.iter().enumerate().take(last.min(N / 2)).skip(first) {
        if c.abs() > out.cepstral_peak {
            out.cepstral_peak = c.abs();
            out.peak_ms = n as f32 / rate * 1000.0;
        }
    }
    if count > 0 {
        let mean = sum / count as f64;
        out.ripple_db = (sum2 / count as f64 - mean * mean).max(0.0).sqrt() as f32;
    }
    out
}
