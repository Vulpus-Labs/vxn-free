---
id: "0019"
title: "Charm faceplate, editor and flock display"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0016", "0017", "0018", "0022"]
---

## Summary

An original SVG faceplate and an editor showing the flock at work.

## Acceptance criteria

- [ ] Faceplate drawn from scratch in SVG. Controls grouped as FLOCK
      (VOICES, DETUNE, SPREAD, COHESION, SEPARATION, TURBULENCE), SPACE
      (WIDTH, AUTO WIDTH, LOW XOVER, MID DEPTH, HIGH DEPTH) and LISTEN
      (CLARITY, DUCK, BLOOM), with MIX. Any merging of controls is
      decided here and the ADR updated to match.
- [ ] The flock display (ADR 0002, point 10), a custom widget:
      - axes: pan horizontal (with AUTO WIDTH walls), delay vertical
        (floor at the bottom)
      - MID and HIGH voices in two colours, each with a detune tail as a
        fraction of DETUNE
      - Clarity's no-go bands shaded at d ≈ (k + ½) / p ± r / p, with
        intensity = strength × gate
      - dot size and brightness following Duck and Bloom
- [ ] The engine writes one telemetry frame per flock tick (0022), with
      no allocation or lock on the audio thread. The editor draws the
      latest frame. A frame that is torn or out of date is never drawn.
- [ ] Hand check: on a solo note, voices visibly turn back at the bands.
      On a dense chord, the bands fade out.
- [ ] Controls that do nothing in the current state are dimmed (for
      example WIDTH's meaning while AUTO WIDTH is on, if it changes).
- [ ] Snapshot tests for the faceplate and for frozen display frames:
      a sparse tone with bands, a dense chord with the gate closed, and
      a ducked onset.
