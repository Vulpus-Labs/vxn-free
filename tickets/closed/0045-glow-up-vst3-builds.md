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

- [x] The VST3 bundles and passes Steinberg's validator here.
- [x] `glow-up-clap` in the release workflow's packages.
- [x] A run by hand on main passes on all three platforms: every CLAP
      through clap-validator, every VST3 through Steinberg's validator.
- [x] The macOS CLAP and VST3 are signed, notarized and stapled, and
      universal.

## Close-out (2026-11-26)

- `cargo silkscreen bundle glow-up-clap --vst3 --validate` here:
  Steinberg's validator passed 47 of 47. Nothing in `glow-up-clap` needed
  changing: the VST3 comes from the same `[package.metadata.silkscreen.clap]`.
- `PACKAGES` in `.github/workflows/release.yml` gains `glow-up-clap`.
- Run 45882858836 by hand on main (3bb9430) passed on macOS, Windows and
  Linux. (An earlier run, 45881891927, started before the push landed and
  built 7f4e25e, without Glow-Up.) On each platform every VST3 passed
  Steinberg's validator (47 of 47) and every CLAP clap-validator (0
  failed: 33 passed, 11 skipped; 32 and 12 on Windows).
- The macOS zips, unpacked here with `ditto`: `Glow-Up.clap` and
  `Glow-Up.vst3` each "accepted, Notarized Developer ID" by Gatekeeper,
  stapled, hardened runtime, team 357M9JY6XD, x86_64 + arm64.
- Artifacts only (`glow-up-dev-3bb9430-{macos,windows,linux}.{clap,vst3}.zip`);
  no tag, nothing published.
