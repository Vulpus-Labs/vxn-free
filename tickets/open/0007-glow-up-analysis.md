---
id: "0007"
title: "Glow-Up analysis DSP: STFT, peaks, partials, SVF"
priority: medium
created: 2026-11-24
epic: E003
depends: ["0001"]
---

## Summary

The building blocks of Glow-Up's listening and filtering, in
`vxn-free-dsp`.

## Acceptance criteria

- [ ] A streaming STFT: windows of 512, 1024 or 2048 samples, hop and
      window switchable without a glitch, running inside the audio thread
      with no allocation after construction.
- [ ] Peak detection over a frame: local maxima standing a threshold
      above the frame's mean power, with sub-bin frequency estimates.
- [ ] Partial tracking: peaks matched frame to frame into partials with
      a strength.
- [ ] A TPT state-variable filter with per-sample frequency and Q.
- [ ] Tests: sinusoids found to within a few cents at each window size;
      two close partials separated at 2048; tracking follows a glide;
      the SVF's response matches its design at several settings.
- [ ] Builds for `wasm32-unknown-unknown`.
