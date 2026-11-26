---
id: "0009"
title: "Glow-Up faceplate, editor and tracking display"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0010"]
---

## Acceptance criteria

- [ ] Looks like a guitar pedal: a portrait enclosure, painted, with its
      edges and screws showing; the main knobs (A and B AMOUNT,
      RESONANCE, FOCUS, MIX) in pedal-style rows, each boost's NUMERATOR
      and DENOMINATOR beside its AMOUNT; the rest smaller, as trim
      controls would be; a footswitch-style button and status LED for
      something that earns it (bypass, if the host contract allows, or
      DRY CUT).
- [ ] Faceplate SVG drawn from scratch with the controls in 0008,
      replacing 0010's plain one.
- [ ] A display of what the engine is doing: the found peaks and the
      filters' targets and levels, fed by bridge telemetry.
- [ ] Hover text as the module's (for example `A: 2/1, the octave above
      each peak, 25%`).
- [ ] Snapshot tests, the display included.
