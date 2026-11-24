---
id: "0019"
title: "Charm faceplate, editor and flock display"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0016", "0018", "0020", "0022", "0028"]
---

## Summary

An original SVG faceplate, replacing 0020's plain one, and an editor
showing the flock at work. The display can be developed against recorded
bench traces (0023) before it is connected to live frames.

## Acceptance criteria

- [ ] Faceplate drawn from scratch in SVG. Controls grouped as FLOCK
      (VOICES, DETUNE, SPREAD, COHESION, SEPARATION, TURBULENCE), SPACE
      (WIDTH, AUTO WIDTH, LOW XOVER, MID DEPTH, HIGH DEPTH) and LISTEN
      (DUCK, BLOOM), with MIX, plus whatever control 0028 gives the
      allpass decorrelation. Any merging of controls is
      decided here and the ADR updated to match.
- [ ] The flock display (ADR 0002, point 10), a custom widget:
      - axes: pan horizontal (with AUTO WIDTH walls), delay vertical
        (floor at the bottom)
      - MID and HIGH voices in two colours, each with a detune tail as a
        fraction of DETUNE
      - dot size and brightness following Duck and Bloom
- [ ] The engine writes one telemetry frame per flock tick (0022), with
      no allocation or lock on the audio thread. The editor draws the
      latest frame. A frame that is torn or out of date is never drawn.
- [ ] Hand check: on a drum loop, each hit shows as a pulse across the
      flock; on a pad, the flock wanders without pulses.
- [ ] Controls that do nothing in the current state are dimmed (for
      example WIDTH's meaning while AUTO WIDTH is on, if it changes).
- [ ] Snapshot tests for the faceplate and for frozen display frames:
      a sustained tone, a wide stereo input with AUTO WIDTH's walls
      closed in, and a ducked onset.
