---
id: "0001"
title: "Workspace on silkscreen v0.2.1, vxn-free-dsp, CI"
priority: medium
created: 2026-11-24
epic: E001
depends: []
---

## Summary

The Cargo workspace the plugins live in.

## Acceptance criteria

- [x] Root `Cargo.toml`: every silkscreen crate from
      `https://github.com/Vulpus-Labs/silkscreen` at tag `v0.2.1`; clack
      at the rev silkscreen-clap uses; workspace lints as silkscreen's;
      `build-override` opt-level 3; a commented `[patch]` to
      `../silkscreen` for working across both.
- [x] `crates/vxn-free-dsp`, empty but documented, building for
      `wasm32-unknown-unknown`.
- [x] `.cargo/config.toml` with the `silkscreen` alias for
      `cargo silkscreen` if useful.
- [x] CI on push and pull request: fmt, clippy `-D warnings`, tests,
      docs `-D warnings`, the dsp crate for wasm32; macOS full, Linux and
      Windows clippy and tests.
- [x] CLAUDE.md gains the verification list.

## Close-out (2026-11-24)

- Root `Cargo.toml`: workspace with `crates/vxn-free-dsp`; silkscreen,
  -bridge, -build, -clap, -harness and -host-native at `v0.2.1`; clack
  at `f874e858`, the rev silkscreen-clap pins at that tag; silkscreen's
  lints; `build-override` opt-level 3 for dev and release; a commented
  `[patch]` to `../silkscreen`. All of these resolved together in a
  scratch crate (one clack in the lock).
- `crates/vxn-free-dsp`: empty, with crate docs stating its rules
  (±1.0 audio, host-rate coefficients, no allocation or locking, no host
  or UI dependencies). Builds for `wasm32-unknown-unknown`.
- `.cargo/config.toml`: `+simd128` for wasm32, as silkscreen.
  Deviation: no `silkscreen` alias. `cargo run -p silkscreen-tool` only
  reaches workspace members, not git dependencies, so CLAUDE.md instead
  gives the `cargo install --git … --tag v0.2.1 silkscreen-tool` line.
- `.github/workflows/ci.yml`: macOS full list (fmt, clippy `-D warnings`,
  tests, docs `-D warnings`, dsp for wasm32); Ubuntu and Windows clippy
  and tests, with the Linux packages silkscreen's hosts need.
- CLAUDE.md: a Verification section. All of it passes locally; CI runs on
  first push.
