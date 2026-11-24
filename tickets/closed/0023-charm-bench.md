---
id: "0023"
title: "Charm bench: offline render and measurement"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0014"]
---

## Summary

A way to run Charm's engine over any audio and measure what it does, before
there is a plugin or a UI. Every later Charm ticket records its numbers
from it.

## Acceptance criteria

- [x] A bench binary in the repo (for example
      `cargo run -p charm --example bench -- <inputs> --set name=value`),
      built and checked by CI. It runs the engine offline at a chosen
      rate and block size, and parameters can be set or ramped.
- [x] A test set:
      - Synthetic signals generated in code: a sustained harmonic tone,
        a clean four-note chord, a distorted chord, pink noise, a click
        train and a synthetic drum loop. No licensed audio goes in the
        repo.
      - Recordings read from a folder given by an environment variable
        (`CHARM_BENCH_AUDIO`), kept outside the repo.
- [x] Outputs per run:
      - the rendered wav, and its mono sum
      - CSV traces of the observation struct (0014) per flock tick
      - a metrics summary as text and JSON
- [x] Metrics from the start:
      - maximum instantaneous detune per voice, in cents
      - RMS delay velocity per flock
      - minimum spacing between voices
      - mono-sum level loss per band
      - CPU per block at block sizes 16, 32, 64 and 512, at 96 kHz
      - output identical whether the input is fed at one block size or
        another, in one run
- [x] Metrics can be added per ticket. 0016–0018 add theirs.
- [x] A note in the ticket on making plots from the CSV traces. Plotting
      scripts stay outside the repo.

## Close-out (2026-11-24)

- `crates/charm/examples/bench/`:
  - `main.rs`: arguments, rendering and reports
  - `signals.rs`: the synthetic test set
  - `metrics.rs`: the measurements; later tickets add their fields here

  CI builds and lints it with `--all-targets`. WAV I/O uses `hound`,
  which is a dev-dependency of `charm` only.
- Usage:

  ```sh
  cargo run --release -p charm --example bench -- [options] [inputs]
  ```

  - With no inputs, all six synthetic signals run.
  - `--corpus` adds every WAV in `$CHARM_BENCH_AUDIO`.
  - `--set NAME=VALUE` sets a param by name, ignoring case and spaces.
    VOICES takes a count.
  - `--ramp NAME=FROM:TO` moves a param in a straight line over the run.
  - `--rate`, `--seconds` and `--out` (default `target/charm-bench`)
    are also available.
  - `--cpu` adds the CPU table at 96 kHz.
- Params are set every 512 samples. The main render runs one tick
  (32 samples) at a time to capture every observation. A second render
  in blocks of 1, 7, 64, 300, 33 and 512 samples must match it bit for
  bit.
- The synthetic signals:
  - `tone`: 220 Hz with 12 harmonics at 1/k
  - `chord`: C4 E4 G4 B4, 8 harmonics each
  - `dist-chord`: E2 B2 E3 through tanh
  - `pink`: independent pink noise on each channel (Kellet filter)
  - `clicks`: 4 Hz decaying bursts
  - `drums`: a 120 bpm loop of kick, snare and hats
- Outputs per input: `NAME.wav` (stereo, 32-bit float), `NAME-mono.wav`
  and `NAME-voices.csv`. The CSV has one row per active voice per tick,
  with columns tick, seconds, band, voice, delay_ms, pan, cents and gain.
  `metrics.txt` and `metrics.json` cover the whole run.
- Metrics:
  - per flock: maximum and RMS detune against DETUNE, and RMS speed as
    a share of the limit
  - per flock: closest pair in the unit square and in delay alone, after
    the first second
  - mono-sum loss for LOW, MID and HIGH, input and output
  - whether the block-split render matched
  - with `--cpu`: % of a core, ns per sample, and worst block against
    the block's period, at blocks of 16, 32, 64 and 512
- First run, at defaults:
  - **Flock:** the same for every input, since nothing listens yet. MID
    reaches 12.000 of 12 cents at most, with RMS 6.1 cents. Speed RMS is
    0.51 of the limit. The closest pair is 0.167 apart in the unit
    square.
  - **Voices crossing:** in delay alone the closest gap is 0. Voices pass
    each other in delay while apart in pan, and doubling up at one delay
    costs density. Listen for it in 0024.
  - **Mono sum:** the mono inputs lose at most 0.03 dB per band. Pink
    noise goes from −3.02 dB in to −2.88 dB out, because the wet signal
    comes from the mid signal.
  - **CPU at 96 kHz:** 0.39–0.47 % of a core. The worst 16-sample block
    took 108 µs of its 167 µs, a one-off warm-up (steady blocks take
    about 1 µs).
- Plotting: CSVs load into anything. For example, in Python with pandas
  and matplotlib, kept outside the repo:

  ```python
  v = pd.read_csv("target/charm-bench/chord-voices.csv")
  for (band, i), g in v.groupby(["band", "voice"]):
      plt.plot(g.pan, g.delay_ms)    # each voice's path
  ```
