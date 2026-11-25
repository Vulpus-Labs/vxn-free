---
id: "0040"
title: "Wesley's crush displays, one a band"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0038", "0034"]
---

## Summary

The crush display (0038) in each of Wesley's band columns, on the plain
faceplate now and carried into 0036's design.

## Acceptance criteria

- [x] A display at the head of each band column, dimmed with the band
      when it is off.
- [x] Snapshots; every control still moves its param.

## Close-out (2026-11-25)

- The plain faceplate grows 96 px: each band column opens with its
  display (`canvas.low_display`, `mid_display`, `high_display`), in a
  dark well under the band's name. Each is `Unbound` and dims with its
  band while ON is off, as the band's knobs do.
- Tests: the layout has the 36 params' controls and three displays;
  every control still moves its param; moving MID's BIT DEPTH repaints
  MID's display only; snapshots `initial_1x` (re-blessed for the taller
  faceplate) and `mid_two_bits_1x`.
- clap-validator passes; installed.
