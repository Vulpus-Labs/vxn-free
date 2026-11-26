---
id: "0009"
title: "Glow-Up faceplate, editor and tracking display"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0010"]
---

## Summary

Glow-Up's real faceplate, replacing 0010's plain one. First planned as a
guitar pedal; there are too many controls for a pedal, so it is a rack
unit like Beverley and Wesley, but modern rather than 1990s (decided
2026-11-26).

## Acceptance criteria

- [x] A modern rack unit: anodised aluminium, light legends and fine
      scales, machined knobs (large for the main controls, small for the
      trims), keys with LEDs, rack screws; original art, rendered in
      Blender as Beverley's is.
- [x] Faceplate SVG written by a script from scratch, with every
      control in 0008, replacing 0010's plain one.
- [ ] A display of what the engine is doing: the found peaks and the
      filters' targets and levels, fed by bridge telemetry.
- [ ] Hover text as the module's (for example `A: 2/1, the octave above
      each peak, 25%`).
- [x] Snapshot tests.
- [ ] Snapshot tests of the display.

## Progress (2026-11-26)

- `art/glow-up/faceplate.py` writes `crates/glow-up/glow-up.svg` and the
  relief's geometry: 1512 × 330, graphite anodised aluminium with a
  horizontal brush, amber accents (title glow, section ticks, knob
  lines, LEDs), black hex button-head screws, hairlines between sections.
  Title and MODE; BOOST A (AMOUNT, NUM / DEN); the display, with WINDOW,
  THRESHOLD and GRID under it; BOOST B; FILTER (RESONANCE, FOCUS; WIDTH,
  TONE, SPEED); OUTPUT (MIX, DRY CUT).
- `art/glow-up/blender/controls.py`: a machined aluminium knob (60-ridge
  knurl, turned top, an amber line), cycled every 6° with four body
  frames, used as `knob` and `knob_small`; a soft-touch key with an amber
  LED, off and lit. About 40 s.
- Beverley's `blender/panel.py` gains a hex button-head screw with no
  washer (`"screw": "hex"` in the geometry); Beverley's and Wesley's
  pan-head screws are the default and unchanged. Glow-Up's relief: about
  1 min 40 s.
- GRID and MODE are rows of keys, one lit; DRY CUT a lit key.
- The display draws its screen and frequency grid; the engine's
  telemetry is next.
- Tests: snapshots of the defaults and of TET, STEREO and DRY CUT lit;
  every control moves its param; each key picks its position and
  keeps it when pressed again.
