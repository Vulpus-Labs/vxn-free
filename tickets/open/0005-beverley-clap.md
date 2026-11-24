---
id: "0005"
title: "Beverley CLAP plugin"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0004"]
---

## Summary

The `beverley-clap` crate: Beverley as a stereo CLAP audio effect.

## Acceptance criteria

- [ ] `SilkscreenPlugin` with id `labs.vulpus.beverley`, name Beverley,
      features audio-effect, distortion, stereo; audio input; no licence
      check.
- [ ] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      beverley-clap` builds it.
- [ ] Tests with silkscreen-clap's test host: params, state round trip,
      no allocation while processing.
- [ ] Beverley must report its latency accurately. The CLAP `latency`
      extension's value equals the engine's measured latency (0003), and
      a test-host test checks them against each other at 44.1, 48 and
      96 kHz.
      silkscreen-clap reports a fixed `SilkscreenPlugin::LATENCY`; if
      the engine's latency is not a constant (it varies with rate or
      settings), silkscreen first gains per-instance latency with the
      host told when it changes (`request_restart` while deactivated, as
      CLAP requires), as a silkscreen release, before this closes.
- [ ] clap-validator passes.
- [ ] Hand check in a DAW: loads, sounds right, automation works.
