---
id: "0019"
title: "Murmuration faceplate, editor and flock display"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0016", "0017", "0018"]
---

## Summary

An original SVG faceplate and an editor showing the flock at work.

## Acceptance criteria

- [ ] Faceplate drawn from scratch in SVG. Controls grouped as FLOCK
      (VOICES, DETUNE, SPREAD, COHESION, SEPARATION, TURBULENCE), SPACE
      (WIDTH, AUTO WIDTH, LOW XOVER, MID DEPTH, HIGH DEPTH) and LISTEN
      (CLARITY, DUCK, BLOOM), with MIX. Any merging of controls is
      decided here and the ADR updated to match.
- [ ] A display: voices as points in (delay, pan), with the MID/HIGH
      spectrum and the strongest peaks behind them, and onsets shown as
      they duck. Fed from the engine without locks or allocation on the
      audio thread.
- [ ] Controls that do nothing in the current state are dimmed (for
      example WIDTH's meaning while AUTO WIDTH is on, if it changes).
- [ ] Snapshot tests for the faceplate and for a frozen display state.
