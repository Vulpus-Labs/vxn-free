---
id: "0027"
title: "Beverley as a VST3, released signed and notarized"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0002"]
---

## Summary

Release Beverley as a VST3 beside the CLAP, built by silkscreen's
`cargo silkscreen bundle --vst3` (silkscreen 0008, tag v0.2.2), signed
and notarized on macOS as the CLAP is, and published to
downloads.vulpuslabs.com.

## Acceptance criteria

- [x] The workspace on silkscreen v0.2.2.
- [x] The release workflow builds `Beverley.vst3` on macOS (universal,
      signed and notarized), Windows and Linux, runs Steinberg's validator
      on each, and zips them as `beverley-<version>-<os>.vst3.zip` beside
      the CLAP zips.
- [x] Beverley 0.1.1 released: CLAP and VST3 for all three platforms on
      downloads.vulpuslabs.com, linked from its page.

## Close-out (2026-11-24)

- silkscreen 0008 made `cargo silkscreen bundle --vst3 [--validate]`:
  the plugin's library linked statically into free-audio's clap-wrapper
  (v0.18.0, VST3 SDK 3.8.0), so the `.vst3` stands alone. Released in
  silkscreen v0.2.2, after CI bundled and validated its gain example on
  macOS, Ubuntu and Windows.
- The workspace is on v0.2.2 and at 0.1.1. The release workflow bundles
  with `--vst3 --validate` (installing GTK 3 and gtkmm 3 on Linux for the
  validator) and zips each `.vst3` beside its `.clap`. ADR 0001's targets
  now name VST3, and the README notes the SDK's licence and trademark.
- Released from branch `release/0.1.1` (c653c8a): 0027 alone on
  origin/main, without the Charm commits on local main, which aren't
  public yet (the user's choice). Tag `v0.1.1`, run 45444451082:
  - Steinberg's validator passed 47 of 47 on each platform; every CLAP
    passed clap-validator.
  - The macOS CLAP and VST3 are each notarized and stapled, and the
    VST3 is universal.
- Published to `https://downloads.vulpuslabs.com/beverley/0.1.1/` (six
  zips and `SHA256SUMS`). The files fetched back match, and the
  quarantined macOS VST3 passes Gatekeeper. Linked from vulpuslabs.com
  (vulpus-labs-site 3ad89f2).
- Still to do in a DAW (0005 and silkscreen 0008): load, play, automate
  and save, in a CLAP host and in a VST3-only one.
