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

- [ ] Root `Cargo.toml`: every silkscreen crate from
      `https://github.com/Vulpus-Labs/silkscreen` at tag `v0.2.1`; clack
      at the rev silkscreen-clap uses; workspace lints as silkscreen's;
      `build-override` opt-level 3; a commented `[patch]` to
      `../silkscreen` for working across both.
- [ ] `crates/vxn-free-dsp`, empty but documented, building for
      `wasm32-unknown-unknown`.
- [ ] `.cargo/config.toml` with the `silkscreen` alias for
      `cargo silkscreen` if useful.
- [ ] CI on push and pull request: fmt, clippy `-D warnings`, tests,
      docs `-D warnings`, the dsp crate for wasm32; macOS full, Linux and
      Windows clippy and tests.
- [ ] CLAUDE.md gains the verification list.
