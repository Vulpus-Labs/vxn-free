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

- [ ] `SilkscreenPlugin` with id `labs.vulpus.glow-up`, name Glow-Up,
      features audio-effect, filter; audio input; user presets; no
      licence check.
- [ ] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      glow-up-clap` builds it.
- [ ] A plain faceplate with every control.
- [ ] Tests with the test host: params, state round trip, no allocation
      while processing.
- [ ] Glow-Up must report its latency accurately. The CLAP `latency`
      extension's value equals the engine's measured latency (0008), and
      a test-host test checks them against each other at 44.1, 48 and
      96 kHz.
      silkscreen-clap reports a fixed `SilkscreenPlugin::LATENCY`; if
      the engine's latency is not a constant zero (it varies with rate or
      settings), silkscreen first gains per-instance latency with the
      host told when it changes (`request_restart` while deactivated, as
      CLAP requires), as a silkscreen release, before this closes.
- [ ] clap-validator passes.
- [ ] Hand check in a DAW: guitar and synth sources, automation of FOCUS
      and RESONANCE, and the ratios by ear.
