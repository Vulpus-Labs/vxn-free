---
id: "0017"
title: "Charm CLARITY: steering comb notches off partials"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0015"]
---

## Summary

Voices are pushed away from delays whose dry + voice comb notches, at
(2k + 1) / 2d Hz, would land near strong peaks (ADR 0002, point 7). The
push is a force in the flock, so DETUNE's speed limit also limits how fast
a voice moves under it.

Three rules from the ADR apply:
- the term only repels, and only at short range, so a voice never parks
- each peak's push scales with its strength
- the term is gated by how sparse the band is, so it fades out on dense
  polyphony and noise

## Acceptance criteria

- [ ] The bench (0023) reports per-partial level loss in the mono sum,
      the gate value and RMS delay velocity. Before/after numbers on
      the bench's test set are recorded in the close-out.
- [ ] The new params are exposed in the plugin (0020) in the same
      change, on the plain faceplate until 0019.
- [ ] A repel-only term per voice: for each peak within a stated range of
      one of the voice's notches, a push in d away from that alignment,
      scaled by the peak's strength. Zero beyond the range, and no
      attraction anywhere.
- [ ] A sparseness gate per band: either the peak set's share of band
      energy or one minus spectral flatness, chosen by measurement and
      smoothed so the gate doesn't flicker. Effective weight =
      CLARITY × gate.
- [ ] The term is computed on the flock tick from the latest peak set, at
      a bounded cost (stated, and measured with 8 voices and 8 peaks per
      flock). Each flock uses only its own band's peaks.
- [ ] Measurement, recorded in the close-out, using these test signals at
      stated frequencies:
      - a sustained harmonic tone (sparse)
      - a sustained clean four-note chord
      - a distorted-guitar chord (dense)

      Measure at CLARITY 0 and at full:
      - time-averaged level loss at each strong partial in the
        mono-summed output. Target on the sparse tone: at full CLARITY
        the worst partial loses at most half as many dB as at 0.
      - the gate's mean value. Target on the dense chord: near zero
        (bound stated).
      - RMS delay velocity across voices (motion). Target: at full
        CLARITY it is at least 80 % of its value at 0 on every signal, so
        no voice parks.
- [ ] Tests:
      - no voice's detune exceeds DETUNE with CLARITY at full
      - on a sustained tone at full CLARITY, every voice's delay keeps
        moving: no voice stays within a small window for longer than a
        stated time
      - on white noise the gate stays below its dense-material bound
      - with CLARITY at 0 the output is bit-identical to the output
        without the term
      - no allocation
