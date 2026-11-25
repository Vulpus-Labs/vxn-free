---
id: "0024"
title: "Charm first tuning pass: ranges and defaults"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0023", "0020"]
---

## Summary

The flock and band controls get their real ranges and defaults, chosen by
ear and checked against the bench (0023). This is milestone M1: a chorus
with the flock, detune as a speed limit, bands and unchorused lows,
measured offline.

## Acceptance criteria

- [ ] Ranges, curves and defaults settled for VOICES, DETUNE, SPREAD,
      COHESION, SEPARATION, TURBULENCE, WIDTH, LOW XOVER, MID DEPTH, HIGH
      DEPTH and MIX. The fixed MID/HIGH crossover is settled too. All are
      recorded in the close-out with the reasons.
- [ ] Rendered and listened to on pads, guitar, voice and a drum loop,
      plus the synthetic test set. Notes recorded.
- [ ] The default setting's bench metrics are recorded as the baseline
      that later tickets compare against: detune, motion, spacing,
      mono-sum loss and CPU.
- [ ] The voices' allpass decorrelation (0028) A/B'd with the bench's
      `--no-decorrelate`, on sustained material especially: kept on,
      made a control, or removed, and ADR 0002 updated.
- [ ] ADR 0002's draft values (the 2.5 kHz crossover, the 7.5 ms floor) are
      updated if tuning changes them.

## Progress (2026-11-24): first listen

The first listen, in a DAW, found two modes: too subtle to hear, or an
unmusical warble.

- **Warble.** The bench's voice traces showed MID voices' detune
  reversing 0.8–2.1 times a second, at 4–7 cents RMS. The pitch changed
  at 12 cents a second typically, but 60 at the 95th percentile and up
  to 180: jerks from walls, separation and the hard stop at the delay
  range's edges, at irregular times. That's tape wow.
  - Fix: each voice's steering on the delay axis (its acceleration,
    which is its pitch's rate of change) is smoothed over 0.15 of a
    crossing time and capped at 2 × the speed limit squared.
  - Walls start 0.3 of the range in.
  - A voice within its stopping distance of an edge brakes at the cap,
    so the hard stop doesn't snap the pitch.
  - Pan, which carries no pitch, keeps quick steering (cap 10×,
    unsmoothed), so crowding is resolved across the stereo field.
  - Result at the defaults: pitch slope 17 cents a second median and
    95th percentile, 18 at most; reversals 0.56 a second; 7.6 cents
    RMS. Each voice's pitch ramps, holds near ±DETUNE and eases back,
    like a triangle-swept chorus but out of step from voice to voice.
  - Voices can now pass each other in delay. With eight voices and
    cohesion and separation at full, some pair is within 0.05 (unit
    square) 3.9 % of the time, for at most 143 ms. At the defaults it's
    0.5 % of the time. The engine test now checks that (under 10 % of
    the time, under 250 ms a spell) in place of a minimum distance.
  - A flock test checks pitch never changes faster than the cap: within
    1.5 × at the edges, at the most extreme setting.
- **Subtle.** At MIX 100 % the ensemble sat 2.6–4.6 dB under the dry
  signal in each channel, and the default MIX of 50 % took it another
  6 dB down. Equal-power panning leaves each channel half of each voice.
  - The ensemble gets +3 dB (√2), so MIX 100 % is now level with the dry
    signal: −1.5 to +0.4 dB on white noise and a chord.
  - MIX defaults to 100 %, the even blend of a classic chorus.
  - Mono output takes the mean of left and right.
- Rate and depth: DETUNE sets how far out of tune the voices go, and
  with SPREAD, how fast they swing. Pitch reverses about every
  SPREAD / speed limit (1.45 s at the defaults). A smaller SPREAD
  swings faster.

## Fix (2026-11-25): voices stuck on the floor after SPREAD moves

Seen on the flock display (0019): after SPREAD was raised and lowered a
few times, voices sat on the delay floor and never left, only panning.
Reproduced offline: three sweeps between 1 and 25 ms put all eight
voices on the floor, at rest, for good.

- Cause: the edge brake added in the first listen above. A voice at rest
  exactly on the floor has no room and a stopping distance of zero, so
  `stopping >= room` held, and `-signum(0.0)` is −1: it braked
  downwards, was clamped back to the floor at rest, and the same
  happened every tick. SPREAD sweeps land voices on the floor because
  the hard stop clamps them there. (Pan at hard left could stick the
  same way.)
- Fix: only a voice moving towards an edge brakes.
- After the sweeps the flock is like a fresh one: the spread of
  heights within a band, averaged over 5 s windows for 60 s, was
  0.08–0.18 of the range, against 0.11–0.17 from reset.
- Tests: a flock with every voice at rest on the floor and at the sides
  is lifted off within 2 s; the engine, after eight SPREAD sweeps, takes
  every voice above 0.3 of its range within 5 s. Both fail without the
  fix.
- Noted for tuning: at the defaults the flock often moves as one herd
  (all near the top, then all near the bottom, every few seconds),
  fresh or not. That is cohesion and alignment at work, and may be
  heard as the voices sweeping together; worth a listen here.
