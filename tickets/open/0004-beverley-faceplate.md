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

- [ ] Looks like a rack unit: a wide, short landscape panel with rack
      ears and their screw slots, a painted or brushed front, controls in
      a row with silk-screened labels and scales.
- [ ] Faceplate SVG drawn from scratch: four knobs (GAIN, GAMMA, BIT
      DEPTH, SMOOTHING), the AUTO GAIN toggle, the CRUSH MODE switch, the
      three-way STEREO switch (MONO, SEPARATE, LINKED),
      MIX; labels in silkscreen's print style.
- [ ] Bindings to the engine's parameters; values shown in units (dB,
      bits) on hover.
- [ ] Snapshot tests of the editor at rest and with values changed.
