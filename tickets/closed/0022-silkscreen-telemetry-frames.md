---
id: "0022"
title: "silkscreen telemetry frames: release and bump"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0014"]
---

## Summary

Charm's flock display (ADR 0002, point 10) needs, per tick:
- two flocks × 8 voices × (delay, pan, velocity, gain)
- each band's onset and coherence

about 70 f32 values in all, as one consistent snapshot. Silkscreen's
telemetry carries 16 separate engine values, written one at a time. The
library gains telemetry frames, made and released in `~/src/silkscreen`
by its own process, and this repo moves to the new tag.

## Acceptance criteria

- [x] In silkscreen, released as a new tag:
      - a plugin declares a frame size at build time
      - the engine writes a whole frame, without allocating or locking
      - the UI reads the latest complete frame and never sees a torn one
      - works on both native and web hosts
      - covered by silkscreen's own tests and docs
- [x] The root `Cargo.toml` here moves every silkscreen crate to the new
      tag, with `rust-toolchain.toml` matched to silkscreen's at that tag.
      The silkscreen-tool install line in CLAUDE.md is updated too.
- [x] The workspace passes the full verification suite on the new tag.
      Beverley's snapshots are unchanged, or re-blessed with the reason
      recorded.
- [x] Charm's engine publishes a frame per flock tick. A test reads
      frames while the engine runs and checks that every frame is
      complete.

## Close-out (2026-11-25)

- silkscreen ticket 0020, released as **v0.3.0** (breaking, so a minor
  bump):
  - `PluginUi::frame` declares the frame's size in words at build time;
    the CLAP layer (`bridge_with_frame`) and the web page
    (`connect(initial, frame)`, `SabLayout::with_frame`, shared-buffer
    layout version 4) make room for it.
  - `Engine::frame(write)`, called once per block after `process` by
    every host's `write_block`; `EditorState::engine_frame()` returns
    the latest whole frame and a generation.
  - A seqlock over two buffers at the end of the telemetry block: the
    tap writes the other buffer and publishes a sequence number, never
    waiting; the reader copies with acquire loads and checks the
    sequence again, retrying up to four times or keeping the frame it
    has. Its race test fails within ~1000 frames if the check is
    removed.
  - Documented in silkscreen's `design/engine-bridge.md` and CHANGELOG.
- Here: every silkscreen crate on `v0.3.0`; `rust-toolchain.toml`
  already matched (1.95); CLAUDE.md's silkscreen-tool line moved to
  v0.3.0. Full verification passes; Beverley's snapshots are unchanged
  (Beverley declares `frame: 0`).
- Charm's frame is `observe::FRAME_WORDS` (71 words): the tick modulo
  2^24, then per band the active count, onset, coherence and 8 voices ×
  (delay, pan, velocity, gain). `Observation::to_frame` and
  `from_frame` encode it; `onset_start` stays on the audio side.
- Deviation: the engine can't write telemetry mid-block, so it publishes
  once per block, and only when a new tick has run since the last frame.
  The display only ever wants the newest tick, so nothing it would draw
  is lost.
- Tests (`crates/charm/tests/frames.rs`): one frame per new tick, holding
  the latest observation; the engine on its own thread publishing
  through a real bridge while the editor's end polls flat out: 3000
  blocks, every frame taken is bit-for-bit the one encoded for its tick,
  in order, ending on the last tick. `frame` added to the no-allocation
  test.
