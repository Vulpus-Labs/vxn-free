//! Charm's bench: run the engine offline over audio and measure what it
//! does (ticket 0023).
//!
//! ```text
//! cargo run --release -p charm --example bench -- [options] [inputs]
//! ```
//!
//! Inputs are WAV files or the names of synthetic signals (`tone`,
//! `chord`, `dist-chord`, `pink`, `clicks`, `drums`). With none, every
//! synthetic signal runs. `--corpus` adds every WAV in the folder named by
//! `CHARM_BENCH_AUDIO`.
//!
//! Options:
//!
//! - `--set NAME=VALUE`: set a param (`--set detune=20 --set "mid depth=50"`;
//!   VOICES takes a count, 2–8).
//! - `--ramp NAME=FROM:TO`: move a param in a straight line over the run.
//! - `--rate HZ`: the synthetic signals' rate (default 48000).
//! - `--seconds S`: the synthetic signals' length (default 10).
//! - `--out DIR`: where results go (default the workspace's
//!   `target/charm-bench`).
//! - `--cpu`: measure CPU per block at 16, 32, 64 and 512 samples, 96 kHz.
//! - `--no-decorrelate`: turn the voices' allpass chains off.
//!
//! For each input it writes `NAME.wav` (the output, stereo float),
//! `NAME-mono.wav` (its mono sum), `NAME-voices.csv` (every voice at every
//! tick) and `NAME-analysis.csv` (onsets and coherence per band at
//! every tick), and adds a section to `metrics.txt` and `metrics.json`.

mod metrics;
mod signals;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use charm::engine::{CharmEngine, TICK};
use charm::observe::{BANDS, Observation};
use charm::{DETUNE, LOW_XOVER, MIN_VOICES, PARAMS, SPREAD, VOICES, WIDTH};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;

use metrics::{Metrics, MonoLoss, TickSettings};

/// Params are set (and ramps stepped) at multiples of this many samples.
const PARAM_EVERY: usize = 512;

#[derive(Clone, Copy, Debug)]
struct Ramp {
    param: ParamIndex,
    from: f32,
    to: f32,
}

#[derive(Debug, Default)]
struct Args {
    sets: Vec<(ParamIndex, f32)>,
    ramps: Vec<Ramp>,
    inputs: Vec<String>,
    corpus: bool,
    rate: f32,
    seconds: f32,
    out: PathBuf,
    cpu: bool,
    /// Turn the voices' allpass decorrelation off (0028).
    no_decorrelate: bool,
}

/// The param called `name`, matched without case, spaces, `-` or `_`.
fn param(name: &str) -> Result<ParamIndex, String> {
    let squash = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    };
    let want = squash(name);
    PARAMS
        .iter()
        .position(|p| squash(p.name) == want)
        .map(|i| ParamIndex(i as u32))
        .ok_or_else(|| {
            let names: Vec<_> = PARAMS.iter().map(|p| p.name).collect();
            format!("no param `{name}`; the params are {}", names.join(", "))
        })
}

/// A value as typed to the plain value the engine takes.
fn plain(index: ParamIndex, typed: f32) -> f32 {
    if index == VOICES {
        typed - MIN_VOICES as f32
    } else {
        typed
    }
}

/// A plain value as the user types it.
fn typed(index: ParamIndex, plain: f32) -> f32 {
    if index == VOICES {
        plain + MIN_VOICES as f32
    } else {
        plain
    }
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        rate: 48_000.0,
        seconds: 10.0,
        // The workspace's target folder, wherever the bench is run from.
        out: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/charm-bench"),
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    let number = |s: &str, what: &str| {
        s.trim()
            .parse::<f32>()
            .map_err(|_| format!("{what}: `{s}` is not a number"))
    };
    while let Some(a) = it.next() {
        let mut value = |flag: &str| it.next().ok_or_else(|| format!("{flag} needs a value"));
        match a.as_str() {
            "--set" => {
                let v = value("--set")?;
                let (name, x) = v
                    .split_once('=')
                    .ok_or_else(|| format!("--set takes NAME=VALUE, not `{v}`"))?;
                let p = param(name)?;
                args.sets.push((p, plain(p, number(x, name)?)));
            }
            "--ramp" => {
                let v = value("--ramp")?;
                let (name, range) = v
                    .split_once('=')
                    .ok_or_else(|| format!("--ramp takes NAME=FROM:TO, not `{v}`"))?;
                let (from, to) = range
                    .split_once(':')
                    .ok_or_else(|| format!("--ramp takes NAME=FROM:TO, not `{v}`"))?;
                let p = param(name)?;
                args.ramps.push(Ramp {
                    param: p,
                    from: plain(p, number(from, name)?),
                    to: plain(p, number(to, name)?),
                });
            }
            "--rate" => args.rate = number(&value("--rate")?, "--rate")?,
            "--seconds" => args.seconds = number(&value("--seconds")?, "--seconds")?,
            "--out" => args.out = PathBuf::from(value("--out")?),
            "--corpus" => args.corpus = true,
            "--cpu" => args.cpu = true,
            "--no-decorrelate" => args.no_decorrelate = true,
            "--help" | "-h" => {
                return Err("see the doc comment at the top of examples/bench/main.rs".into());
            }
            _ if a.starts_with("--") => return Err(format!("unknown option `{a}`")),
            _ => args.inputs.push(a),
        }
    }
    Ok(args)
}

/// One input, as stereo.
struct Input {
    name: String,
    rate: f32,
    channels: [Vec<f32>; 2],
}

fn read_wav(path: &Path) -> Result<Input, String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = reader.spec();
    let n = usize::from(spec.channels);
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap_or(0) as f32 * scale)
                .collect()
        }
    };
    let left: Vec<f32> = samples.iter().step_by(n).copied().collect();
    let right: Vec<f32> = if n > 1 {
        samples.iter().skip(1).step_by(n).copied().collect()
    } else {
        left.clone()
    };
    Ok(Input {
        name: path
            .file_stem()
            .map_or("input".into(), |s| s.to_string_lossy().into_owned()),
        rate: spec.sample_rate as f32,
        channels: [left, right],
    })
}

fn write_wav(path: &Path, rate: f32, channels: &[&[f32]]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: channels.len() as u16,
        sample_rate: rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w =
        hound::WavWriter::create(path, spec).map_err(|e| format!("{}: {e}", path.display()))?;
    for n in 0..channels[0].len() {
        for c in channels {
            w.write_sample(c[n]).map_err(|e| e.to_string())?;
        }
    }
    w.finalize().map_err(|e| e.to_string())
}

/// The value of every param at sample `at` of `len`.
fn values_at(args: &Args, at: usize, len: usize) -> Vec<(ParamIndex, f32)> {
    let mut v: Vec<(ParamIndex, f32)> = (0..PARAMS.len())
        .map(|i| (ParamIndex(i as u32), PARAMS[i].default))
        .collect();
    for &(p, x) in &args.sets {
        v[p.0 as usize].1 = x;
    }
    let share = at as f32 / len.max(1) as f32;
    for r in &args.ramps {
        v[r.param.0 as usize].1 = r.from + (r.to - r.from) * share;
    }
    v
}

/// Render `input` through a fresh engine, `chunk(at)` samples at a time
/// from sample `at`, calling `each` after every chunk. Params are set at
/// every multiple of [`PARAM_EVERY`], so chunks never cross one.
fn render(
    args: &Args,
    input: &Input,
    mut chunk: impl FnMut(usize) -> usize,
    mut each: impl FnMut(&CharmEngine, usize),
) -> [Vec<f32>; 2] {
    let len = input.channels[0].len();
    let mut e = CharmEngine::new(input.rate);
    e.set_decorrelation(!args.no_decorrelate);
    for (p, v) in values_at(args, 0, len) {
        e.set_param(p, v);
    }
    e.reset();
    let [mut l, mut r] = input.channels.clone();
    let mut at = 0;
    while at < len {
        if at % PARAM_EVERY == 0 {
            for (p, v) in values_at(args, at, len) {
                e.set_param(p, v);
            }
        }
        let to_param = PARAM_EVERY - at % PARAM_EVERY;
        let n = chunk(at).clamp(1, to_param).min(len - at);
        e.process(&mut [&mut l[at..at + n], &mut r[at..at + n]]);
        at += n;
        each(&e, at);
    }
    [l, r]
}

fn run(
    args: &Args,
    input: &Input,
    report: &mut String,
    json: &mut Vec<String>,
) -> Result<(), String> {
    let len = input.channels[0].len();
    // Tick by tick, keeping every observation and the settings in force.
    let mut obs: Vec<Observation> = Vec::with_capacity(len / TICK + 1);
    let mut settings = Vec::with_capacity(len / TICK + 1);
    let main = render(
        args,
        input,
        |_| TICK,
        |e, at| {
            let v = values_at(args, at, len);
            obs.push(*e.observation());
            settings.push(TickSettings {
                detune: v[DETUNE.0 as usize].1,
                spread: v[SPREAD.0 as usize].1 * 0.001,
                width: v[WIDTH.0 as usize].1 * 0.01,
                low_xover: v[LOW_XOVER.0 as usize].1,
            });
        },
    );
    // The same input in awkward blocks must give the same output.
    let pattern = [1, 7, 64, 300, 33, 512];
    let mut k = 0;
    let other = render(
        args,
        input,
        |_| {
            k += 1;
            pattern[k % pattern.len()]
        },
        |_, _| {},
    );
    let identical = main
        .iter()
        .zip(&other)
        .all(|(a, b)| a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits()));

    std::fs::create_dir_all(&args.out).map_err(|e| e.to_string())?;
    let [l, r] = &main;
    let mono: Vec<f32> = l.iter().zip(r).map(|(a, b)| 0.5 * (a + b)).collect();
    write_wav(
        &args.out.join(format!("{}.wav", input.name)),
        input.rate,
        &[l, r],
    )?;
    write_wav(
        &args.out.join(format!("{}-mono.wav", input.name)),
        input.rate,
        &[&mono],
    )?;
    write_csv(
        &args.out.join(format!("{}-voices.csv", input.name)),
        &obs,
        input.rate,
    )?;
    write_analysis_csv(
        &args.out.join(format!("{}-analysis.csv", input.name)),
        &obs,
        input.rate,
    )?;

    let xover = settings.last().map_or(150.0, |s| s.low_xover);
    let m = Metrics {
        flocks: metrics::flocks(&obs, &settings, input.rate),
        mono: MonoLoss {
            input: metrics::mono_loss(&input.channels[0], &input.channels[1], input.rate, xover),
            output: metrics::mono_loss(l, r, input.rate, xover),
        },
        split_identical: identical,
        crest: {
            let mono_in: Vec<f32> = input.channels[0]
                .iter()
                .zip(&input.channels[1])
                .map(|(a, b)| 0.5 * (a + b))
                .collect();
            metrics::crest(&mono_in, &mono, input.rate)
        },
        onset_scores: signals::onsets(&input.name, input.rate, len).map_or([None, None], |truth| {
            metrics::score_onsets(&obs, &truth, input.rate)
        }),
        comb: {
            let mono_in: Vec<f32> = input.channels[0]
                .iter()
                .zip(&input.channels[1])
                .map(|(a, b)| 0.5 * (a + b))
                .collect();
            let spread = settings.last().map_or(0.01, |s| s.spread);
            metrics::comb(&mono_in, &mono, input.rate, spread)
        },
        analysis: metrics::analysis(&obs, input.rate),
        partial_loss: signals::partials(&input.name, input.rate).map(|p| {
            let mono_in: Vec<f32> = input.channels[0]
                .iter()
                .zip(&input.channels[1])
                .map(|(a, b)| 0.5 * (a + b))
                .collect();
            metrics::partial_loss(&mono_in, &mono, &p, input.rate, xover)
        }),
    };
    write_report(report, json, &input.name, &m);
    Ok(())
}

fn write_csv(path: &Path, obs: &[Observation], rate: f32) -> Result<(), String> {
    let mut s = String::from("tick,seconds,band,voice,delay_ms,pan,cents,gain\n");
    for o in obs {
        let secs = o.tick as f32 * TICK as f32 / rate;
        for (b, band) in o.bands.iter().enumerate() {
            let name = ["mid", "high"][b];
            for (i, v) in band.voices.iter().enumerate().take(band.active as usize) {
                let _ = writeln!(
                    s,
                    "{},{secs:.5},{name},{i},{:.4},{:.4},{:.3},{:.4}",
                    o.tick,
                    v.delay * 1000.0,
                    v.pan,
                    v.cents(),
                    v.gain
                );
            }
        }
    }
    std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

/// One row per band per tick: the analysis as the engine saw it.
fn write_analysis_csv(path: &Path, obs: &[Observation], rate: f32) -> Result<(), String> {
    let mut s = String::from("tick,seconds,band,onset,onset_start,coherence\n");
    for o in obs {
        let secs = o.tick as f32 * TICK as f32 / rate;
        for (b, band) in o.bands.iter().enumerate() {
            let _ = write!(
                s,
                "{},{secs:.5},{},{:.4},{},{:.4}",
                o.tick,
                ["mid", "high"][b],
                band.onset,
                band.onset_start,
                band.coherence
            );
            s.push('\n');
        }
    }
    std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_report(report: &mut String, json: &mut Vec<String>, name: &str, m: &Metrics) {
    let _ = writeln!(report, "== {name}");
    let _ = writeln!(
        report,
        "  block-split identical: {}",
        if m.split_identical { "yes" } else { "NO" }
    );
    let mut flocks = Vec::new();
    for (b, f) in m.flocks.iter().enumerate() {
        let band = ["MID", "HIGH"][b];
        let _ = writeln!(
            report,
            "  {band:<4} detune max {:.3} of {:.3} cents, rms {:.2} cents; speed rms {:.2} of limit; \
             closest pair {:.3} (unit square), {:.3} ms (delay)",
            f.max_cents,
            f.detune_limit,
            f.rms_cents,
            f.rms_speed,
            f.min_spacing,
            f.min_delay_gap_ms
        );
        flocks.push(format!(
            "\"{}\":{{\"max_cents\":{},\"detune_limit\":{},\"rms_cents\":{},\"rms_speed\":{},\
             \"min_spacing\":{},\"min_delay_gap_ms\":{}}}",
            band.to_lowercase(),
            f.max_cents,
            f.detune_limit,
            f.rms_cents,
            f.rms_speed,
            finite(f.min_spacing),
            finite(f.min_delay_gap_ms)
        ));
    }
    for (b, a) in m.analysis.iter().enumerate() {
        let _ = writeln!(
            report,
            "  {:<4} onsets {} ({:.1} a second); coherence mean {:.2}",
            ["MID", "HIGH"][b],
            a.onsets,
            a.onsets_per_second,
            a.mean_coherence
        );
    }
    write_measures(report, json, name, m);
    let bands = ["LOW", "MID", "HIGH"];
    let _ = write!(report, "  mono-sum loss dB (in → out):");
    for ((band, input), output) in bands.iter().zip(m.mono.input).zip(m.mono.output) {
        let _ = write!(report, " {band} {input:.2} → {output:.2};");
    }
    let _ = writeln!(report);
    let analysis: Vec<String> = m
        .analysis
        .iter()
        .zip(["mid", "high"])
        .map(|(a, band)| {
            format!(
                "\"{band}\":{{\"onsets\":{},\"onsets_per_second\":{},\"mean_coherence\":{}}}",
                a.onsets, a.onsets_per_second, a.mean_coherence
            )
        })
        .collect();
    json.push(format!(
        "\"{name}\":{{\"split_identical\":{},{},\"analysis\":{{{}}},\"mono_loss_db\":{{\"input\":{:?},\"output\":{:?}}}}}",
        m.split_identical,
        flocks.join(","),
        analysis.join(","),
        m.mono.input,
        m.mono.output
    ));
}

/// The onset scores, partial loss and comb, where measured.
fn write_measures(report: &mut String, json: &mut Vec<String>, name: &str, m: &Metrics) {
    let _ = writeln!(
        report,
        "  crest factor (mono): input {:.1} dB, output {:.1} dB, wet {:.1} dB",
        m.crest.input, m.crest.output, m.crest.wet
    );
    json.push(format!(
        "\"{name}-crest\":{{\"input\":{},\"output\":{},\"wet\":{}}}",
        m.crest.input, m.crest.output, m.crest.wet
    ));
    for (b, s) in m.onset_scores.iter().enumerate() {
        if let Some(s) = s {
            let _ = writeln!(
                report,
                "  {:<4} onsets against truth: {} of {} found, {} false; delay mean {:.1} ms, max {:.1} ms",
                ["MID", "HIGH"][b],
                s.hits,
                s.truth,
                s.false_onsets,
                s.mean_delay_ms,
                s.max_delay_ms
            );
            json.push(format!(
                "\"{name}-onsets-{}\":{{\"truth\":{},\"hits\":{},\"false\":{},\"mean_delay_ms\":{},\"max_delay_ms\":{}}}",
                ["mid", "high"][b],
                s.truth,
                s.hits,
                s.false_onsets,
                s.mean_delay_ms,
                s.max_delay_ms
            ));
        }
    }
    if let Some(p) = m.partial_loss {
        let _ = writeln!(
            report,
            "  partials (mono): {} above LOW XOVER; mean loss {:.2} dB; worst {:.2} dB at {:.0} Hz",
            p.partials, p.mean_db, p.worst_db, p.worst_hz
        );
        json.push(format!(
            "\"{name}-partials\":{{\"count\":{},\"mean_db\":{},\"worst_db\":{},\"worst_hz\":{}}}",
            p.partials, p.mean_db, p.worst_db, p.worst_hz
        ));
    }
    let _ = writeln!(
        report,
        "  comb (mono, in → out): cepstral peak {:.3} at {:.2} ms; ripple {:.2} dB (300 Hz–10 kHz)",
        m.comb.cepstral_peak, m.comb.peak_ms, m.comb.ripple_db
    );
    json.push(format!(
        "\"{name}-comb\":{{\"cepstral_peak\":{},\"peak_ms\":{},\"ripple_db\":{}}}",
        m.comb.cepstral_peak, m.comb.peak_ms, m.comb.ripple_db
    ));
}

/// JSON has no infinity.
fn finite(x: f32) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        "null".into()
    }
}

/// CPU per block at 96 kHz, stereo pink noise, with the settings given.
fn cpu(args: &Args, report: &mut String) {
    let rate = 96_000.0;
    let seconds = 10.0;
    let input = signals::make("pink", rate, seconds).expect("pink is a signal");
    let _ = writeln!(report, "== CPU at 96 kHz (stereo pink noise, {seconds} s)");
    for block in [16usize, 32, 64, 512] {
        let mut e = CharmEngine::new(rate);
        e.set_decorrelation(!args.no_decorrelate);
        for (p, v) in values_at(args, 0, 1) {
            e.set_param(p, v);
        }
        e.reset();
        let [mut l, mut r] = input.clone();
        let mut worst = 0.0f64;
        let start = Instant::now();
        for (cl, cr) in l.chunks_mut(block).zip(r.chunks_mut(block)) {
            let t = Instant::now();
            e.process(&mut [cl, cr]);
            worst = worst.max(t.elapsed().as_secs_f64());
        }
        let total = start.elapsed().as_secs_f64();
        let period = block as f64 / f64::from(rate);
        let _ = writeln!(
            report,
            "  block {block:>3}: {:.2} % of a core, {:.1} ns a sample; worst block {:.1} µs of {:.1} µs",
            100.0 * total / f64::from(seconds),
            total * 1.0e9 / f64::from(rate) / f64::from(seconds),
            worst * 1.0e6,
            period * 1.0e6
        );
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("bench: {e}");
            std::process::exit(2);
        }
    };
    let mut inputs: Vec<String> = args.inputs.clone();
    if args.corpus {
        match std::env::var("CHARM_BENCH_AUDIO") {
            Ok(dir) => {
                let mut wavs: Vec<String> = std::fs::read_dir(&dir)
                    .map(|d| {
                        d.filter_map(Result::ok)
                            .map(|e| e.path())
                            .filter(|p| {
                                p.extension().is_some_and(|x| x.eq_ignore_ascii_case("wav"))
                            })
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect()
                    })
                    .unwrap_or_default();
                wavs.sort();
                inputs.extend(wavs);
            }
            Err(_) => eprintln!("bench: --corpus needs CHARM_BENCH_AUDIO set to a folder of WAVs"),
        }
    }
    if inputs.is_empty() && !args.cpu {
        inputs = signals::NAMES.iter().map(|s| (*s).to_string()).collect();
    }

    let mut report = String::new();
    let mut json = Vec::new();
    let _ = writeln!(report, "Charm bench. Settings: {}", describe(&args));
    for name in &inputs {
        let input = match signals::make(name, args.rate, args.seconds) {
            Some(channels) => Ok(Input {
                name: name.clone(),
                rate: args.rate,
                channels,
            }),
            None => read_wav(Path::new(name)),
        };
        match input.and_then(|i| run(&args, &i, &mut report, &mut json)) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("bench: {e}");
                std::process::exit(1);
            }
        }
    }
    if args.cpu {
        cpu(&args, &mut report);
    }
    print!("{report}");
    if std::fs::create_dir_all(&args.out).is_ok() {
        let _ = std::fs::write(args.out.join("metrics.txt"), &report);
        let _ = std::fs::write(
            args.out.join("metrics.json"),
            format!(
                "{{\"settings\":\"{}\",\"bands\":{BANDS},{}}}\n",
                describe(&args),
                json.join(",")
            ),
        );
        eprintln!("bench: results in {}", args.out.display());
    }
}

fn describe(args: &Args) -> String {
    let mut parts: Vec<String> = args
        .sets
        .iter()
        .map(|&(p, v)| format!("{}={}", PARAMS[p.0 as usize].name, typed(p, v)))
        .collect();
    parts.extend(args.ramps.iter().map(|r| {
        format!(
            "{}={}:{}",
            PARAMS[r.param.0 as usize].name,
            typed(r.param, r.from),
            typed(r.param, r.to)
        )
    }));
    if parts.is_empty() {
        "defaults".into()
    } else {
        parts.join(" ")
    }
}
