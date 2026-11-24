---
id: E003
title: "Glow-Up: the harmonic exciter as a CLAP effect"
status: open
created: 2026-11-24
---

## Goal

Glow-Up as a CLAP effect that keeps the rebuilt module's behaviour: peaks
found by a running spectrum, a bank of twelve narrow filters boosting
their fifths, octaves and double octaves, resonance up to
self-oscillation, GRID to TRACK focus, and a display of what it is doing.
A MODE switch picks MONO (left in, copied to both outputs) or STEREO (mid
analysed once, each channel filtered).

## Tickets

| Ticket | What |
|---|---|
| 0007 | Analysis DSP: FFT, streaming STFT, peak detection (shared with E004) |
| 0025 | Partial tracking, TPT SVF |
| 0008 | Engine: the filter bank, grids and control mapping |
| 0009 | Faceplate, editor and tracking display |
| 0010 | CLAP plugin: params, state, tests, clap-validator |
| 0011 | Manual |

## Dependency order

0007 after 0001; 0025 after 0007; 0008 after 0025; 0009 after 0008; 0010 after 0009;
0011 after 0010. E003 can run beside E002.
