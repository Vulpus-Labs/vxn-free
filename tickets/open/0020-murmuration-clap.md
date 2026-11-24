---
id: "0020"
title: "Murmuration CLAP plugin"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0019"]
---

## Acceptance criteria

- [ ] `SilkscreenPlugin` with id `labs.vulpus.murmuration`, name
      Murmuration, features audio-effect, chorus; stereo audio input and
      output (a mono input is accepted and fed to both channels); no
      licence check.
- [ ] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      murmuration-clap` builds it.
- [ ] Tests with the test host: params, state round trip, no allocation
      while processing, and output bit-identical across host block sizes.
- [ ] Murmuration must report its latency accurately. It reports zero,
      and a test-host test confirms zero at 44.1, 48 and 96 kHz across
      settings.
- [ ] clap-validator passes.
- [ ] Hand check in a DAW: pads, clean guitar, vocals and a drum bus;
      automation of DETUNE, WIDTH and MIX; mono sum checked.
