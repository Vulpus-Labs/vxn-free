---
id: "0014"
title: "Charm engine core: flock, bands, mix"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0026"]
---

## Summary

A new `charm` crate (editor/engine, laid out like `beverley`) with
an engine that already sounds like the finished effect, before any
analysis is added. It has the flock (ADR 0002, point 3), the bands
(point 4), equal-power panning (point 5) and the dry/wet mix.

## Acceptance criteria

- [x] Two flocks, one for MID and one for HIGH, each with its own delay
      line and seed and sharing the flock controls.
- [x] Flock: 2–8 voices in (delay, pan) with cohesion, separation,
      alignment and turbulence, updated on a fixed control tick. Delay
      velocity is clamped to DETUNE, delay is bounded softly within
      SPREAD above the 7 ms floor, and pan is bounded within ±WIDTH.
      Turbulence comes from a seeded generator.
- [x] Bands: LOW/MID/HIGH from LR4 crossovers. LOW is never chorused.
      The MID/HIGH crossover is fixed (draft 2.5 kHz; 0024 settles it).
      MID and HIGH have their own depth. Voices read each band's mid
      signal. Dry is the unfiltered input.
- [x] Params with fixed ids, ranges and defaults: VOICES, DETUNE, SPREAD,
      COHESION, SEPARATION, TURBULENCE, WIDTH, LOW XOVER, MID DEPTH,
      HIGH DEPTH, MIX. Ids are reserved now for CLARITY, DUCK, BLOOM and
      AUTO WIDTH. Every param is smoothed. Ranges and defaults are
      provisional until 0024.
- [x] An observation struct, filled once per flock tick without
      allocation: every voice's delay, pan, velocity and gain per flock,
      with room reserved for each band's peaks, gate, onset and
      coherence (filled by 0015 onwards). The bench (0023), telemetry
      frames (0022) and the display (0019) all read this one struct.
- [x] Tests:
      - no voice's instantaneous detune exceeds DETUNE, across settings
      - no voice's delay leaves [floor, floor + SPREAD]
      - with SEPARATION up, no two voices sit within a stated minimum
        spacing once settled
      - output is bit-identical at block sizes 1, 64 and 512
      - LOW-band content under the crossover passes through with no wet
        part
      - latency is zero at every setting and at 44.1, 48 and 96 kHz
      - no allocation
- [x] CPU for two flocks of 8 voices at 96 kHz measured and recorded.

## Close-out (2026-11-24)

- New crate `charm` (params, `engine`, `flock`, `observe`). It has no
  editor yet: 0020 adds the plain faceplate and the plugin.
- Params, with ids 1–11 fixed (12–15 reserved):

  | Param | Range | Default |
  |---|---|---|
  | VOICES | 2–8, stepped | 4 |
  | DETUNE | 1–40 cents, exp | 12 |
  | SPREAD | 1–25 ms, exp | 10 |
  | COHESION, SEPARATION | 0–100 % | 50 |
  | TURBULENCE | 0–100 % | 40 |
  | WIDTH | 0–100 % | 70 |
  | LOW XOVER | 50–500 Hz, exp | 150 |
  | MID DEPTH, HIGH DEPTH | 0–100 % | 100 |
  | MIX | 0–100 % | 50 |

  Every value is provisional until 0024. The MID/HIGH crossover is
  fixed at 2.5 kHz until 0024 settles it.
- Flock: boids in a unit square of (delay, pan).
  - Every force is scaled by the speed limit squared, so DETUNE changes
    the motion's speed, not its shape.
  - Containment: a voice outside its range comes back no faster than
    the limit, so shrinking SPREAD can't break the detune guarantee.
  - Delays and velocities are f64. The engine slides each voice's delay
    across a tick in f64 and rounds each sample to f32 for the read.
    With f32 seconds, a voice at the limit read about 1e-4 over it
    (0.001 cent).
  - Pan speed has its own limit of half the width per second, because
    with SPREAD at 1 ms the delay axis's limit let a voice cross the
    field in 44 ms.
  - SEPARATION's force was raised from 24 to 80 after measurement. At
    24, eight voices with cohesion and separation at full came within
    0.05 of each other in the unit square; at 80, the closest was 0.12.
- Mix: the dry signal stays at unity and MIX is the ensemble's level
  against it (100 % is equal level). This is recorded in ADR 0002. A
  crossfade would remove the bass, which has no wet part.
- Each voice's level is 1/√VOICES, equal-power panned. A voice fades
  over 30 ms when VOICES changes. Mono output sums each voice's left and
  right and scales by 1/√2.
- Observation: once per tick, each band's active count and each voice's
  delay, pan (WIDTH applied), velocity and gain, plus reserved peaks,
  gate, onset and coherence. `VoiceObs::cents()` gives each voice's
  detune.
- Tests:
  - across four settings (including every flock control at full, and
    SPREAD at 1 and 25 ms), over 20,000 ticks: no voice exceeds DETUNE
    and none leaves [7 ms, 7 ms + SPREAD]
  - with eight voices and cohesion and separation at full, every pair
    stays more than 0.1 apart (closest measured: 0.12)
  - output is bit-identical with blocks of 512, 1 and 64, and with a
    mixed 7/300/1/33/512 pattern
  - 40 Hz under a 150 Hz LOW XOVER has its wet part below −40 dB, while
    1 kHz is chorused
  - latency is zero: an impulse comes out at once and nothing else
    follows before the floor, at 44.1, 48 and 96 kHz with each setting
  - one channel is chorused too
  - no allocation, through param changes, VOICES changes and reset
  - unit tests on the flock: speed limit, range, keeps moving with
    turbulence, same seed gives the same motion
- CPU, release build on Apple silicon, at 96 kHz: two flocks of 8
  voices take 0.68 % of one core (71 ns a stereo sample); two flocks of
  4 take 0.38 %.
- Not yet done: listening. 0023 (the bench) and 0024 (tuning) do that.
  Until then the flock's constants are first guesses, except
  SEPARATION's.
- The seeds are fixed per band, so two Charm instances with the same
  settings move identically. If that matters when listening, 0024 can
  vary the seed per instance and keep bounces reproducible.
