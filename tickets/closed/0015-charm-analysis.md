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

- [x] Charm runs 0007's streaming STFT at the next power of two at or
      above 5 ms at the host's rate, with a hop of a quarter-window. The
      STFT's framing makes the schedule independent of the host's block
      size.
- [x] The engine states its analysis lag (window + hop) and enforces it
      as the voices' delay floor. The floor is rounded up to 7 ms unless
      measurement shows a smaller value holds at every rate.
- [x] The analysis results go into the observation struct, and the bench
      (0023) writes them as CSV traces beside the audio, so they can be
      checked against the input before anything acts on them.
- [x] Per band (MID, HIGH):
      - onsets by spectral flux, with an onset's frame start recorded
      - the strongest N peaks (draft N = 8) with strength
      - L/R coherence smoothed over about 300 ms
- [x] Worst-case CPU per host block measured at block sizes 16, 32 and
      64 at 96 kHz, where one block can contain a whole frame. The FFT
      work is not spread across samples: a plugin's deadline is the
      block, not the sample.
- [x] Tests:
      - an impulse train's onsets are found in the frame that contains
        them
      - sinusoids above the LOW crossover appear as peaks within a bin
      - coherence reads ~1 for L = R, ~0 for independent noise and ~−1
        for L = −R
      - results are bit-identical at block sizes 1, 64 and 512
      - no allocation

## Close-out (2026-11-24)

- `charm::analysis::Analysis`, owned by the engine and fed the dry mid
  signal, the MID and HIGH parts of it, and the side signal, a tick at
  a time. The engine copies its results into the observation at each
  tick. Nothing in the audio path reads them yet.
- STFT: the next power of two at or above 5 ms (256 at 44.1 and 48 kHz,
  512 at 96, 1024 at 192), with a quarter-window hop. MID's features
  use the bins from LOW XOVER to 2.5 kHz; HIGH's use those above.
- Floor: **7.5 ms**, not 7. The lag is window + hop + 1 sample for the
  interpolator, which is 7.28 ms at 44.1, 88.2 and 176.4 kHz. A fixed
  7.5 ms keeps Charm sounding the same at every rate. The engine
  asserts it when built. ADR 0002 is updated.
- Onsets: spectral flux against a per-bin reference, the largest
  magnitude of the bin or either neighbour over the last 30 ms.
  - A frame is an onset when its summed rise exceeds 0.6 of the
    reference's sum, the band isn't near silence, and the band is
    armed. A band re-arms when the rise falls below 0.3 or it goes
    quiet.
  - The onset's frame start goes into the observation
    (`onset_start`, a new field).
  - How the detector got there, by bench onset counts per 10 s:

    | Version | Tone (MID) | Distorted chord (HIGH) | Clicks |
    |---|---|---|---|
    | Flux against the last frame only | 2200 | 1538 | 40 |
    | + re-arming, reference over the last window (5 ms) | 2 | 542 | 40 |
    | + reference over 30 ms | 2 | 1 | 40 |

  - Why each step was needed: on a sustained tone, neighbouring
    partials leak into shared bins and beat as the frames slide. On the
    distorted chord, the 41 Hz common period puts a burst of top end
    into every 24 ms, which is a buzz rather than onsets.
  - Results now:
    - each sustained signal (tone, chords, pink noise) gives one onset
      per band at its start; the tone gives two in MID
    - clicks: 40 of 40
    - drums: MID 10 (mostly the snare; the kick is below LOW XOVER) and
      HIGH 30, against 40 hats. Hats right after a snare are masked by
      its tail. That's for 0016 to look at when it tunes the duck.
- Peaks: `vxn_free_dsp::peaks_between` (new) per band, 8 per band.
  - The threshold is 6 dB above the band's mean power. At 10 dB a lone
    sine in MID went unseen, because MID is only about 13 bins wide at a
    256 window and the sine's main lobe lifts the band's mean.
  - Spurious peaks in noise are expected (about 3 a tick in HIGH on
    pink noise). Clarity's sparseness gate (0017) is there for them.
- Coherence: (M² − S²)/(M² + S²) per band, from the band's mid and side
  signals smoothed over 300 ms. Measured: 1.00 for mono inputs and
  −0.00 for independent pink noise.
- Bench: `NAME-analysis.csv` has one row per band per tick: onset,
  onset start, coherence and eight peaks (Hz and amplitude). The report
  adds onsets (count and per second), mean coherence and peaks per tick
  for each band.
- CPU at 96 kHz with the defaults:
  - 0.62–0.74 % of a core, up from 0.39–0.47 %
  - worst block: 7.7 µs of 167 µs at 16 samples, 15 of 333 at 32, and
    19 of 667 at 64
  - The flux reference first cost 1.2 % of a core. Spreading each frame
    across neighbouring bins as it enters the history turned the
    reference into a bin-wise maximum that vectorises.
- Tests (`tests/analysis.rs`):
  - an impulse train at 44.1, 48 and 96 kHz: one onset per click per
    band, each in a frame whose window holds the click
  - 1234 Hz and 6789 Hz found within a bin in MID and HIGH, at amplitude
    within 0.05
  - coherence within 0.1 of 1, 0 and −1 for L = R, independent noise and
    L = −R
  - observations identical at blocks of 1, 64 and 512
  - the dry path is untouched
  - no allocation, through the engine's existing test
