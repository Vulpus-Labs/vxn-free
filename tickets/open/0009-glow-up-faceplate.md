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
- [x] A display of what the engine is doing: the found peaks and the
      filters' targets and levels, fed by bridge telemetry.
- [ ] Hover text as the module's (for example `A: 2/1, the octave above
      each peak, 25%`).
- [x] Snapshot tests.
- [x] Snapshot tests of the display.

## Progress (2026-11-26)

- `art/glow-up/faceplate.py` writes `crates/glow-up/glow-up.svg` and the
  relief's geometry: 1512 × 330, graphite anodised aluminium with a
  horizontal brush, amber accents (title glow, section ticks, knob
  lines, LEDs), black hex button-head screws, hairlines between sections.
  Title and MODE; BOOST A (AMOUNT, NUM / DEN); the display, with WINDOW,
  THRESHOLD and GRID under it; BOOST B; FILTER (RESONANCE, FOCUS; WIDTH,
  TONE, SPEED); OUTPUT (MIX, DRY CUT).
- `art/glow-up/blender/controls.py`: a machined aluminium knob (60-ridge
  knurl, turned top, a dark polished-metal line), cycled every 6° with four body
  frames, used as `knob` and `knob_small`; a soft-touch key with an amber
  LED, off and lit. About 40 s.
- Beverley's `blender/panel.py` gains a hex button-head screw with no
  washer (`"screw": "hex"` in the geometry); Beverley's and Wesley's
  pan-head screws are the default and unchanged. Glow-Up's relief: about
  1 min 40 s.
- GRID and MODE are rows of keys, one lit; DRY CUT a lit key.
- The display draws its screen and frequency grid; the engine's
  telemetry is next.

## Progress (2026-11-26): the tracking display

- `src/observe.rs`: `Observation`, a telemetry frame of 142 words: the
  engine's time in samples (modulo 2^24), the sample rate, the latest
  analysis frame's peaks (at most 32, the most prominent, ascending;
  frequency and prominence) and every filter of the pool (centre, level
  as a share of the most boost, and BOOSTING, SINGING, FADING or free).
- `GlowUpEngine::frame` publishes it after every block, without
  allocating.
- `src/display.rs`: `TrackDisplay` keeps the frames on the editor's
  side, placed by the engine's clock (so the traces scroll at the
  audio's pace however often the editor draws; the 2^24 wrap and resets
  handled), and draws the last 2 s: time left to right, frequency up a
  log axis from 40 Hz to 16 kHz with lines at each decade's 1, 2 and 5;
  each filter an amber trace, brighter and heavier with its level,
  near-white while singing, thin while fading, ending in a dot; the
  peaks as faint blue-grey ticks, brighter with prominence. Traces are
  grouped into a few paths a filter by look, not a stroke a segment.
  250 ms without a frame (the host stopped) dims them.
- Tests: the engine reports a 440 Hz peak and a 2/1 filter at 880 Hz at
  full level, singing with RESONANCE up; at most 32 peaks, ascending;
  frames round-trip; the clock across the wrap and a reset; the ring
  keeps its newest; snapshots of a bend, a singing filter and a stopped
  host, from hand-built frames; no allocation with frames published.
- Tests: snapshots of the defaults and of TET, STEREO and DRY CUT lit;
  every control moves its param; each key picks its position and
  keeps it when pressed again.
