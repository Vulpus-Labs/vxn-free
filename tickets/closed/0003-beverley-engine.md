---
id: "0003"
title: "Beverley engine: the crush chain, ported"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0001"]
---

## Summary

Port the module's processing to an `Engine`, stereo, at the host's rate.

Per sample: gain (fixed 0–20 dB, or automatic, following the input's peak
envelope), hard clip to ±1, gamma (power-law), quantisation to a
fractional bit depth (interpolating between the two whole depths beside
it), smoothing between levels (0 hard steps, 0.5 smoothstep, 1
transparent), symmetric or asymmetric crush, then reverse gamma and
reverse gain.

## Acceptance criteria

- [x] `beverley` crate with the engine and its parameters: GAIN, AUTO
      GAIN, GAMMA, BIT DEPTH, SMOOTHING, CRUSH MODE, plus MIX (new;
      defaults fully wet).
- [x] STEREO (new): MONO (left and right summed, crushed once, sent to
      both), SEPARATE (each channel with its own auto gain), LINKED
      (one auto gain for both, following the mid signal). Default
      LINKED, so auto gain doesn't move the stereo image.
- [x] Parameters applied at control rate (a tick of a few dozen
      samples), ramped across samples where a step would click.
- [x] Parameters smoothed so automation doesn't zipper; the peak
      follower's time constant computed from the sample rate.
- [x] Tests: a sine through each stage against values worked out by
      hand; fractional depth interpolates; symmetric and asymmetric modes
      differ as described; mono input feeds both outputs.
- [x] No allocation in `process` (a no-alloc test).
- [x] Aliasing measured at 44.1 and 96 kHz on a 1 kHz sine. Whether to
      oversample is decided from the numbers, which go in the close-out.
- [x] Latency is known exactly. Without oversampling it is zero; with
      it, the resampling filters' delay, rounded to whole samples with
      the dry path (MIX) delayed to match so dry and wet stay aligned.
      The engine exposes its latency in samples as a function of its
      settings and rate, and a test measures it: an impulse and a step
      through the engine at every setting that could change it and at
      44.1, 48 and 96 kHz land on the sample the engine says.

## Close-out (2026-11-24)

**Landed.** `crates/beverley`: `PARAMS` (GAIN, AUTO GAIN, GAMMA, BIT
DEPTH, SMOOTHING, CRUSH MODE, MIX, STEREO, ids 1–8), `crush` (the chain
per sample) and `engine` (`BeverleyEngine`). In `vxn-free-dsp`: `Glide`
and `Ramp` (control-rate smoothing) and `PeakFollower`, with time
constants from the host's rate.

**Port notes.**
- The module's peak follower assumed 48 kHz; it now takes the host's
  rate (0.5 ms attack, 50 ms release, as before).
- The module's clamp helper took its arguments in an order that only
  bounded gamma and depth from below. Its knobs kept them in range, so
  nothing audible changes; here they're clamped both ways.
- `signum(0.0)` is 1 in Rust; zero input returns zero before the sign
  is taken, as the module's `Math.signum` did.
- GAMMA's knob runs −1..1 and sets the power 10^knob (0.1–10), shown as
  the power. BIT DEPTH and SMOOTHING keep the module's knob centres (4
  bits, 20 %) with exponential tapers. SMOOTHING reads 0–100 %.

**Control rate.** Settings are recomputed every 32 samples (`TICK`),
each gliding with a 10 ms time constant. Gain and mix also ramp in a
straight line across each tick. The tick count carries across `process`
calls, so a test checks that block splits don't change the output.

**Aliasing** (no oversampling, 0.8-amplitude 1 kHz sine). Gain is
fixed here, so the chain has no memory: one period sampled 65 536 times
gives its exact harmonic series. Alias power counts the harmonics above
Nyquist that fold below 20 kHz, relative to the fundamental. The M×
columns assume an ideal anti-alias filter at M× the rate.

| Setting | fs kHz | wanted harmonics | alias 1× | 2× | 4× | 8× | 16× |
|---|---|---|---|---|---|---|---|
| 4 bit, hard, asym (default) | 44.1 | −27.7 | −25.6 | −32.6 | −38.8 | −44.1 | −50.7 |
| 4 bit, hard, asym (default) | 96 | −24.5 | −33.6 | −39.1 | −45.3 | −51.5 | −57.2 |
| 4 bit, hard, sym | 44.1 | −41.4 | −30.4 | −34.7 | −42.9 | −47.5 | −54.3 |
| 8 bit, hard, asym | 44.1 | −59.6 | −48.7 | −51.5 | −55.4 | −59.9 | −66.7 |
| 1 bit, hard, asym | 44.1 | −7.0 | −26.3 | −42.0 | −55.0 | −69.0 | −81.1 |
| 4 bit, smoothing 50 % | 44.1 | −40.8 | −38.3 | −56.0 | −80.5 | −95.8 | −111.5 |
| 4 bit, smoothing 50 % | 96 | −36.4 | −62.6 | −82.6 | −96.0 | −114.5 | −126.5 |
| 4 bit, smoothing 80 % | 44.1 | −48.7 | −46.3 | −64.0 | −88.4 | −103.7 | −119.6 |
| 4 bit, gamma 0.3 | 44.1 | −24.8 | −28.4 | −35.6 | −42.8 | −48.4 | −54.0 |
| 4 bit, gamma 3 | 44.1 | −19.0 | −25.3 | −31.5 | −37.1 | −43.3 | −49.6 |
| 4.5 bit, hard | 44.1 | −34.3 | −30.9 | −36.3 | −44.1 | −49.3 | −56.0 |

**Decision: no oversampling.** Hard steps are discontinuities, so their
harmonics fall only as 1/k. Each doubling of rate buys about 6 dB: 4×
buys 13 dB at 4× the CPU, plus a resampling delay. Aliasing that loud
is part of a hard bitcrusher's sound. Smoothing is how to get rid of
it: at 50 % it is already ~13 dB lower at 44.1 kHz, and 29 dB lower at
96 kHz, with no oversampling. If a cleaner mode is wanted later,
antiderivative anti-aliasing of the steps would do it without
oversampling.

**Latency.** Zero at every setting and rate (`BeverleyEngine::LATENCY`).
The test runs an impulse and a step through 288 settings (stereo mode,
auto gain, crush mode, depth and smoothing, mix) at 44.1, 48 and
96 kHz, and every first non-zero output lands on the input's sample.
Since it is a constant, 0005 can report it with silkscreen-clap's fixed
`LATENCY`: no silkscreen change is needed.

**CPU** (release build on Apple silicon, stereo at 48 kHz, 256-frame
blocks):

| Setting | `f32::powf` | fast pow (polynomial log2/exp2) |
|---|---|---|
| 4 bit, gamma 0.4, linked | 50.6 ns/frame (0.24 % of a core) | 58.6 ns |
| 4.5 bit, smoothing 30 %, auto, separate | 37.3 ns (0.18 %) | 49.6 ns |
| mono, gamma 0.4 | 17.8 ns (0.09 %) | 22.1 ns |

A degree-5 polynomial `exp2(p·log2 x)` (rel. error ~2e-4) was tried for
the two powers per sample. On its own it ran at 3.6 ns against
`powf`'s 3.1 ns, and in the engine it was slower throughout, so it was
dropped. `powf` is exact, and the whole engine costs under 0.3 % of a
core. At gamma 1, `powf` takes a short cut: about 14 ns/frame.

**No allocation.** `tests/no_alloc.rs` counts allocations across
`set_param`, `process` (one and two channels, every stereo mode) and
`reset`: none.
