---
id: "0046"
title: "Manuals: Steinberg's VST logo, ® and trademark line"
priority: medium
created: 2026-11-27
epic: none
depends: []
---

## Summary

The user (2026-11-27): fix up and republish the manuals. Steinberg's
VST Usage Guidelines, which ship with the VST3 SDK, ask product
documentation for the VST Compatible logo, ® on the first "VST" and the
trademark line. The manuals had the line only.

## Acceptance criteria

- [x] `vst-logo` and `vst-trademark` in the template; the logo in
      `docs/manual/` (ADR 0003, point 4).
- [x] Beverley's, Glow-Up's and Wesley's title pages show the logo and
      read "CLAP and VST®3 plugin"; their reference uses `vst-trademark`.
- [x] Rebuilt and published on vulpuslabs.com.

## Close-out (2026-11-27)

All three rebuilt with `build.sh`; no CSV in `data/` changed. Wesley's
manual also takes the user's clarification that every band off with each
LEVEL at 0 dB, or MIX at DRY, is a clean bypass. Published to the site's
manual pages (10, 9 and 8 pages), dated today.
