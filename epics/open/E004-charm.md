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

Phase 1: an engine to hear and measure.

| Ticket | What |
|---|---|
| 0026 | DSP: modulated delay line (measured), LR4 crossovers, side-buffer |
| 0014 | Engine core: flock, bands, mix, observation struct |
| 0023 | Bench: offline render, metrics, traces |
| 0024 | First tuning pass (milestone M1) |
| 0020 | CLAP plugin with a plain faceplate |

Phase 2: the analysis features, each measured on the bench.

| Ticket | What |
|---|---|
| 0007 | (E003) FFT, streaming STFT, peaks |
| 0015 | Analysis sidechain: onsets, peaks, coherence, traced |
| 0017 | Clarity: comb-notch avoidance (riskiest, so first) |
| 0016 | Duck and Bloom through the side-buffer |
| 0018 | Auto width and the mono guard (milestone M2) |

Phase 3: the product around it.

| Ticket | What |
|---|---|
| 0022 | silkscreen telemetry frames: release and bump |
| 0019 | Faceplate, editor and flock display |
| 0021 | Manual and release checks |

## Dependency order

0026 after 0001; 0014 after 0026; 0023 after 0014; 0024 after 0023;
0020 after 0024; 0015 after 0007 and 0023; 0016, 0017 and 0018 after
0015, worked in the order 0017, 0016, 0018; 0022 after 0014; 0019 after
0016, 0017, 0018, 0020 and 0022; 0021 after 0019.

0007 belongs to E003, which can run beside E004. Glow-Up's own analysis
work is in 0025, so Charm doesn't wait on it.
