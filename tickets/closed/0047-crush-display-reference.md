---
id: "0047"
title: "Crush display: −3 dBFS reference with AUTO GAIN off"
priority: medium
created: 2026-11-27
epic: none
depends: ["0038"]
---

## Summary

The user (2026-11-27): Beverley's display drew a flat line at 1 bit,
gamma 1, symmetric, smoothing 0, gain 0 dB, AUTO GAIN off, where the
top of the sine should come out as a pulse.

The display was right for what it drew. With AUTO GAIN off it takes a
sine at `FIXED_GAIN_REFERENCE` before GAIN (0038), which was −6 dBFS:
a peak of 0.5. Symmetric 1 bit steps from 0 to 1 at a magnitude of 0.5,
and the display samples at the middle of each of its points, never at
the peak itself, so every point fell just under the step and drew as 0.
The engine was checked against a sine at 0.9 at the same settings: it
gives ±1 pulses as expected. Only the display changes.

## Acceptance criteria

- [x] `FIXED_GAIN_REFERENCE` is −3 dBFS (0.707 945 8): off symmetric
      1 bit's step, and +3 dB of GAIN still reaches full scale before
      clipping.
- [x] A test draws the settings above and finds the pulses.
- [x] The clipping test takes its gain from the reference, not a
      hard-coded 18.06 dB.
- [x] Beverley's manual says the tone is 3 dB below full level, not
      half of it.
- [x] Snapshots re-blessed and inspected.

## Close-out (2026-11-27)

- `crates/vxn-free-ui/src/crush_view.rs`: the reference, its docs, the
  new `symmetric_one_bit_at_unity_gain_shows_its_pulses` test and the
  clipping test's gain.
- `crates/beverley/manual/beverley-manual.typ`: the display paragraph.
- Beverley's `initial_1x`, `initial_2x` and `tooltip_auto_1x` snapshots
  (AUTO GAIN is off by default): the default 4-bit curve now uses about
  0.7 of the quantiser, so more steps and no flat top. Wesley's
  snapshots are unchanged: its bands draw with AUTO GAIN on.
- With AUTO GAIN off the 1-bit pulses are full scale, louder than the
  −3 dBFS reference, so they draw clamped at the display's edge.
- The engine is unchanged; the manual's measured data regenerates
  identically.
