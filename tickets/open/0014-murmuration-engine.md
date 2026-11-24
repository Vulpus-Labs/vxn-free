---
id: "0014"
title: "Murmuration engine core: flock, bands, mix"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0013"]
---

## Summary

A new `murmuration` crate (editor/engine, laid out like `beverley`) with
an engine that already sounds like the finished effect, before any
analysis is added. It has the flock (ADR 0002, point 3), the bands
(point 4), equal-power panning (point 5) and the dry/wet mix.

## Acceptance criteria

- [ ] Flock: 2–8 voices in (delay, pan) with cohesion, separation,
      alignment and turbulence, updated on a fixed control tick. Delay
      velocity is clamped to DETUNE, delay is bounded softly within
      SPREAD above the 7 ms floor, and pan is bounded within ±WIDTH.
      Turbulence comes from a seeded generator.
- [ ] Bands: LOW/MID/HIGH from LR4 crossovers. LOW is never chorused.
      MID and HIGH have their own depth. Voices read each band's mid
      signal. Dry is the unfiltered input.
- [ ] Params with fixed ids, ranges and defaults: VOICES, DETUNE, SPREAD,
      COHESION, SEPARATION, TURBULENCE, WIDTH, LOW XOVER, MID DEPTH,
      HIGH DEPTH, MIX. Ids are reserved now for CLARITY, DUCK, BLOOM and
      AUTO WIDTH. Every param is smoothed.
- [ ] Ranges and defaults tuned by ear on pads, guitar, voice and a drum
      loop. Notes on the tuning go in the close-out.
- [ ] Tests:
      - no voice's instantaneous detune exceeds DETUNE, across settings
      - no voice's delay leaves [floor, floor + SPREAD]
      - with SEPARATION up, no two voices sit within a stated minimum
        spacing once settled
      - output is bit-identical at block sizes 1, 64 and 512
      - LOW-band content under the crossover passes through with no wet
        part
      - latency is zero at every setting and at 44.1, 48 and 96 kHz
      - no allocation
- [ ] CPU for 8 voices at 96 kHz measured and recorded.
