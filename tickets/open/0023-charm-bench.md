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

- [ ] A bench binary in the repo (for example
      `cargo run -p charm --example bench -- <inputs> --set name=value`),
      built and checked by CI. It runs the engine offline at a chosen
      rate and block size, and parameters can be set or ramped.
- [ ] A test set:
      - Synthetic signals generated in code: a sustained harmonic tone,
        a clean four-note chord, a distorted chord, pink noise, a click
        train and a synthetic drum loop. No licensed audio goes in the
        repo.
      - Recordings read from a folder given by an environment variable
        (`CHARM_BENCH_AUDIO`), kept outside the repo.
- [ ] Outputs per run:
      - the rendered wav, and its mono sum
      - CSV traces of the observation struct (0014) per flock tick
      - a metrics summary as text and JSON
- [ ] Metrics from the start:
      - maximum instantaneous detune per voice, in cents
      - RMS delay velocity per flock
      - minimum spacing between voices
      - mono-sum level loss per band
      - CPU per block at block sizes 16, 32, 64 and 512, at 96 kHz
      - output identical whether the input is fed at one block size or
        another, in one run
- [ ] Metrics can be added per ticket. 0016–0018 add theirs.
- [ ] A note in the ticket on making plots from the CSV traces. Plotting
      scripts stay outside the repo.
