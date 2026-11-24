---
id: "0018"
title: "Charm AUTO WIDTH and the mono guard"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0015"]
---

## Summary

Bands that are already wide get less spread; narrow bands get the full
WIDTH. The output's mono sum is kept from collapsing (ADR 0002, point 7).

## Acceptance criteria

- [ ] The bench (0023) reports per-band coherence and mono-sum level
      loss. Before/after numbers on the bench's test set are recorded
      in the close-out.
- [ ] The new params are exposed in the plugin (0020) in the same
      change, on the plain faceplate until 0019.
- [ ] AUTO WIDTH (on/off): per band, the flock's pan bound becomes WIDTH
      scaled by a function of input coherence (stated in the close-out).
      Changes are slewed so the image never jumps.
- [ ] A mono guard: per band, the wet part's contribution to the
      level lost in the mono sum is kept within a bound, chosen by
      measurement on pads, guitar and voice (the measurements are
      recorded). The guard is always on.
- [ ] Tests:
      - mono input gets the full WIDTH
      - wide decorrelated input gets less spread
      - over a long run of pink noise and a sustained chord, the
        per-band mono-sum level loss stays within the bound
      - with AUTO WIDTH off, behaviour matches the fixed WIDTH apart from
        the guard
      - no allocation
