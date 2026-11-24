---
id: E001
title: "Foundation: workspace, CI, releases"
status: closed
created: 2026-11-24
---

## Goal

A workspace on silkscreen that builds, checks and releases the plugins.
Decisions: [ADR 0001](../../adrs/0001-vxn-free.md).

## Tickets

| Ticket | What |
|---|---|
| 0001 (closed) | Workspace on silkscreen v0.2.1, `vxn-free-dsp`, CI |
| 0002 (closed) | Release builds: signed and notarized macOS, Windows, Linux, on GitHub releases |

## Dependency order

0001 first; 0002 after 0005 (a plugin to release).

## Close-out (2026-11-24)

The workspace builds on silkscreen v0.2.1, and CI checks every push on
macOS, Linux and Windows (0001). Tagging `vX.Y.Z` builds every plugin
signed and notarized for macOS, and for Windows and Linux, into a draft
GitHub release (0002). Beverley 0.1.0 was the first release, published to
downloads.vulpuslabs.com.
