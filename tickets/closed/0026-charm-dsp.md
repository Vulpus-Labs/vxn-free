---
id: "0026"
title: "Charm DSP: modulated delay, LR4 crossovers, side-buffer"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0001"]
---

## Summary

The building blocks Charm's audio path needs, in `vxn-free-dsp`
(ADR 0002, points 4, 6 and 10).

## Acceptance criteria

- [x] A scratch harness measures aliasing and CPU for modulated
      fractional-delay reads: linear, cubic Hermite, 4-point Lagrange and
      first-order allpass. Test conditions: a sine sweep through a voice
      moving at the largest DETUNE (draft: 30 cents), at 44.1, 48 and
      96 kHz. The numbers and the choice go in this ticket's close-out.
- [x] A modulated delay line built on the chosen interpolation: a
      per-sample fractional delay, a capacity fixed at construction, and
      no allocation after construction.
- [x] A 4th-order Linkwitz-Riley crossover (low/high pair) with a
      crossover frequency that can be set while running without zipper
      noise.
- [x] A gain side-buffer the same length as a delay line. Values can be
      written at past positions up to a stated lag, and are read at
      fractional positions with the same interpolation as the audio.
- [x] Tests:
      - a static fractional delay matches its design in group delay and
        magnitude
      - a voice moving at constant d′ comes out at ratio 1 − d′, to
        within a cent
      - the LR4 low and high outputs sum allpass with magnitude flat
        within 0.01 dB
      - a value written to the side-buffer is read back at the matching
        audio sample
      - no allocation
- [x] Builds for `wasm32-unknown-unknown`.

## Close-out (2026-11-24)

- Measured with a scratch harness, not kept in the repo. A sine at f0
  went through a voice whose delay grows by d′ per sample, which should
  give a pure sine at f0 (1 − d′). A least-squares fit of that sine
  leaves everything the interpolation got wrong: images, aliases and the
  amplitude ripple as the fraction sweeps. Stepped sines replaced the
  planned sweep because each frequency's error is then exact.
  The table shows SNR in dB at 30 cents and 48 kHz, with gain in dB in
  brackets:

  | f0 | Linear | Hermite | Lagrange | Allpass |
  |---|---|---|---|---|
  | 1 kHz | 63.9 | 91.7 | 110.1 | 66.9 |
  | 4 kHz | 39.5 (−0.20) | 55.0 | 65.5 | 53.7 |
  | 8 kHz | 26.7 (−0.80) | 35.5 (−0.12) | 41.4 (−0.14) | 38.7 |
  | 12 kHz | 18.3 (−1.82) | 23.5 (−0.55) | 27.2 (−0.66) | 27.5 |
  | 16 kHz | 11.6 (−3.30) | 14.5 (−1.54) | 16.7 (−1.83) | 18.7 (−0.22) |

  - At 44.1 kHz every figure is about 1.5 dB worse. At 96 kHz each
    figure moves up an octave: the 16 kHz row reads as the 8 kHz row
    above.
  - At 5 cents, the FIR interpolators match the table within 0.5 dB.
    Allpass is better at low frequencies, because its fraction moves
    more slowly.
  - CPU, push plus one read, release build on Apple silicon, in ns per
    sample: linear 2.28, Hermite 2.63, Lagrange 3.04, allpass 2.50.
- Chosen: **4-point Lagrange**.
  - From 4 to 16 kHz it beats Hermite by 10 down to 2 dB.
  - It matches allpass up to 12 kHz and has no state, so the
    side-buffer can read exactly as the audio does.
  - Two flocks of 8 voices at 96 kHz cost about 0.5 % of one core for
    the reads.
  - Allpass loses because it is recursive: a tap's state belongs to that
    tap, and it can't serve a second buffer.
- `vxn-free-dsp`:
  - `DelayLine`: a power-of-two ring with `push` and `read(delay)`. The
    delay is clamped to 1..=`max_delay`, because the interpolator
    reaches one sample newer than its position.
  - `SideBuffer`: a delay line with `set_past(age, value)` for ages up to
    its stated lag. A read at a delay of at least lag + 1 sees only
    final values. Charm's delay floor must allow for that extra sample.
  - `Svf`: a TPT state-variable filter giving LP, BP and HP at once.
    0025 builds on it for Glow-Up.
  - `Crossover`: LR4 from three SVFs, since the first stage gives both
    the LP and the HP. `set_frequency` keeps state.
- Tests:
  - static delays of 10, 100.3, 100.5 and 257.81 samples: phase delay
    within 0.002 samples, group delay within 0.01, gain within 0.01 dB
  - ±30 and 5 cents of motion come out within 1 cent of 1 − d′
  - the LR4 sum is flat within 0.01 dB from 20 Hz to 18 kHz, at 44.1,
    48 and 96 kHz, crossing at 150 and 2500 Hz
  - each band is −6.02 dB at the crossover and falls past 45 dB two
    octaves out
  - a crossover gliding 150 → 600 Hz in 32-sample ticks shows no step
    in a sine's second difference
  - a side-buffer written 300 samples late reads bit-identically to its
    audio at every delay of at least lag + 1
  - no allocation, with a counting allocator in `tests/no_alloc.rs`
- Builds for `wasm32-unknown-unknown`.
