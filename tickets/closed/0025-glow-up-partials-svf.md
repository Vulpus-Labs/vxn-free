---
id: "0025"
title: "Glow-Up DSP: peak detector, held resonator, partial tracker"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0007"]
---

## Summary

The parts of Glow-Up's analysis and filtering that only Glow-Up uses,
split out of 0007, in `vxn-free-dsp` (ADR 0005, points 4, 6 and 8).

## Acceptance criteria

- [x] A peak detector over a frame (ADR 0005, point 4): peaks ascending
      in frequency, each with a centre, an amplitude and a prominence;
      power smoothed frame to frame; a level floor; a window change
      restarts it without allocating.
- [x] A resonator: a TPT state-variable band-pass with per-sample
      frequency and damping, in `f64`, whose damping can reach zero and
      below, with its level read from its integrators and held at a
      target by damping.
- [x] A partial tracker: a band-pass on one partial, its frequency
      measured between zero crossings, re-centring on what it measures,
      seeded and checked by the FFT.
- [x] Tests:
      - sines found within a few cents and with their amplitude, peaks
        ascending, silence and sines under the floor finding none
      - prominence rising with level above THRESHOLD
      - the resonator unity at its centre, its −3 dB band where the
        damping puts it, self-oscillating within 0.01 cent of its
        tuning, its level held through a glide
      - the tracker follows a glide and a vibrato
      - no allocation
- [x] Builds for `wasm32-unknown-unknown`.

## Close-out (2026-11-26)

- `vxn-free-dsp/src/detector.rs`: `PeakDetector` and `DetectedPeak`.
  `detect(frame, rate)` finds a frame's peaks ascending: maxima over two
  bins either side, smoothed power (0.8 new) standing THRESHOLD over the
  previous frame's mean and above a floor 60 dB under full scale,
  prominence over the next 12 dB. Centre from the larger neighbour's
  share, exact for Hann. Deviation: the amplitude is also divided by
  Hann's response at the estimated offset, so it is right between bins
  (it read 6 % low 0.3 bins off without). `set_window` and `restart`
  never allocate.
- `vxn-free-dsp/src/resonator.rs`: `Resonator`, the TPT band-pass in
  `f64` with damping down to zero and below, `level()` from its
  integrators, `holding_damping` (the slope capped so a block corrects at
  most half the level's error) and `seed`.
- `vxn-free-dsp/src/tracker.rs`: `PartialTracker`, a `Resonator` band
  with four periods between interpolated upward crossings; settles after
  six crossings, accepts only within half its band, presence against a
  50 ms average, anchor and snap left to the caller.
- Measured, in the tests:
  - detector: sines within 2 cents and 3 % of their amplitude at 512,
    1024 and 2048 (amplitude now within 0.01 %); a sine at 5e-4 finds
    nothing, at 5e-3 it is found; prominence rises with level
  - resonator: unity at the centre within 0.1 % from 100 Hz to 15 kHz;
    −3 dB edges within 0.05 dB of where the damping puts them;
    self-oscillation within 0.01 cent of its tuning from 55 Hz to
    19 kHz, its level held within 0.1 %, within 1 % through an octave's
    glide; a ring dies once damped
  - tracker: a steady partial within 0.5 cent from a seed 30 cents off
    (82 Hz needs 0.6 s, as its crossings come a cycle apart; 440 Hz and
    up, 0.2 s); a 1400-cents-a-second bend within 25 cents, landing
    within 1 cent; a 5.5 Hz, ±30 cent vibrato within 15 cents; it lets
    go when its partial ends
  - no allocation for any of them (`tests/no_alloc.rs`)
- Builds for `wasm32-unknown-unknown`.
