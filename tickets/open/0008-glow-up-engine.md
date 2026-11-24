---
id: "0008"
title: "Glow-Up engine: filter bank, grids, mapping"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0007"]
---

## Summary

The engine: twelve filters assigned to the strongest requests for a
fifth, octave or double octave above each peak, never moved while
sounding, faded over 10 ms when stolen.

## Acceptance criteria

- [ ] Parameters: +7, +12, +24, WIDTH (10–200 cents), RESONANCE, FOCUS
      (GRID to TRACK), MIX, WINDOW (512/1024/2048), THRESHOLD (0–30 dB),
      TONE (±6 dB/oct about 1 kHz), GRID (three grids), SPEED, DRY CUT,
      MODE (below).
      Defaults as the module's.
- [ ] Self-oscillation held at a steady level, within a fraction of a
      cent of its target; it can't run away.
- [ ] A MODE switch, MONO or STEREO:
      - MONO reads only the left input; the right output is a copy of
        the left output.
      - STEREO reads both inputs, analyses their mid signal
        ((L + R) / 2) once, and runs a separate filter bank on each
        channel, both banks following the same targets.
      CPU measured in each mode.
- [ ] Tests: a sine passes almost untouched; a sawtooth's even partials
      rise with +12; MONO ignores the right input and copies left to
      right; STEREO filters each channel; FOCUS fully right tracks a
      glide; resonance past half way sings on a strong note and fades
      with it; no allocation.
- [ ] Zero latency: the signal path never waits for the analysis.
