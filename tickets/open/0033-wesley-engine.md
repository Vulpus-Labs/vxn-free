---
id: "0033"
title: "Wesley engine: three chains, solo and bypass, shared settings"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0031", "0032"]
---

## Summary

The `wesley` crate: params and engine (ADR 0003, points 3–7).

## Acceptance criteria

- [ ] Params: per band GAIN, AUTO GAIN, GAMMA, BIT DEPTH, SMOOTHING,
      CRUSH MODE, MIX, LEVEL, ON, SOLO; shared LOW XOVER, HIGH XOVER,
      STEREO, DETECT, DECAY, MIX. Draft ranges and defaults from
      Beverley's; ids fixed here.
- [ ] LOW's ON defaults to off: at the defaults the bass passes
      uncrushed (test: a bass sine below the lower crossover comes out
      as it went in, through the split's allpass), and MID and HIGH are
      crushed.
- [ ] Every band off (or every MIX at 0) gives the input through the
      split's allpass: flat within 0.01 dB.
- [ ] Global MIX blends against the uncrushed bands' sum, so a partial
      mix doesn't comb at the crossovers (test: MIX 50 % with the
      bands uncrushed is flat).
- [ ] Solo: one soloed band alone is heard; several soloed are heard
      together.
- [ ] Output identical whatever the host's block size; no allocation;
      zero latency.
- [ ] Measured: aliasing and out-of-band products of a crushed band
      (point 7), CPU against Beverley.
