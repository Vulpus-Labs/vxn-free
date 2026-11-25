---
id: "0030"
title: "Charm: up to 64 voices, run as SIMD lanes"
priority: medium
created: 2026-11-25
epic: E004
depends: ["0028"]
---

## Summary

VOICES goes from 2–8 to 2–64 a flock. The per-voice work is reorganised
so the cost scales well: voices run eight to a group as
structure-of-arrays.

## Acceptance criteria

- [x] VOICES 2–64; saved values keep their meaning.
- [x] CPU measured before and after, at 96 kHz (the bench's `--cpu`).
- [x] Unused voices cost nothing.
- [x] Every engine and flock test passes at 64 voices.

## Close-out (2026-11-25)

- `MAX_VOICES` 64. VOICES is still stepped, position `k` being `k + 2`
  voices, so a saved 4 is still 4. The default stays 4, which now sits
  near the start of the knob's travel: 0019 or 0024 may want a taper.
- Flock start positions are spaced by golden-ratio steps rather than
  evenly across `MAX_VOICES`, so any number of voices starts spread out
  (evenly over 64, eight voices began in the bottom eighth). Over 60 s
  of pink noise at the defaults the motion is unchanged (detune RMS
  8.0 / 7.9 cents MID / HIGH, before 8.15 / 8.0); only the first
  seconds after a reset differ.
- The edge brake looks one step ahead: with 64 voices in 1 ms of SPREAD,
  crowding pushed a voice into the top edge between checks, and its
  pitch changed at 1.57× the cap. The pitch-glide test now runs 64
  voices too. No change to motion at the defaults (detune RMS 6.77 vs
  6.76 cents over 10 s).
- The voice loop (`engine::Lanes`): per sample, for each group of eight
  - delays as whole + fraction + step × (i + 1), exact and drift-free
  - Lagrange weights once (`vxn_free_dsp::lagrange4_weights`), shared
    by the audio and side-buffer reads (`DelayLine::gather`): each voice
    used to work them out twice
  - the allpass chains a stage at a time across the group
    (`vxn_free_dsp::AllpassBank`); their coefficients are fixed at
    build, as before
  - pan gains ramped across the lanes
  - the loop stops at the group holding the highest sounding voice, and
    silent voices skip their per-tick pan and side-buffer work
- Flock separation compares squared distances before taking a root.
- CPU, % of a core at 96 kHz, stereo pink noise, block 32 (Apple
  silicon, NEON):

  | VOICES | 8 max, scalar (before) | 64 max, scalar | 64 max, lanes |
  |---|---|---|---|
  | 4 | 1.46 | 1.54 | 1.27 |
  | 8 | 2.43 | 2.62 | 1.33 |
  | 16 | | 5.94 | 2.23 |
  | 32 | | 10.37 | 4.30 |
  | 64 | | 19.73 | 9.45 |

  At 64 voices, without decorrelation: 10.05 scalar, 8.11 lanes (the
  chains went from half the cost to a seventh). The flock step at 64
  voices is about 200 ns a sample for both bands (300 before the
  squared-distance check), a fifth of the total. Not measured on x86
  (AVX) here.
- Worst blocks: a few runs showed a single block of 6–24 ms against a
  333 µs period, at any voice count and in the scalar build too, and not
  on rerun: the machine, not the engine. Typical worst blocks at 64
  voices are 110–280 µs at block 16–64.
- Tests: `lagrange4_weights` + `gather` match `read`; `AllpassBank`
  matches `Svf::allpass` lane by lane; 64-voice cases in the flock tests
  (speed limit, range, pitch glide) and in the engine's settings (so
  block-size identity, levels, mono guard and no allocation run at 64);
  hand-built display frames stay at eight voices.

## Re-measured on mains power (2026-11-25)

The figures above were taken partly on a nearly flat battery, when macOS
throttles the CPU (one build swung from 10.6 % to 14.5 % between runs).
Re-measured on AC, best of three, block 32, 96 kHz; "before" is the
build just before this ticket (`ef803be`):

| VOICES | before (8 max, scalar) | now, chains on | now, chains off |
|---|---|---|---|
| 4 | 1.45 | 1.27 | |
| 8 | 2.58 | 1.35 | 1.14 (before: 1.21) |
| 16 | | 2.21 | 1.86 |
| 32 | | 4.07 | 3.34 |
| 64 | | 8.40 | 7.06 |

The "64 max, scalar" column above has no commit of its own and wasn't
re-measured. The conclusions stand: the lanes halve the cost at 8
voices and more, and the chains, half the scalar cost, are now about a
sixth.
