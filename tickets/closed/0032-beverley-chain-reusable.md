---
id: "0032"
title: "Beverley's crush chain as a reusable type"
priority: medium
created: 2026-11-25
epic: E005
depends: []
---

## Summary

Beverley's per-channel processing (its settings and glides, auto gain's
detectors, the crush) becomes one type, so Wesley can run three of
them (ADR 0003, point 8). Beverley then uses it too, unchanged in sound.

## Acceptance criteria

- [x] A chain type in the `beverley` crate: set its params, run a
      channel (or a linked pair) a sample at a time, reset.
- [x] `BeverleyEngine` built on it; every Beverley test passes unchanged,
      snapshots included.
- [x] Beverley's output is unchanged: rendered before and after the
      change on the same inputs and settings, compared sample for sample
      in one run (the old engine kept as a test fixture for this
      ticket, then removed).
- [x] No allocation, CPU no worse (measured).

## Close-out (2026-11-25)

- `beverley::chain::Chain`: one crush chain (a channel, or a linked or
  mono pair) with its settings, glides, auto gain detectors and dry/wet
  mix. `Chain::set(Setting)` takes each setting in Beverley's units
  (`Setting::Gain(dB)`, `BitDepth(bits)`, `Mix(%)`, ...); the owner
  calls `tick()` at each control tick and `run_one` or `run_two` for the
  tick's samples; `reset()`. `Stereo`, `Detect` and `TICK` moved with
  it and are re-exported from `engine`.
- `BeverleyEngine` drives one `Chain` and keeps only the tick count;
  `engine::setting(index, value)` maps Beverley's params to settings.
  Wesley will map each band's params the same way.
- Unchanged output, checked (commit before this one): the engine as it
  was, kept as a test fixture, and the new one rendered a decaying tone
  with noise and an offset, at 44.1 and 96 kHz, at seven settings
  (fixed and auto gain, PEAK and RANGE, all three STEREO modes, both
  crush modes, fractional depths, full smoothing), mono and stereo, in
  blocks of 1, 7, 32, 64 and 500 samples, with every param automated
  half way through. All 140 renders matched bit for bit. The check
  failed when the glide time was nudged from 10 to 10.01 ms. The
  fixture is then removed, as planned.
- CPU the same within noise: 46–59 ns a stereo frame before, 40–55
  after (release build, 96 kHz, best of five; includes making the test
  signal).
- Every Beverley test passes unchanged, snapshots included.
