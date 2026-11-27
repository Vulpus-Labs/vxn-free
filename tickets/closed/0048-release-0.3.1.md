---
id: "0048"
title: "Release 0.3.1: Beverley and Wesley with the crush display fix"
priority: medium
created: 2026-11-27
epic: none
depends: ["0047"]
---

## Summary

The user (2026-11-27): rebuild in CI and publish, with the downloads and
the site's links updated. 0047 changed the crush display both Beverley
and Wesley draw; Glow-Up is unchanged since 0.3.0.

## Acceptance criteria

- [x] The workspace at 0.3.1, tag `v0.3.1`; the release workflow green
      on every platform, with a draft release of the zips.
- [x] `beverley/0.3.1` and `wesley/0.3.1` on downloads.vulpuslabs.com
      with `SHA256SUMS`; fetched back, they match, and the macOS
      bundles, quarantined, pass Gatekeeper.
- [x] vulpuslabs.com's Beverley and Wesley pages link 0.3.1, and
      Beverley's manual on the site says the display's tone is 3 dB
      below full level.

## Close-out (2026-11-27)

- Tag `v0.3.1`, run 46003367787: every platform green, and a draft
  GitHub release of the eighteen zips. The four macOS bundles (Beverley
  and Wesley, CLAP and VST3): universal, Gatekeeper `Notarized Developer
  ID`.
- `beverley/0.3.1` and `wesley/0.3.1` on downloads.vulpuslabs.com: six
  zips and `SHA256SUMS` each. All twelve fetched back match, and a
  quarantined download of the Wesley VST3 passes Gatekeeper. Glow-Up at
  0.3.1 is unchanged from 0.3.0 and was not published.
- vulpuslabs.com (vulpus-labs-site f3ec802): Beverley's and Wesley's
  pages link 0.3.1; Beverley's manual on the site is the rebuilt one.
  Wesley's manual doesn't give the display's level and was not rebuilt.
