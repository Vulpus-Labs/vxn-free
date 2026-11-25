---
id: "0031"
title: "DSP: a three-band split that sums flat"
priority: medium
created: 2026-11-25
epic: E005
depends: []
---

## Summary

`vxn-free-dsp` gains a three-band split for Wesley (ADR 0004, point 3):
two Linkwitz-Riley crossovers, with the low band passed through the
upper crossover's allpass so the three bands sum to an allpass of the
input.

## Acceptance criteria

- [x] `ThreeBand` (or similar): `split(x) -> [low, mid, high]`, both
      frequencies movable while running without clicks, the upper held
      at least an octave above the lower.
- [x] Tests:
      - the bands' sum is flat within 0.01 dB from 20 Hz to 20 kHz, at
        several crossover settings and at 44.1, 48 and 96 kHz
      - each band is 6 dB down at its crossovers and falls at 24 dB an
        octave beyond
      - zero latency: no fixed delay; the sum's impulse response starts
        at once (amended: see the close-out)
      - no allocation; builds for `wasm32-unknown-unknown`
- [x] CPU per sample measured and recorded.

## Close-out (2026-11-25)

- `vxn_free_dsp::ThreeBand`: `new(low, high, rate)`, `split(x) ->
  [low, mid, high]`, `set_frequencies`, `reset`; `MIN_BAND_RATIO` (2)
  holds the upper crossover an octave above the lower. Two `Crossover`s
  and one `Svf` allpass: LOW goes through the upper crossover's allpass
  (Q = 1/√2), the same filter as the crossovers', so the sum is the two
  allpasses in series, flat in magnitude sample for sample.
- Measured at 48 kHz, crossovers 200 Hz and 3.2 kHz: each band −6.02 dB
  at its crossovers (MID too), about 24 dB more an octave further out;
  MID's slope steepens towards Nyquist (31 dB from 6.4 to 12.8 kHz), as
  the bilinear transform does. The sum is within 0.01 dB from 20 Hz to
  20 kHz at 44.1, 48 and 96 kHz and four crossover pairs, among them an
  octave apart (300/600 Hz).
- Amended criterion. "The impulse's sum peaks at sample 0" doesn't hold
  and isn't what zero latency means: the allpasses turn the phase, so
  the response rings over the next samples. First samples at 48 kHz:
  200/2500 Hz 0.61, −0.60; 40/500 Hz 0.91, −0.18; 1000/12000 Hz 0.14,
  −0.05, 0.76 (the peak two samples in, the upper crossover near a
  quarter of the rate). There is no fixed delay to report or
  compensate, so the latency stays 0; the test checks the response
  starts at once (first sample over 0.1 at every setting tried). Charm
  never met this, since its dry path bypassed the crossovers; in Wesley
  every path goes through the split.
- Moving both crossovers (gliding 100 → 400 Hz and 2 → 6 kHz under a
  700 Hz sine) is as smooth as holding them: the sum's worst second
  difference stays within 1.5× of the still split's.
- CPU: 10.5–13 ns a sample a channel on Apple silicon, retuned every 32
  samples (one `Crossover` alone is 4.5–5 ns): about 0.2 % of a core in
  stereo at 96 kHz.
- No allocation (`tests/no_alloc.rs`); builds for wasm32.
