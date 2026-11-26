---
id: "0008"
title: "Glow-Up engine: filter bank, grids, mapping"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0025"]
---

## Summary

The engine (ADR 0005): twelve filters, with twelve spares for tails,
assigned to the strongest requests for boost A's and boost B's ratio of
each peak, never moved while sounding, faded over 10 ms when stolen.

## Acceptance criteria

- [ ] Parameters: A AMOUNT, A NUM, A DEN, B AMOUNT, B NUM, B DEN
      (NUM and DEN stepped, 1–8), WIDTH (10–200 cents), RESONANCE,
      FOCUS (GRID to TRACK), MIX, WINDOW (512/1024/2048), THRESHOLD
      (0–30 dB), TONE (±6 dB/oct about 1 kHz), GRID (LIN, IRR, TET),
      SPEED, DRY CUT, MODE. Defaults as ADR 0005 and the module's.
- [ ] Self-oscillation held at a steady level, within a cent of its
      target; it can't run away.
- [ ] A MODE switch, MONO or STEREO:
      - MONO reads only the left input; the right output is a copy of
        the left output.
      - STEREO reads both inputs, analyses their mid signal
        ((L + R) / 2) once, and runs a separate filter bank on each
        channel, both banks following the same targets.
      MODE loads at MONO: Glow-Up is chiefly a guitar effect.
      CPU measured in each mode.
- [ ] Tests: every amount at 0 passes the input sample for sample; a
      sine passes almost untouched; a sawtooth's even partials rise with
      A at 2/1; a ratio of 3/1 lifts a sine's third partial; MONO ignores
      the right input and copies left to right; STEREO filters each
      channel; FOCUS fully right follows a glide and no sounding filter
      is retuned; resonance sings on a strong note within a cent and
      fades with it; loud noise with everything up stays bounded; the
      output is the same however the input is split into blocks; no
      allocation.
- [ ] Latency is known exactly. The design is zero: the signal path
      never waits for the analysis, and the dry path is not delayed.
      A test measures it: an impulse (and a step) through the engine at
      every WINDOW, MODE, sample rate and MIX setting, with the
      filters' contribution separated out, shows the output's dry
      component at the sample the engine says. The engine exposes its
      latency in samples as a function of its settings and rate.
