---
id: "0016"
title: "Charm DUCK and BLOOM through the side-buffer"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0015"]
---

## Summary

Attacks stay dry and sustains bloom (ADR 0002, point 7). Onsets from the
sidechain write wet gain into the side-buffer, starting at the onset
frame's start, so the duck takes effect before the attack reaches any
voice.

## Acceptance criteria

- [ ] The bench (0023) reports transient peak and crest factor, dry
      against wet. Before/after numbers on the bench's test set are
      recorded in the close-out.
- [ ] The new params are exposed in the plugin (0020) in the same
      change, on the plain faceplate until 0019.
- [ ] DUCK (0–100 %): wet gain at an onset falls by DUCK and recovers over
      a set time (draft 30 ms; settled by ear).
- [ ] BLOOM (0 = off, up to 2 s): after an onset, wet depth rises from
      zero to full over BLOOM.
- [ ] Each voice reads its gain at the same position as its audio tap.
- [ ] Tests:
      - on a click train with DUCK at 100 %, wet energy in the first 2 ms
        after each click, measured at every voice's output, is at least
        40 dB below the unducked case
      - BLOOM's rise reaches −3 dB at the time the design says
      - latency is still zero
      - no allocation
- [ ] Before/after numbers on a drum loop (transient peak and crest
      factor, dry vs wet) recorded in the close-out.
