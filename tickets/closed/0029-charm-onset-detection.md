---
id: "0029"
title: "Charm: onset detection by spectral flux against band envelopes"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0015"]
---

## Summary

Duck and Bloom (0016) need onsets. With Clarity dropped, onsets are the
only use of the short STFT. Band envelopes need no FFT: each band (MID,
HIGH) has a fast envelope compared with a slow reference, the largest the
band has been over the last 30 ms. That is the same trick that stopped
the flux detector firing on sustained tones (0015), applied to whole
bands instead of bins. This ticket measures both detectors and picks one.

## Acceptance criteria

- [x] An envelope detector beside the flux detector, choosable for the
      bench.
- [x] Bench signals with known onset times: the click train, the drum
      loop (snare and hats by band), and a legato line (notes changing
      pitch at a steady level, where only flux can see the change).
- [x] Measured for each detector, recorded in the close-out:
      - hits, misses and false onsets against the known times, per band
      - detection delay after each true onset
      - false onsets on the sustained signals (tone, chords, pink and
        white noise)
      - CPU at 96 kHz
- [x] A choice recorded, with ADR 0002 updated. If the envelope detector
      wins, the STFT, peaks and their observation fields leave Charm,
      and the delay floor is reconsidered.
- [x] Tests for the chosen detector: as 0015's onset tests; block-split
      identity; no allocation.

## Close-out (2026-11-24)

- Both detectors were built side by side, selectable for the bench, and
  compared on the test set at the defaults, 48 kHz.
- The bench gained a `legato` signal (a steady-level line changing note
  every 0.5 s), known onset times for `clicks`, `drums` and `legato`,
  and scoring: a reported onset is a hit if it comes within 40 ms after
  an unmatched true onset; otherwise it is false.
- Ground truth for the drums is every eighth note in both bands. The
  hats are broadband and the kick sweeps down from 120 Hz, so every hit
  reaches MID. A first scoring with only the snares as MID's truth
  wrongly counted kicks and hats as false.

  | Signal | Flux (5 ms STFT, 30 ms per-bin reference) | Envelope (1 ms follower, 30 ms reference) |
  |---|---|---|
  | Clicks, MID / HIGH | 40/40, 0 false / 40/40, 0 false | 40/40, 0 false / 40/40, 0 false |
  | Drums, MID | 10 of 40 | 28 of 40, 0 false |
  | Drums, HIGH | 30 of 40 | 31 of 40, 0 false |
  | Legato, MID | 1 of 20 | 1 of 20 |
  | Sustained (tone, chord, dist-chord, pink, white) | one onset each at the start (two on the tone, MID) | one each (two on pink noise, MID) |
  | Delay after a true onset | 1.7–2.0 ms | 1.3 ms |
  | CPU at 96 kHz, defaults, decorrelation off | 0.67 % of a core | 0.54 % |

- Flux's expected advantage didn't appear. At 188 Hz bins, with the
  reference spread over neighbouring bins, a legato step from 220 to
  261 Hz stays within reach of the old partials' bins.
- The envelope detector's first version fired twice on 4 of 40 clicks.
  A click's energy can rise in two steps about a millisecond apart, and
  the band re-armed between them. A band now waits out the 30 ms
  reference before firing again. That also caps two onsets in one band
  at 30 ms apart; sixteenths at 140 bpm are 107 ms. A first guess at
  the cause, a tail near the silence level, was also fixed (references
  below silence count as silence) but wasn't it.
- Hats missed in both detectors are the ones right after a snare,
  masked by its tail. That's for 0016 to judge by ear.
- Choice: the envelope detector. `charm::analysis` is rewritten around
  it and coherence.
  - The STFT, flux, peaks, `OnsetDetector` and the observation's peaks
    are gone from Charm. The STFT and peaks stay in `vxn-free-dsp` for
    Glow-Up.
  - The bench loses its peaks columns and `--onsets`.
  - Onsets record `start` (3 ms and a block before they were found) and
    `found`.
- Floor: the analysis now lags 3.7 ms at 44.1 kHz, against 7.28 ms for
  the STFT. The floor stays at 7.5 ms: lowering it would change the
  sound and edge towards flanging, and the margin costs nothing. ADR
  0002 records this.
- CPU at 96 kHz, defaults (4 voices, decorrelation on): 1.1–1.2 % of a
  core.
- Tests:
  - clicks at 44.1, 48 and 96 kHz: one onset each per band, marked from
    before the click and found within 3 ms
  - a beating harmonic tone and white noise: no onsets after their start
  - coherence, block-split identity and no allocation as before
