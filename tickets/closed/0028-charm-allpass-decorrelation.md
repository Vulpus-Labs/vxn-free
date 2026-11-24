---
id: "0028"
title: "Charm: decorrelate the voices with allpass filters"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0023"]
---

## Summary

Each voice is a delayed copy of its band, so with the dry signal it forms
a comb with evenly spaced notches, heard as hollowness when the voices
move slowly. Giving each voice its own allpass chain turns its phase
differently at each frequency, so its notches are no longer evenly
spaced, and voices no longer comb identically with each other. This is
the fixed, per-voice decorrelation that Acon Digital Multiply uses (ADR
0002, prior art), and it needs no analysis.

## Acceptance criteria

- [x] A per-voice allpass chain after each voice's delay read, with
      different settings per voice and per band, fixed at construction
      and scaled to the sample rate.
- [x] Measured on the bench, with decorrelation on and off:
      - comb regularity: the strongest peak of the cepstrum of the
        static dry + wet response (voices held still), in the voices'
        delay range. A regular comb shows as a strong peak there; a
        scattered one doesn't.
      - partial loss (mean and worst) on tone, chord and dist-chord
      - smearing: how far a click's energy spreads in time through one
        voice
      - CPU at 96 kHz
- [x] A choice recorded in the close-out: always on, a control, or not
      worth it, with ADR 0002 updated to match.
- [x] Tests: each voice's chain is allpass (flat magnitude within
      0.01 dB); output still identical across block splits; latency
      still zero; no allocation.

## Close-out (2026-11-24)

- `charm::decorrelate::Chain`: each voice in each band has
  `STAGES` (4) second-order allpasses after its delay read.
  - The allpass is `Svf::allpass`, new in `vxn-free-dsp`: x − 2·bp/Q.
  - Centres are log-spaced across the band, 200 Hz–2.5 kHz for MID and
    2.5–14 kHz for HIGH, with each voice offset by a golden-ratio
    fraction of a step.
  - Q runs from 1.5 to 3.0.
  - The chains are fixed when built.
  - `CharmEngine::set_decorrelation` and the bench's `--no-decorrelate`
    turn it off, for comparing.
- Measured statically, outside the repo, with the engine's own blocks:
  an impulse through the dry path plus voices (crossovers, a fixed
  delay, optionally a chain), its FFT, and the cepstrum of the log
  level searched from 7 to 20 ms. The flock can't be held still in the
  engine: alignment carries every voice along with any one the walls
  push.

  The table gives the cepstral peak: the higher, the more regular the
  comb.

  | Setup | Off | 4 stages, Q 0.7–1.5 | 4 stages, Q 1.5–3 | 8 stages, Q 1.5–3 | 16 stages, Q 1–2 |
  |---|---|---|---|---|---|
  | 4 voices at 0.35 each | 0.080 | 0.062 | 0.067 | 0.052 | 0.044 |
  | 4 voices at 0.7 each | 0.082 | 0.082 | 0.057 | 0.055 | 0.056 |
  | 1 voice at 0.35 | 0.107 | 0.072 | 0.064 | 0.057 | 0.051 |
  | Smearing (90 % of a click's energy within) | 0 | 0.46 ms | 0.52 ms | 0.92 ms | 1.56 ms |

  Ripple, from 300 Hz to 10 kHz, stays at 3.7–5.7 dB in every case.
  The number of dips below −6 dB goes up rather than down: the comb is
  scattered, not made shallower, which is all an allpass can do.
- Moving flock, bench at MIX 100 %:
  - Partial loss is unchanged within 0.1–0.3 dB (tone mean −1.26 dB off
    against −1.36 dB with 8 stages), as expected for the same reason.
  - CPU at 96 kHz with 8 voices: 1.0 % of a core off, 2.2 % with 4
    stages, 4.2 % with 8.
- Choice: 4 stages at Q 1.5–3, always on, no control. That gets most of
  the scattering of 8 stages for half the cost, with smearing well
  under a millisecond. The open question is whether hollowness is heard
  at all with a moving flock, which only listening answers. 0024 A/Bs
  it with `--no-decorrelate` and either keeps it, makes it a control,
  or removes it. ADR 0002 records this.
- Tests:
  - each chain flat within 0.01 dB from 100 Hz to 9 kHz
  - no two voices' chains alike
  - `Svf::allpass` flat at Q 0.5, 1 and 3
  - the engine's block-split identity, zero latency and no-allocation
    tests pass with the chains on (the default)
