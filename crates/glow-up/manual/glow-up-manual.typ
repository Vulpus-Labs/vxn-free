// Glow-Up's manual. Build with ./build.sh, which draws the face and measures the engine first:
// every face, plot and table here is read from data/, which the example
// `cargo run -p glow-up --example manual` writes from the plugin's own code (ADR 0003).
#import "/docs/manual/vulpus-manual.typ": *

#show: manual.with(title: "Glow-Up", kind: "Harmonic filter", version: "0.3.0")

#let pos = json("data/positions.json")
#let img(name) = read("data/faces/" + name + ".png", encoding: none)
#let data(name) = table-of(csv("data/" + name + ".csv"))

// The faceplate's sections, in panel units (art/glow-up/faceplate.py).
#let output-group = (36, 0, 152, 330)
#let boost-group = (188, 0, 224, 330)
#let display-group = (412, 0, 356, 330)
#let filter-group = (768, 0, 196, 330)

// The partials of the test note, 1 to 16, and a boost's axis.
#let partial-x = (min: 0.3, max: 16.7, ticks: (1, 2, 4, 6, 8, 10, 12, 14, 16), title: "partial")
#let boost-y(first) = (min: 0, max: 16, ticks: (0, 6, 12), title: if first { "dB raised" })
#let small(title) = align(center, text(font: sans, size: 7.5pt, weight: "semibold", title))

// One knob at three settings: how much each partial is raised.
#let kb = data("knobs")
#let knob-row(prefix, settings, y: boost-y) = grid(columns: 3, column-gutter: 2mm,
  ..settings.enumerate().map(((i, (v, name))) => {
    small(name)
    plot(height: 32mm, legend: false, x: partial-x, y: y(i == 0),
      series: ((data: column(kb, prefix + "_" + v), color: cat.at(0), kind: "stem",
        width: 0.6pt),))
  }))

// --- Title page ----------------------------------------------------------------------------------

#page(header: none, footer: none, margin: (x: 20mm, top: 28mm, bottom: 20mm))[
  #set par(justify: false)
  #text(font: sans, size: 30pt, weight: "bold")[Glow-Up]
  #v(-4mm)
  #text(font: sans, size: 12pt, fill: ink-2)[Harmonic filter · CLAP and VST3 plugin · free and
    open source]
  #v(6mm)
  #align(center, face(img("init"), pos, width: 100%))
  #v(6mm)
  #grid(columns: (1fr, 1fr), column-gutter: 10mm,
    [
      Glow-Up listens for the strongest notes and overtones in a sound and brightens it at
      chosen intervals above them: an octave, a fifth, or any ratio you set. It does this with
      narrow filters that boost what is already there, so it adds shimmer and focus without
      adding notes, until you turn up its resonance and the filters start to sing.

      It adds no delay. Start with the Quick start on the next page.
    ],
    [
      #set text(font: sans, size: 8.5pt)
      #outline(title: text(size: 9pt, weight: "semibold")[Contents], depth: 1, indent: 0pt)
    ],
  )
  #place(bottom + left, text(font: sans, size: 7.5pt, fill: ink-3)[
    Vulpus Labs · manual for version 0.3.0
  ])
]

= Quick start

Put Glow-Up on a guitar, a synth, a voice or a pad and play. At its defaults, boost A brightens
each strong pitch an octave up and boost B a fifth up, each at a quarter of its strength. The
sound gets a gentle glow; the screen shows the input in red and, in amber, where Glow-Up is
lifting it. Then try:

- *AMOUNT* in BOOST A and BOOST B: up for more glow. Turn one down to zero to hear the other alone.
- *MULT* and *DIV*, under each *AMOUNT*: the boost's interval, as a ratio. MULT 3 over DIV 1 is
  an octave and a fifth up; MULT 1 over DIV 2 is an octave below.
- *FOCUS*, top right: towards TRACK, the boosts follow bends and vibrato exactly; towards GRID
  they stay put and a bend sweeps through them.
- *RESONANCE*: up towards SING for a ringing, vocal tone. Past SING, strong notes make the
  filters sing at their own pitch.
- *DRY CUT*, bottom left, to hear only what Glow-Up adds.

#figure(
  face(img("init"), pos, width: 100%, callouts: (
    (1, "mix"), (2, "mono"), (3, "stereo"), (4, "dry_cut"),
    (5, "a_amount"), (6, "a_mult"), (7, "a_div"),
    (8, "display"), (9, "grid_irr"),
    (10, "resonance"), (11, "focus"), (12, "width"), (13, "speed"), (14, "tone"),
    (15, "threshold"),
  )),
  caption: [The controls, at their defaults, with a note playing. BOOST B has the same three
    controls as BOOST A.],
)

#key(
  (1, "MIX", [The whole of Glow-Up's sound against the clean one.]),
  (2, "MONO", [Left input to both outputs. The default.]),
  (3, "STEREO", [Each side boosted on its own.]),
  (4, "DRY CUT", [Hear only the boosts.]),
  (5, "AMOUNT", [How strong the boost is.]),
  (6, "MULT", [The boost's ratio: times this…]),
  (7, "DIV", […divided by this.]),
  (8, "Screen", [The input, the boosts, and the pitches found.]),
  (9, "GRID", [Where the resonances sit at GRID: LIN, IRR or TET.]),
  (10, "RESONANCE", [How sharp and ringing the boosts are.]),
  (11, "FOCUS", [Fixed resonances (GRID) or exact tracking (TRACK).]),
  (12, "WIDTH", [How narrow each boost is.]),
  (13, "SPEED", [How quickly boosts come and go.]),
  (14, "TONE", [Favour the low or the high boosts.]),
  (15, "THRESHOLD", [How strong a pitch must be to be boosted.]),
  columns: 2,
)

= What Glow-Up does

#summary[Glow-Up finds the strongest pitches in the sound, then puts a narrow boost at a chosen
  ratio above (or below) each. It brightens what is already there.]

A note from a guitar, a voice or most synths is not one pitch but many: the note itself and a
row of overtones above it, at two, three, four times its pitch and so on. Glow-Up keeps a
running picture of the sound's pitches and picks out the strongest: the peaks. For each peak, each
boost aims a narrow filter at the peak's pitch times its ratio. Where the sound has something
there, an overtone, or another note, the filter lifts it.

So an octave boost (2/1) on a note lifts its second overtone, its fourth, its sixth: everything
an octave above something strong. The result is brighter and more focused, a glow around the
note rather than a new note on top. It is not a pitch shifter or a harmoniser: where the ratio
lands on nothing, nothing is added. The exception is *RESONANCE* past its SING mark, where
strong peaks make their filters sing on their own (see below).

Glow-Up uses at most twelve filters at once, given to the strongest targets, so on a thick chord
or a full mix it concentrates on what stands out.

The sound itself is never delayed. Finding the peaks takes a moment, so a boost arrives a few
hundredths of a second after its note (how quickly is up to *SPEED*).

= The boosts

#summary[Two boosts, A and B, each an *AMOUNT* and a ratio, *MULT* over *DIV*. The defaults
  are an octave (2/1) and a fifth (3/2) up, each at 25%.]

#block(breakable: false, below: 5mm, grid(columns: (52mm, 1fr), column-gutter: 6mm, align: horizon,
  face(img("init"), pos, crop: boost-group, width: 52mm, badge-size: 3.8mm, callouts: (
    (1, "a_amount"), (2, "a_mult"), (3, "a_div"), (4, "b_amount"), (5, "b_mult"),
    (6, "b_div"))),
  key(
    (1, "A AMOUNT", [0 to 100%, default 25%.]),
    (2, "A MULT", [1 to 8, default 2.]),
    (3, "A DIV", [1 to 8, default 1.]),
    (4, "B AMOUNT", [0 to 100%, default 25%.]),
    (5, "B MULT", [1 to 8, default 3.]),
    (6, "B DIV", [1 to 8, default 2.]),
  )))

*AMOUNT* sets how strongly the boost lifts its targets. At 100% a filter boosts its target by
up to 14 dB, about five times; a boost on a quieter peak is gentler. At 0 the boost is off.

*MULT* and *DIV* set the boost's ratio: the boost sits at each peak's pitch times *MULT*,
divided by *DIV*. Some useful ratios:

#figure(
  table(columns: 4, align: (left, left, left, left),
    [*MULT / DIV*], [*Interval*], [*On a single note it lifts*], [*Character*],
    [2 / 1], [an octave up], [the 2nd, 4th, 6th… overtones], [bright, open; the default A],
    [3 / 2], [a fifth up], [the 3rd, 6th, 9th…], [rich, organ-like; the default B],
    [3 / 1], [an octave and a fifth], [the 3rd, 6th, 9th, 12th…], [hollow, reedy],
    [4 / 1], [two octaves up], [the 4th, 8th, 12th…], [airy, glassy],
    [5 / 4], [a major third up], [mainly the 5th], [subtle on one note, more on chords],
    [1 / 1], [the peaks themselves], [the strongest pitches], [focused, nasal],
    [1 / 2], [an octave down], [the lower overtones], [fuller, warmer],
  ),
  caption: [A few ratios. Any *MULT* from 1 to 8 over any *DIV* from 1 to 8 works.],
)

Ratios that land on a note's overtones (2/1, 3/2, 3/1, 4/1) are where the effect is strongest.
A ratio such as 5/4 mostly lands between a single note's overtones and lifts little; on a chord
or a mix, where other notes fill those gaps, it can do much more. A ratio below 1 boosts below
each peak, where there is something there to boost: a lower overtone, or another instrument.
Two boosts at the same ratio act as one.

#let rt = data("ratios")
#figure(
  grid(columns: 5, column-gutter: 1mm,
    ..(("r21", "2 / 1"), ("r32", "3 / 2"), ("r31", "3 / 1"), ("r54", "5 / 4"), ("r12", "1 / 2"))
      .enumerate().map(((i, (c, name))) => {
        small(name)
        plot(height: 34mm, legend: false, x: partial-x + (ticks: (1, 4, 8, 12, 16)),
          y: boost-y(i == 0),
          series: ((data: column(rt, c), color: cat.at(0), kind: "stem", width: 0.6pt),))
      }),
  ),
  caption: [How much each overtone of a held note (a 200 Hz sawtooth, sixteen partials shown)
    is raised by boost A alone at full *AMOUNT*, at five ratios, with B off. Partial 1 is the
    note itself.],
)

= The filters

#summary[The FILTER section shapes all the boosts at once: how sharp they are, whether they
  stay put or follow the notes, how quickly they react, and which pitches get them.]

#block(breakable: false, below: 5mm, grid(columns: (46mm, 1fr), column-gutter: 6mm, align: horizon,
  face(img("init"), pos, crop: filter-group, width: 46mm, badge-size: 3.8mm, callouts: (
    (1, "resonance"), (2, "focus"), (3, "width"), (4, "speed"), (5, "tone"),
    (6, "threshold"))),
  key(
    (1, "RESONANCE", [0 to 100%, default 0. SING is half way.]),
    (2, "FOCUS", [GRID (0) to TRACK (100%), default GRID.]),
    (3, "WIDTH", [10 to 200 cents, default 50.]),
    (4, "SPEED", [SLOW (0) to FAST (100%), default 80%.]),
    (5, "TONE", [−6 to +6 dB an octave, default 0.]),
    (6, "THRESHOLD", [0 to 30 dB, default 6 dB.]),
  )))

== Focus and grid

*FOCUS* decides where the filters sit.

- At *GRID* (fully anticlockwise, the default) the filters sit on fixed pitches, the points of
  a grid, and never move. A target excites the grid points nearest it. Play a bend or a slide
  and the overtones sweep through these fixed resonances, swelling as they pass: the character
  of a resonant body, or a room.
- At *TRACK* (fully clockwise) each filter sits exactly on its target and follows it, through
  bends and vibrato, within a cycle or two. The boost stays even whatever the note does.
- In between, each filter is pulled from its grid point towards its target, part of the way.

#let gl = data("glide")
#figure(
  grid(columns: 3, column-gutter: 2mm,
    ..(("0", "FOCUS GRID"), ("50", "FOCUS 50%"), ("100", "FOCUS TRACK"))
      .enumerate().map(((i, (f, name))) => {
        small(name)
        plot(height: 40mm, legend: false,
          x: (min: 0, max: 2000, ticks: ((0, "0"), (500, "0.5"), (1000, "1"), (1500, "1.5"),
            (2000, "2")), title: "seconds"),
          y: (min: 300, max: 700, ticks: (300, 400, 500, 600, 700),
            title: if i == 0 { "Hz" }),
          series: (
            ..range(6).map(s => (data: column(gl, f + "_" + str(s)), color: cat.at(1),
              kind: "dot")),
            (data: column(gl, "target"), color: ink, width: 0.6pt, dash: "dashed"),
          ))
      }),
  ),
  caption: [A note held at 200 Hz, bent up to 300 Hz over a second, then held, with boost A at
    2/1: the dashed line is the octave the boost aims at, the dots the filters boosting near
    it, every 25 ms. At GRID the filters stay on fixed pitches and the note passes through
    them; at TRACK one filter rides the octave exactly.],
)

The *GRID* keys under the screen choose the grid. They matter only while *FOCUS* is short of
TRACK.

- *LIN*: evenly spaced pitches, about 23 Hz apart, so the grid is sparse in the bass and dense
  in the treble.
- *IRR* (the default): an irregular pattern, densest through the midrange, like the resonances
  of a room or a wooden body.
- *TET*: the notes of the ordinary twelve-note scale, tuned to A = 440 Hz. Boosts land on
  notes, which suits music in tune with it.

== Resonance

*RESONANCE* sharpens the filters around strong targets, so the boosts ring rather than glow.
Up to the SING mark (half way), the stronger a peak, the sharper and louder its boost. Past SING,
a strong peak makes its filter sing on its own at the target's pitch: a pure, whistling tone
that follows the note and stops when the note does. The screen marks a singing filter with a
white dot. Its level is kept to a share of the note that drives it, so it never runs away.

Use it sparingly on a mix, and freely on a lead line or a drone.

#figure(
  knob-row("resonance", (("0", "RESONANCE 0 (default)"), ("50", "50% (SING)"),
    ("100", "100%")), y: first => (min: -8, max: 24, ticks: (-6, 0, 6, 12, 18, 24),
    title: if first { "dB raised" })),
  caption: [The same note and boost at three *RESONANCE* settings, the axis taller than the
    others'. At 50% the octave boosts rise by up to another 10 dB and the partials between them
    dip by up to 6 dB. At 100% the filters on the strongest partials sing, held to a share of
    the partial that drives them, so the boosts there are no louder than at 0; the weaker upper
    ones ring louder.],
)

== Width

*WIDTH* is the width of each boost, in cents (a hundredth of a semitone), the same at every
pitch. Narrow, a boost lifts only its target; wide, it spreads to the overtones beside it and
sounds more like a broad tone control. Half travel is about 45 cents.


#figure(
  knob-row("width", (("10", "WIDTH 10 cents"), ("50", "50 cents (default)"),
    ("200", "200 cents"))),
  caption: [How much each partial of a 200 Hz sawtooth is raised by boost A at 2/1, full
    *AMOUNT*, *FOCUS* at TRACK, at three widths. At 200 cents the boosts spill onto the odd
    partials between them.],
)

== Speed

*SPEED* sets how quickly the boosts react: how soon a new pitch gets a filter, how fast it
swells and how fast it lets go. Slow is smooth and steady, fast is lively and twitchy. At the
default (80%), a boost begins about 40 ms after a note starts and is within a decibel of full
some 60 ms later.

#let sp = data("speed")
#figure(
  grid(columns: 4, column-gutter: 1mm,
    ..(("s0", "SPEED 0"), ("s50", "50%"), ("s80", "80% (default)"), ("s100", "100%"))
      .enumerate().map(((i, (c, name))) => {
        small(name)
        plot(height: 32mm, legend: false,
          x: (min: 0, max: 600, ticks: (0, 200, 400, 600), title: "ms"),
          y: (min: -30, max: 3, ticks: (-30, -20, -10, 0), title: if i == 0 { "dB" }),
          series: ((data: column(sp, c), color: cat.at(0), width: 0.9pt),))
      }),
  ),
  caption: [A note starts at 50 ms. The boosts' level (heard with *DRY CUT*), against where it
    settles, at four *SPEEDs*. At 0 the boost takes about half a second to come in fully; at 100%
    it is quickest, and briefly overshoots.],
)

== Tone

*TONE* tilts the boosts by pitch, about 1 kHz: turned right, high boosts get stronger and low
ones weaker; turned left, high boosts get weaker. A boost already at full strength can't rise
further. *TONE* changes the boosts only, not the rest of the sound.

#figure(
  knob-row("tone", (("-6", "TONE −6 dB/oct"), ("0", "0 (default)"), ("6", "+6 dB/oct"))),
  caption: [The same note and boost at three *TONE* settings. The note's partials run from
    200 Hz to 3.2 kHz.],
)

== Threshold

*THRESHOLD* is how far a pitch must stand out from the sound around it before Glow-Up treats it
as a peak, from 0 to 30 dB. Low, more pitches qualify, quieter overtones included; high, only
the strongest. Silence and quiet hiss never make peaks, whatever the setting.

#figure(
  knob-row("threshold", (("0", "THRESHOLD 0 dB"), ("6", "6 dB (default)"),
    ("30", "30 dB"))),
  caption: [The same note and boost at three *THRESHOLD* settings. At 0 dB the upper partials
    qualify too; at 30 dB nothing in this note stands out enough, and nothing is boosted.],
)

= The screen

#block(breakable: false, below: 5mm, grid(columns: (80mm, 1fr), column-gutter: 6mm, align: horizon,
  face(img("init"), pos, crop: display-group, width: 80mm, badge-size: 3.8mm, callouts: ()),
  [
    The screen shows what Glow-Up is doing, pitch from left to right (40 Hz to 16 kHz), level
    upwards.

    - The *red line* is the input.
    - The *amber line* is the output, drawn where it differs from the input. The glow between
      them shows where each boost lands and how far it spreads.
    - The *vertical lines* are the peaks found, brighter the stronger they are.
    - The *dots* are the filters, white while one sings.
  ]))

When the output falls below the input (*DRY CUT*, or *MIX* turned down) the gap shows in dark
red. With the host stopped, the screen dims and holds its last picture.

= Output and mode

#block(breakable: false, below: 5mm, grid(columns: (36mm, 1fr), column-gutter: 6mm, align: horizon,
  face(img("init"), pos, crop: output-group, width: 36mm, badge-size: 3.8mm, callouts: (
    (1, "mix"), (2, "mono"), (3, "stereo"), (4, "dry_cut"))),
  key(
    (1, "MIX", [DRY to WET, default WET.]),
    (2, "MONO", [Left input, both outputs. The default.]),
    (3, "STEREO", [Each side boosted.]),
    (4, "DRY CUT", [Off by default.]),
  )))

*MIX* blends Glow-Up's sound with the clean sound, from DRY to WET (the default), keeping the
loudness even through the middle. At WET the output is the input plus the boosts.

*DRY CUT* removes the input from the wet side, leaving only the boosts: a thin, ghostly layer of
overtones. Use it to hear exactly what Glow-Up is adding, or with *MIX* to blend that layer back
in by hand.

*MONO* and *STEREO* choose the mode. Glow-Up is chiefly a guitar effect, so it starts in
*MONO*: it takes the left input and sends the result to both outputs. *STEREO* finds the peaks
in left and right together, then boosts each side with its own filters, so the stereo picture
is kept; it costs a little more processing.

Glow-Up's output is kept from clipping harshly: anything above full scale is rounded off
smoothly, and the output never goes past twice full scale. The knobs respond smoothly as you
turn or automate them. Glow-Up adds no delay, so it can be used while recording or playing live.

= Starting points

Each of these changes only the controls it names; leave the rest at their defaults.

/ Shimmer: A *AMOUNT* 50%, B *MULT* 4, B *DIV* 1, B *AMOUNT* 30%, *TONE* +3 dB/oct. An airy
  octave-and-two-octaves sheen on clean guitar or pads.
/ Organ body: A *AMOUNT* 40%, B *AMOUNT* 40% (the defaults' octave and fifth, stronger). Rich and
  full on synths and keys.
/ Warm floor: A *MULT* 1, A *DIV* 2, A *AMOUNT* 50%, B *AMOUNT* 0. Lifts what lies an octave
  under the strongest pitches: fuller chords and basses.
/ Singing lead: *FOCUS* TRACK, *RESONANCE* 70%, *WIDTH* 20 cents. Whistling overtones that
  follow every bend.
/ Resonant body: *FOCUS* GRID, *GRID* IRR, *RESONANCE* 40%, *WIDTH* 100 cents. Bends and slides
  bloom as they pass fixed resonances, like an acoustic body.
/ Vocal air: A *AMOUNT* 30%, B *AMOUNT* 0, *TONE* +6 dB/oct, *MIX* 60%. Brightens a voice's
  upper overtones.

= Tips

/ Nothing seems to happen: check the *AMOUNTs*, and lower *THRESHOLD*. A ratio such as 5/4 on a
  single note lands between its overtones; try 2/1 or 3/2.
/ The boosts wobble on chords: turn *SPEED* down, or move *FOCUS* towards GRID.
/ Bends sound uneven: move *FOCUS* to TRACK.
/ Too harsh or whistly: lower *RESONANCE*, widen *WIDTH*, or turn *TONE* left.
/ Too boomy: turn *TONE* right, or raise the ratios.
/ Only one side of a stereo track comes through: *MONO* is lit; press *STEREO*.
/ The effect is too much: bring *MIX* down rather than the *AMOUNTs*; you keep the character and
  get the clarity back.

= Reference

== Controls

#let pr = csv("data/params.csv").slice(1)
#figure(
  table(columns: 4, align: left,
    [*Control*], [*From*], [*To*], [*Default*],
    ..pr.map(r => r.map(c => c.replace("-", "−"))).flatten()),
  caption: [Every control, as your host lists it for automation. Multiplier and Divisor are
    *MULT* and *DIV*.],
)

== The plugin

- Formats: CLAP and VST3, for macOS (Apple silicon and Intel), Windows and Linux. On a Mac it
  installs and opens without security warnings.
- Audio: stereo in and out. A mono track works too.
- Delay: none.
- Licence: free, under the MIT or Apache-2.0 licence. The source code is at
  #link("https://github.com/Vulpus-Labs/vxn-free").
- VST is a registered trademark of Steinberg Media Technologies GmbH.
