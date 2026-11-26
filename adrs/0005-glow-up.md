# 0005: Glow-Up, a targeted harmonic filter with two boosts

Status: proposed (2026-11-26)

## Context

ADR 0001 names Glow-Up as a port of the rebuilt Glow Up module: a
running spectrum finds the input's peaks and steers a small bank of
narrow band-pass filters that boost what is already there at a fifth,
an octave and two octaves above each peak. It is a filter, not a pitch
shifter: at RESONANCE 0 it adds nothing that is not in the input.

The three fixed multiples (1.5×, 2×, 4×) are a choice, not a law. A
player wanting the third harmonic (a twelfth), a major third (5/4) or
the octave below (1/2) has no way to get it.

## Decision

1. **Plugin.** Glow-Up, id `labs.vulpus.glow-up`, features audio-effect,
   filter. Zero latency (point 3).

2. **Two boosts, each a ratio.** Where the module had three amounts at
   fixed multiples, Glow-Up has two boosts, A and B. Each has an AMOUNT
   and a ratio set by two stepped selectors, NUMERATOR and DENOMINATOR,
   each 1 to 8: the boost sits at N/D times each peak's frequency.
   - Defaults: A at 2/1 (the octave) and B at 3/2 (the fifth), both at
     25 %: the module's two most characteristic boosts.
   - Any ratio from 1/8 to 8 is allowed. N = D boosts the peaks
     themselves; N < D boosts below them, where something is there to
     boost (a lower partial, another instrument).
   - Two equal ratios are one boost, as two near targets always are
     (point 5).
   - The ranges and defaults are drafts, settled by ear (point 10).

3. **Analysis steers; the audio never waits.** An STFT (Hann, overlap
   4, a fixed window of 2048 samples) is used only to find peaks; the
   audio path is the input plus the filters' sum and has no latency.
   Only the detection lags, by about a window and a hop. The window was
   first a control (WINDOW: 512, 1024 or 2048); it was dropped
   (2026-11-26) as not worth a control, keeping 2048, the size it
   loaded at.

4. **Peaks.** Once a frame, from the frame's magnitudes:
   - a peak is a maximum over two bins either side, in this frame alone
     (a smoothed maximum lags a moving peak);
   - its smoothed power (each frame's power mixed in at 0.8) must stand
     THRESHOLD (0–30 dB) above the previous frame's mean power, and above
     a floor of 60 dB under full scale, so silence and hiss make none;
   - its prominence rises from 0 to 1 over the 12 dB above THRESHOLD, so
     a boost comes in smoothly as its peak rises;
   - its centre is estimated between bins from the two larger
     magnitudes, by the closed form that is exact for a steady sine
     through a Hann window; its amplitude from its magnitude.

5. **Targets.** Each peak gives a candidate at A's ratio and at B's.
   A candidate's salience is its boost's AMOUNT × the peak's prominence
   × TONE's tilt (±6 dB an octave about 1 kHz, by the target's
   frequency), capped at 1, × a bound that fades boosts from a quarter
   of the sample rate to nothing at 0.42 of it. Candidates closer than
   the tracking window (half the bandwidth, at least 50 cents) are one
   target, taking the larger salience. The twelve most salient are
   kept, a target a filter already holds counting 0.15 more.

6. **The bank and FOCUS.** Twelve filters sound at once, with twelve
   spares for tails. Each is a TPT state-variable band-pass, scaled to
   unity at its centre, its output gain up to 4 (+14 dB). FOCUS blends
   two ends:
   - **GRID** (fully anticlockwise): filters sit on the points of a fixed
     grid, each point driven by the candidates near it through the
     band-pass's own response. Filters never move; a bend sweeps
     partials through resonances, which swell as it passes. The module's
     character.
   - **TRACK** (fully clockwise): filters sit on each peak's exact
     ratio, steered at audio rate by a partial tracker on the peak (a
     band-pass and its zero crossings), so they follow bends and
     vibrato within a cycle or two.
   - Between, each filter's centre is pulled from its grid point toward
     its target, in log frequency, by FOCUS.
   - GRID picks the grid: LIN (the analysis bins' spacing), IRR (a fixed
     irregular pattern, densest through the midrange, like a room's
     modes) or TET (twelve-tone equal temperament at A = 440 Hz).

7. **A sounding filter is never retuned.** A target is matched to the
   filter that held it, which follows it. A filter whose target goes
   gets one frame's grace, then releases (to −60 dB in 150 ms). A new
   target takes a free filter, then the quietest tail, then steals the
   weakest active filter if it beats it by 0.15 and that filter has held
   long enough; a steal fades the filter out over 10 ms before it moves.
   A new target must persist for a number of frames first, so clicks and
   noise bursts make none.

8. **RESONANCE, up to self-oscillation.** RESONANCE lowers each
   filter's damping by its target's salience, as far as a quarter of
   WIDTH's. Past that a strong peak makes the filter sing at its target.
   The oscillation's level is held by the damping (nothing in the loop
   saturates, which would pull the pitch), at a share of its source
   peak, and it dies when its peak goes. The filters' outputs are scaled
   by WIDTH's damping, never the live one, so nothing jumps as a filter
   enters or leaves oscillation.

9. **The rest of the controls,** as the module's, adapted per ADR 0001:
   WIDTH (10–200 cents, the same at every ratio), SPEED (attack, decay,
   hold and persistence together, slow and smooth to fast and twitchy),
   DRY CUT (the filters alone), MIX (equal power), and a soft limit
   transparent to ±1 and never past ±2. MODE picks MONO (left in, copied
   to both outputs; the default, Glow-Up being chiefly a guitar effect)
   or STEREO (the mid, (L + R) / 2, analysed once, and each channel
   filtered by its own bank following the same targets).

10. **Listen early.** As for Wesley (ADR 0004, point 9): a plain
    faceplate and a CLAP build follow the engine at once, and ratios,
    ranges and defaults are settled by ear before the real faceplate.

## Consequences

- Params: A AMOUNT, A NUM, A DEN, B AMOUNT, B NUM, B DEN, WIDTH,
  RESONANCE, FOCUS, MIX, THRESHOLD, TONE, GRID, SPEED, DRY CUT, MODE:
  16. WINDOW's id, 11, is retired.
- `vxn-free-dsp` gains the peak detector, the resonator with its held
  oscillation, and the partial tracker (ticket 0025).
- STEREO costs a second bank's filters, not a second analysis.
- The ratios are not tied to the harmonic series: a ratio like 5/4 on a
  saw lands between partials and boosts little but the skirts. Ratios
  that land on partials (2, 3/2, 3, 4) are where the effect lives; the
  manual says so.
