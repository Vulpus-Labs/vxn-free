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

- [x] Patches' `RealPackedFft` (`patches-dsp/src/fft.rs`, tag v0.7.2)
      copied into `vxn-free-dsp` with its tests. Its header names the
      source and states the relicensing under MIT OR Apache-2.0
      (ADR 0001, point 5). Nothing else is taken from Patches.
- [x] A streaming STFT over that FFT, with windows of 128,
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
- [x] Peak detection over a frame: local maxima standing a threshold
      above the frame's mean power, with sub-bin frequency estimates.
- [x] Tests:
      - sinusoids found to within a few cents at each window size
      - two close partials separated at 2048
      - frames are identical when the same input is split into blocks of
        1, 7, 64 and 512 samples and into random sizes
      - frame timing follows the stream position, from the first frame on
      - no allocation
- [x] Builds for `wasm32-unknown-unknown`, with the scalar FFT path.

## Close-out (2026-11-24)

- `vxn-free-dsp/src/fft.rs`: Patches' `RealPackedFft`, copied from tag
  v0.7.2 with its seven tests.
  - The header names the source and states the relicensing.
  - Module-level allowances cover `unsafe` (for the SIMD paths) and
    2021-edition `unsafe fn` bodies. Patches is on 2021; this workspace
    is on 2024 and denies unsafe code elsewhere.
  - Patches' style lints are allowed, and `#[rustfmt::skip]` on the
    module keeps its formatting, so the copy compares directly with the
    source.
  - `assert_within!` is redefined in the test module. Nothing else is
    changed.
- `vxn-free-dsp/src/stft.rs`: `Stft`.
  - `new(max_window, max_overlap, window, hop)` allocates FFTs and Hann
    windows for every size from 128 to the maximum, and
    `max_overlap + 1` frame buffers.
  - `configure(window, hop)` and `restart()` never allocate.
  - `push(input, on_frame)` writes window × input into each filling
    frame in one pass. Frame `m` covers `m·hop − window .. m·hop` and
    uses buffer `m mod (O + 1)`. At each boundary it transforms in
    place and calls `on_frame` with the frame's index, its end position
    and its bins.
  - `latest()` returns the newest spectrum.
  - Deviation: the buffer a new frame takes over is not cleared in the
    steady state. The new frame writes every one of its samples before
    it is transformed, so clearing would be wasted work. Buffers are
    zeroed only on restart, which gives early frames their silent
    history.
- `vxn-free-dsp/src/spectrum.rs`: `peaks(frame, rate, threshold_db, out)`.
  - It finds local maxima above the frame's mean power by
    `threshold_db`.
  - Each is refined by a parabola through log magnitudes.
  - Amplitude is corrected for Hann's coherent gain.
  - The strongest `out.len()` peaks are kept, strongest first, without
    allocating.
- Accuracy, measured by sweeping a sine from 3 to 50 bins at every
  window size: worst error 0.016 bins at every size. That is at most
  2.2 cents from 12 bins up: at 48 kHz, 4.5 kHz with a 128 window and
  280 Hz with 2048. Charm's 256 window (5.3 ms) is within 2.2 cents
  from 2.25 kHz up, and coarser in cents below that, as ADR 0002
  expected. Amplitude is within 0.04 of a 0.5 sine.
- Tests:
  - sines found within 5 cents at windows 128–2048, from 12 to 50 bins
  - 1000 Hz and a partial three bins above it separated at 2048
  - peaks strongest first, capped at the count asked for
  - silence has no peaks
  - frames bit-identical with blocks of 1, 7, 64, 512 and random sizes,
    at four window/hop pairs
  - an impulse lit in exactly the frames whose windows hold it (except
    the one starting on it, where Hann is zero)
  - the first frame sees zeros before the stream
  - reconfiguring restarts from silence, matching a fresh STFT
  - no allocation, with the counting allocator now watching only the
    test's own thread (the old global count also saw tests running in
    parallel)
- Builds for `wasm32-unknown-unknown` with the scalar FFT path.
