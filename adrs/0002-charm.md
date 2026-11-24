# 0002: Charm, a listening ensemble chorus

Status: proposed (2026-11-24)

## Context

The suite gains a third free effect: a chorus. Most choruses are a few
delayed copies of the input moved by sine LFOs. Their weaknesses are
known:

- The pitch of each copy swings back and forth with the LFO's slope,
  which reads as seasick wobble rather than players slightly out of tune.
- A periodic LFO is easy to hear as a cycle.
- Dry and delayed copies form comb filters whose notches sweep through
  sustained notes and hollow them out.
- Chorusing the bass smears it and widens what should stay central.
- Attacks are chorused as much as sustains, so drums and plucks soften.
- Widening material that is already wide makes the mono sum collapse.

Charm addresses each of these, and stays a zero-latency effect.

## Decision

1. **Plugin.** Charm, id `labs.vulpus.charm`, vendor Vulpus Labs,
   features audio-effect, chorus. This extends ADR 0001's list of
   plugins. It is an original design, not a port, so it has no module
   behaviour to keep.

   *Name.* "A charm of goldfinches": in older English a charm is also
   the blended sound of many birds singing together, a chorus made by a
   flock. The working name, Murmuration, was dropped because Harmonico
   DSP sells a reverb of that name, also modulated by a flock. Web
   searches (2026-11-24) found no audio plugin named Charm; the nearest
   is MeldaProduction's MCharmVerb. A trademark search in class 9 (USPTO
   and EUIPO) is due before the first release (ticket 0020). In listings
   the name is paired with the vendor or the word chorus, since "Charm"
   alone is hard to search for.

2. **Zero latency.** The audio path has no block processing: each output
   sample depends only on input up to that sample. The reported latency
   is zero at every setting and every rate. Analysis runs beside the
   audio path and steers it (point 6); it never sits in it.

3. **Voices are a flock.** Two to eight voices each read a modulated delay
   line. Each chorused band has its own flock (point 4). A voice is an agent in a two-dimensional space of (delay, pan):
   - Its position on the delay axis is its delay time. Its velocity on
     that axis is its pitch shift: a voice whose delay changes at rate
     d′ plays at ratio 1 − d′. Detune is therefore a speed limit. DETUNE
     (in cents) clamps every voice's delay velocity, so no voice is ever
     further out of tune than that.
   - Steering follows the boids rules. *Cohesion* pulls voices towards
     the flock's centre, so they stay one section. *Separation* pushes
     apart voices whose delays come close, since two voices at the same
     delay double up and comb. *Alignment* matches neighbours' velocities,
     so the section drifts together.
   - *Turbulence* adds a band-limited random force per voice, so the
     motion never settles and has no period.
   - Delay is bounded softly within a range set by SPREAD, with a floor
     set by point 6. Pan is bounded within ±WIDTH.

   The flock updates at a control rate (once per fixed internal tick, not
   per host block), and each voice's delay is interpolated per sample
   between ticks. There is no LFO, so there is no tempo sync. Its
   absence is deliberate.

4. **Bands.** Linkwitz-Riley (4th order) crossovers split the input into
   LOW, MID and HIGH.
   - Voices are fed from MID and HIGH only. LOW is never chorused, so
     the bass stays tight and central. The LOW crossover is a control
     (default 150 Hz).
   - The MID/HIGH crossover is fixed (draft 2.5 kHz, settled in the
     engine ticket), not a control.
   - MID and HIGH each have **their own flock**: their own delay line,
     VOICES voices and seed. The flock controls apply to both. Each
     flock responds only to its own band's analysis (onsets, peaks,
     coherence), so Clarity weighs only the peaks its voices' combs can
     actually notch. Two flocks also decorrelate the bands from each
     other, at the cost of twice the taps.
   - MID and HIGH have separate depth, so the upper band can be
     decorrelated more, as in a real section.
   - The dry signal is the unfiltered input. The wet signal is the sum of
     the chorused bands. Because LOW has no wet part, the crossovers never
     need to sum flat in the dry path.
   - Voices read the mid signal (L + R) / 2 of their band and are placed
     by pan. The dry signal keeps the input's own stereo image.

5. **Pan law.** Equal-power panning. Interaural time and level difference
   panning was considered and left out: it combs when summed to mono,
   which works against point 8.

6. **Analysis runs behind and still looks ahead.** A sidechain analyses the
   dry input on a fixed internal hop, independent of the host's block
   size. Its window is about 5 ms (256 samples at 48 kHz, scaled to the
   next power of two at other rates), with a hop of a quarter-window.
   - The analysis lags the input by at most one window plus one hop. Every
     voice reads at least that far into the past, so the analysis sees
     each sample before any voice plays it. The plugin gets this
     lookahead for free and still reports zero latency.
   - Results reach the voices through a **side-buffer** that runs
     alongside each delay line: the analysis writes gain values there
     after the fact, at the positions it has just examined, and each
     voice reads them at the same position as its audio tap.
   - The minimum voice delay is window + hop, rounded up (about 7 ms at
     every rate), and is enforced whatever the parameters.
   - Small windows give coarse bins (about 190 Hz at 48 kHz). That is
     acceptable because the analysis only serves MID and HIGH: LOW, where
     fine resolution would matter, is never chorused.

7. **What the analysis drives.**
   - **Duck.** Onsets (spectral flux per band) lower wet gain over the
     attack. They are written into the side-buffer from the start of the
     frame in which the onset was found, so the duck begins before the
     attack reaches any voice. DUCK sets the depth.
   - **Bloom.** After an onset, wet depth rises from zero over BLOOM
     time, the way a singer brings in vibrato late in a note. BLOOM = 0
     turns it off.
   - **Clarity.** Strong spectral peaks in MID and HIGH add a term to the
     flock's steering. For a voice at delay d, the dry + voice comb has
     notches at (2k + 1) / 2d Hz. Three rules shape the term:
     - *Repel only.* A peak pushes a voice away only while one of the
       voice's notches lies within a short range of it. Beyond that range
       the term is zero. There is no pull towards notch-free delays, so a
       voice never has a valley to settle in. A voice that stopped moving
       would leave a fixed comb, which is a static filter and no chorus.
     - *Weighted by strength.* Each peak's push scales with its strength,
       so on a chord the strongest partials, usually the fundamentals and
       low harmonics, get most of the protection.
     - *Gated by sparseness.* The term's overall weight is CLARITY times
       how peaky the band is: the share of the band's energy in its peak
       set, or one minus its spectral flatness, chosen in the ticket.
       On dense polyphony or noise the gate closes, and the flock moves
       freely. Comb notches are most audible on sparse, sustained tones
       and are masked on dense material, so Clarity acts where it matters
       and backs off where it can't help.

     Because the push goes through the flock's dynamics, the detune speed
     limit also limits how fast a voice can be pushed, so steering adds no
     pitch glitches of its own.
   - **Auto width.** Inter-channel coherence of the input, per band,
     smoothed over about 300 ms. Bands that are already wide get less pan
     spread; narrow bands get the full WIDTH. A guard limits the drop in
     each band's level when the output is summed to mono to a set bound
     (decided by measurement in its ticket).

8. **Determinism.** The same input and automation give the same output
   whatever the host's block size, and on every run: the flock and
   analysis ticks are fixed, and turbulence comes from a per-instance
   seeded generator. Offline bounces match realtime playback.

9. **Controls (first draft).** VOICES, DETUNE, SPREAD, COHESION,
   SEPARATION, TURBULENCE, WIDTH, AUTO WIDTH, LOW XOVER, MID DEPTH, HIGH
   DEPTH, CLARITY, DUCK, BLOOM, MIX. The engine ticket fixes ranges,
   defaults and ids. The faceplate ticket may group or merge controls.

10. **The flock display.** The editor plots what the engine is doing. It
    is part of the design, because it shows what the flock controls mean.
    - Horizontal: pan, −WIDTH to +WIDTH. AUTO WIDTH's limits are drawn as
      walls that close in.
    - Vertical: delay, from the floor (bottom, "front of the stage") to
      floor + SPREAD.
    - Each voice is a dot, MID and HIGH in two colours. A tail shows its
      velocity on the delay axis, which is its detune, as a fraction of
      DETUNE.
    - Clarity's no-go bands are shaded horizontally in the flock's
      colour, at d ≈ (k + ½) / p ± r / p for each peak p, with intensity
      = strength × sparseness gate.
    - Duck and Bloom scale each dot's size and brightness, so an onset
      shows as a pulse across the flock.

    The engine publishes one consistent frame per flock tick: every
    voice's delay, pan, velocity and gain, plus each band's peaks and
    gate. Silkscreen's telemetry holds 16 separate values with no
    consistent snapshot, which is not enough for this. Silkscreen
    therefore gains telemetry frames: a fixed-size block per instance,
    published as one snapshot, with no allocation or lock on the audio
    thread. The change is made and released in silkscreen first, and the
    tag is bumped here.

11. **Measured DSP.** The interpolation for modulated delay reads is
    chosen by measuring aliasing and CPU in a scratch harness, at the
    largest DETUNE and at 44.1, 48 and 96 kHz. The numbers go in the
    ticket.

## Prior art

Each building block has been done before:
- Multiband choruses: MeldaProduction MChorusMB, which can also split
  transients from tonal content, and Minimal Audio Flex Chorus.
- Many voices with random modulation: Caelum Audio Choric.
- Constant detune: triangle-swept bucket-brigade choruses.
- Fixed per-voice decorrelation against combing: Acon Digital Multiply.
- Flocks driving the modulation of time-based effects: ToneCarver Boids,
  where boids drive grains and delay taps, and Harmonico DSP Murmuration,
  where a flock replaces an FDN reverb's LFO.

As far as searches found, these parts are original:
- treating the flock's velocity on the delay axis as detune, so that
  DETUNE is a speed limit for every steering force
- Clarity
- ducking with lookahead at zero latency, using the wet path's own delay
- width driven by per-band coherence, with a mono guard

The manual describes Charm in those terms. It makes no claim to be the
first adaptive or flock-driven chorus.

## Out of scope

- Spectral resynthesis of partials (per-partial ensembles in the
  frequency domain). It needs block latency and is not wanted.
- Pitch (f0) detection and pitch-locked modulation. Specifying detune in
  cents already makes beating follow pitch, and peak picking serves
  Clarity without an f0.
- A frequency-shift layer, odd/even harmonic splitting, and a diffusion
  network. Candidates for later, each with its own ADR if taken up.

## Consequences

- `vxn-free-dsp` gains a modulated delay line, Linkwitz-Riley crossovers
  and a gain side-buffer. Its STFT (from Glow-Up, ticket 0007) gains
  window sizes down to 128.
- The flock is a small physics simulation. Its parameters interact, so
  its controls need tuning by ear before the ranges are fixed.
- Charm needs a silkscreen release with telemetry frames before its
  display can be built. Glow-Up's tracking display may use the same
  frames.
- The minimum delay of about 7 ms rules out flanger territory. Charm is
  a chorus and ensemble, not a flanger.
