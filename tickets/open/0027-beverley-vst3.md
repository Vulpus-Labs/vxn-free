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
- [ ] The release workflow builds `Beverley.vst3` on macOS (universal,
      signed and notarized), Windows and Linux, runs Steinberg's validator
      on each, and zips them as `beverley-<version>-<os>.vst3.zip` beside
      the CLAP zips.
- [ ] Beverley 0.1.1 released: CLAP and VST3 for all three platforms on
      downloads.vulpuslabs.com, linked from its page.
