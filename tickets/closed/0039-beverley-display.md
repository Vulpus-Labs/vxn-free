---
id: "0039"
title: "Beverley's crush display: an LCD window in the rack panel"
priority: medium
created: 2026-11-25
epic: E002
depends: ["0038"]
---

## Summary

The crush display (0038) in Beverley's faceplate, as a backlit LCD
window in its own section of the 1990s rack panel.

## Acceptance criteria

- [x] `art/beverley/faceplate.py` gains the section and window; the
      panel relief re-rendered in Blender.
- [x] The display in an LCD style that suits the panel.
- [x] Snapshots re-blessed with the reason; every control still moves
      its param.

## Close-out (2026-11-25)

- `art/beverley/faceplate.py`: the panel widens from 1090 to 1290. A new
  section, "1 CYCLE", sits after GAMMA, BIT DEPTH and SMOOTHING (the
  display shows their result) and before CRUSH and STEREO: a 160 × 84
  window with a dark bezel, and the canvas (`canvas.display`) inside it.
  CRUSH, STEREO and MIX move right by 200.
- The relief gains `windows` (geometry.json): `blender/panel.py` cuts
  each 1.6 units into the face with 3-unit corners, so the window reads
  as a recess with the panel's bevel round it. Re-rendered (48 s).
  Blender warns that some cutters are non-manifold while it evaluates
  the cuts; the panel before this change gives the same warnings, and
  the maps come out right.
- `CrushStyle::LCD` in `vxn-free-ui`: a backlit yellow-green LCD, a dark
  trace, the sine faint under it. The display takes no input
  (`Unbound`).
- Snapshots re-blessed: every one changes because the panel is wider.
  Every editor test's assertions pass unchanged (controls move their
  params, tooltips, AUTO lighting its controls and dimming GAIN).
- clap-validator passes; installed.
