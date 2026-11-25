---
id: "0034"
title: "Wesley CLAP plugin with a plain faceplate"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0033"]
---

## Summary

Wesley in a DAW as soon as it plays (ADR 0003, point 9): the plugin
plumbing and a plain SVG grid of its controls, enough to listen. 0036
replaces the faceplate.

## Acceptance criteria

- [ ] `SilkscreenPlugin`, id `labs.vulpus.wesley`, name Wesley, features
      audio-effect, distortion, multiband, stereo; stereo in and out;
      user presets; no licence.
- [ ] `cargo silkscreen bundle wesley-clap` builds it; clap-validator
      passes.
- [ ] Test-host tests: params, state round trip, latency 0 at 44.1, 48
      and 96 kHz, no allocation.
- [ ] A plain faceplate: three band columns and the shared controls.
- [ ] Loads and plays in a DAW.
