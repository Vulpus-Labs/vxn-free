---
id: "0028"
title: "Charm: decorrelate the voices with allpass filters"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0023"]
---

## Summary

Each voice is a delayed copy of its band, so with the dry signal it forms
a comb with evenly spaced notches, heard as hollowness when the voices
move slowly. Giving each voice its own allpass chain turns its phase
differently at each frequency, so its notches are no longer evenly
spaced, and voices no longer comb identically with each other. This is
the fixed, per-voice decorrelation that Acon Digital Multiply uses (ADR
0002, prior art), and it needs no analysis.

## Acceptance criteria

- [ ] A per-voice allpass chain after each voice's delay read, with
      different settings per voice and per band, fixed at construction
      and scaled to the sample rate.
- [ ] Measured on the bench, with decorrelation on and off:
      - comb regularity: the strongest peak of the cepstrum of the
        static dry + wet response (voices held still), in the voices'
        delay range. A regular comb shows as a strong peak there; a
        scattered one doesn't.
      - partial loss (mean and worst) on tone, chord and dist-chord
      - smearing: how far a click's energy spreads in time through one
        voice
      - CPU at 96 kHz
- [ ] A choice recorded in the close-out: always on, a control, or not
      worth it, with ADR 0002 updated to match.
- [ ] Tests: each voice's chain is allpass (flat magnitude within
      0.01 dB); output still identical across block splits; latency
      still zero; no allocation.
