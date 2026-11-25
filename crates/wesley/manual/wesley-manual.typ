// Wesley's manual. Build with ./build.sh, which draws the faces and measures the engine first:
// every face, plot and table here is read from data/, which the example
// `cargo run -p wesley --example manual` writes from the plugin's own code (ADR 0003).
#import "/docs/manual/vulpus-manual.typ": *

#show: manual.with(title: "Wesley", kind: "Three-band crusher", version: "0.2.0")

#let pos = json("data/positions.json")
#let img(name) = read("data/faces/" + name + ".png", encoding: none)
#let data(name) = table-of(csv("data/" + name + ".csv"))

// The faceplate's sections, in panel units (crates/wesley/wesley.svg): a band's column, and the
// bottom row's crossover and output sections.
#let mid-column = (336, 0, 300, 552)
#let xover-group = (336, 552, 300, 148)
#let output-group = (636, 552, 336, 148)

#let band-colors = (cat.at(0), cat.at(1), cat.at(2))

// --- Title page ----------------------------------------------------------------------------------

#page(header: none, footer: none, margin: (x: 20mm, top: 28mm, bottom: 20mm))[
  #set par(justify: false)
  #text(font: sans, size: 30pt, weight: "bold")[Wesley]
  #v(-4mm)
  #text(font: sans, size: 12pt, fill: ink-2)[Three-band crusher · CLAP and VST3 plugin · free and
    open source]
  #v(6mm)
  #align(center, face(img("init"), pos, width: 78%))
  #v(6mm)
  #grid(columns: (1fr, 1fr), column-gutter: 10mm,
    [
      Wesley is a bitcrusher that works in three bands. It splits the sound into lows, mids and
      highs and crushes each on its own, with its own amount and character, then puts them back
      together. You can grit up a vocal's presence while its body stays smooth, fuzz a drum bus's
      snare and cymbals and leave the kick clean, or crush the bass alone.

      Each band has the controls of Beverley, Wesley's single-band sibling. Start with the Quick
      start on the next page.
    ],
    [
      #set text(font: sans, size: 8.5pt)
      #outline(title: text(size: 9pt, weight: "semibold")[Contents], depth: 1, indent: 0pt)
    ],
  )
  #place(bottom + left, text(font: sans, size: 7.5pt, fill: ink-3)[
    Vulpus Labs · manual for version 0.2.0
  ])
]

= Quick start

Put Wesley on a track or a bus and play. At its defaults the bass passes clean, and the mids and
highs are each crushed to 4 bits, each brought up to full level first, so a quiet band is
crushed as hard as a loud one. The result is gritty and lo-fi on top, with the low end intact.
Then try:

- *BIT DEPTH* in the MID and HIGH columns: down for more crunch, up for less. Setting them apart
  (say 3 bits in MID and 8 in HIGH) is where Wesley differs from an ordinary crusher.
- *SMOOTHING* in HIGH, to take off the fizz without losing the crunch. Try 20 to 50%.
- *SOLO* on any band, to hear what it is doing on its own. Press it again to hear everything.
- *ON* in the LOW column, to crush the bass too.
- *MIX*, bottom right, to blend in the original under the grit.

#figure(
  face(img("init"), pos, width: 84%, callouts: (
    (1, "mid_on"), (2, "mid_solo"), (3, "mid_display"),
    (4, "mid_gain"), (5, "mid_auto_gain"), (6, "mid_detect"), (7, "mid_decay"),
    (8, "mid_gamma"), (9, "mid_bit_depth"), (10, "mid_smoothing"),
    (11, "mid_crush_mode"), (12, "mid_level"), (13, "mid_mix"),
    (14, "low_xover"), (15, "high_xover"), (16, "stereo"), (17, "mix"),
  )),
  caption: [The controls, at their defaults. LOW is off, so its controls are dimmed. Every band
    has the controls numbered here in MID's column.],
)

#key(
  (1, "ON", [Crushes the band. Off, it passes clean.]),
  (2, "SOLO", [Hear this band alone.]),
  (3, "1 CYCLE", [What the band's settings do to a wave.]),
  (4, "GAIN", [How hard the band is driven into the crush.]),
  (5, "AUTO", [Sets the drive for you, following the band's level.]),
  (6, "DETECT", [How AUTO listens: PEAK or RANGE.]),
  (7, "DECAY", [How quickly AUTO lets go after a loud moment.]),
  (8, "GAMMA", [Whether quiet or loud parts break up first.]),
  (9, "BIT DEPTH", [How coarse the steps are: the amount.]),
  (10, "SMOOTHING", [How sharp the steps' edges are.]),
  (11, "CRUSH", [Buzzing (ASYM) or gating (SYM) character.]),
  (12, "LEVEL", [The band's volume in the mix.]),
  (13, "MIX", [The band's clean sound against its crushed one.]),
  (14, "LOW X", [Where LOW ends and MID begins.]),
  (15, "HIGH X", [Where MID ends and HIGH begins.]),
  (16, "STEREO", [How left and right are treated.]),
  (17, "MIX", [The whole of Wesley's sound against the clean one.]),
  columns: 2,
)

= The three bands

#summary[*LOW X* and *HIGH X* set where the sound is split. With every band off, the three add
  back up to the sound you put in.]

#block(breakable: false, below: 5mm, grid(columns: (80mm, 1fr), column-gutter: 6mm, align: horizon,
  face(img("init"), pos, crop: xover-group, width: 80mm, badge-size: 3.8mm, callouts: (
    (1, "low_xover"), (2, "high_xover"))),
  key(
    (1, "LOW X", [40 Hz to 1 kHz, default 200 Hz.]),
    (2, "HIGH X", [500 Hz to 12 kHz, default 2.5 kHz.]),
  )))

Below *LOW X* is LOW: the kick, the bass, the body of a voice or guitar. Between the two is MID,
where most notes and words live. Above *HIGH X* is HIGH: cymbals, breath, string noise and
sparkle. The defaults suit a full mix, a drum bus or a voice; move the crossovers to put a part
of the sound where you want it. To pull the kick drum's click into MID while its thump stays in
LOW, for example, bring *LOW X* down to about 100 Hz.

The bands overlap a little at each crossover, so nothing is cut off sharply, and they always add
back to an even sound: turn every band off and Wesley sounds like the original. MID is always at
least an octave wide: if *HIGH X* is set less than an octave above *LOW X*, Wesley splits an
octave above *LOW X* instead, though the knob stays where you left it.

#let bd = data("bands")
#figure(
  grid(columns: 3, column-gutter: 2mm,
    ..(("100 Hz / 1 kHz", 0), ("200 Hz / 2.5 kHz (default)", 1), ("500 Hz / 8 kHz", 2))
      .map(((name, i)) => {
        align(center, text(font: sans, size: 7.5pt, weight: "semibold", name))
        plot(height: 40mm, legend: false,
          x: (min: 20, max: 20000, log: true, ticks: ((20, "20"), (100, "100"), (1000, "1k"),
            (10000, "10k")), title: "Hz"),
          y: (min: -36, max: 6, ticks: (-36, -24, -12, 0), title: if i == 0 { "dB" }),
          series: (
            (data: column(bd, str(i) + "_low"), color: band-colors.at(0), width: 0.9pt),
            (data: column(bd, str(i) + "_mid"), color: band-colors.at(1), width: 0.9pt),
            (data: column(bd, str(i) + "_high"), color: band-colors.at(2), width: 0.9pt),
            (data: column(bd, str(i) + "_sum"), color: ink, width: 0.7pt, dash: "dashed"),
          ))
      }),
  ),
  caption: [How much of each pitch goes into each band, at three crossover settings (*LOW X* /
    *HIGH X*): LOW in blue, MID in orange, HIGH in green. Where two bands cross, each has half
    of it. The black dashed line is the three added together, every band off: level at every
    pitch.],
)

#pagebreak(weak: true)
= A band

#summary[Each band has the controls of a whole bitcrusher, plus its own *LEVEL* and *MIX*.
  Turn them in one band and the other two don't change.]

#block(breakable: false, below: 4mm, grid(columns: (62mm, 1fr), column-gutter: 6mm, align: top,
  face(img("solo"), pos, crop: mid-column, width: 62mm, badge-size: 3.8mm, callouts: (
    (1, "mid_on"), (2, "mid_solo"), (3, "mid_display"),
    (4, "mid_gain"), (5, "mid_auto_gain"), (6, "mid_detect"), (7, "mid_decay"),
    (8, "mid_gamma"), (9, "mid_bit_depth"), (10, "mid_smoothing"),
    (11, "mid_crush_mode"), (12, "mid_level"), (13, "mid_mix"))),
  {
    key(
      (1, "ON", [Lit while the band is crushed.]),
      (2, "SOLO", [Lit while the band is soloed.]),
      (3, "1 CYCLE", [The band's crush, on one cycle of a tone.]),
      (4, "GAIN", [0 to 20 dB. Greyed out while *AUTO* is on.]),
      (5, "AUTO", [On by default in every band.]),
      (6, "DETECT", [PEAK (down) or RANGE (up).]),
      (7, "DECAY", [1 ms to 5 s.]),
      (8, "GAMMA", [0.1 to 10.]),
      (9, "BIT DEPTH", [1 to 16 bits.]),
      (10, "SMOOTHING", [0 to 100%.]),
      (11, "CRUSH", [ASYM (down) or SYM (up).]),
      (12, "LEVEL", [−24 to +12 dB.]),
      (13, "MIX", [DRY to WET.]),
    )
    v(2mm)
    note[MID's column, with LOW switched on and MID soloed. While a band is off its crush
      controls are greyed out; *LEVEL* still works.]
  }))

== On and solo

*ON* switches the band's crush on and off. Off, the band passes through clean, so you can crush
any one or two bands and leave the rest alone. LOW starts off and MID and HIGH on.

*SOLO* lets you hear one band on its own, to set it up by ear. Only one band can be soloed at a
time: soloing another moves the solo to it, and pressing the lit *SOLO* again brings back all
three. The solo is saved with your project like any other setting, so clear it when you're done.

== The crush

These controls work as they do in Beverley, on the band alone. Beverley's manual
(#link("https://vulpuslabs.com/manuals/beverley/")[vulpuslabs.com/manuals/beverley]) shows each
of them in graphs.

- *BIT DEPTH* is the amount: 1 bit turns the band into a square wave, 4 bits (the default) is
  clearly crushed, 8 and above is a fine grain. It moves smoothly between whole numbers of bits.
- *SMOOTHING* rounds off the steps' edges, which turns down the harsh, whining overtones. At
  100% the steps are gone.
- *GAMMA* below 1 makes quiet parts break up first, so tails grit up and peaks stay clean;
  above 1, loud parts break up first.
- *CRUSH* on ASYM keeps buzzing as a sound fades, like an old sampler; on SYM quiet sound drops
  to silence, like a rough gate.
- The *1 CYCLE* display shows the band's settings on one cycle of a tone (faint) and what comes
  out (dark). It follows the knobs whether or not anything is playing.

== Gain and auto gain

*GAIN* drives the band harder into the crush and turns it back down afterwards, so it changes
how rough the band sounds without making it louder (until its peaks clip, when they come out
flatter and quieter). *AUTO* sets the gain for you from the band's level, so loud and quiet
passages are crushed alike.

*AUTO* is on in every band by default. It matters more in Wesley than in Beverley: a band holds
only part of the sound and is quieter than the whole, and without auto gain a quiet band only
touches the first few steps. *DETECT* picks how auto gain listens: RANGE (the default) follows
the wave's highest and lowest points and never lets a peak clip; PEAK follows its loudest
points and lets the very start of an attack clip a little, for bite.

*DECAY* sets how quickly auto gain lets go after a loud moment. Each band has its own, because
what suits the treble doesn't suit the bass: a bass note's wave is slow, and with a short *DECAY*
the gain swells between the peaks of every cycle. The steps then shrink to almost nothing over
most of the wave, and the bass loses its crush. The defaults
are 300 ms in LOW, 115 ms in MID and 40 ms in HIGH.

#let dc = data("decay")
#figure(
  grid(columns: 3, column-gutter: 2mm,
    ..(("d5", "DECAY 5 ms"), ("d40", "40 ms"), ("d300", "300 ms (LOW's default)"))
      .map(((c, name)) => {
        align(center, text(font: sans, size: 7.5pt, weight: "semibold", name))
        plot(height: 34mm, legend: false,
          x: (min: 0, max: 40, ticks: (0, 20, 40), title: "ms"),
          y: (min: -0.6, max: 0.6, ticks: (-0.5, 0, 0.5)),
          series: ((data: column(dc, c), color: cat.at(0), width: 0.9pt),))
      }),
  ),
  caption: [A 55 Hz bass note through LOW, switched on and soloed, at 3 bits. At 5 and 40 ms the
    gain swells between the wave's peaks, and the steps shrink until the wave is almost
    smooth. At 300 ms the gain holds steady and the steps are even.],
)

== Level and mix

*LEVEL* is the band's volume in Wesley's output, from −24 to +12 dB. Crushing can make a band
sound louder or brighter than it was; bring it back with *LEVEL*. It works whether the band is
on or off, so it doubles as a simple three-band tone control.

The band's *MIX* blends the band's crushed sound with its clean sound, from DRY to WET (the
default). Use it to keep some of a band's clarity under its grit.

= Why the bass starts clean

On a full mix, a drum bus or anything with a bass line, the low end is what most needs
protecting. Crushing a slow bass wave turns it into a staircase, and its sharp edges add buzz
far above the bass itself, in MID's and HIGH's range: a fuzz of its own, on top of whatever
those bands are doing. That's why LOW starts off.

On a bass part alone, that square-edged fuzz may be just what you want: switch LOW on. To fuzz
only the bass and leave the rest clean, switch LOW on and MID and HIGH off.

#let wv = data("wave")
#let sp = data("spectrum")
#figure(
  grid(columns: 2, column-gutter: 4mm, row-gutter: 2mm,
    ..(("defaults", "Defaults: LOW off"), ("low", "LOW on")).map(((c, name)) => {
      align(center, text(font: sans, size: 7.5pt, weight: "semibold", name))
    }),
    ..("defaults", "low").map(c => plot(height: 30mm, legend: false,
      x: (min: 0, max: 25, ticks: (0, 12.5, 25), title: "ms"),
      y: (min: -0.8, max: 0.8, ticks: (-0.5, 0, 0.5)),
      series: ((data: column(wv, c), color: cat.at(0), width: 0.8pt),))),
    ..("defaults", "low").map(c => plot(height: 34mm, legend: false,
      x: (min: 0, max: 4000, ticks: ((0, "0"), (1000, "1k"), (2000, "2k"), (3000, "3k"),
        (4000, "4k")), title: "Hz"),
      y: (min: -100, max: 0, ticks: (-100, -75, -50, -25, 0), title: if c == "defaults" { "dB" }),
      series: ((data: column(sp, c), color: cat.at(0), kind: "stem", width: 0.5pt),))),
  ),
  caption: [A bass line (80 Hz) and a lead (660 Hz) together, at the defaults and with LOW
    switched on: the waveform (top) and what it is made of (bottom; the two tall bars are the bass
    and the lead, the rest is what the crush adds). With LOW on, the bass is cut into steps,
    and what the crush adds between 600 Hz and 4 kHz is about 17 dB louder, roughly seven times.],
)

Crushing any band adds overtones above it, so crushing MID adds a little to HIGH's range too.
Wesley leaves them in: that edge is part of the sound. Turn *SMOOTHING* up in the band if it is
too much.

= Stereo and mix

#block(breakable: false, below: 5mm, grid(columns: (80mm, 1fr), column-gutter: 6mm, align: horizon,
  face(img("init"), pos, crop: output-group, width: 80mm, badge-size: 3.8mm, callouts: (
    (1, "stereo"), (2, "mix"))),
  key(
    (1, "STEREO", [MONO (down), SEP (middle) or LINK (up).]),
    (2, "MIX", [DRY to WET.]),
  )))

*STEREO* applies to every band, and matters mostly with *AUTO* on.

- *LINK* (switch up, the default) crushes left and right separately, but each band's auto gain
  follows both sides together, so the stereo picture stays put. Use it on most stereo material.
- *SEP* (middle) gives each side its own auto gain. A quiet side is pushed as hard as a loud one,
  which can make the image move around.
- *MONO* (switch down) mixes left and right together and crushes the result once, sending the
  same sound to both sides.

*MIX*, at the bottom right, blends the whole of Wesley's sound with the clean sound, from DRY to
WET (the default). The clean side is the three bands added back together uncrushed and
without their *LEVEL* settings, so at DRY Wesley sounds like the original.

The knobs respond smoothly as you turn or automate them, without clicks; the switches change at
once. Wesley adds no delay, so it can be used while recording or playing live.

= Starting points

Each of these changes only the controls it names; leave the rest at their defaults.

/ Drum bus grit: MID *BIT DEPTH* 5 bits, HIGH *BIT DEPTH* 6 bits, HIGH *SMOOTHING* 30%, *MIX*
  60%. Snare and cymbals get dirty while the kick stays round.
/ Telephone voice: LOW *LEVEL* −24 dB, HIGH *LEVEL* −12 dB, MID *BIT DEPTH* 4 bits, *LOW X*
  400 Hz, *HIGH X* 3 kHz. A thin, crushed, band-limited voice.
/ Bass fuzz: LOW *ON*, LOW *BIT DEPTH* 2 bits, MID *ON* off, HIGH *ON* off, *LOW X* 250 Hz. The
  bass becomes a square-edged fuzz and the rest of the sound is left clean.
/ Crunchy top: MID *ON* off, HIGH *BIT DEPTH* 3 bits, HIGH *MIX* 50%. Only the top end crackles.
/ Lo-fi mix: LOW *ON*, LOW *BIT DEPTH* 8 bits, MID and HIGH *BIT DEPTH* 6 bits, MID and HIGH
  *SMOOTHING* 25%. A gentle, even grain across the whole mix.
/ Fragile tails: MID *GAMMA* 0.3, MID *CRUSH* SYM. Notes are clean as they start and break up as
  they fade.

= Tips

/ Too harsh or fizzy: turn up *SMOOTHING* in HIGH first, then in MID; or raise *BIT DEPTH*.
/ The low end turns to mush: leave LOW off, or bring *LOW X* down.
/ One band is too loud or too bright after crushing: lower its *LEVEL*.
/ LOW is on but the bass barely sounds crushed: lengthen LOW's *DECAY*.
/ Quiet parts drop out: *CRUSH* is on SYM in that band. Switch it to ASYM, or check that *AUTO*
  is on.
/ Only one band is playing: a *SOLO* is lit. Press it to hear all three.
/ The effect is too much: bring the bottom-right *MIX* down rather than the bit depths up; you
  keep the character and get the clarity back.

= Reference

== Controls

#let pr = csv("data/params.csv").slice(1)
#figure(
  {
    // Rows 0–10 are LOW's, 11–21 MID's, 22–32 HIGH's: one row a control, a default a band.
    let band = range(11).map(i => {
      let (name, lo, hi, d-low) = pr.at(i)
      (name.slice(4), lo, hi, d-low, pr.at(11 + i).at(3), pr.at(22 + i).at(3))
    })
    table(columns: 6, align: left,
      [*Control*], [*From*], [*To*], [*LOW*], [*MID*], [*HIGH*],
      ..band.flatten())
  },
  caption: [Each band's controls: range, and default in each band. Your host lists them for
    automation as Low Gain, Mid Gain, High Gain and so on.],
)

#figure(
  table(columns: 4, align: left,
    [*Control*], [*From*], [*To*], [*Default*],
    ..pr.slice(33).flatten()),
  caption: [The shared controls. Low Xover and High Xover are *LOW X* and *HIGH X*.],
)

#let tp = data("tapers")
#figure(
  {
    let fmt(v, d) = str(calc.round(v, digits: d))
    table(columns: 10, align: (left,) + (right,) * 9,
      [*Knob at*], [*GAIN* (dB)], [*GAMMA*], [*BIT DEPTH*], [*SMOOTH.* (%)], [*DECAY* (ms)],
      [*LEVEL* (dB)], [*LOW X* (Hz)], [*HIGH X* (Hz)], [*MIX* (%)],
      ..tp.rows.map(r => (
        ("0", "¼", "½", "¾", "1").at(int(r.at(0) * 4)),
        fmt(r.at(1), 1), fmt(r.at(2), 2), fmt(r.at(3), 1), fmt(r.at(4), 1), fmt(r.at(5), 0),
        fmt(r.at(6), 1), fmt(r.at(7), 0), fmt(r.at(8), 0), fmt(r.at(9), 0),
      )).flatten())
  },
  caption: [What each knob reads at points along its travel. Band MIX reads as the bottom-right
    MIX.],
)

== The plugin

- Formats: CLAP and VST3, for macOS (Apple silicon and Intel), Windows and Linux. On a Mac it
  installs and opens without security warnings.
- Audio: stereo in and out. A mono track works too.
- Delay: none.
- Licence: free, under the MIT or Apache-2.0 licence. The source code is at
  #link("https://github.com/Vulpus-Labs/vxn-free").
- VST is a registered trademark of Steinberg Media Technologies GmbH.
