---
id: "0041"
title: "Beverley's panel in dark teal-blue"
priority: low
created: 2026-11-25
epic: E002
depends: ["0039"]
---

## Summary

Beverley's panel is painted dark teal-blue rather than charcoal, and
Wesley's (0036) will be wine red: the two plugins as a pair.

## Acceptance criteria

- [x] The paint changed in `art/beverley/faceplate.py`; snapshots
      re-blessed.

## Close-out (2026-11-25)

- `PAINT` `#232427` → `#0d3550`. A first try, `#1d3549`, read as slate
  under the relief maps (the sheen lifts and greys the paint); the
  deeper, more saturated one reads as the intended teal-blue. The
  legends, the orange title stripe and the LCD keep their colours.
- The relief maps don't depend on the paint: no Blender render.
- Snapshots re-blessed; every editor assertion passes unchanged.
