---
id: "0011"
title: "Glow-Up manual and release"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0009"]
---

## Acceptance criteria

Revised 2026-11-26: the manual is a PDF built as Beverley's and
Wesley's are (ADR 0003), not `docs/glow-up.md`; and the release is part
of this ticket, as Wesley's was (0037).

- [x] `crates/glow-up/manual/`: `build.sh`, `glow-up-manual.typ` and
      `data/`, from `cargo run -p glow-up --example manual`: what it
      does, each control, starting points, for players.
- [x] Linked from the README.
- [x] Glow-Up 0.3.0 on downloads.vulpuslabs.com, linked from
      vulpuslabs.com, with the manual.

## Progress (2026-11-26)

- The manual (nine A4 pages): quick start with every control numbered;
  what Glow-Up does; the boosts, with a table of ratios and how much
  each partial of a held 200 Hz saw is raised at 2/1, 3/2, 3/1, 5/4
  and 1/2; FOCUS on a note bent from 200 to 300 Hz (filter centres
  every 25 ms at GRID, 50% and TRACK); GRID's three grids; RESONANCE,
  WIDTH, TONE and THRESHOLD each at three settings on the held saw;
  SPEED as the boosts' onset at four settings; the screen; output and
  mode; starting points; tips; a reference.
- The face is drawn with the engine running: a second of a G3 saw at
  the defaults, a frame fed to the scope every 10 ms.
- What measuring showed, and the manual says:
  - at RESONANCE 50% the octave boosts rise by up to another 10 dB and
    the odd partials between them dip by up to 6 dB; at 100% the
    singing filters are held to a share of their partial, so the low
    boosts are no louder than at 0, and the upper ones (10th, 12th)
    rise by 8 to 9 dB;
  - at SPEED 80% a boost begins about 40 ms after a note and is within
    1 dB of full 60 ms later; at 0 it takes about half a second; at
    100% it overshoots by 1.5 dB;
  - at THRESHOLD 30 dB a held saw makes no peaks at all;
  - TONE left weakens the high boosts but can't lift the low ones,
    already at full strength.
- The workspace moves to 0.3.0 for Glow-Up's first release. Beverley
  and Wesley are rebuilt at 0.3.0 by the tag but not republished: they
  haven't changed since 0.2.0.
- The DAW check (0010) and the faceplate ticket (0009) are still open;
  0.3.0 ships the draft ranges and defaults.

## Close-out (2026-11-26)

- Tag `v0.3.0`, run 45904661325: every platform green, and a draft
  GitHub release of the eighteen zips. The macOS Glow-Up CLAP and VST3,
  unpacked with `ditto`: Gatekeeper `Notarized Developer ID`, stapled,
  x86_64 + arm64.
- `glow-up/0.3.0` on downloads.vulpuslabs.com: six zips and
  `SHA256SUMS`. All six fetched back match, and a quarantined download
  of the macOS VST3 passes Gatekeeper. Beverley and Wesley at 0.3.0 are
  unchanged from 0.2.0 and were not published.
- vulpuslabs.com (vulpus-labs-site bbffb7d): Glow-Up's product page, in
  the free effects, and its manual at `/manuals/glow-up/`.
- The DAW check (0010) and the faceplate ticket (0009) stay open.
