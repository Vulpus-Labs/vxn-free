---
id: E002
title: "Beverley: the bitcrusher as a CLAP effect"
status: closed
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
| 0005 (closed) | CLAP plugin: params, state, tests, clap-validator |
| 0006 (closed) | Manual |
| 0012 (closed) | Auto gain: PEAK and RANGE detectors, DECAY |
| 0013 (closed) | Faceplate rendered in Blender, 1990s rack style |
| 0039 | Crush display: an LCD window in the panel (after E005's 0038) |

## Dependency order

0003 after 0001; 0004 after 0003; 0005 after 0004; 0006 after 0005; 0012 after 0004, before 0005 closes.

## Close-out (2026-11-24)

Beverley shipped as a stereo CLAP effect (0.1.0) and then as CLAP and
VST3 (0.1.1). It was signed and notarized for macOS, validated, published
to downloads.vulpuslabs.com and checked in REAPER. Beside the module's
sound and controls it gained MIX and STEREO (0003), PEAK and RANGE auto
gain with DECAY (0012), a rendered 1990s rack faceplate (0004, 0013) and
a PDF manual measured from the engine (0006).
