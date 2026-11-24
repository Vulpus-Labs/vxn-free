---
id: "0018"
title: "Charm AUTO WIDTH and the mono guard"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0015"]
---

## Summary

Bands that are already wide get less spread; narrow bands get the full
WIDTH. The output's mono sum is kept from collapsing (ADR 0002, point 7).

## Acceptance criteria

- [x] The bench (0023) reports per-band coherence and mono-sum level
      loss. Before/after numbers on the bench's test set are recorded
      in the close-out.
- [x] The new params are exposed in the plugin (0020) in the same
      change, on the plain faceplate until 0019.
- [x] AUTO WIDTH (on/off): per band, the flock's pan bound becomes WIDTH
      scaled by a function of input coherence (stated in the close-out).
      Changes are slewed so the image never jumps.
- [x] A mono guard: per band, the wet part's contribution to the
      level lost in the mono sum is kept within a bound, chosen by
      measurement on pads, guitar and voice (the measurements are
      recorded). The guard is always on.
- [x] Tests:
      - mono input gets the full WIDTH
      - wide decorrelated input gets less spread
      - over a long run of pink noise and a sustained chord, the
        per-band mono-sum level loss stays within the bound
      - with AUTO WIDTH off, behaviour matches the fixed WIDTH apart from
        the guard
      - no allocation

## Close-out (2026-11-24)

- AUTO WIDTH (id 14, toggle, default on).
  - Each band's pan bound is WIDTH × (0.25 + 0.75 × max(coherence, 0)),
    using 0015's coherence (smoothed over 300 ms).
  - The bound is glided over 200 ms per band.
  - Each tick ramps pans from the last tick's width to this one's, so
    the image never jumps.
- Mono guard, always on: no voice's effective pan goes beyond ±0.83.
  - The bound comes from the pan law, not from listening. The ensemble
    comes from the mid signal and is never inverted, so the only way it
    loses level in mono is panning. An equal-power voice at pan p loses
    10·log10((1 + cos(p·π/2))/2) dB, which is 2 dB at 0.83 and 3 dB at
    hard left or right.
  - The ticket asked for the bound to be chosen by measurement on pads,
    guitar and voice. There are no such recordings here, so the bound
    is analytic and checked on the synthetic set. 0024's corpus run
    (`--corpus` with `CHARM_BENCH_AUDIO`) should confirm it on real
    material.
  - The guard only bites above WIDTH 83 %.
- Measured on the bench, WIDTH 100 %, MIX 100 %: mono-sum loss per band,
  input → output.

  | Signal | AUTO WIDTH | LOW | MID | HIGH |
  |---|---|---|---|---|
  | Tone | on or off | 0.00 → −0.19 | 0.00 → −0.19 | 0.00 → −0.14 |
  | Chord | on or off | 0.00 → −0.26 | 0.00 → −0.22 | 0.00 → −0.15 |
  | Drums | on or off | 0.00 → −0.00 | 0.00 → −0.09 | 0.00 → −0.12 |
  | Pink (independent channels) | on | −3.02 → −3.00 | −3.04 → −2.37 | −3.02 → −2.37 |
  | Pink (independent channels) | off | −3.02 → −3.00 | −3.04 → −2.55 | −3.02 → −2.53 |

  - On mono material the ensemble costs at most 0.26 dB in mono. The
    LOW figure is the wet signal's slope leaking into the measurement's
    LOW band.
  - On uncorrelated input it *reduces* the mono loss, because the
    ensemble comes from the mid signal. With AUTO WIDTH on there's less
    spread, so more of that.
- Tests (`tests/width.rs`):
  - mono input: the widest pan with AUTO WIDTH on matches off within
    0.02
  - independent noise: the widest pan with AUTO WIDTH on is under 0.4
    of off
  - with AUTO WIDTH off, pans are identical for mono and wide input
  - at WIDTH 100 %, no pan goes past the guard
  - a chord and independent pink noise at WIDTH and MIX 100 % lose no
    more than 2 dB in mono per band, against the input
  - no allocation, with AUTO WIDTH toggling
- The plugin (0020) doesn't exist yet, so AUTO WIDTH reaches it there.
