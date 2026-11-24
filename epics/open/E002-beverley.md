---
id: E002
title: "Beverley: the bitcrusher as a CLAP effect"
status: open
created: 2026-11-24
---

## Goal

Beverley as a stereo CLAP effect that sounds like the module: gain or
auto-gain, gamma, fractional bit depth, smoothing, symmetric or
asymmetric crush, and mono, separate or linked stereo.

## Tickets

| Ticket | What |
|---|---|
| 0003 (closed) | Engine: the crush chain, ported, with tests |
| 0004 (closed) | Faceplate and editor |
| 0005 | CLAP plugin: params, state, tests, clap-validator |
| 0006 | Manual |
| 0012 (closed) | Auto gain: PEAK and RANGE detectors, DECAY |
| 0013 (closed) | Faceplate rendered in Blender, 1990s rack style |

## Dependency order

0003 after 0001; 0004 after 0003; 0005 after 0004; 0006 after 0005; 0012 after 0004, before 0005 closes.
