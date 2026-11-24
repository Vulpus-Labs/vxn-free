---
id: "0024"
title: "Charm first tuning pass: ranges and defaults"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0023"]
---

## Summary

The flock and band controls get their real ranges and defaults, chosen by
ear and checked against the bench (0023). This is milestone M1: a chorus
with the flock, detune as a speed limit, bands and unchorused lows,
measured offline.

## Acceptance criteria

- [ ] Ranges, curves and defaults settled for VOICES, DETUNE, SPREAD,
      COHESION, SEPARATION, TURBULENCE, WIDTH, LOW XOVER, MID DEPTH, HIGH
      DEPTH and MIX. The fixed MID/HIGH crossover is settled too. All are
      recorded in the close-out with the reasons.
- [ ] Rendered and listened to on pads, guitar, voice and a drum loop,
      plus the synthetic test set. Notes recorded.
- [ ] The default setting's bench metrics are recorded as the baseline
      that later tickets compare against: detune, motion, spacing,
      mono-sum loss and CPU.
- [ ] ADR 0002's draft values (the 2.5 kHz crossover, the 7 ms floor) are
      updated if tuning changes them.
