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
- [ ] clap-validator passes.
- [ ] Hand check in a DAW: loads, sounds right, automation works.
