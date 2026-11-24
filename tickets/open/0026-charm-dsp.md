---
id: "0026"
title: "Charm DSP: modulated delay, LR4 crossovers, side-buffer"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0001"]
---

## Summary

The building blocks Charm's audio path needs, in `vxn-free-dsp`
(ADR 0002, points 4, 6 and 10).

## Acceptance criteria

- [ ] A scratch harness measures aliasing and CPU for modulated
      fractional-delay reads: linear, cubic Hermite, 4-point Lagrange and
      first-order allpass. Test conditions: a sine sweep through a voice
      moving at the largest DETUNE (draft: 30 cents), at 44.1, 48 and
      96 kHz. The numbers and the choice go in this ticket's close-out.
- [ ] A modulated delay line built on the chosen interpolation: a
      per-sample fractional delay, a capacity fixed at construction, and
      no allocation after construction.
- [ ] A 4th-order Linkwitz-Riley crossover (low/high pair) with a
      crossover frequency that can be set while running without zipper
      noise.
- [ ] A gain side-buffer the same length as a delay line. Values can be
      written at past positions up to a stated lag, and are read at
      fractional positions with the same interpolation as the audio.
- [ ] Tests:
      - a static fractional delay matches its design in group delay and
        magnitude
      - a voice moving at constant d′ comes out at ratio 1 − d′, to
        within a cent
      - the LR4 low and high outputs sum allpass with magnitude flat
        within 0.01 dB
      - a value written to the side-buffer is read back at the matching
        audio sample
      - no allocation
- [ ] Builds for `wasm32-unknown-unknown`.
