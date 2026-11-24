---
id: "0012"
title: "Beverley auto gain: PEAK and RANGE detectors, DECAY"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0004"]
---

## Summary

Auto gain gains a second way of following the input, and a knob for how
fast either lets go.

- **PEAK** (the module's): one follower on the magnitude, 0.5 ms
  attack, released at DECAY's rate. The chain crushes about zero.
- **RANGE**: a maximum and a minimum follow the input. A sample beyond
  either replaces it at once (instant attack); between such samples both
  fall towards their midpoint at DECAY's rate. The gain brings the
  half-range (max − min) / 2 to full scale. The signal is crushed about
  the midpoint: the midpoint is taken off, the rest crushed, and the
  midpoint put back. An offset input is crushed on its swing, and is
  never clipped by a gain meant for its swing.
- **DECAY**: the time either detector takes to fall 20 dB, 1 ms to 5 s,
  exponential, default 115 ms (the module's 50 ms release time
  constant).

With AUTO GAIN off, neither applies: DECAY and DETECT dim.

## Acceptance criteria

- [x] Params DECAY (id 9) and DETECT (id 10, PEAK or RANGE, default
      RANGE). Existing ids unchanged.
- [x] STEREO with RANGE: SEPARATE keeps a max/min pair per channel;
      LINKED feeds both channels into one pair, so out-of-phase stereo
      isn't mistaken for silence; MONO follows the summed signal. With
      PEAK, LINKED follows the mid signal as before.
- [x] Tests: RANGE's attack is instant and its fall reaches −20 dB at
      DECAY; PEAK's release follows DECAY; an offset input under RANGE
      comes out crushed about its offset, unclipped; LINKED RANGE on
      L = −R gains on the swing; latency still zero at every setting,
      detectors and DECAY included; no allocation.
- [x] Faceplate: a DECAY knob and a DETECT switch beside AUTO, dimmed
      while AUTO is off; snapshots re-blessed.
- [x] CLAP tests and clap-validator pass with the new params.

## Close-out (2026-11-24)

- `vxn-free-dsp`: `RangeFollower` (max and min, instant attack, falling
  towards their midpoint; `decay` and `push` separately, so linked stereo
  pushes both channels per decay), `fall(seconds, rate)` (the per-sample
  share left after a fall of 20 dB in `seconds`), and `PeakFollower`
  falling at a settable rate in place of its fixed 50 ms release.
- `beverley`: DECAY (id 9, 1 ms – 5 s, geometric over its travel, so
  the middle reads about 71 ms; default 115 ms) and DETECT (id 10, a
  toggle labelled Peak / Range, default Range). The engine works out the
  fall from DECAY once per control tick when it changes. With RANGE, each
  sample has the midpoint taken off before the chain and put back after.
  The chain itself is unchanged.
- STEREO with RANGE: SEPARATE has a pair per channel, LINKED one pair
  fed by both channels, and MONO follows the sum. With PEAK, LINKED still
  follows the mid signal.
- Tests:
  - Both detectors in every stereo mode against reference followers,
    bit for bit.
  - DECAY reaching both detectors (at 2 ms and 300 ms).
  - L = −R: PEAK LINKED gates and clips to ±1/200, while RANGE LINKED
    gives the input back through a transparent chain.
  - 0.5 + 0.1 sin at one bit under RANGE becomes a square about 0.5,
    between 0.4 and 0.6.
  - Latency still zero across 450 settings (detectors and DECAY at both
    extremes) at three rates, and through the plugin.
  - No allocation.
- Faceplate: AUTO, DETECT (a bat switch, up RANGE, down PEAK) and DECAY
  (scale 1 ms, 70, 5 s) form the gain group. The panel is now 1090 × 150.
  DETECT and DECAY dim while AUTO is off (`dimmed_when`); a snapshot shows
  them lit with AUTO on. All snapshots re-blessed.
- clap-validator: 16 passed, 5 skipped, 0 failed. The first run after a
  fresh bundle warns on `scan-time` (416 ms against a 100 ms target): it
  is the first load of a new unsigned binary, and every later run scans
  in time. This is the warning noted in 0005.
- Behaviour to know: RANGE starts at rest at 0, so an input with an
  offset takes a few DECAY times to settle on its swing. Between peaks
  the range sags by DECAY's rate, and the crushed output's height with it.
