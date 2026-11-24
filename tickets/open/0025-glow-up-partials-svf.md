---
id: "0025"
title: "Glow-Up DSP: partial tracking and SVF"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0007"]
---

## Summary

The parts of Glow-Up's analysis and filtering that only Glow-Up uses,
split out of 0007, in `vxn-free-dsp`.

## Acceptance criteria

- [ ] Partial tracking: peaks from 0007 matched frame to frame into
      partials with a strength.
- [ ] A TPT state-variable filter with per-sample frequency and Q.
- [ ] Tests:
      - tracking follows a glide
      - the SVF's response matches its design at several settings
      - no allocation
- [ ] Builds for `wasm32-unknown-unknown`.
