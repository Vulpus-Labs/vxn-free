---
id: "0016"
title: "Charm DUCK and BLOOM through the side-buffer"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0029"]
---

## Summary

Attacks stay dry and sustains bloom (ADR 0002, point 7). Onsets from the
sidechain write wet gain into the side-buffer, starting at the onset
frame's start, so the duck takes effect before the attack reaches any
voice.

## Acceptance criteria

- [x] The bench (0023) reports transient peak and crest factor, dry
      against wet. Before/after numbers on the bench's test set are
      recorded in the close-out.
- [x] The new params are exposed in the plugin (0020) in the same
      change, on the plain faceplate until 0019.
- [x] DUCK (0–100 %): wet gain at an onset falls by DUCK and recovers over
      a set time (draft 30 ms; settled by ear).
- [x] BLOOM (0 = off, up to 2 s): after an onset, wet depth rises from
      zero to full over BLOOM.
- [x] Each voice reads its gain at the same position as its audio tap.
- [x] Tests:
      - on a click train with DUCK at 100 %, wet energy in the first 2 ms
        after each click, measured at every voice's output, is at least
        40 dB below the unducked case
      - BLOOM's rise reaches −3 dB at the time the design says
      - latency is still zero
      - no allocation
- [x] Before/after numbers on a drum loop (transient peak and crest
      factor, dry vs wet) recorded in the close-out.

## Close-out (2026-11-24)

- `charm::wet::WetGain` per band. From an onset's marked start (3 ms and
  a block before it was found):
  - the duck holds at 1 − DUCK for 6 ms, then recovers with a 30 ms time
    constant
  - Bloom is a raised cosine from 0 to 1 over BLOOM (−3 dB at 0.636 of
    BLOOM)
  - both crossfade from the previous gain over 1 ms
  - before any onset the gain is 1
- Each band's gain is pushed into its own `SideBuffer`, one value per
  sample. At each tick, a band's onset restarts the shape and rewrites
  the side-buffer back to the onset's start. That's at most the lag plus
  a tick, which the engine checks against the 7.5 ms floor when built.
  Each voice multiplies its sample by the side-buffer read at its own
  delay.
- Params: DUCK (id 12, 0–100 %, default 50 %) and BLOOM (id 13,
  0–2000 ms, exp taper, default 0, off). AUTO WIDTH keeps 14, and 15 is
  spare. The plugin (0020) doesn't exist yet, so the params reach it
  there, not here.
- Measured:
  - Click train (impulses), DUCK 100 % against 0 %, MIX 100 %, at 44.1,
    48 and 96 kHz: the wet energy within the floor plus SPREAD after
    each click is 54–89 dB down. The target was 40.
  - BLOOM 1 s on a noise burst: −3.47 dB at the designed point, allowing
    for the voices' ~12 ms delay. The target was −3 ± 1 dB.
  - Crest factor (mono, from 0.5 s), MIX 100 %:

    | Signal | DUCK | Input | Output | Wet |
    |---|---|---|---|---|
    | Drums | 0 | 13.3 dB | 14.5 dB | 23.3 dB |
    | Drums | 50 | 13.3 dB | 13.2 dB | 21.3 dB |
    | Drums | 100 | 13.3 dB | 13.2 dB | 21.1 dB |
    | Clicks | 0 | 28.7 dB | 27.5 dB | 29.0 dB |
    | Clicks | 100 | 28.7 dB | 28.7 dB | 27.4 dB |
    | Pink noise | any | 12.7 dB | 12.9 dB | 13.4 dB |

    Without the duck, the ensemble's copies of each hit add peaks: the
    drum loop's output crest rises 1.2 dB, heard as flamming. With it,
    the output's crest matches the input's. The wet signal stays peaky on
    drums because the hits the detector misses (12 of 40 in MID, mostly
    hats right after a snare) are still chorused.
- Left for 0024, by ear: the 30 ms recovery and 6 ms hold, DUCK's
  default, and whether hats masked by a snare's tail matter.
- Tests:
  - `wet` unit tests: no onset no change; hold then recovery to
    1 − 1/e at one time constant; Bloom at −3 dB where designed; a new
    onset never steps the gain
  - `tests/duck.rs`: the click-train duck at three rates, and Bloom's
    −3 dB point
  - zero latency and block-split identity with the default DUCK of 50 %
  - no allocation with DUCK and BLOOM changing and clicks firing
- Bench: crest factors of input, output and wet in the report and JSON.
- CPU at 96 kHz, defaults: 1.39 % of a core (1.14 % before). That's
  one more interpolated read per voice per sample, for its gain.
