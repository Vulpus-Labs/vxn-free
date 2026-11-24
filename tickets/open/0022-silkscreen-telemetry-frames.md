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
- up to 8 peaks per band, and each band's gate

about 100 f32 values in all, as one consistent snapshot. Silkscreen's
telemetry carries 16 separate engine values, written one at a time. The
library gains telemetry frames, made and released in `~/src/silkscreen`
by its own process, and this repo moves to the new tag.

## Acceptance criteria

- [ ] In silkscreen, released as a new tag:
      - a plugin declares a frame size at build time
      - the engine writes a whole frame, without allocating or locking
      - the UI reads the latest complete frame and never sees a torn one
      - works on both native and web hosts
      - covered by silkscreen's own tests and docs
- [ ] The root `Cargo.toml` here moves every silkscreen crate to the new
      tag, with `rust-toolchain.toml` matched to silkscreen's at that tag.
      The silkscreen-tool install line in CLAUDE.md is updated too.
- [ ] The workspace passes the full verification suite on the new tag.
      Beverley's snapshots are unchanged, or re-blessed with the reason
      recorded.
- [ ] Charm's engine publishes a frame per flock tick. A test reads
      frames while the engine runs and checks that every frame is
      complete.
