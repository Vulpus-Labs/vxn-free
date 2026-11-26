---
id: "0010"
title: "Glow-Up CLAP plugin with a plain faceplate"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0008"]
---

## Summary

Glow-Up in a DAW as soon as it plays (ADR 0005, point 10): the plugin
plumbing and a plain SVG grid of its controls, enough to listen and to
settle the ratios, ranges and defaults by ear. 0009 replaces the
faceplate.

## Acceptance criteria

- [x] `SilkscreenPlugin` with id `labs.vulpus.glow-up`, name Glow-Up,
      features audio-effect, filter; audio input; user presets; no
      licence check.
- [x] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      glow-up-clap` builds it.
- [x] A plain faceplate with every control.
- [x] Tests with the test host: params, state round trip, no allocation
      while processing.
- [x] Glow-Up must report its latency accurately. The CLAP `latency`
      extension's value equals the engine's measured latency (0008), and
      a test-host test checks them against each other at 44.1, 48 and
      96 kHz.
      silkscreen-clap reports a fixed `SilkscreenPlugin::LATENCY`; if
      the engine's latency is not a constant zero (it varies with rate or
      settings), silkscreen first gains per-instance latency with the
      host told when it changes (`request_restart` while deactivated, as
      CLAP requires), as a silkscreen release, before this closes.
- [x] clap-validator passes.
- [ ] Hand check in a DAW: guitar and synth sources, automation of FOCUS
      and RESONANCE, and the ratios by ear.

## Progress (2026-11-26)

Everything but the DAW check is done; this closes when you've listened.

- `crates/glow-up` gains the editor: `glow-up.svg`, the plain faceplate
  (720 × 436, drawn from scratch): boost A's and B's AMOUNT, NUM and DEN
  in two boxes along the top; WIDTH, RESONANCE, FOCUS, MIX, THRESHOLD,
  TONE and SPEED in a row; WINDOW, GRID, MODE and DRY CUT along the
  bottom. NUM, DEN and WINDOW are stepped knobs. The panel doesn't
  print the ratios; the host's parameter readout does (`2`, `3`).
- `crates/glow-up-clap`: `GlowUpPlugin`, `labs.vulpus.glow-up`, features
  audio-effect and filter; user presets; latency 0, a constant, so
  silkscreen needs no per-instance latency.
- Tests:
  - editor: a snapshot; every one of the 17 controls moves its param
  - params: all 17 listed with ids 1–17, ranges and defaults; values in
    their units (`25 %`, `2`, `50 cents`, `2048`, `IRR`, `Stereo`);
    both amounts off pass a sine pair bit for bit, and automating A to
    full boosts it, in the model too
  - state: a save round trip of all 17 comes back with its sound
  - latency: reported 0, and an impulse and a step come out on their
    own sample at 44.1, 48 and 96 kHz across five settings
  - no allocation in the process call, with param events
- `cargo silkscreen bundle glow-up-clap` builds `Glow-Up.clap`;
  clap-validator: 20 tests, 15 passed, 5 skipped, none failed.
- The release workflow doesn't build Glow-Up yet: it is added when
  Glow-Up is to be released.
