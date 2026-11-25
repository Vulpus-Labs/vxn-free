---
id: "0032"
title: "Beverley's crush chain as a reusable type"
priority: medium
created: 2026-11-25
epic: E005
depends: []
---

## Summary

Beverley's per-channel processing (its settings and glides, auto gain's
detectors, the crush) becomes one type, so Wesley can run three of
them (ADR 0003, point 8). Beverley then uses it too, unchanged in sound.

## Acceptance criteria

- [ ] A chain type in the `beverley` crate: set its params, run a
      channel (or a linked pair) a sample at a time, reset.
- [ ] `BeverleyEngine` built on it; every Beverley test passes unchanged,
      snapshots included.
- [ ] Beverley's output is unchanged: rendered before and after the
      change on the same inputs and settings, compared sample for sample
      in one run (the old engine kept as a test fixture for this
      ticket, then removed).
- [ ] No allocation, CPU no worse (measured).
