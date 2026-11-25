---
id: "0019"
title: "Charm faceplate, editor and flock display"
priority: medium
created: 2026-11-24
epic: E004
depends: ["0016", "0018", "0020", "0022", "0028"]
---

## Summary

An original SVG faceplate, replacing 0020's plain one, and an editor
showing the flock at work. The display can be developed against recorded
bench traces (0023) before it is connected to live frames.

## Acceptance criteria

- [ ] Faceplate drawn from scratch in SVG. Controls grouped as FLOCK
      (VOICES, DETUNE, SPREAD, COHESION, SEPARATION, TURBULENCE), SPACE
      (WIDTH, AUTO WIDTH, LOW XOVER, MID DEPTH, HIGH DEPTH) and LISTEN
      (DUCK, BLOOM), with MIX, plus whatever control 0028 gives the
      allpass decorrelation. Any merging of controls is
      decided here and the ADR updated to match.
- [ ] The flock display (ADR 0002, point 10), a custom widget:
      - axes: pan horizontal (with AUTO WIDTH walls), delay vertical
        (floor at the bottom)
      - MID and HIGH voices in two colours, each with a detune tail as a
        fraction of DETUNE
      - dot size and brightness following Duck and Bloom
- [x] The engine writes one telemetry frame per flock tick (0022), with
      no allocation or lock on the audio thread. The editor draws the
      latest frame. A frame that is torn or out of date is never drawn.
- [ ] Hand check: on a drum loop, each hit shows as a pulse across the
      flock; on a pad, the flock wanders without pulses.
- [ ] Controls that do nothing in the current state are dimmed (for
      example WIDTH's meaning while AUTO WIDTH is on, if it changes).
- [ ] Snapshot tests for the faceplate and for frozen display frames:
      a sustained tone, a wide stereo input with AUTO WIDTH's walls
      closed in, and a ducked onset.

## Progress (2026-11-25): the flock display

The display is built and on the plain faceplate, ahead of 0020's DAW
check, as the summary allows. The faceplate design itself waits for
0024, which decides the controls (the decorrelation control among them).

- Frame: grows from 71 to 91 floats (`observe::FRAME_WORDS`), adding what
  the display needs: the tick's SPREAD and speed limit, each band's pan
  limit (`BandObs::width`, AUTO WIDTH's walls) and each voice's Duck and
  Bloom gain where it reads (`VoiceObs::shape`, the side-buffer at its
  delay).
- `charm::display::FlockDisplay`, a canvas widget on `canvas.flock`:
  - pan across, hard left to hard right; delay up, floor at the bottom
  - MID gold, HIGH teal; walls at each band's limit in its colour, MID's
    on the top half and HIGH's on the bottom so equal walls both show;
    the mono guard (±0.83) dashed
  - each voice a dot with a tail back along the delay axis, as long as
    its share of the speed limit (26 px at the limit)
  - shape scales the dot's radius (55–100 %) and brightness, and a
    voice's level its opacity, so voices fade in and out with VOICES
  - a frame older than `STALE_MS` (250 ms) isn't drawn; a frame of the
    wrong size isn't either
- ADR 0002 point 10 updated: the horizontal axis is absolute pan, not
  −WIDTH to +WIDTH, so WIDTH's narrowing shows.
- The plain faceplate grows to 720 × 484 with the display across the
  top; its snapshot is re-blessed.
- Tests (`crates/charm/tests/display.rs`):
  - snapshots of hand-built frames (not engine renders, which round
    differently per platform): a sustained tone, a wide input with the
    walls closed in, a ducked onset part way up the flock
  - a stale frame and a wrong-sized frame draw nothing; a new frame
    brings the flock back; only the display's rect repaints
  - from the engine: every voice inside the plot and within its walls;
    wide stereo noise closes the walls below 40 % of WIDTH and mono
    opens them past 95 %; a burst after silence at DUCK 100 % drops
    some voice's shape below 0.5
- To do: the faceplate design and grouping, dimmed controls, the hand
  check on a drum loop and a pad.

## Fix (2026-11-25): frames reach the display in a DAW

First look in a DAW: the display's grid drew but no flock. The CLAP
processor wraps the engine in silkscreen's licence gate, which didn't
forward `Engine::frame`, so no frame was ever published. Fixed in
silkscreen 0.3.1 (with a test that every `Engine` hook passes the gate);
the workspace moves to v0.3.1. A test here now runs the editor over a
real bridge sized by `UI.frame`, as the plugin does.
