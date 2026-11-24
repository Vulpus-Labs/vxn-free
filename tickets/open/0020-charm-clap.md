---
id: "0020"
title: "Charm CLAP plugin"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0018"]
---

## Summary

The plugin plumbing, early, so Charm can be heard in a DAW from milestone
M1. It starts with a plain faceplate, which 0019 replaces. Release checks
are in 0021.

## Acceptance criteria

- [x] `SilkscreenPlugin` with id `labs.vulpus.charm`, name
      Charm, features audio-effect, chorus; stereo audio input and
      output (a mono input is accepted and fed to both channels); no
      licence check.
- [x] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      charm-clap` builds it.
- [x] Tests with the test host: params, state round trip, no allocation
      while processing, and output bit-identical across host block sizes.
- [x] Charm must report its latency accurately. It reports zero,
      and a test-host test confirms zero at 44.1, 48 and 96 kHz across
      settings.
- [x] clap-validator passes.
- [x] A plain faceplate: an original SVG grid of labelled knobs and
      switches, one per param, enough for silkscreen to build the editor.
      It is not the design. Params added later (0016–0018) are added to it
      in their own tickets.
- [ ] Loads and plays in a DAW: a quick listen on real material with
      automation of DETUNE and MIX.

## Progress (2026-11-24)

Everything but the DAW check is done; this closes when you've listened.

- Moved ahead of 0024, so tuning can happen in a DAW rather than from
  offline renders.
- `crates/charm` gains the editor:
  - `charm.svg`, the plain faceplate (720 × 300, drawn from scratch):
    the flock controls and MIX on top; WIDTH, AUTO WIDTH (a toggle),
    LOW XOVER, the depths, DUCK and BLOOM below
  - `build.rs`, `state`, `UI` and `editor`, as Beverley's
- `crates/charm-clap`:
  - `CharmPlugin`: `labs.vulpus.charm`; audio-effect, chorus, stereo;
    user presets; latency `CharmEngine::LATENCY` (0)
  - `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
    charm-clap` builds `Charm.clap`
- Tests:
  - params: all 14 listed with ids 1–14, ranges and defaults; values in
    their units; automation reaching audio and the model
  - state: a save round trip comes back with its sound
  - latency: reported 0 and measured 0 for an impulse and a step at
    44.1, 48 and 96 kHz across five settings
  - no allocation in process with param events
  - editor: a snapshot of the faceplate; every knob moves its param;
    AUTO WIDTH toggles
  - Output identical across host block sizes is covered by the engine's
    tests, since the plugin passes blocks straight through.
- clap-validator: 16 passed, 0 failed, 5 skipped (presets, note ports).
  The first run after a fresh bundle warned on scan time, as for
  Beverley (0005).
- Installed with `cargo silkscreen bundle charm-clap --install`.
