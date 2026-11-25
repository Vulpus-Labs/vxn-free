---
id: "0031"
title: "DSP: a three-band split that sums flat"
priority: medium
created: 2026-11-25
epic: E005
depends: []
---

## Summary

`vxn-free-dsp` gains a three-band split for Wesley (ADR 0003, point 3):
two Linkwitz-Riley crossovers, with the low band passed through the
upper crossover's allpass so the three bands sum to an allpass of the
input.

## Acceptance criteria

- [ ] `ThreeBand` (or similar): `split(x) -> [low, mid, high]`, both
      frequencies movable while running without clicks, the upper held
      at least an octave above the lower.
- [ ] Tests:
      - the bands' sum is flat within 0.01 dB from 20 Hz to 20 kHz, at
        several crossover settings and at 44.1, 48 and 96 kHz
      - each band is 6 dB down at its crossovers and falls at 24 dB an
        octave beyond
      - zero latency: an impulse's sum peaks at sample 0 (the phase
        turns; nothing is delayed)
      - no allocation; builds for `wasm32-unknown-unknown`
- [ ] CPU per sample measured and recorded.
