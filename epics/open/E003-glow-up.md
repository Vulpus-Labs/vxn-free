---
id: E003
title: "Glow-Up: the harmonic exciter as a CLAP effect"
status: open
created: 2026-11-24
---

## Goal

Glow-Up as a zero-latency CLAP (and VST3) effect, as ADR 0005 decides:
peaks found by a running spectrum, a bank of twelve narrow filters
boosting what is already there at two ratios of each peak (each set as
NUMERATOR / DENOMINATOR, 2/1 and 3/2 by default), resonance up to
self-oscillation, GRID to TRACK focus, and a display of what it is
doing. A MODE switch picks MONO (left in, copied to both outputs) or
STEREO (mid analysed once, each channel filtered). In a DAW early,
tuned by ear, then given its faceplate.

## Tickets

| Ticket | What |
|---|---|
| 0007 | Analysis DSP: FFT, streaming STFT, peak detection (shared with E004) |
| 0025 | Peak detector, held resonator, partial tracker |
| 0008 | Engine: the filter bank, grids and control mapping |
| 0010 | CLAP plugin with a plain faceplate (in a DAW) |
| 0009 | Faceplate, editor and tracking display |
| 0011 | Manual |
| 0043 | Polyphonic warble, found in the DAW |
| 0043 | Polyphonic warble (closed) |

## Dependency order

0007 after 0001; 0025 after 0007; 0008 after 0025; 0010 after 0008;
0009 after 0010; 0011 after 0009. E003 can run beside E002.
