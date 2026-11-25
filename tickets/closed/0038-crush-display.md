---
id: "0038"
title: "A crush display: the settings applied to one cycle of a sine"
priority: medium
created: 2026-11-25
epic: E005
depends: ["0033"]
---

## Summary

A display for Beverley and Wesley showing what the dialled-in settings
do: one cycle of a sine through the crush chain, drawn over the sine
itself. It reads the settings from the model, not the engine, so it
moves as the knobs do, with or without audio.

## Acceptance criteria

- [x] A shared editor crate, `vxn-free-ui`, with a canvas widget built
      from one chain's params (GAIN, AUTO GAIN, GAMMA, BIT DEPTH,
      SMOOTHING, CRUSH MODE, MIX) and a style.
- [x] The curve is the engine's own crush (`vxn_free_dsp::crush`), one
      point a pixel column. With AUTO GAIN on, the sine is at full scale
      (what the chain sees); with it off, at a reference of −6 dBFS
      before GAIN, so GAIN visibly drives it into the quantiser and then
      into clipping.
- [x] It repaints only when one of its params changes.
- [x] Tests: the curve matches `Prepared::crush` point for point; it
      changes with each param it reads; snapshots of a few settings.

## Close-out (2026-11-25)

- `crates/vxn-free-ui`: editor widgets the plugins share (engines never
  depend on it). `CrushView::new(ChainParams, CrushStyle)`, a canvas
  widget; `crush_cycle(values, n)` computes it; `CrushStyle::WELL` (a
  dark well, amber trace) for plain faceplates.
- The curve is `vxn_free_dsp::crush::Prepared::crush` on one cycle of a
  sine, one point a logical pixel column, the output blended with the
  sine by MIX. Both are drawn relative to the sine's peak: the sine (faint)
  fills the height, the output shows its shape and level against it.
  AUTO GAIN on: a full-scale sine. Off: −6 dBFS before GAIN
  (`FIXED_GAIN_REFERENCE`), so +6 dB fills the quantiser and more clips;
  a clipped sine comes out quieter, as Beverley undoes the gain after the
  crush (+18 dB: clipped at a quarter of the sine's peak).
- It repaints only when one of its seven params changes (checked in
  Wesley's editor test: moving MID's depth repaints MID's display and
  not HIGH's).
- Tests: the curve is the engine's crush point for point; each param
  changes it (GAIN only with AUTO GAIN off); MIX 0 or 16 bits give the
  sine; driving past full scale clips as above. Snapshots are in
  Wesley's editor tests (0040) and Beverley's (0039).
