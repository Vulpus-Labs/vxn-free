---
id: "0036"
title: "Wesley faceplate in Beverley's family"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0034"]
---

## Summary

Wesley's real faceplate, in the 1990s rack style of Beverley's (ticket
0013), drawn from scratch (ADR 0003, point 10).

## Acceptance criteria

- [x] Three band strips and the shared section; controls that do
      nothing in the current state dimmed (as Beverley's GAIN under
      AUTO GAIN).
- [x] Snapshot tests; every control moves its param.

## Close-out (2026-11-25)

Drawn before tuning (0035), at the user's request; tuning may still
change the controls, and the script redraws the panel in a minute.

- `art/wesley/faceplate.py` writes `crates/wesley/wesley.svg` and the
  relief's geometry; Beverley's `blender/panel.py` renders the relief
  (2 min 20 s: a 1444 × 430 panel). Wine red (`#561019`), a gold title
  stripe, Beverley's legends and scales.
- Layout: each band a row laid out as Beverley's strip — name, ON and
  SOLO (LED buttons); GAIN, AUTO, DETECT, DECAY; GAMMA, BIT DEPTH,
  SMOOTHING; its crush display (an LCD in a recessed window, as
  Beverley's, `CrushStyle::LCD`); CRUSH, LEVEL, MIX. The rows share
  columns, so the engraved dividers run the panel's full height (the
  relief splits its face only along vertical grooves); printed rules
  part the rows. The first column holds the title, LOW X and HIGH X,
  STEREO and the global MIX. Three screws an ear.
- The controls are Beverley's skins (`build.rs` registers
  `crates/beverley/art/controls/`), so the two plugins' knobs and
  switches match.
- With it: per-band DETECT and DECAY (ADR 0003, point 5), and solo as
  one SOLO param shown as an LED button a band, one lit at a time
  (point 6). Ids renumbered (unreleased): 11 a band, 1–33; LOW XOVER,
  HIGH XOVER, STEREO, MIX, SOLO 34–38.
- Dimming: a band's crush controls dim while it is off; its GAIN while
  AUTO is on; its DETECT and DECAY while AUTO is off.
- Tests: snapshots `initial_1x` and `mid_soloed_1x`; every control moves
  its param (the three SOLO buttons all SOLO); one solo lit at a time,
  and pressing the lit one clears it; LOW's row undims when switched
  on; a display repaints only for its own band.

## Amended (2026-11-25): bands as columns

The first panel (each band a row, 1444 × 430) was too wide. Redrawn
closer to the plain faceplate: a column a band (300 px), Beverley's strip
folded into three rows under the band's name, ON and SOLO and its LCD;
the title, LOW X, HIGH X, STEREO and MIX in a first column (220 px).
1192 × 560. The dividers between columns still run the full height, so
the relief needed no change; re-rendered (2 min 40 s). Snapshots
re-blessed; every editor test passes unchanged.
