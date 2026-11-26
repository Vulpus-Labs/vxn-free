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

## Progress (2026-11-26): the scope replaces the tracking display

The display is now an EQ-style plot, as asked: frequency across (log,
40 Hz to 16 kHz), level up (−78 to +6 dB under full scale, a line every
12 dB).

- `src/observe.rs`: the frame grows to 326 words. It adds the input's
  spectrum (160 log-spaced points, dB, from the detector's smoothed
  power: each point the loudest bin in its share of the axis, or
  interpolated where bins are sparser than points), each filter's live
  damping, WIDTH's `k0`, and the output's direct and wet gains (MIX and
  DRY CUT). `Observation::response(hz)` gives the output's gain over the
  input: `|direct + wet · Σ gain · k0 · bp|`, each band-pass the TPT
  form's own (prewarped) response, a singing filter's damping taken at
  `k0 / 4` so its peak is finite.
- `vxn-free-dsp`'s `PeakDetector` gains `window()` and
  `smoothed_power()`; the bank's `Filter` gains `damping()`.
- `src/display.rs`, `ScopeDisplay`: the input as a pale line over a faint
  area, rising at once and falling at 60 dB/s by the engine's clock; the
  output as an amber line where it is 0.5 dB or more off the input; the
  gap between them filled a column at a time with a gradient, amber
  bright at the boosted line fading to the input's (a cool blue-grey
  where the output falls under the input); each detected peak a vertical
  line at its frequency, brighter with prominence; each filter a dot on
  the output's line, near-white while singing. 250 ms without a frame
  dims it all.
- Checked on the engine's own frames for a held four-saw chord (a
  scratch render, not kept): at TRACK the boosts land on the partials as
  spikes of 6–10 dB, and the dozen filters' skirts lift the crowded
  midrange by about half a dB; at GRID the dots sit on grid points; with
  DRY CUT the output falls under the input everywhere but the boosts.
- Tests: the engine's frame reads a 0.4 sine at its level within 1.5 dB
  and under −60 dB at 5 kHz, with +14 dB at its 2/1 filter and flat far
  off; the response is +14 dB at a centre, 3 dB down half a bandwidth
  off, flat far off, the filter alone with DRY CUT; the input line rises
  at once and falls by the engine's clock, across the 2^24 wrap, and
  starts again on a reset; snapshots of a glide with boosts, a singing
  filter, DRY CUT and a stopped host.

## Progress (2026-11-26): no WINDOW, a red input, a smaller face

- WINDOW is gone: the analysis is fixed at 2048 (`glow_up::WINDOW`), and
  ADR 0005 says so. The params are 16; WINDOW's id, 11, is retired and
  the others keep theirs (a test checks the ids ascend, skipping only
  retired ones). The engine keeps one LIN grid, not one a window.
- The scope's input line is red, its area and the cut fill in the same
  red family, so the amber output reads as the input brightened.
- The face is 1300 × 330 (was 1512): every section pulled in (title 176,
  each boost 168, the scope's section 356, FILTER 244, OUTPUT 116), the
  scope's window 324 × 184 (was 400), THRESHOLD and GRID's keys under it.
  The relief is re-rendered; the snapshots re-blessed.
