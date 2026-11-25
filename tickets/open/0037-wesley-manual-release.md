---
id: "0037"
title: "Wesley manual and release"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0036"]
---

## Acceptance criteria

Revised 2026-11-25: the manual is a PDF built as Beverley's is (ADR
0003), not `docs/wesley.md`.

- [x] `crates/wesley/manual/`: `build.sh`, `wesley-manual.typ` and
      `data/`, from `cargo run -p wesley --example manual`: what it
      does, each control, starting points, for players.
- [ ] Trademark search in class 9 (ADR 0004, point 1).
- [x] The release workflow builds `wesley-clap` (CLAP and VST3, signed
      and notarized on macOS) beside Beverley.
- [x] Wesley 0.2.0 (and Beverley 0.2.0) on downloads.vulpuslabs.com,
      linked from vulpuslabs.com, with the manual.

## Progress (2026-11-25)

- The workspace is at 0.2.0, so Beverley's display and teal-blue panel
  ship with Wesley. `PACKAGES` in the release workflow names
  `wesley-clap`.
- Tag `v0.2.0`, run 45722183529: every platform green. Steinberg's
  validator passed 47 of 47 for each VST3, clap-validator failed none,
  and all four macOS bundles are notarized and stapled, universal, and
  pass Gatekeeper as `Notarized Developer ID`. A draft GitHub release
  holds the twelve zips.
- The manual (eight A4 pages): quick start with every control numbered;
  the band split, measured from an impulse through the engine at three
  crossover settings (each band soloed, and their flat sum); a band's
  controls, pointing to Beverley's manual for the crush graphs; per-band
  DECAY on a 55 Hz note; why LOW starts off, from a bass and lead at the
  defaults and with LOW on (what the crush adds between 600 Hz and 4 kHz
  rises from −52 to −35 dB); stereo and mix; starting points; tips; and
  a reference with each band's defaults side by side.
- Measuring showed that a short DECAY on a bass note doesn't distort it,
  as first drafted: the gain swells between the peaks and the steps
  shrink until the bass is barely crushed. The manual says so.
- Web searches (2026-11-25) found no class 9 mark for Wesley on audio
  software; that isn't a register search, so the box stays open.
- The DAW check (0034) and tuning by ear (0035) are still open; 0.2.0
  ships the draft ranges and defaults.

## Published (2026-11-25)

- `beverley/0.2.0` and `wesley/0.2.0` on downloads.vulpuslabs.com, six
  zips and `SHA256SUMS` each. All twelve fetched back match, and the
  quarantined macOS CLAPs and VST3s pass Gatekeeper as `Notarized
  Developer ID`.
- vulpuslabs.com (vulpus-labs-site 837ef9e): Wesley's product page and
  manual; Beverley's page and manual at 0.2.0 with the new panel.
- Open until the class 9 trademark search is done.
