---
id: "0020"
title: "Charm CLAP plugin"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0024"]
---

## Summary

The plugin plumbing, early, so Charm can be heard in a DAW from milestone
M1. It starts with a plain faceplate, which 0019 replaces. Release checks
are in 0021.

## Acceptance criteria

- [ ] `SilkscreenPlugin` with id `labs.vulpus.charm`, name
      Charm, features audio-effect, chorus; stereo audio input and
      output (a mono input is accepted and fed to both channels); no
      licence check.
- [ ] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      charm-clap` builds it.
- [ ] Tests with the test host: params, state round trip, no allocation
      while processing, and output bit-identical across host block sizes.
- [ ] Charm must report its latency accurately. It reports zero,
      and a test-host test confirms zero at 44.1, 48 and 96 kHz across
      settings.
- [ ] clap-validator passes.
- [ ] A plain faceplate: an original SVG grid of labelled knobs and
      switches, one per param, enough for silkscreen to build the editor.
      It is not the design. Params added later (0016–0018) are added to it
      in their own tickets.
- [ ] Loads and plays in a DAW: a quick listen on real material with
      automation of DETUNE and MIX.
