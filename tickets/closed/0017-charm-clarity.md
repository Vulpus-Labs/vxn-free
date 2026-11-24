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

## Close-out (2026-11-24): dropped

None of the acceptance criteria is met: Clarity was built, measured and
dropped, and its code removed. ADR 0002 records the decision (point 7).
What was learnt:

- Bench at MIX 100 %, defaults otherwise. Partial loss is the
  time-averaged dip of each known partial in the output's mono sum
  against the input's, counting losses only. Motion is the MID flock's
  RMS speed as a share of the limit.

  | Version | Tone, worst partial | Chord, worst partial | Motion |
  |---|---|---|---|
  | Clarity off | −2.45 dB | −2.60 dB | 0.51 |
  | Each voice pushed off its own notch with the dry signal (the ADR's design) | −2.92 dB | −3.09 dB | 0.44–0.50 |
  | The same with a 40 ms window for peaks | −3.47 dB | −2.95 dB | 0.25–0.37 |
  | The same, allowing for the band filters' phase | −2.23 dB | −5.49 dB | 0.25 |
  | Push only while a partial's actual sum dips (force 60) | −1.94 dB | −2.86 dB | 0.40–0.46 |
  | The same, force 200 | −1.68 dB | −4.10 dB | 0.36 |
  | The same, force 600 | −2.60 dB | −4.25 dB | 0.34–0.44 |

- Why each step:
  - The 5 ms window can't resolve a 220 Hz tone's partials. With
    188 Hz bins, about one smeared peak was found per tick.
  - Each voice is its band's signal, and the LR4 filters turn its phase
    by a good part of a cycle at 880 Hz, so notches weren't where the
    first form assumed.
  - Per-voice notch zones exist whether a partial is dipping or not. A
    chord's eight partials fenced the voices in, and unprotected
    partials sat in standing dips.
- Why it can't work well enough: voices moving within DETUNE sweep
  their combs through several dips a second at 1 kHz. That sweep is the
  chorus's beating. Steering bounded by the same speed limit can't
  outrun it, and pushing harder slows the flock, so the dips stop moving
  and get worse.
- What stays:
  - the bench's partial-loss metric
  - a white-noise signal in the bench's test set
  - two gate measures were compared on the way: tonality (1 − spectral
    flatness) is fooled by near-empty bands and by coloured noise; the
    peaks' share of band power ranks signals correctly
- The work is kept outside the repo as a patch (not committed),
  including a closed-form crossover response and a second, 40 ms STFT.
- Comb colouring is to be handled by decorrelating the voices with
  allpass filters (0028).
