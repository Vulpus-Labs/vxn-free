# 0003: Wesley, a three-band Beverley

Status: proposed (2026-11-25)

## Context

Beverley crushes the whole signal at once: one gain, one bit depth, one
curve. On a full mix or a bass-heavy part the lows dominate what the
quantiser does, so crushing enough to grit the top end wrecks the
bottom, and taming the bottom leaves the top clean.

Charm (ADR 0002, shelved) split its input into bands with zero-latency
Linkwitz-Riley crossovers, and that part worked. Wesley puts the two
together: Beverley's crush chain on each of three bands.

## Decision

1. **Plugin.** Wesley, id `labs.vulpus.wesley`, vendor Vulpus Labs,
   features audio-effect, distortion, multiband, stereo. Beverley's
   sibling, named to match. Web searches (2026-11-25) found no audio
   plugin named Wesley; a trademark search in class 9 is due before the
   first release, as for Charm.

2. **Zero latency.** As Beverley: no oversampling, no block processing.
   The crossovers are recursive filters, so their phase turns, but
   nothing is delayed.

3. **Three bands that sum flat.** LOW, MID and HIGH, split by two
   fourth-order Linkwitz-Riley crossovers, LOW/MID and MID/HIGH, both
   movable (draft ranges 40 Hz–1 kHz and 500 Hz–12 kHz, defaults 200 Hz
   and 2.5 kHz). The input splits at the lower one; the rest splits at
   the upper one. LOW also passes through the upper crossover's allpass
   (a second-order allpass at that frequency, Q = 1/√2), so it turns in
   phase as MID and HIGH do between them. With every band uncrushed the
   three sum to the input through a fixed allpass: flat in magnitude.
   The upper crossover is kept at least an octave above the lower, so
   MID never vanishes and the filters never cross.

4. **Beverley's chain on each band.** Each band has its own GAIN, AUTO
   GAIN, GAMMA, BIT DEPTH, SMOOTHING and CRUSH MODE, as Beverley's, and
   its own MIX (that band's dry against crushed) and LEVEL (its output,
   in dB). Per-band auto gain matters more here than in Beverley: a band
   is quieter than the whole, and auto gain brings each to full scale
   before it is crushed.

   LOW's crusher is off by default (its ON param), so out of the box
   Wesley leaves the bass clean below the lower crossover and crushes
   MID and HIGH apart. Crushed bass is a sound of its own (a slow
   waveform quantised into a staircase gives a square-edged fuzz) and is
   wanted on a bass part alone, but on a full mix or a drum bus the bass
   is what most needs protecting, and crushing it adds products in MID
   and HIGH's range. Tuning decides whether LOW keeps a full strip, a
   reduced one (ON, BIT DEPTH, MIX, LEVEL) or none.

5. **Shared settings.** STEREO (mono, separate, linked, as Beverley's)
   and auto gain's DETECT and DECAY apply to every band, so the bands
   stay one instrument. A global MIX blends the summed bands against the
   dry input. The dry path is the uncrushed bands' sum, not the raw
   input: the bands are turned in phase, and mixing them with the raw
   input would comb around the crossovers.

6. **Solo and bypass.** Each band can be switched off (passing its band
   uncrushed) or soloed, to set it up by ear. Solo is a param, so it is
   saved, but only for setting up; a soloed band mutes the others.

7. **Out-of-band products stay.** Crushing a band makes harmonics and
   aliases outside it. They are left in, as in most multiband
   distortion: refiltering each band after the crush would take the
   edge off and cost phase. Measured in the engine ticket, and revisited
   by ear.

8. **One chain, shared with Beverley.** Beverley's per-channel chain
   (settings, glides, auto gain detectors, the crush) becomes a type
   Beverley and Wesley both use. Beverley's sound and behaviour don't
   change, and its tests keep passing.

9. **Listen early.** Charm reached a DAW late, after its distinctive
   features were built, and they turned out to be inaudible. Wesley
   gets a plain faceplate and a CLAP build as soon as its engine plays,
   and ranges, defaults and point 7 are settled by ear before the real
   faceplate is drawn.

10. **Faceplate.** Original, in Beverley's family (the 1990s rack style
    of ticket 0013): three band strips side by side, with the
    crossovers and shared settings apart.

## Consequences

- Up to about 33 params: 9 a band (GAIN, AUTO GAIN, GAMMA, BIT DEPTH,
  SMOOTHING, CRUSH MODE, MIX, LEVEL, and ON or SOLO) and 6 shared
  (LOW XOVER, HIGH XOVER, STEREO, DETECT, DECAY, MIX), fewer if LOW's
  strip is cut down. The faceplate is wide; whether every band needs
  every control is a question for tuning.
- CPU is about three Beverleys plus five filters a channel; Beverley's
  chain is cheap, so Wesley stays well under a percent of a core.
- `vxn-free-dsp` gains a three-band split. Charm's crates keep using
  the two-way crossover as they are.
- The release workflow builds `wesley-clap` beside `beverley-clap`.
