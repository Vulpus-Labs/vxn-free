---
id: "0017"
title: "Murmuration CLARITY: steering comb notches off partials"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0015"]
---

## Summary

Voices steer away from delays whose dry + voice comb notches, at
(2k + 1) / 2d Hz, would land on strong peaks (ADR 0002, point 7). The
steering is a force in the flock, so DETUNE's speed limit also limits
how fast a voice moves under it.

## Acceptance criteria

- [ ] A notch-cost term per voice, summing over peaks weighted by
      strength and by each peak's distance from the nearest notch, with
      its gradient in d added to the voice's steering with weight CLARITY.
- [ ] The cost is computed on the flock tick from the latest peak set, at
      a bounded cost (stated, and measured with 8 voices and 8 peaks).
- [ ] Measurement, recorded in the close-out:
      - test signals: a sustained chord and a sustained harmonic tone
        (stated frequencies)
      - with CLARITY at 0 and at full, measure the time-averaged loss in
        level at each partial in the mono-summed output
      - target: at full CLARITY the worst partial loses at least half as
        many dB as at 0
- [ ] Tests:
      - no voice's detune exceeds DETUNE with CLARITY at full
      - with CLARITY at 0 the output is bit-identical to the output
        without the term
      - no allocation
