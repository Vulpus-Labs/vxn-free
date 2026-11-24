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

/// Everything about one render.
#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub flocks: [FlockMetrics; BANDS],
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
