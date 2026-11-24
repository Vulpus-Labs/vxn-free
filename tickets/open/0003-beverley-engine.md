---
id: "0003"
title: "Beverley engine: the crush chain, ported"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0001"]
---

## Summary

Port the module's processing to an `Engine`, stereo, at the host's rate.

Per sample: gain (fixed 0–20 dB, or automatic, following the input's peak
envelope), hard clip to ±1, gamma (power-law), quantisation to a
fractional bit depth (interpolating between the two whole depths beside
it), smoothing between levels (0 hard steps, 0.5 smoothstep, 1
transparent), symmetric or asymmetric crush, then reverse gamma and
reverse gain.

## Acceptance criteria

- [ ] `beverley` crate with the engine and its parameters: GAIN, AUTO
      GAIN, GAMMA, BIT DEPTH, SMOOTHING, CRUSH MODE, plus MIX (new;
      defaults fully wet).
- [ ] Parameters smoothed so automation doesn't zipper; the peak
      follower's time constant computed from the sample rate.
- [ ] Tests: a sine through each stage against values worked out by
      hand; fractional depth interpolates; symmetric and asymmetric modes
      differ as described; mono input feeds both outputs.
- [ ] No allocation in `process` (a no-alloc test).
- [ ] Aliasing measured at 44.1 and 96 kHz on a 1 kHz sine. Whether to
      oversample is decided from the numbers, which go in the close-out.
- [ ] Latency is known exactly. Without oversampling it is zero; with
      it, the resampling filters' delay, rounded to whole samples with
      the dry path (MIX) delayed to match so dry and wet stay aligned.
      The engine exposes its latency in samples as a function of its
      settings and rate, and a test measures it: an impulse and a step
      through the engine at every setting that could change it and at
      44.1, 48 and 96 kHz land on the sample the engine says.
