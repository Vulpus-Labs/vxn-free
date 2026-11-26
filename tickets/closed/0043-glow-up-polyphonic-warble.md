---
id: "0043"
title: "Glow-Up: filters warble on polyphonic material"
priority: high
created: 2026-11-26
epic: E003
depends: ["0008"]
---

## Summary

Heard in a DAW: on polyphonic material the boosts warble, as if the
filters' centres were jumping about. Measured on a held chord (C3, E3,
G3, C4 as saws, `crates/glow-up/examples/warble.rs`), and fixed where the
measurements pointed.

## Acceptance criteria

- [x] The cause measured before anything is changed.
- [x] On a held chord, at FOCUS 0, 50 and 100, filters neither churn nor
      wander: a regression test.
- [x] Every earlier test still passes.
- [x] Heard in a DAW.

## Close-out (2026-11-26)

Measured before (1.5 s of the held chord, at 48 kHz):

| FOCUS | starts/s | sounding centres, cents a block | trackers, cents a block (max) |
|---|---|---|---|
| 0 | 4.0 | 0 | — |
| 50 | 20.0 | 1.74 | 4.0 (181) |
| 100 | 26.8 | 3.13 | 4.1 (127) |

Causes, each found in the frame-by-frame data:

1. The fundamentals (131, 165, 196 Hz) are closer than the 2048 window
   resolves. They made one maximum whose centre wandered 147–180 Hz as
   they beat, and its ratios' targets with it.
2. Weak peaks just over THRESHOLD came and went as partials beat, and
   started filters whenever one was free; with one frame's grace, a
   peak missing for a frame or two released its filter and restarted it.
3. Trackers jittered: a chord's neighbouring partials leak into their
   bands.
4. Toward the grid end, a target's centre and tracker followed its
   strongest driver, and near partials took turns at that.

Fixes:

- `vxn-free-dsp/src/detector.rs`:
  - a peak must have a lone sine's shape: given its centre, Hann fixes
    the smaller neighbour's share too. Measured: lone sines 0.00 dB off,
    a saw's partials up to 0.7 dB, unresolved partials 5–7 dB. Full
    prominence to 1.5 dB, none at 3 dB.
  - the centre comes from the bin's phase advance over the hop, as a
    phase vocoder's does (the magnitudes give it on the first frame after
    a restart). Against the magnitude estimate: worst error on a clean
    sine 0.037 against 0.069 cents, in heavy noise 3.4 against 5.1; frame
    to frame on a saw 0.18 against 0.43 cents, on the chord 0.97 against
    1.6.
- `vxn-free-dsp/src/tracker.rs`:
  - crossings placed by the cubic through four samples, not a straight
    line through two: a steady 1760 Hz sine's readings were 100 times
    steadier.
  - `roughness()`, the smoothed second difference of successive
    readings in cents/s². A ±30 cent vibrato at 5.5 Hz reads 34 000 to
    65 000 from 110 to 880 Hz; neighbours in the band 570 000 to 6.5
    million.
- `glow-up/src/bank.rs`:
  - a new target needs prominence 0.25 to start a filter; a held one
    keeps it at any prominence
  - a filter waits about 100 ms for a lost target, its boost fading and
    first to be stolen, before releasing (was one frame)
  - a tracker steers only while its roughness is under 300 000 cents/s²,
    the limit rising as the square of the frequency above 880 Hz, where
    the crossings' own error grows; otherwise the filter glides to the
    FFT's target
  - without a tracker, that glide has a time constant of half a hop, so
    a frame's change is a glide across it, not a step
  - a grid point's centre is pulled toward its drivers' drive-weighted
    mean, not its strongest
  - a filter keeps its tracker while that still lands within the window
    of where its target aims

Measured after:

| FOCUS | starts/s | sounding centres, cents a block | moves over 2 cents/s |
|---|---|---|---|
| 0 | 2.4 | 0 | 0 |
| 50 | 4.8 | 0.11 | 61 |
| 100 | 3.6 | 0.04 | 2 |

Tried and dropped: steering from the FFT alone (a diagnostic; the
trackers follow bends); roughness from readings a span apart (it
averaged the beating away and let jittery trackers steer); roughness
scaled by the frequency instead of its square (not even across pitch);
the grid pull weighted by amplitude (no better).

Left: FOCUS 50 still has some movement, from trackers snapping back to
the FFT. Heard in a DAW and judged good (2026-11-26).

`tests/engine.rs` gains a held-chord test (at most 12 starts in 1.5 s and
under 0.3 cents a block, at each FOCUS); `vxn-free-dsp` gains tests for
unresolved partials, phase centres' steadiness, and roughness telling a
vibrato from a neighbour.
