---
id: "0033"
title: "Wesley engine: three chains, solo and bypass, shared settings"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0031", "0032"]
---

## Summary

The `wesley` crate: params and engine (ADR 0004, points 3–7).

## Acceptance criteria

- [x] Params: per band GAIN, AUTO GAIN, GAMMA, BIT DEPTH, SMOOTHING,
      CRUSH MODE, MIX, LEVEL, ON, SOLO; shared LOW XOVER, HIGH XOVER,
      STEREO, DETECT, DECAY, MIX. Draft ranges and defaults from
      Beverley's; ids fixed here.
- [x] LOW's ON defaults to off: at the defaults the bass passes
      uncrushed (test: a bass sine below the lower crossover comes out
      as it went in, through the split's allpass), and MID and HIGH are
      crushed.
- [x] Every band off (or every MIX at 0) gives the input through the
      split's allpass: flat within 0.01 dB.
- [x] Global MIX blends against the uncrushed bands' sum, so a partial
      mix doesn't comb at the crossovers (test: MIX 50 % with the
      bands uncrushed is flat).
- [x] Solo: one soloed band alone is heard; several soloed are heard
      together.
- [x] Output identical whatever the host's block size; no allocation;
      zero latency.
- [x] Measured: aliasing and out-of-band products of a crushed band
      (point 7), CPU against Beverley.

## Close-out (2026-11-25)

- `beverley`'s `crush` and `chain` moved into `vxn-free-dsp`
  (`crush`, `crush_chain`), so Wesley's engine depends only on the bridge
  and the DSP crate, as CLAUDE.md requires; `beverley` re-exports both
  under their old names. Beverley's tests pass unchanged.
- `crates/wesley`: params and engine; the editor and plugin come in
  0034.
  - Params: 10 a band (GAIN, AUTO GAIN, GAMMA, BIT DEPTH, SMOOTHING,
    CRUSH MODE, MIX, LEVEL, ON, SOLO; LOW ids 1–10, MID 11–20, HIGH
    21–30) and 6 shared (LOW XOVER 40 Hz–1 kHz, default 200; HIGH XOVER
    500 Hz–12 kHz, default 2.5 kHz; STEREO; DETECT; DECAY; MIX; ids
    31–36). Ranges and defaults are Beverley's but AUTO GAIN, on in every
    band (a band is quieter than the whole). LOW's ON defaults to off.
  - Engine: two `ThreeBand`s (left, right), three `Chain`s. Each tick the
    crossovers glide and the chains tick; each channel is split a sample
    at a time into per-band stretches, each band's stretch runs through
    its chain, and the bands are summed. ON crossfades a band between
    uncrushed and crushed, solo fades bands in and out, LEVEL scales the
    band, and MIX blends against the uncrushed bands' sum (heard bands
    only). STEREO, DETECT and DECAY go to all three chains.
- Tests (`crates/wesley/tests`):
  - every band off, or every band's MIX at 0, gives the split summed
    (within 1e-6), at global MIX 100, 50 and 0
  - global MIX 50 % with the bands crushed (16 bits) keeps a sine at
    each crossover within 0.1 dB: no comb
  - at the defaults a 60 Hz sine comes out within −66 dB of clean
    (MID's and HIGH's crushers act only on what leaks into them); with
    LOW on, 4 bits leaves it −28 dB from clean (4-bit quantisation)
  - solo: one soloed band alone, two together; LEVEL scales its band
  - switching a band on fades in (no step)
  - bit-identical output across block sizes 1, 7, 32, 33, 512 and one
    block
  - a mono input works; every param but STEREO, DETECT and DECAY
    changes the output of a crushed band (those need stereo difference
    or auto gain to act); no allocation
- Measured (Apple silicon, on mains):
  - CPU, stereo 96 kHz, block 64: Wesley 0.77 % of a core, Beverley
    0.16 %
  - out-of-band products, MID alone crushed at 4 bits with auto gain:

    | sine | into LOW | within MID | into HIGH |
    |---|---|---|---|
    | 400 Hz | −76 dB | −41 dB | −27 dB |
    | 1 kHz | −88 dB | −41 dB | −28 dB |
    | 2 kHz | −90 dB | −36 dB | −27 dB |

    Most of what a crushed band adds lands above it. Recorded in ADR
    0003, point 7, for 0035 to judge by ear.
  - Aliasing is Beverley's chain's (measured in 0003); not re-measured.
