---
id: "0006"
title: "Beverley manual"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0005"]
---

## Acceptance criteria

Revised 2026-11-24 at the user's request: a full PDF, built as Vulpus
Labs' instrument manuals are, with graphs generated from the engine
(ADR 0003).

- [x] `docs/manual/vulpus-manual.typ`, the template; `crates/beverley/
      manual/` with `build.sh`, `beverley-manual.typ` and `data/`.
- [x] `cargo run -p beverley --example manual` draws the faceplate (at
      rest, and with AUTO on) and writes every control's box, then
      measures the engine: transfer curves for depth, crush mode,
      smoothing, gamma, fractional depth and gain; waveforms; harmonic
      spectra; aliasing at 44.1 and 48 kHz; auto gain's response by
      detector and DECAY; RANGE on an offset signal; the tapers and the
      param table.
- [x] The manual: what it does, every control with a labelled face,
      starting points, a reference, written for players.
- [x] Published on vulpuslabs.com, linked from Beverley's page and from
      the README.

## Close-out (2026-11-24)

- ADR 0003 records how manuals are built.
- `docs/manual/vulpus-manual.typ`: the Vulpus Labs manual template (page,
  type, labelled faces, plots), without the rack-and-cable diagrams a
  modular manual needs.
- `crates/beverley/examples/manual/`:
  - `faces.rs`: the faceplate at 2× through the harness (`init`, and
    `auto` with AUTO on), and `positions.json`.
  - `measure.rs`: the engine at 48 kHz on one channel, with transfer
    curves from an 801-point ramp (the chain is memoryless with AUTO
    off); harmonics by Goertzel at a 997 Hz tone (so aliases fall between
    them); aliasing as the power left after DC and the harmonics
    (Parseval), at 44.1 and 48 kHz; auto gain's traces from the dsp
    crate's followers at three DECAYs; RANGE and PEAK on an offset
    signal; the tapers and the param table.
- `crates/beverley/manual/`: `build.sh`, `beverley-manual.typ` (nine A4
  pages: overview with every control numbered, the crush, gain and auto
  gain, stereo and mix, starting points, reference) and `data/`. Faces
  and the PDF are ignored, as ADR 0003 says.
- Two captions were corrected against the measurements before
  publishing. Smoothing at 50% lowers most harmonics, the lowest by about
  20 dB, but doesn't remove the highest. At DECAY 20 ms, PEAK's gain
  ripples with every cycle.
- The aliasing figures agree with 0003's analysis: 4 bits hard,
  −25.2 dBc here against −25.6 there.
- Published at vulpuslabs.com/manuals/beverley/, linked from Beverley's
  product page and the README.
