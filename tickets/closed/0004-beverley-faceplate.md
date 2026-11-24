---
id: "0004"
title: "Beverley faceplate and editor"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0003"]
---

## Summary

An original faceplate SVG and the editor built from it.

## Acceptance criteria

- [x] Looks like a rack unit: a wide, short landscape panel with rack
      ears and their screw slots, a painted or brushed front, controls in
      a row with silk-screened labels and scales.
- [x] Faceplate SVG drawn from scratch: four knobs (GAIN, GAMMA, BIT
      DEPTH, SMOOTHING), the AUTO GAIN toggle, the CRUSH MODE switch, the
      three-way STEREO switch (MONO, SEPARATE, LINKED),
      MIX; labels in silkscreen's print style.
- [x] Bindings to the engine's parameters; values shown in units (dB,
      bits) on hover.
- [x] Snapshot tests of the editor at rest and with values changed.

## Close-out (2026-11-24)

- `crates/beverley/beverley.svg`: a 900 × 150 panel in 2U proportions.
  It has rack ears with oval slots and pan-head screws, and a dark
  anodised front with faint brushing (stretched `feTurbulence`). A title
  block (BEVERLEY, BITCRUSHER, VULPUS LABS, with a red stripe) sits
  beside one row of controls, split into groups by engraved dividers.
  Legends are Jost Medium, in silkscreen's runtime face. Every knob has
  a printed 270° scale with its values: GAIN 0–20 dB; GAMMA 0.1, 1, 10;
  BIT DEPTH 1, 2, 4, 8, 16 on its exponential taper; SMOOTHING 0, 20,
  100 %; MIX DRY–WET. Drawn from scratch. A scratch script computed the
  tick geometry; the SVG is the source.
- Controls: five vector knobs, a lit button for AUTO GAIN, a two-way
  bat switch for CRUSH MODE (up SYM, down ASYM), and a three-way bat
  switch for STEREO (up LINK, middle SEP, down MONO), using
  `ParamSelector`.
- Deviation: CRUSH MODE is now a toggle with labels (Asymmetric,
  Symmetric) instead of a two-position stepped param, so that
  `ParamToggle` drives the switch. Its id (6), its values (0 and 1) and
  the engine are unchanged.
- `tests/editor.rs`: snapshots at rest (1× and 2×), with every control
  moved, and with a tooltip. Each control moves its param, and hovering
  checks that every tooltip reads in its unit ("Gain 0.0 dB",
  "Bit Depth 4.0 bits", "Mix 100 %"). Snapshots are blessed on ARM64
  macOS, as CI's full job runs there.
