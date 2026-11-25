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
     set by point 6. Pan is bounded within ±WIDTH. Pan speed has its own
     limit (half the width per second), because with a narrow SPREAD the
     delay axis's limit would let voices sweep the stereo field fast
     enough to be heard as tremolo.
   - Each voice passes through its own fixed allpass chain (four
     second-order stages, log-spaced across its band, offset per voice).
     Its level is untouched at every frequency, but its phase turns
     differently at each, so its comb with the dry signal and with other
     voices is no longer evenly spaced. Measured (ticket 0028): the
     comb's regularity falls by 16–40 %, how deep the dips go doesn't
     change, a click smears by under 0.6 ms, and it costs 1.2 % of a
     core at 96 kHz with eight voices. Whether it is heard is decided by
     ear in 0024; it is on until then.
   - Each voice's level is 1/√VOICES, so the ensemble's level holds as
     voices come and go. A voice added or removed fades over about 30 ms.

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
     flock responds only to its own band's analysis (onsets and
     coherence). Two flocks also decorrelate the bands from each other,
     at the cost of twice the taps.
   - MID and HIGH have separate depth, so the upper band can be
     decorrelated more, as in a real section.
   - The dry signal is the unfiltered input. The wet signal is the sum of
     the chorused bands. Because LOW has no wet part, the crossovers never
     need to sum flat in the dry path.
   - Voices read the mid signal (L + R) / 2 of their band and are placed
     by pan. The dry signal keeps the input's own stereo image.
   - The dry signal stays at unity. MIX sets the ensemble's level against
     it, from nothing to equal level at 100 %. A crossfade would take the
     bass away with the dry signal, since LOW has no wet part.

5. **Pan law.** Equal-power panning. Interaural time and level difference
   panning was considered and left out: it combs when summed to mono,
   which works against point 8.

6. **Analysis runs behind and still looks ahead.** A sidechain analyses the
   dry input's MID and HIGH bands on a fixed schedule (blocks of 16
   samples), independent of the host's block size. It uses no FFT: onsets
   come from each band's energy envelope (point 7), and coherence from
   each band's mid and side signals.
   - The analysis lags the input by a few milliseconds (an onset is
     marked as starting 3 ms and a block before it was found: 3.7 ms at
     44.1 kHz). Every voice reads further into the past than that, so the
     analysis sees each sample before any voice plays it. The plugin gets
     this lookahead for free and still reports zero latency.
   - Results reach the voices through a **side-buffer** that runs
     alongside each delay line: the analysis writes gain values there
     after the fact, at the positions it has just examined, and each
     voice reads them at the same position as its audio tap.
   - The minimum voice delay must exceed the analysis's lag plus one
     sample for the interpolator's reach. It is a fixed 7.5 ms at every
     rate, so Charm sounds the same at any rate, and the engine checks it
     when built. 7.5 ms was set when the analysis was a 5 ms STFT, which
     lagged 7.28 ms at 44.1 kHz (ticket 0015). The envelope detector
     (ticket 0029) lags far less, but the floor stays: lowering it would
     change the sound and edge towards flanging.

7. **What the analysis drives.**
   - **Duck.** Onsets lower wet gain over the attack. An onset is a 16-
     sample block whose band energy (a 1 ms follower) rises about 4 dB
     over the largest the band has been in the last 30 ms; the 30 ms
     reference keeps beating partials and buzzes from firing. It is
     written into the side-buffer from 3 ms before it was found, so the
     duck begins before the attack reaches any voice. DUCK sets the
     depth. Spectral flux from a 5 ms STFT was built first (ticket 0015)
     and compared (ticket 0029): the envelope found more drum hits, sooner,
     for less CPU, and neither caught a legato line's note changes.
   - **Bloom.** After an onset, wet depth rises from zero over BLOOM
     time, the way a singer brings in vibrato late in a note. BLOOM = 0
     turns it off.
   - **The shape** (ticket 0016), from an onset's marked start: the duck
     holds at 1 − DUCK for 6 ms, past the lookback and the attack, then
     recovers with a 30 ms time constant (both to be settled by ear in
     0024). Bloom is a raised cosine over BLOOM, at −3 dB 0.636 of the way
     through. Both crossfade from the previous gain over 1 ms, so a new
     onset never steps the wet signal. Each band ducks on its own onsets.
     Each voice reads the gain from the side-buffer at its own delay.
   - **Clarity: dropped (ticket 0017).** Clarity was to steer voices so
     their comb notches missed strong partials. It was built and measured
     three ways and doesn't work well enough to keep:
     - Pushing each voice off its own notch with the dry signal, as first
       designed, pushed voices *into* notches: each voice is its band's
       signal, and the band filters turn its phase by a good part of a
       cycle.
     - With that phase allowed for, a chord's partials fenced the voices
       in. Motion fell to half, and an unprotected partial sat in a
       standing dip 5.5 dB deep.
     - Pushing only while a partial's actual sum (dry plus every voice)
       dipped kept the motion, but improved the worst partial on a
       harmonic tone by 20–30 % (the target was 50 %), and made chords'
       unprotected partials worse.

     The limit is physical. Voices moving within DETUNE sweep their combs
     through several dips a second at 1 kHz: that sweep is the chorus's
     beating, and steering bounded by the same speed limit can't outrun
     it. Comb colouring is handled instead by decorrelating the voices
     (ticket 0028).
   - **Auto width.** Inter-channel coherence of the input, per band,
     smoothed over about 300 ms. With AUTO WIDTH on, each band's pan
     bound is WIDTH × (0.25 + 0.75 × max(coherence, 0)), glided over
     200 ms. Mono material gets the full WIDTH; uncorrelated or
     out-of-phase material, already wide, a quarter of it.
   - **Mono guard** (always on). The ensemble comes from the mid signal
     and is never inverted, so the only way it can lose level in mono is
     panning: a voice panned p with equal power loses
     10·log10((1 + cos(p·π/2))/2) dB, 3 dB at hard left or right. No
     voice is panned beyond ±0.83, which caps that at 2 dB a voice.
     Measured at WIDTH and MIX 100 % (ticket 0018): the output loses at
     most 0.26 dB more than the input in mono on mono material, and less
     than the input on uncorrelated noise.

8. **Determinism.** The same input and automation give the same output
   whatever the host's block size, and on every run: the flock and
   analysis ticks are fixed, and turbulence comes from a per-instance
   seeded generator. Offline bounces match realtime playback.

9. **Controls (first draft).** VOICES, DETUNE, SPREAD, COHESION,
   SEPARATION, TURBULENCE, WIDTH, AUTO WIDTH, LOW XOVER, MID DEPTH, HIGH
   DEPTH, DUCK, BLOOM, MIX. The engine ticket fixes ranges,
   defaults and ids. The faceplate ticket may group or merge controls.

10. **The flock display.** The editor plots what the engine is doing. It
    is part of the design, because it shows what the flock controls mean.
    - Horizontal: pan, hard left to hard right, so narrowing shows as
      narrowing. Each band's limit (WIDTH times AUTO WIDTH's share) is
      drawn as walls in its colour that close in, and the mono guard as
      faint lines. (First drafted as −WIDTH to +WIDTH; changed in ticket
      0019, since that scale hid WIDTH's own effect.)
    - Vertical: delay, from the floor (bottom, "front of the stage") to
      floor + SPREAD.
    - Each voice is a dot, MID and HIGH in two colours. A tail shows its
      velocity on the delay axis, which is its detune, as a fraction of
      DETUNE.
    - Duck and Bloom scale each dot's size and brightness, as the gain
      where that voice reads, so an onset shows as a pulse climbing the
      flock from the floor.
    - A frame older than 250 ms isn't drawn: with the host stopped, the
      flock goes dark rather than freezing.

    The engine publishes one consistent frame per flock tick: every
    voice's delay, pan, velocity, gain and Duck/Bloom gain, each band's
    onset, coherence and pan limit, and the tick's SPREAD and speed
    limit (91 floats). Silkscreen's telemetry holds 16 separate values with no
    consistent snapshot, which is not enough for this. Silkscreen
    therefore gains telemetry frames: a fixed-size block per instance,
    published as one snapshot, with no allocation or lock on the audio
    thread. The change is made and released in silkscreen first, and the
    tag is bumped here.

11. **Measured DSP.** The interpolation for modulated delay reads is
    chosen by measuring aliasing and CPU in a scratch harness, at the
    largest DETUNE and at 44.1, 48 and 96 kHz. The numbers go in the
    ticket. Ticket 0026 chose 4-point (third-order) Lagrange: it has the
    least error of the non-recursive interpolators, for about 0.8 ns a
    sample more than linear.

## Prior art

Each building block has been done before:
- Multiband choruses: MeldaProduction MChorusMB, which can also split
  transients from tonal content, and Minimal Audio Flex Chorus.
- Many voices with random modulation: Caelum Audio Choric.
- Constant detune: triangle-swept bucket-brigade choruses.
- Fixed per-voice decorrelation against combing: Acon Digital Multiply.
  Charm's allpass decorrelation (ticket 0028) is that idea.
- Flocks driving the modulation of time-based effects: ToneCarver Boids,
  where boids drive grains and delay taps, and Harmonico DSP Murmuration,
  where a flock replaces an FDN reverb's LFO.

As far as searches found, these parts are original:
- treating the flock's velocity on the delay axis as detune, so that
  DETUNE is a speed limit for every steering force
- ducking with lookahead at zero latency, using the wet path's own delay
- width driven by per-band coherence, with a mono guard

The manual describes Charm in those terms. It makes no claim to be the
first adaptive or flock-driven chorus.

## Out of scope

- Spectral resynthesis of partials (per-partial ensembles in the
  frequency domain). It needs block latency and is not wanted.
- Pitch (f0) detection and pitch-locked modulation. Specifying detune in
  cents already makes beating follow pitch.
- Steering voices away from comb notches (Clarity): built, measured and
  dropped (point 7, ticket 0017).
- A frequency-shift layer, odd/even harmonic splitting, and a diffusion
  network. Candidates for later, each with its own ADR if taken up.

## Consequences

- `vxn-free-dsp` gains a modulated delay line, Linkwitz-Riley crossovers,
  allpass filters and a gain side-buffer. Charm uses no FFT; the streaming
  STFT built for it (ticket 0007) serves Glow-Up.
- The flock is a small physics simulation. Its parameters interact, so
  its controls need tuning by ear before the ranges are fixed.
- Charm needs a silkscreen release with telemetry frames before its
  display can be built. Glow-Up's tracking display may use the same
  frames.
- The minimum delay of 7.5 ms rules out flanger territory. Charm is
  a chorus and ensemble, not a flanger.
