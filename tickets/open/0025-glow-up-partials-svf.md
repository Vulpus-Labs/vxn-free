---
id: "0025"
title: "Glow-Up DSP: peak detector, held resonator, partial tracker"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0007"]
---

## Summary

The parts of Glow-Up's analysis and filtering that only Glow-Up uses,
split out of 0007, in `vxn-free-dsp` (ADR 0005, points 4, 6 and 8).

## Acceptance criteria

- [ ] A peak detector over a frame (ADR 0005, point 4): peaks ascending
      in frequency, each with a centre, an amplitude and a prominence;
      power smoothed frame to frame; a level floor; a window change
      restarts it without allocating.
- [ ] A resonator: a TPT state-variable band-pass with per-sample
      frequency and damping, in `f64`, whose damping can reach zero and
      below, with its level read from its integrators and held at a
      target by damping.
- [ ] A partial tracker: a band-pass on one partial, its frequency
      measured between zero crossings, re-centring on what it measures,
      seeded and checked by the FFT.
- [ ] Tests:
      - sines found within a few cents and with their amplitude, peaks
        ascending, silence and sines under the floor finding none
      - prominence rising with level above THRESHOLD
      - the resonator unity at its centre, its −3 dB band where the
        damping puts it, self-oscillating within 0.01 cent of its
        tuning, its level held through a glide
      - the tracker follows a glide and a vibrato
      - no allocation
- [ ] Builds for `wasm32-unknown-unknown`.
