---
id: "0042"
title: "Mid/side: a fourth STEREO mode for Beverley and Wesley"
priority: low
created: 2026-11-26
epic: E005
depends: []
---

## Summary

Add M/S to STEREO: left and right are encoded to mid and side, each is
crushed on its own (auto gain following each, as SEPARATE), and the
result is decoded back to left and right. Both plugins get it through
the shared chain (`vxn-free-dsp/src/crush_chain.rs`, `Stereo`), so the
engine work is done once.

What it is for: crushing the sides apart from the centre. With auto gain
on, the side signal, usually well below the mid, is brought up to full
scale before it is crushed, so grit spreads into the stereo field while
the centre (kick, bass, voice) is crushed at its own level. With auto
gain off, the quiet side is crushed far less than the mid at the same
GAIN, which is a gentler sound of its own.

## Design

- `Stereo::MidSide`, appended after LINKED so saved plain values 0–2
  keep their meaning. STEREO's labels become MONO, SEP, LINK, M/S.
- Encode `m = (l + r) / 2`, `s = (l − r) / 2`; decode `l = m + s`,
  `r = m − s`. Uncrushed, the round trip is exact up to rounding.
- Two `Follow`s, one on mid and one on side, as SEPARATE uses for left
  and right. MIX blends the decoded wet against the raw left and right,
  as now.
- One input channel: M/S means nothing, and `run_one` ignores it, as it
  does the other modes.
- Wesley: STEREO stays shared, so every band is crushed in M/S. The
  split is linear and the same on both channels, so encoding before or
  after it is the same; encode per band inside the chain and leave the
  split alone.

## Risks to measure first

- **Silence in the side.** On mono material the side is zero. Check
  what each crush mode puts out for an input of zero (ASYMMETRIC's grid
  may not hold zero), and what auto gain does with a side below its gate
  (`crush::GATE`, gain held at most +46 dB). Anything non-zero there
  becomes a left/right imbalance on a mono source. If it does, the fix
  belongs in the chain (for example, pass the side dry below the gate),
  decided here.
- **Side gain.** On near-mono mixes the side sits 20–30 dB under the
  mid; auto gain raising it to full scale may make M/S much louder or
  harsher in the sides than expected. Listen on drums, a full mix and a
  wide pad, and record whether a side-gain offset or a cap is wanted.
- **CPU.** Expect none worth measuring (two adds and two multiplies a
  sample each way); confirm with the existing bench.

## Compatibility

0.2.0 is released. CLAP state and automation use plain values, so
appending the option keeps old projects as they were. VST3 (through
clap-wrapper) automates in normalized values: a STEREO lane written at
1.0 (LINK) under 0.2.0 reads as M/S after the change. Presets and saved
state are CLAP state and are unaffected. Accept and note in the release
notes, unless the wrapper offers a way round.

## Faceplates

STEREO is a three-position switch on both panels. A fourth position
needs a different control: a four-way selector, or a separate M/S
toggle beside the switch (which would make it a second param, and the
chain a mode plus a flag). Decide here; redraw both panels in their own
style and update the crush display if it reflects STEREO.

## Acceptance criteria

- [ ] `Stereo::MidSide` in the chain, with tests: uncrushed round trip
      within rounding; with identical left and right, output left equals
      output right (bit-exact within a run) in every crush mode, auto
      gain on and off; block-size independence as the other modes.
- [ ] Silence-in-side behaviour measured and the numbers kept here.
- [ ] STEREO's spec in Beverley and Wesley gains M/S; plain values 0–2
      unchanged.
- [ ] Control decided and both faceplates updated; snapshots
      regenerated.
- [ ] Listened to in a DAW on drums, a full mix and a wide pad; notes
      recorded here.
- [ ] Both manuals describe M/S (plain English, per the manuals ADR).
- [ ] ADR 0004 point 5 amended; Beverley's STEREO description updated
      wherever it is recorded.
- [ ] VST3 automation note in the next release's notes.

## Deferred (2026-11-26)

Shelved before any work: not needed for now. The design and risks above
stand; take it up again from the silence-in-side measurement.
