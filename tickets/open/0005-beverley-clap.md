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

- [x] `SilkscreenPlugin` with id `labs.vulpus.beverley`, name Beverley,
      features audio-effect, distortion, stereo; audio input; no licence
      check.
- [x] `[package.metadata.silkscreen.clap]`; `cargo silkscreen bundle
      beverley-clap` builds it.
- [x] Tests with silkscreen-clap's test host: params, state round trip,
      no allocation while processing.
- [x] Beverley must report its latency accurately. The CLAP `latency`
      extension's value equals the engine's measured latency (0003), and
      a test-host test checks them against each other at 44.1, 48 and
      96 kHz.
      silkscreen-clap reports a fixed `SilkscreenPlugin::LATENCY`; if
      the engine's latency is not a constant (it varies with rate or
      settings), silkscreen first gains per-instance latency with the
      host told when it changes (`request_restart` while deactivated, as
      CLAP requires), as a silkscreen release, before this closes.
- [x] clap-validator passes.
- [ ] Hand check in a DAW: loads, sounds right, automation works.

## Progress (2026-11-24)

Everything but the DAW check has landed. The ticket stays open until
that is done.

- `crates/beverley-clap`: `BeverleyPlugin`, id `labs.vulpus.beverley`,
  features audio-effect, distortion and stereo, with an audio input, user
  presets and no licence. `LATENCY` is `BeverleyEngine::LATENCY` (0).
  The engine's latency is a constant (0003), so silkscreen needed no
  per-instance latency.
- `cargo silkscreen bundle beverley-clap` builds
  `target/bundled/Beverley.clap` (silkscreen-tool from tag v0.2.1).
- 0012 added DECAY (id 9) and DETECT (id 10); the tests below cover
  them.
- Tests through the test host:
  - `params`: ids 1–8, names, ranges and defaults as `PARAMS`; text in
    units both ways; automation reaching the audio and the model.
  - `state`: save, change every param, load; values and sound come
    back.
  - `no_alloc`: eight process calls with param events, none allocating.
  - `latency`: at 44.1, 48 and 96 kHz the latency extension reports
    `BeverleyEngine::LATENCY`. Impulses and steps through the plugin, at
    five settings, land on the sample it reports.
- clap-validator 0.3.2: 16 passed, 5 skipped (preset discovery and note
  ports, which Beverley doesn't have), 0 failed. The first run after
  each fresh bundle warns on `scan-time` (416 ms against 100 ms): the
  first load of a new unsigned binary. Every later run is clean (21
  tests, no warnings). Worth checking again once 0002 signs the bundle.
- Hand check in a DAW: to do. `cargo silkscreen bundle beverley-clap
  --install` puts the plugin in the user's CLAP folder.
