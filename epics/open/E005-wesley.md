---
id: E005
title: "Wesley: Beverley in three bands"
status: open
created: 2026-11-25
---

## Goal

Wesley as a zero-latency CLAP (and VST3) effect, as ADR 0003 proposes:
Beverley's crush chain on each of three bands, split by Linkwitz-Riley
crossovers that sum flat, with per-band controls and shared stereo and
auto gain settings. In a DAW early, tuned by ear, then given its
faceplate.

## Tickets

| Ticket | What |
|---|---|
| 0031 | DSP: a three-band split that sums flat |
| 0032 | Beverley's chain made a reusable type, Beverley unchanged |
| 0033 | Wesley engine: three chains, solo and bypass, shared settings |
| 0034 | CLAP plugin with a plain faceplate (in a DAW) |
| 0035 | Tuning by ear: ranges, defaults, out-of-band products |
| 0036 | Faceplate in Beverley's family |
| 0037 | Manual and release |

Beverley's own open tickets (0005, 0027) go on in parallel; 0032 is the
only one here that touches Beverley, and leaves its sound unchanged.

## Dependency order

0031 and 0032 first, in either order; 0033 after both; 0034 after 0033;
0035 after 0034; 0036 after 0035; 0037 after 0036.
