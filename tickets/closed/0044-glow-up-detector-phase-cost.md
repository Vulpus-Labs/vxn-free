---
id: "0044"
title: "Glow-Up's detector: one atan2 a peak, and onsets read from the magnitudes"
priority: medium
created: 2026-11-26
epic: E003
depends: ["0043"]
---

## Summary

0043 gave the peak detector phase-vocoder centres by keeping every bin's
phase, one `atan2` a bin a frame. Only peaks need the advance: keep the
last frame's bins instead and take the angle of the bin times the
conjugate of the last, one `atan2` a peak. And where a peak's bin has
risen sharply since the last frame, its last phase was something
else's: take the centre from the magnitudes there.

## Acceptance criteria

- [x] The detector's cost measured before and after.
- [x] How often the onset guard fires, and what it changes, measured.
- [x] A test that fails without the guard.
- [x] Glow-Up's held-chord figures unchanged; every test passes.

## Close-out (2026-11-26)

- `vxn-free-dsp/src/detector.rs`: the last frame's packed bins are kept
  (one copy a frame) in place of each bin's phase; the advance is
  `atan2(im·lr − re·li, re·lr + im·li)`, already wrapped. A peak whose
  bin had under a sixteenth of its power (−12 dB) in the last frame takes
  its centre from the magnitudes.
- `examples/detector_cost.rs`: `detect`'s mean time a frame on a held
  chord of four saws, best of five 10 s runs, `--release`:

  | window | before, ns | after, ns |
  |---|---|---|
  | 512 | 3 200–4 000 | 1 150 |
  | 1024 | 6 600–9 600 | 2 250 |
  | 2048 | 12 000–19 000 | 3 900 |

  The before figures ranged widely between runs; the after figures held
  within 2 %. About a third of the cost at every window.
- The onset guard, measured with a scratch harness that ran the detector
  with and without it in lockstep (removed since). It changed the centre
  of 1 peak in about 3 000 over these onsets: a sine starting over quiet
  and over louder noise, a sine jumping 70 Hz, saws moving a semitone and
  a fifth, a saw joining another, a saw starting over noise. That one,
  in the saw's fifth, came 21.7 cents from a true partial instead of
  41.7. It fires so rarely because a new partial is first a peak only
  once the smoothed power is over THRESHOLD, by when the frame before it
  already held the partial within 12 dB. Kept: it costs one comparison a
  peak and is right when it fires.
- Test: `an_onset_takes_its_centre_from_the_magnitudes`, a bin 40 dB up
  with an unrelated phase. Without the guard it reads 168 cents off;
  with it, within 0.5.
- `examples/warble.rs` on the held chord, as after 0043: 2.4, 4.8 and
  3.6 starts a second at FOCUS 0, 50 and 100; 0, 0.11 and 0.04 cents a
  block.
