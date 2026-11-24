---
id: E004
title: "Charm: a listening ensemble chorus"
status: open
created: 2026-11-24
---

## Goal

Charm as a zero-latency CLAP chorus, as ADR 0002 decides:

- voices moving as a flock, with detune as a speed limit
- bass left unchorused
- attacks ducked and sustains blooming, using analysis that sees each
  sample before any voice plays it
- comb notches steered off strong partials
- width that adapts to the input and holds up in mono

## Tickets

| Ticket | What |
|---|---|
| 0013 | DSP: modulated delay line (measured), LR4 crossovers, side-buffer |
| 0014 | Engine core: flock, bands, mix, controls |
| 0015 | Analysis sidechain: small STFT, onsets, peaks, coherence |
| 0016 | Duck and Bloom through the side-buffer |
| 0017 | Clarity: comb-notch avoidance |
| 0018 | Auto width and the mono guard |
| 0022 | silkscreen telemetry frames: release and bump |
| 0019 | Faceplate, editor and flock display |
| 0020 | CLAP plugin |
| 0021 | Manual |

## Dependency order

0013 after 0001; 0014 after 0013; 0015 after 0007 and 0014; 0016, 0017
and 0018 after 0015, in any order; 0022 after 0014; 0019 after 0016,
0017, 0018 and 0022; 0020 after 0019; 0021 after 0020. E004 can run
beside E002. It shares 0007 with E003.
