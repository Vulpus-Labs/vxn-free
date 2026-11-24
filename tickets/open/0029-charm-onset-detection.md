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

- [ ] An envelope detector beside the flux detector, choosable for the
      bench.
- [ ] Bench signals with known onset times: the click train, the drum
      loop (snare and hats by band), and a legato line (notes changing
      pitch at a steady level, where only flux can see the change).
- [ ] Measured for each detector, recorded in the close-out:
      - hits, misses and false onsets against the known times, per band
      - detection delay after each true onset
      - false onsets on the sustained signals (tone, chords, pink and
        white noise)
      - CPU at 96 kHz
- [ ] A choice recorded, with ADR 0002 updated. If the envelope detector
      wins, the STFT, peaks and their observation fields leave Charm,
      and the delay floor is reconsidered.
- [ ] Tests for the chosen detector: as 0015's onset tests; block-split
      identity; no allocation.
