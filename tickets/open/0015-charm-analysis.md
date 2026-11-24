---
id: "0015"
title: "Charm analysis sidechain: onsets, peaks, coherence"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0007", "0023"]
---

## Summary

The sidechain that listens to the dry input and runs behind it (ADR 0002,
point 6). It produces the per-band onsets, spectral peaks and coherence
that 0016–0018 use. It is computed and tested in this ticket but does not
yet affect the audio.

## Acceptance criteria

- [ ] Charm runs 0007's streaming STFT at the next power of two at or
      above 5 ms at the host's rate, with a hop of a quarter-window. The
      STFT's framing makes the schedule independent of the host's block
      size.
- [ ] The engine states its analysis lag (window + hop) and enforces it
      as the voices' delay floor. The floor is rounded up to 7 ms unless
      measurement shows a smaller value holds at every rate.
- [ ] The analysis results go into the observation struct, and the bench
      (0023) writes them as CSV traces beside the audio, so they can be
      checked against the input before anything acts on them.
- [ ] Per band (MID, HIGH):
      - onsets by spectral flux, with an onset's frame start recorded
      - the strongest N peaks (draft N = 8) with strength
      - L/R coherence smoothed over about 300 ms
- [ ] Worst-case CPU per host block measured at block sizes 16, 32 and
      64 at 96 kHz, where one block can contain a whole frame. The FFT
      work is not spread across samples: a plugin's deadline is the
      block, not the sample.
- [ ] Tests:
      - an impulse train's onsets are found in the frame that contains
        them
      - sinusoids above the LOW crossover appear as peaks within a bin
      - coherence reads ~1 for L = R, ~0 for independent noise and ~−1
        for L = −R
      - results are bit-identical at block sizes 1, 64 and 512
      - no allocation
