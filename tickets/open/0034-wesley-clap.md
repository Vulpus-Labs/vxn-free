---
id: "0034"
title: "Wesley CLAP plugin with a plain faceplate"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0033"]
---

## Summary

Wesley in a DAW as soon as it plays (ADR 0004, point 9): the plugin
plumbing and a plain SVG grid of its controls, enough to listen. 0036
replaces the faceplate.

## Acceptance criteria

- [x] `SilkscreenPlugin`, id `labs.vulpus.wesley`, name Wesley, features
      audio-effect, distortion, stereo; stereo in and out;
      user presets; no licence.
- [x] `cargo silkscreen bundle wesley-clap` builds it; clap-validator
      passes.
- [x] Test-host tests: params, state round trip, latency 0 at 44.1, 48
      and 96 kHz, no allocation.
- [x] A plain faceplate: three band columns and the shared controls.
- [ ] Loads and plays in a DAW.

## Progress (2026-11-25)

Everything but the DAW check is done; this closes when you've listened.

- `crates/wesley` gains the editor: `wesley.svg`, the plain faceplate
  (960 × 540, drawn from scratch): LOW, MID and HIGH as three columns
  (GAIN, GAMMA, BIT DEPTH; SMOOTHING, MIX, LEVEL; AUTO, SYMM, ON, SOLO
  switches), and the shared LOW XOVER, HIGH XOVER, DECAY, DETECT, STEREO
  and MIX along the bottom. A band's crush controls dim while it is off
  (LOW, at the defaults), and its GAIN while AUTO GAIN is on.
- `crates/wesley-clap`: `WesleyPlugin`, `labs.vulpus.wesley`, features
  audio-effect, distortion, stereo (CLAP has no standard multiband
  feature; ADR 0004 corrected); user presets; latency 0.
- Tests:
  - editor: a snapshot; every one of the 36 controls moves its param;
    LOW's strip undims when it is switched on
  - params: all 36 listed with ids 1–36, ranges and defaults; values in
    their units; automation of MID and HIGH ON reaching audio (the
    split passes clean, within 1e-5) and the model
  - state: a save round trip comes back with its sound
  - latency: reported 0, and an impulse and a step come out on their
    own sample at 44.1, 48 and 96 kHz across five settings (crossovers
    at their extremes among them)
  - no allocation in process with param events
- clap-validator: 15 passed, 0 failed, 5 skipped; the first run after a
  fresh bundle warns on scan time, as for Beverley and Charm, and a
  second is clean (16 passed, 0 warnings).
- Installed with `cargo silkscreen bundle wesley-clap --install`.

Later (2026-11-25): the plain faceplate is replaced by 0036's rack panel;
params are renumbered there (38). The plugin tests follow.
