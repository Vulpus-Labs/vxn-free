// Beverley's manual. Build with ./build.sh, which draws the faces and measures the engine first:
// every face, plot and table here is read from data/, which the example
// `cargo run -p beverley --example manual` writes from the plugin's own code (ADR 0003).
#import "/docs/manual/vulpus-manual.typ": *

#show: manual.with(title: "Beverley", kind: "Bitcrusher", version: "0.1.1")

#let pos = json("data/positions.json")
#let img(name) = read("data/faces/" + name + ".png", encoding: none)
#let data(name) = table-of(csv("data/" + name + ".csv"))

// The faceplate's groups, in panel units (crates/beverley/beverley.svg).
#let gain-group = (200, 0, 300, 150)
#let right-group = (808, 0, 282, 150)

#let unit-axis(title) = (min: -1, max: 1, ticks: (-1, -0.5, 0, 0.5, 1), title: title)
#let curve(t, name, i, label) = (data: column(t, name), color: cat.at(i), label: label,
  width: 1pt)
#let ramp-curve(t, name, i, n, label) = (data: column(t, name), color: ramp.at(str(n)).at(i),
  label: label, width: 1pt)
#let transfer(t, series, height: 50mm) = plot(height: height, x: unit-axis("Level in"),
  y: unit-axis("Level out"), series: series)

// --- Title page ----------------------------------------------------------------------------------

#page(header: none, footer: none, margin: (x: 20mm, top: 28mm, bottom: 20mm))[
  #set par(justify: false)
  #text(font: sans, size: 30pt, weight: "bold")[Beverley]
  #v(-4mm)
  #text(font: sans, size: 12pt, fill: ink-2)[Bitcrusher · CLAP and VST3 plugin · free and open
    source]
  #v(8mm)
  #face(img("init"), pos)
  #v(8mm)
  #grid(columns: (1fr, 1fr), column-gutter: 10mm,
    [
      Beverley is a bitcrusher for guitars, drums, synths and anything else that could do with
      some dirt. Like an old sampler or a cheap converter, it cuts the sound into coarse steps.
      Unlike them, it lets you choose how many steps there are, where they fall and how sharp
      their edges are, so it goes from a faint grain to a square-wave fuzz.

      Start with the Quick start on the next page. The chapters after it take each control in
      turn, with graphs that show what it does.
    ],
    [
      #set text(font: sans, size: 8.5pt)
      #outline(title: text(size: 9pt, weight: "semibold")[Contents], depth: 1, indent: 0pt)
    ],
  )
  #place(bottom + left, text(font: sans, size: 7.5pt, fill: ink-3)[
    Vulpus Labs · manual for version 0.1.1
  ])
]

= Quick start

Put Beverley on a track and play. At its defaults you hear a 4-bit crush: gritty, buzzy, and
clearly lo-fi, at about the same loudness as the dry sound. Then try three knobs:

- *BIT DEPTH* is the amount. Turn it down for more crunch (1 bit is a square-wave fuzz), up for
  less (8 bits and above is a faint grain).
- *SMOOTHING* takes the harsh, fizzy edge off without losing the crunch. Try 20 to 50%.
- *MIX* blends the crushed sound with the original. Around 50% keeps the punch of the dry
  sound under the grit.

If quiet parts sound different from loud ones (cleaner, or cutting out), press *AUTO*: Beverley
then adjusts itself to the level, so quiet and loud passages are crushed alike.

#figure(
  face(img("init"), pos, callouts: (
    (1, "gain"), (2, "auto_gain"), (3, "detect"), (4, "decay"), (5, "gamma"),
    (6, "bit_depth"), (7, "smoothing"), (8, "crush_mode"), (9, "stereo"), (10, "mix"),
  )),
  caption: [The controls, at their defaults.],
)

#key(
  (1, "GAIN", [How hard the sound is driven into the crush.]),
  (2, "AUTO", [Sets the drive for you, following the level.]),
  (3, "DETECT", [How AUTO listens: PEAK or RANGE.]),
  (4, "DECAY", [How quickly AUTO lets go after a loud moment.]),
  (5, "GAMMA", [Whether quiet or loud parts break up first.]),
  (6, "BIT DEPTH", [How coarse the steps are: the amount.]),
  (7, "SMOOTHING", [How sharp the steps' edges are.]),
  (8, "CRUSH", [Buzzing (ASYM) or gating (SYM) character.]),
  (9, "STEREO", [How left and right are treated.]),
  (10, "MIX", [Dry to crushed.]),
  columns: 2,
)

== Reading the graphs

Most graphs in this manual show what Beverley does to a level going in (along the bottom) by
the level that comes out (up the side). Full scale is ±1. A straight diagonal line would mean
no change; Beverley's lines are staircases, and each control changes the stairs.

= The crush

#let tb = data("transfer-bits")
#let tm = data("transfer-modes")
#let ts = data("transfer-smoothing")
#let tg = data("transfer-gamma")
#let tf = data("transfer-fraction")

#summary[*BIT DEPTH* sets how coarse the steps are, *CRUSH* where they sit, *SMOOTHING* how sharp
  their edges are, and *GAMMA* whether they bunch up where the sound is quiet or where it is
  loud.]

== Bit depth

*BIT DEPTH* is the main control: the lower it is, the fewer the steps and the rougher the
sound. At 1 bit every wave becomes a square, at 4 bits (the default) the sound is clearly
crushed, and from about 8 bits it is a fine grain; at 16 bits you will hear almost nothing. The
knob's scale reads 1, 2, 4, 8 and 16 bits, so most of its travel is in the rough, low
settings.

#figure(
  transfer(tb, (
    curve(tb, "b1", 0, "1 bit"), curve(tb, "b2", 1, "2 bits"), curve(tb, "b3", 2, "3 bits"),
    curve(tb, "b4", 3, "4 bits"),
  )),
  caption: [Fewer bits, fewer and bigger steps. *CRUSH* on ASYM.],
)

The knob moves smoothly, not in jumps from one whole number of bits to the next: 2.5 bits sits
halfway between 2 and 3. Sweeping it, by hand or with automation, morphs gradually from one
roughness to another.

#figure(
  transfer(tf, (
    curve(tf, "b2", 0, "2 bits"), curve(tf, "b25", 1, "2.5 bits"), curve(tf, "b3", 2, "3 bits"),
  ), height: 44mm),
  caption: [Between whole numbers of bits, the steps blend from one set to the next. *CRUSH* on
    SYM.],
)

== Crush mode

*CRUSH* changes the character of the crush, most of all on quiet sounds.

- *ASYM* (switch down, the default) has no step at silence, so the sound never quite goes
  quiet: as a note fades it keeps buzzing on the steps either side of zero, like an old
  sampler. Very quiet sounds are let through cleanly rather than buzzing.
- *SYM* (switch up) has a step at silence, so quiet sounds drop out altogether, which works like
  a rough gate. It also has twice as many steps across the wave, so at the same *BIT DEPTH* it
  sounds a little finer.

#figure(
  transfer(tm, (curve(tm, "asym", 0, "ASYM"), curve(tm, "sym", 1, "SYM")), height: 44mm),
  caption: [At 2 bits. Near the middle (quiet sound), ASYM jumps straight past zero; SYM has a
    flat step there, where quiet sound becomes silence.],
)

== Smoothing

*SMOOTHING* rounds off the edges of the steps. At 0% they are sharp, which gives the brightest,
fizziest crush. Around 20 to 50% the edges become curves and the sound turns warmer and less
harsh while keeping its crunch. At 100% the steps disappear and the sound passes unchanged. Most
of the knob's character is in its first half: its centre reads 20%.

#figure(
  transfer(ts, (
    ramp-curve(ts, "s0", 0, 5, "0%"), ramp-curve(ts, "s25", 1, 5, "25%"),
    ramp-curve(ts, "s50", 2, 5, "50%"), ramp-curve(ts, "s75", 3, 5, "75%"),
    ramp-curve(ts, "s100", 4, 5, "100%"),
  )),
  caption: [From sharp steps (0%) to none (100%), at 2 bits, *CRUSH* on SYM.],
)

Sharp steps also make a high, out-of-tune whine on top of the note, called aliasing (see Fizz
and whine, below). Smoothing is the cure.

== Gamma

*GAMMA* chooses which parts of the sound break up first. At 1 (the knob's centre) the steps are
evenly spaced.

- Turned down (towards 0.1), the steps bunch up where the sound is quiet. Fades, tails and soft
  notes grit up, while loud peaks stay comparatively clean.
- Turned up (towards 10), the steps bunch up where the sound is loud. Attacks and peaks break up,
  while quieter material passes more cleanly.

#figure(
  transfer(tg, (
    curve(tg, "g032", 0, "0.32"), curve(tg, "g1", 1, "1"), curve(tg, "g316", 2, "3.16"),
  )),
  caption: [*GAMMA* at 0.32, 1 and 3.16, at 3 bits, *CRUSH* on SYM: the same number of steps,
    placed differently.],
)

== What it sounds like

#let wv = data("waves")
#figure(
  plot(height: 50mm, x: (min: 0, max: 1, ticks: ((0, "0"), (0.5, "½"), (1, "1")),
      title: "One cycle of the wave"), y: unit-axis("Level"),
    series: (
      (data: column(wv, "input"), color: ink-3, label: "Original", width: 0.8pt, dash: "dashed"),
      (data: column(wv, "default"), color: cat.at(0), label: "Defaults (4 bits, ASYM)",
        width: 1pt),
      (data: column(wv, "sym2"), color: cat.at(1), label: "2 bits, SYM", width: 1pt),
      (data: column(wv, "smooth50"), color: cat.at(2), label: "2 bits, SYM, smoothing 50%",
        width: 1pt),
    )),
  caption: [A pure tone through three settings. The steps are what you hear as grit.],
)

#let sp = data("spectra")
#figure(
  grid(columns: 4, column-gutter: 2mm,
    ..(("b2", "2 bits"), ("b4", "4 bits"), ("b8", "8 bits"), ("b4s50", "4 bits, smoothing 50%"))
      .map(((c, name)) => {
        align(center, text(font: sans, size: 7.5pt, weight: "semibold", name))
        plot(height: 34mm, legend: false,
          x: (min: 0, max: 21, ticks: (1, 10, 20), title: "Harmonic"),
          y: (min: -90, max: 0, ticks: (-90, -60, -30, 0), title: if c == "b2" { "Level (dB)" }),
          series: ((data: column(sp, c), color: cat.at(0), kind: "stem", width: 0.8pt),))
      }),
  ),
  caption: [The overtones crushing adds to a pure tone (the first bar is the tone itself). Fewer
    bits add louder overtones, which is the grit; smoothing at 50% turns most of them down,
    the lowest by about 20 dB.],
)

== Fizz and whine

#let al = data("aliasing")
Sharp steps add overtones far higher than a digital system can hold, and they come back down as
a thin, metallic whine that isn't in tune with the note: this is aliasing. A little of it is
part of a bitcrusher's sound. If it bothers you, turn up *SMOOTHING*; turning up *BIT DEPTH*
helps too. The table shows how loud the whine is compared with the note (lower is quieter;
every 20 dB is ten times quieter). Your project's sample rate makes little difference.

#figure(
  {
    let rows = al.rows
    table(columns: 4, align: (left, left, right, right),
      [*BIT DEPTH*], [*SMOOTHING*], [44.1 kHz], [48 kHz],
      ..rows.map(r => (
        str(int(r.at(0))) + " bits", str(int(r.at(1))) + "%",
        if r.at(1) == 100 { [none] } else { str(calc.round(r.at(2), digits: 0)) + " dB" },
        if r.at(1) == 100 { [none] } else { str(calc.round(r.at(3), digits: 0)) + " dB" },
      )).flatten())
  },
  caption: [The whine against the note, for a pure tone of about 1 kHz.],
)

= Gain

#summary[*GAIN* decides how hard the sound is crushed without making it louder. *AUTO* sets it for
  you from the sound's level, so quiet and loud passages are crushed alike.]

#block(breakable: false, below: 5mm, grid(columns: (90mm, 1fr), column-gutter: 6mm, align: top,
  face(img("auto"), pos, crop: gain-group, width: 90mm, badge-size: 3.8mm, callouts: (
    (1, "gain"), (2, "auto_gain"), (3, "detect"), (4, "decay"))),
  key(
    (1, "GAIN", [0 to 20 dB. Greyed out while *AUTO* is on.]),
    (2, "AUTO", [Lit while on.]),
    (3, "DETECT", [PEAK (down) or RANGE (up). Greyed out while *AUTO* is off.]),
    (4, "DECAY", [1 ms to 5 s. Greyed out while *AUTO* is off.]),
  )))

== Gain

*GAIN* drives the sound harder into the crush and turns it back down afterwards, so the output
level stays about the same. Use it when your sound is quiet: a quiet sound only touches the
first few steps and comes out very coarse, or (with *CRUSH* on SYM) not at all. Turning *GAIN*
up spreads it over more steps. Pushed far enough, the loudest parts are also clipped flat, for
a harder, fuzzier edge. At its maximum, 20 dB, a sound at a tenth of full level uses the whole
range.

#let tgn = data("transfer-gain")
#figure(
  transfer(tgn, (
    curve(tgn, "g0", 0, "0 dB"), curve(tgn, "g6", 1, "6 dB"), curve(tgn, "g12", 2, "12 dB"),
    curve(tgn, "g20", 3, "20 dB"),
  )),
  caption: [More *GAIN*: finer steps for quiet sound, and the loudest parts clipped flat. At 3
    bits, *CRUSH* on SYM.],
)

== Auto gain

With *AUTO* on, Beverley follows the sound's level and adjusts the gain itself, so that every
passage, loud or quiet, is crushed by the same amount. A fading note stays crushed to the end
instead of thinning out. The output still follows the level of what goes in. Near silence the
gain stops at a limit (46 dB) rather than rising without end, so faint hiss is crushed only so
far.

*DETECT* chooses how it listens:

- *RANGE* (switch up, the default) follows the highest and lowest points of the wave. It reacts
  instantly to anything louder, so peaks never clip. It also copes with sound that isn't
  centred on zero (a DC offset), crushing it around its own centre.
- *PEAK* (switch down) follows the loudest points. It reacts within about half a
  millisecond, so the very start of a sharp attack can clip slightly, which can add bite.

*DECAY* sets how quickly auto gain lets go after something loud, from 1 ms to 5 s (default
115 ms). Short settings follow every swell and dip, keeping the crush even right through a
fade. Long settings hold back after a loud moment, so the quiet that follows is crushed more
gently for a while, a little like a compressor's release. Its centre reads about 70 ms.

#let ag = data("autogain")
#figure(
  plot(height: 50mm, x: (min: 0, max: 400, ticks: (0, 60, 100, 200, 300, 400), title: "Time (ms)"),
    y: (min: 0, max: 30, ticks: (0, 10, 20, 30), title: "Gain (dB)"),
    series: (
      (data: column(ag, "peak20"), color: ramp.at("3").at(0), label: "PEAK, DECAY 20 ms",
        width: 1pt),
      (data: column(ag, "peak115"), color: ramp.at("3").at(1), label: "PEAK, 115 ms",
        width: 1pt),
      (data: column(ag, "peak500"), color: ramp.at("3").at(2), label: "PEAK, 500 ms",
        width: 1pt),
      (data: column(ag, "range115"), color: cat.at(1), label: "RANGE, 115 ms", width: 1pt),
    ),
    notes: ((x: 30, y: 4, body: [loud], anchor: "center"),
      (x: 230, y: 4, body: [20 dB quieter], anchor: "center"))),
  caption: [How auto gain responds when a loud passage gives way to a quieter one. The longer
    *DECAY*, the more slowly the gain rises to suit the quiet part. At very short settings
    PEAK wobbles with the wave itself, which you hear as extra roughness.],
)

#let os = data("offset")
#figure(
  plot(height: 44mm, x: (min: 0, max: 3.33, ticks: (0, 1, 2, 3), title: "Time (ms)"),
    y: (min: 0, max: 1, ticks: (0, 0.25, 0.5, 0.75, 1), title: "Level"),
    series: (
      (data: column(os, "input"), color: ink-3, label: "Original", width: 0.8pt,
        dash: "dashed"),
      (data: column(os, "peak"), color: cat.at(0), label: "PEAK", width: 1pt),
      (data: column(os, "range"), color: cat.at(1), label: "RANGE", width: 1pt),
    )),
  caption: [A wave that isn't centred on zero, at 1 bit. PEAK squashes it almost flat; RANGE
    crushes it around its own centre and keeps it moving.],
)

= Stereo and mix

#block(breakable: false, below: 5mm, grid(columns: (82mm, 1fr), column-gutter: 6mm, align: top,
  face(img("init"), pos, crop: right-group, width: 82mm, badge-size: 3.8mm, callouts: (
    (1, "crush_mode"), (2, "stereo"), (3, "mix"))),
  key(
    (1, "CRUSH", [ASYM (down) or SYM (up); see The crush.]),
    (2, "STEREO", [MONO (down), SEP (middle) or LINK (up).]),
    (3, "MIX", [DRY to WET.]),
  )))

*STEREO* matters mostly with *AUTO* on.

- *LINK* (switch up, the default) crushes left and right separately but with one auto gain for
  both, so the stereo picture stays put. Use it on most stereo material.
- *SEP* (middle) gives each side its own auto gain. A quiet side is pushed as hard as a loud
  one, which makes the crush even across the stereo field but can make the image move around.
- *MONO* (switch down) mixes left and right together and crushes the result once, sending the
  same sound to both sides. Use it for a deliberately narrow, old-school sound, or on a mono
  source.

With *AUTO* off, LINK and SEP sound the same.

*MIX* blends the crushed sound with the original, from DRY (0%) to WET (100%, the default).
Blending in some of the original keeps the clarity and punch of the source under the grit,
without any phasing.

The knobs respond smoothly as you turn or automate them, without clicks; the switches change at
once. Beverley adds no delay, so it can be used while recording or playing live.

= Starting points

Each of these changes only the controls it names; leave the rest at their defaults.

/ Lo-fi drums: *BIT DEPTH* 6 bits, *CRUSH* SYM, *AUTO* on, *DETECT* PEAK, *DECAY* 20 ms, *MIX*
  50%. Every hit is crushed by the same amount, tails grit up as they fade, and the dry signal
  keeps the punch.
/ Square fuzz: *BIT DEPTH* 1 bit, *CRUSH* ASYM, *AUTO* on. Every note becomes a square wave at
  its own level, like a fuzz pedal that never runs out of gain.
/ Soft grain: *BIT DEPTH* 8 bits, *SMOOTHING* 50%. A faint, warm graininess without the fizz.
/ Gated crunch: *BIT DEPTH* 3 bits, *CRUSH* SYM. Loud notes crunch, and the gaps between them
  fall to silence.
/ Fragile tails: *BIT DEPTH* 4 bits, *GAMMA* 0.3, *CRUSH* SYM. Sustains and reverb tails break up
  while attacks stay clean.
/ Spitting attacks: *BIT DEPTH* 4 bits, *GAMMA* 3, *GAIN* 12 dB. The start of each note spits and
  clips while the body stays smoother.

= Tips

/ Too harsh or fizzy: turn up *SMOOTHING* (start at 25%), or raise *BIT DEPTH* a little.
/ Quiet parts drop out: *CRUSH* is on SYM, which silences quiet sound. Switch to ASYM, turn up
  *GAIN*, or press *AUTO*.
/ Quiet parts sound cleaner than loud ones: press *AUTO*, or turn up *GAIN*.
/ The level pumps with AUTO on: lengthen *DECAY*.
/ The stereo image wanders: set *STEREO* to LINK.
/ The effect is too much: bring *MIX* down rather than *BIT DEPTH* up; you keep the character
  and get the clarity back.

= Reference

== Controls

#figure(
  {
    let rows = csv("data/params.csv").slice(1)
    table(columns: 4, align: (left, left, left, left),
      [*Control*], [*From*], [*To*], [*Default*],
      ..rows.flatten())
  },
  caption: [Every control's range and default. Your host shows the controls under these names
    for automation.],
)

#let tp = data("tapers")
#figure(
  {
    let rows = tp.rows
    let fmt(v, d) = str(calc.round(v, digits: d))
    table(columns: 7, align: (left, right, right, right, right, right, right),
      [*Knob at*], [*GAIN* (dB)], [*GAMMA*], [*BIT DEPTH*], [*SMOOTHING* (%)], [*DECAY* (ms)],
      [*MIX* (%)],
      ..rows.map(r => (
        ("0", "¼", "½", "¾", "1").at(int(r.at(0) * 4)),
        fmt(r.at(1), 1), fmt(r.at(2), 2), fmt(r.at(3), 1), fmt(r.at(4), 1), fmt(r.at(5), 0),
        fmt(r.at(6), 0),
      )).flatten())
  },
  caption: [What each knob reads at points along its travel.],
)

== The plugin

- Formats: CLAP and VST3, for macOS (Apple silicon and Intel), Windows and Linux. On a Mac it
  installs and opens without security warnings.
- Audio: stereo in and out. A mono track works too.
- Delay: none.
- Licence: free, under the MIT or Apache-2.0 licence. The source code is at
  #link("https://github.com/Vulpus-Labs/vxn-free").
- VST is a registered trademark of Steinberg Media Technologies GmbH.
