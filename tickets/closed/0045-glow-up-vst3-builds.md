---
id: "0045"
title: "Glow-Up as a VST3, built for every platform, signed on macOS"
priority: medium
created: 2026-11-26
epic: E003
depends: ["0010"]
---

## Summary

Glow-Up beside Beverley and Wesley in the release workflow: a CLAP and a
VST3 (silkscreen's `--vst3`, clap-wrapper) on macOS (universal, signed
and notarized), Windows and Linux, each validated. A run by hand, not a
release: no tag, nothing published.

## Acceptance criteria

- [ ] The VST3 bundles and passes Steinberg's validator here.
- [ ] `glow-up-clap` in the release workflow's packages.
- [ ] A run by hand on main passes on all three platforms: every CLAP
      through clap-validator, every VST3 through Steinberg's validator.
- [ ] The macOS CLAP and VST3 are signed, notarized and stapled, and
      universal.
