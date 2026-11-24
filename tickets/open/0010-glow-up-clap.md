---
id: "0010"
title: "Glow-Up CLAP plugin"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0009"]
---

## Acceptance criteria

- [ ] `SilkscreenPlugin` with id `labs.vulpus.glow-up`, name Glow-Up,
      features audio-effect, filter; audio input; no licence check.
- [ ] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      glow-up-clap` builds it.
- [ ] Tests with the test host: params, state round trip, no allocation
      while processing.
- [ ] clap-validator passes.
- [ ] Hand check in a DAW: guitar and synth sources, automation of FOCUS
      and RESONANCE.
