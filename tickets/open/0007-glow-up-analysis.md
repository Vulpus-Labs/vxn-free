---
id: "0007"
title: "Analysis DSP: FFT, streaming STFT, peaks"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0001"]
---

## Summary

The spectral building blocks that Glow-Up and Charm both use, in
`vxn-free-dsp`. Glow-Up's partial tracking and SVF are in 0025, so
Charm doesn't wait on them.

## Acceptance criteria

- [ ] Patches' `RealPackedFft` (`patches-dsp/src/fft.rs`, tag v0.7.2)
      copied into `vxn-free-dsp` with its tests. Its header names the
      source and states the relicensing under MIT OR Apache-2.0
      (ADR 0001, point 5). Nothing else is taken from Patches.
- [ ] A streaming STFT over that FFT, with windows of 128,
      256, 512, 1024 or 2048 samples and a hop that divides the window:
      - It takes host blocks of any size and keeps no ring buffer.
      - With overlap O = window / hop, it keeps O + 1 frame buffers. O of
        them are filling, each offset from the next by one hop. Each
        filling buffer takes its slice of the block, multiplied by the
        window as it is written. Write and window are one pass.
      - When a buffer fills, at a hop boundary counted from the start of
        the stream, it is transformed in place. It is then the latest
        spectrum, readable until the next boundary. The buffer that held
        the previous spectrum is cleared and starts the next frame, so
        nothing is written to a buffer that is still being read.
      - At stream start the buffers are staggered as if the history were
        zeros.
      - Changing window or hop restarts the analysis from zero history
        and never reaches the audio path.
      - No allocation after construction.
- [ ] Peak detection over a frame: local maxima standing a threshold
      above the frame's mean power, with sub-bin frequency estimates.
- [ ] Tests:
      - sinusoids found to within a few cents at each window size
      - two close partials separated at 2048
      - frames are identical when the same input is split into blocks of
        1, 7, 64 and 512 samples and into random sizes
      - frame timing follows the stream position, from the first frame on
      - no allocation
- [ ] Builds for `wasm32-unknown-unknown`, with the scalar FFT path.
