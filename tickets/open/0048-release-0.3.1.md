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

- [ ] The workspace at 0.3.1, tag `v0.3.1`; the release workflow green
      on every platform, with a draft release of the zips.
- [ ] `beverley/0.3.1` and `wesley/0.3.1` on downloads.vulpuslabs.com
      with `SHA256SUMS`; fetched back, they match, and the macOS
      bundles, quarantined, pass Gatekeeper.
- [ ] vulpuslabs.com's Beverley and Wesley pages link 0.3.1, and
      Beverley's manual on the site says the display's tone is 3 dB
      below full level.
