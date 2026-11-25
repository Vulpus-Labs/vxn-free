# 0004: Wesley, a three-band Beverley

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
   features audio-effect, distortion, stereo (CLAP has no standard
   multiband feature). Beverley's
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
   applies to every band. Auto gain's DETECT and DECAY are each band's
   own (amended 2026-11-25; first drafted as shared): a short release
   that suits the treble makes the gain pump within each bass cycle, so
   one DECAY can't suit every band. Draft DECAY defaults: LOW 300 ms,
   MID 115 ms (Beverley's), HIGH 40 ms. A global MIX blends the summed bands against the
   dry input. The dry path is the uncrushed bands' sum, not the raw
   input: the bands are turned in phase, and mixing them with the raw
   input would comb around the crossovers.

6. **Solo and bypass.** Each band can be switched off (passing its band
   uncrushed) or soloed, to set it up by ear. Solo means solo: one band
   at a time. It is one param, SOLO (off, LOW, MID or HIGH), shown as an
   LED button a band; pressing a band's solos it (putting out any other),
   pressing the lit one clears it. Being a param, it is saved, but it is
   only for setting up.

7. **Out-of-band products stay.** Crushing a band makes harmonics and
   aliases outside it. They are left in, as in most multiband
   distortion: refiltering each band after the crush would take the
   edge off and cost phase. Measured in the engine ticket, and revisited
   by ear. Measured (0033): with MID alone crushed at 4 bits, a 0.4–2 kHz
   sine's products are 27–28 dB under the signal in HIGH's range, 36–41
   dB within MID, and 76–90 dB down in LOW's. A crushed band mostly adds
   treble above itself.

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
    of ticket 0013), rendered the same way and in wine red where
    Beverley is dark teal-blue: each band a column, as on the plain
    faceplate, holding Beverley's strip folded into rows (name, ON and
    SOLO; its crush display; GAIN, AUTO, DETECT, DECAY; GAMMA, BIT
    DEPTH, SMOOTHING; CRUSH, LEVEL, MIX), and a row along the bottom with
    the title, the crossovers, STEREO and MIX. Engraved grooves part the
    face into that grid (ticket 0036). Earlier layouts, each band a row
    (1444 px wide) and the shared controls in a first column (1192 px),
    were dropped for this one (972 × 700).

## Consequences

- 38 params: 11 a band (GAIN, AUTO GAIN, GAMMA, BIT DEPTH, SMOOTHING,
  CRUSH MODE, MIX, LEVEL, ON, DETECT, DECAY; ids 1–33) and 5 shared
  (LOW XOVER, HIGH XOVER, STEREO, MIX, SOLO; ids 34–38), fewer if
  tuning cuts LOW's strip down. Ids were renumbered before any release.
- CPU is about three Beverleys plus five filters a channel; Beverley's
  chain is cheap, so Wesley stays well under a percent of a core.
- `vxn-free-dsp` gains a three-band split. Charm's crates keep using
  the two-way crossover as they are.
- The release workflow builds `wesley-clap` beside `beverley-clap`.
