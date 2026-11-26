---
id: "0008"
title: "Glow-Up engine: filter bank, grids, mapping"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0025"]
---

## Summary

The engine (ADR 0005): twelve filters, with twelve spares for tails,
assigned to the strongest requests for boost A's and boost B's ratio of
each peak, never moved while sounding, faded over 10 ms when stolen.

## Acceptance criteria

- [x] Parameters: A AMOUNT, A NUM, A DEN, B AMOUNT, B NUM, B DEN
      (NUM and DEN stepped, 1–8), WIDTH (10–200 cents), RESONANCE,
      FOCUS (GRID to TRACK), MIX, WINDOW (512/1024/2048), THRESHOLD
      (0–30 dB), TONE (±6 dB/oct about 1 kHz), GRID (LIN, IRR, TET),
      SPEED, DRY CUT, MODE. Defaults as ADR 0005 and the module's.
- [x] Self-oscillation held at a steady level, within a cent of its
      target; it can't run away.
- [x] A MODE switch, MONO or STEREO:
      - MONO reads only the left input; the right output is a copy of
        the left output.
      - STEREO reads both inputs, analyses their mid signal
        ((L + R) / 2) once, and runs a separate filter bank on each
        channel, both banks following the same targets.
      MODE loads at MONO: Glow-Up is chiefly a guitar effect.
      CPU measured in each mode.
- [x] Tests: every amount at 0 passes the input sample for sample; a
      sine passes almost untouched; a sawtooth's even partials rise with
      A at 2/1; a ratio of 3/1 lifts a sine's third partial; MONO ignores
      the right input and copies left to right; STEREO filters each
      channel; FOCUS fully right follows a glide and no sounding filter
      is retuned; resonance sings on a strong note within a cent and
      fades with it; loud noise with everything up stays bounded; the
      output is the same however the input is split into blocks; no
      allocation.
- [x] Latency is known exactly. The design is zero: the signal path
      never waits for the analysis, and the dry path is not delayed.
      A test measures it: an impulse (and a step) through the engine at
      every WINDOW, MODE, sample rate and MIX setting, with the
      filters' contribution separated out, shows the output's dry
      component at the sample the engine says. The engine exposes its
      latency in samples as a function of its settings and rate.

## Close-out (2026-11-26)

- `crates/glow-up`, new:
  - `lib.rs`: the 17 params, ids 1–17. A at 2/1 and B at 3/2, both 25 %;
    WIDTH 10–200 cents (exponential about 44.7), default 50; RESONANCE
    0; FOCUS 0 (GRID); MIX 100 %; WINDOW 2048; THRESHOLD 6 dB; TONE 0;
    GRID IRR; SPEED 80 %; DRY CUT off; MODE Mono.
  - `grids.rs`: LIN, IRR (120 points from a fixed seed) and TET, 40 Hz
    to 18 kHz, built with the engine.
  - `bank.rs`: `Bank`, twelve filters and twelve spares, the trackers,
    the frame's work (candidates, grid drive or merging, the twelve kept
    with incumbency, tracker matching, assignment with grace, persistence,
    choke and steal) and the per-sample filters, one `Resonator` a
    channel each. `Stats` counts retunes while audible, steals, the
    soonest steal and shortest choke, and tails choked beside a free
    filter, for the tests.
  - `engine.rs`: `GlowUpEngine`, latency 0. The analysis signal (left,
    or the mid in STEREO) feeds the STFT at a quarter-window hop and the
    detector; a frame completes on a hop boundary, always a 32-sample
    block boundary, and the bank takes it at the next block's start, so
    the output doesn't depend on how the host splits its blocks. MIX
    glides 10 ms and is equal power; the soft limit is transparent to
    ±1 and never past ±2.
- Deviation: the module spread a frame's work over the hop in stages;
  here it runs at once, at the block start. The cost is in the CPU
  figures below, and a plugin's host block absorbs it.
- Measured (`tests/engine.rs` and a scratch probe, 48 kHz):
  - every amount at 0, with RESONANCE up: output = input, bit for bit
  - a 440 Hz sine at the defaults: +0.03 dB at TRACK, +0.11 dB at GRID
  - A at 2/1, 100 %, TRACK, on a 110 Hz saw: partials 2 and 4 +13.98 dB
    (the full boost), 6 and 8 +13.2 and +11.2 (the bound and the cap
    on twelve targets), the odd ones +0.23 to +0.36 dB
  - A at 3/1 lifts a 1320 Hz partial over 440 Hz by +13.98 dB and moves
    880 Hz 0.13 dB; at 2/1 the reverse; 1/2 lifts a partial an octave
    under a stronger one by over 10 dB
  - RESONANCE 100 % on a 440 Hz sine: the 2/1 filter oscillates at
    880.0002 Hz (0.0004 cents) and is 60 dB down within 0.5 s of the
    note's end; at 45 % it rings on and dies without oscillating
  - a 300→450 Hz glide with its octave: no sounding filter retuned at
    FOCUS 0, 50 or 100, with RESONANCE off and up; at GRID two steals,
    each after its hold, each choke 479 samples (10 ms); no tail choked
    beside a free filter
  - MONO: right output = left output, the right input ignored; STEREO:
    a right at half the left's level comes out at exactly half; a sine
    on the right alone moves 1160 Hz under 1 dB
  - ±1.6 noise for 10 s with both amounts, RESONANCE, WIDTH and SPEED
    full and THRESHOLD 0: finite, within ±2
  - bit-identical in blocks of 1, 7, 64, 500 and random sizes (STEREO,
    FOCUS 60, RESONANCE 70)
  - the dry component lands on its own sample at every WINDOW, MODE,
    rate (44.1, 48, 96 kHz) and MIX (0, 50, 100), impulse and step
  - a 0.5 mV sine pair and a click train pass untouched
  - no allocation while processing and changing every setting
    (`tests/no_alloc.rs`)
- CPU (`examples/bench.rs`, a five-saw chord, RESONANCE 40 %, release,
  on battery so indicative), ns a sample:

  | MODE | FOCUS | 512 | 1024 | 2048 |
  |---|---|---|---|---|
  | MONO | 0 | 99 | 89 | 67 |
  | MONO | 100 | 98 | 126 | 108 |
  | STEREO | 0 | 119 | 102 | 80 |
  | STEREO | 100 | 115 | 226 | 187 |

  At most about 1.1 % of a core at 48 kHz, STEREO at TRACK.
