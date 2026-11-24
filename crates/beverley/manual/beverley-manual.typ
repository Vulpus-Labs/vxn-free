// Beverley's manual. Build with ./build.sh, which draws the faces and measures the engine first:
// every face, plot and table here is read from data/, which the example
// `cargo run -p beverley --example manual` writes from the plugin's own code.
#import "/docs/manual/vulpus-manual.typ": *

#show: manual.with(title: "Beverley", kind: "Bitcrusher", version: "0.1.1")

#let pos = json("data/positions.json")
#let img(name) = read("data/faces/" + name + ".png", encoding: none)
#let data(name) = table-of(csv("data/" + name + ".csv"))

// The faceplate's groups, in panel units (crates/beverley/beverley.svg).
#let gain-group = (200, 0, 300, 150)
#let crush-group = (500, 0, 308, 150)
#let right-group = (808, 0, 282, 150)

#let unit-axis(title) = (min: -1, max: 1, ticks: (-1, -0.5, 0, 0.5, 1), title: title)
// A transfer curve: one column of a transfer CSV against its input.
#let curve(t, name, i, label) = (data: column(t, name), color: cat.at(i), label: label,
  width: 1pt)
#let ramp-curve(t, name, i, n, label) = (data: column(t, name),
  color: ramp.at(str(n)).at(i),
  label: label, width: 1pt)
#let transfer(t, series, height: 50mm) = plot(height: height, x: unit-axis("Input"),
  y: unit-axis("Output"), series: series)

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
      some dirt. It cuts a signal into steps, as an old sampler or a cheap converter does, but
      lets you choose how many steps there are, where they fall and how sharp their edges are:
      from a faint grain to a square-wave fuzz.

      This manual describes every control. Its pictures of the faceplate are drawn by the
      plugin's own code, and its graphs and tables are measured by running the plugin's engine.
    ],
    [
      #set text(font: sans, size: 8.5pt)
      #outline(title: text(size: 9pt, weight: "semibold")[Contents], depth: 1, indent: 0pt)
    ],
  )
  #place(bottom + left, text(font: sans, size: 7.5pt, fill: ink-3)[
    Vulpus Labs · manual built #datetime.today().display("[day] [month repr:long] [year]")
  ])
]

= Overview

#summary[Beverley amplifies the signal, bends it, cuts it into steps and undoes the bend and the
  gain, so that what comes out is about as loud as what went in, only coarser. Each control
  shapes one of those stages.]

Every sample passes through the same chain. Its sign is set aside, and the chain works on its
size:

+ *Gain* raises the level, by *GAIN* or automatically (*AUTO*), and anything that would pass
  full scale is clipped.
+ *Gamma* bends the level along a power curve.
+ *Quantisation* snaps it to one of a number of steps, set by *BIT DEPTH*, laid out as *CRUSH*
  says, with edges shaped by *SMOOTHING*.
+ The bend and the gain are undone, and the sign put back.

*MIX* then blends the result with the dry signal, and *STEREO* decides how the two channels
share the work. Beverley adds no latency: the output is never later than the input.

#figure(
  face(img("init"), pos, callouts: (
    (1, "gain"), (2, "auto_gain"), (3, "detect"), (4, "decay"), (5, "gamma"),
    (6, "bit_depth"), (7, "smoothing"), (8, "crush_mode"), (9, "stereo"), (10, "mix"),
  )),
  caption: [The faceplate at its defaults.],
)

#key(
  (1, "GAIN", [Fixed gain into the crush, 0 to 20 dB.]),
  (2, "AUTO", [Lets Beverley set the gain from the input.]),
  (3, "DETECT", [How auto gain follows the input: PEAK or RANGE.]),
  (4, "DECAY", [How fast auto gain lets go, 1 ms to 5 s.]),
  (5, "GAMMA", [The curve the signal is bent along, 0.1 to 10.]),
  (6, "BIT DEPTH", [How many steps, 1 to 16 bits.]),
  (7, "SMOOTHING", [The shape of each step, hard to none.]),
  (8, "CRUSH", [Where the steps lie: ASYM or SYM.]),
  (9, "STEREO", [MONO, SEP or LINK.]),
  (10, "MIX", [Dry to wet.]),
  columns: 2,
)

= The crush

#let tb = data("transfer-bits")
#let tm = data("transfer-modes")
#let ts = data("transfer-smoothing")
#let tg = data("transfer-gamma")
#let tf = data("transfer-fraction")

#summary[*BIT DEPTH* sets how many steps the signal is cut into; *CRUSH* where they lie;
  *SMOOTHING* how sharp their edges are; *GAMMA* whether they crowd round the quiet or the loud
  part of each wave.]

The graphs in this chapter are _transfer curves_: what comes out (up the page) for each level
going in (across it), with *GAIN* at 0 dB and *AUTO* off. A straight diagonal line would be
no effect at all.

== Bit depth

*BIT DEPTH* runs from 1 to 16 bits. _n_ bits make $2^n - 1$ steps: one bit makes one step, so
every sample is pushed to one of two levels; four bits make fifteen; sixteen bits make 65,535,
far too fine to hear. The knob's travel is geometric, so each quarter of a turn doubles the
depth: 1, 2, 4, 8 and 16 bits, as its scale shows.

#figure(
  transfer(tb, (
    curve(tb, "b1", 0, "1 bit"), curve(tb, "b2", 1, "2 bits"), curve(tb, "b3", 2, "3 bits"),
    curve(tb, "b4", 3, "4 bits"),
  )),
  caption: [Transfer curves at 1 to 4 bits, *CRUSH* on ASYM.],
)

The depth turns smoothly, without jumps between whole numbers of bits. Between two whole depths
Beverley crushes the signal at both and blends the two results by how far the knob is between
them: at 2.5 bits, half each of 2 and 3 bits. Sweeping *BIT DEPTH*, by hand or by automation,
morphs from one staircase to the next.

#figure(
  transfer(tf, (
    curve(tf, "b2", 0, "2 bits"), curve(tf, "b25", 1, "2.5 bits"), curve(tf, "b3", 2, "3 bits"),
  ), height: 44mm),
  caption: [A fractional depth lies between the whole depths either side of it. *CRUSH* on SYM.],
)

== Crush mode

*CRUSH* chooses where the steps lie.

- *ASYM* (switch down, the default) lays one set of steps across the whole wave, from its lowest
  point to its highest. There is no step at silence: every output sits at least half a step
  away from zero, so a crushed wave keeps buzzing as it fades. Very quiet signals, below about
  a twentieth of full scale after the gain and the curve, fade gradually back to the dry signal
  instead, so they don't chatter between the two steps nearest zero.
- *SYM* (switch up) crushes each half of the wave, positive and negative, on its own set of
  steps, with a step at zero. Quiet passages fall to silence, and at the same depth there are
  twice as many steps across the wave.

#figure(
  transfer(tm, (curve(tm, "asym", 0, "ASYM"), curve(tm, "sym", 1, "SYM")), height: 44mm),
  caption: [The two modes at 2 bits. ASYM has no step at zero; SYM has one.],
)

== Smoothing

*SMOOTHING* shapes the move from one step to the next. At 0% each step is a hard edge. Towards
the middle of its travel the edge becomes an S-curve, rounder and less harsh. At 100% the steps
are gone and the signal passes untouched. The knob is weighted towards its quiet end: its
centre reads 20%.

#figure(
  transfer(ts, (
    ramp-curve(ts, "s0", 0, 5, "0%"), ramp-curve(ts, "s25", 1, 5, "25%"),
    ramp-curve(ts, "s50", 2, 5, "50%"), ramp-curve(ts, "s75", 3, 5, "75%"),
    ramp-curve(ts, "s100", 4, 5, "100%"),
  )),
  caption: [*SMOOTHING* from 0% to 100% at 2 bits, *CRUSH* on SYM.],
)

Hard steps are what give a bitcrusher its fizz, and also what makes it alias: an abrupt step has
harmonics far above what the sample rate can hold, and they fold back down as inharmonic
whine. Smoothing rounds the steps off, and the further it goes the less of that whine is left:
it is the way to keep the crunch and lose the fizz (see Aliasing, below).

== Gamma

*GAMMA* bends the signal before it is crushed and straightens it afterwards, which decides where
the steps crowd together. The knob reads the power of the curve, from 0.1 through 1 (no bend, at
its centre) to 10.

- Below 1, the steps crowd round the quiet part of each wave. Soft tails and quiet notes grit
  up while loud peaks stay comparatively clean.
- Above 1, the steps crowd round the loud part, so peaks and attacks break up first while
  quiet material passes more cleanly.

#figure(
  transfer(tg, (
    curve(tg, "g032", 0, "0.32"), curve(tg, "g1", 1, "1"), curve(tg, "g316", 2, "3.16"),
  )),
  caption: [*GAMMA* at 0.32, 1 and 3.16, at 3 bits, *CRUSH* on SYM: the same seven steps a
    side, placed differently.],
)

== What it does to a wave

#let wv = data("waves")
#figure(
  plot(height: 50mm, x: (min: 0, max: 1, ticks: ((0, "0"), (0.5, "½"), (1, "1")),
      title: "One cycle"), y: unit-axis("Level"),
    series: (
      (data: column(wv, "input"), color: ink-3, label: "Input", width: 0.8pt, dash: "dashed"),
      (data: column(wv, "default"), color: cat.at(0), label: "Defaults (4 bits, ASYM)",
        width: 1pt),
      (data: column(wv, "sym2"), color: cat.at(1), label: "2 bits, SYM", width: 1pt),
      (data: column(wv, "smooth50"), color: cat.at(2), label: "2 bits, SYM, smoothing 50%",
        width: 1pt),
    )),
  caption: [One cycle of a sine at 0.8 of full scale, through three settings.],
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
  caption: [Harmonic levels against the fundamental: a 997 Hz sine at 0.8 of full scale,
    *CRUSH* on ASYM. Crushing adds odd harmonics, louder the fewer the bits. At 50%, smoothing
    lowers most of them, the lowest by about 20 dB.],
)

== Aliasing

#let al = data("aliasing")
Beverley is not oversampled. Hard steps alias: their harmonics above half the sample rate fold
back into the audible range, between the true harmonics. That is part of a bitcrusher's sound,
and *SMOOTHING* is the control for it. The table gives the folded-back power against the
fundamental for the same 997 Hz sine, at two sample rates.

#figure(
  {
    let rows = al.rows
    table(columns: 4, align: (left, left, right, right),
      [*BIT DEPTH*], [*SMOOTHING*], [44.1 kHz], [48 kHz],
      ..rows.map(r => (
        str(int(r.at(0))) + " bits", str(int(r.at(1))) + "%",
        str(calc.round(r.at(2), digits: 1)) + " dB", str(calc.round(r.at(3), digits: 1)) + " dB",
      )).flatten())
  },
  caption: [Aliased power against the fundamental. At 100% smoothing the figure is the
    measurement's own floor: there are no steps left to alias.],
)

= Gain

#summary[*GAIN* decides how hard the signal is crushed without changing how loud it comes out.
  *AUTO* sets it from the input instead, so quiet and loud passages are crushed alike.]

#block(breakable: false, below: 5mm, grid(columns: (90mm, 1fr), column-gutter: 6mm, align: top,
  face(img("auto"), pos, crop: gain-group, width: 90mm, badge-size: 3.8mm, callouts: (
    (1, "gain"), (2, "auto_gain"), (3, "detect"), (4, "decay"))),
  key(
    (1, "GAIN", [0 to 20 dB. Dimmed while *AUTO* is on.]),
    (2, "AUTO", [Lit while on.]),
    (3, "DETECT", [PEAK (down) or RANGE (up). Dimmed while *AUTO* is off.]),
    (4, "DECAY", [1 ms to 5 s. Dimmed while *AUTO* is off.]),
  )))

== Fixed gain

*GAIN* raises the signal by up to 20 dB before the crush, and lowers it by the same amount after.
The output is about as loud as before, but a quiet signal now spans more of the steps, so it is
crushed more finely, and a loud one is pushed past full scale and clipped. At 20 dB a signal at
a tenth of full scale fills the whole range.

#let tgn = data("transfer-gain")
#figure(
  transfer(tgn, (
    curve(tgn, "g0", 0, "0 dB"), curve(tgn, "g6", 1, "6 dB"), curve(tgn, "g12", 2, "12 dB"),
    curve(tgn, "g20", 3, "20 dB"),
  )),
  caption: [*GAIN* at 0, 6, 12 and 20 dB, at 3 bits, *CRUSH* on SYM. More gain: finer steps for
    quiet signals, and the loudest clipped flat.],
)

== Auto gain

With *AUTO* on, Beverley follows the input's level and sets the gain to bring it to full scale,
so each passage, loud or quiet, is crushed across all the steps. The gain is at most 46 dB: on
near-silence it holds there rather than amplifying noise without limit. Its output follows the
input's level, as with fixed gain.

*DETECT* chooses how the level is followed:

- *PEAK* follows the signal's peak size. It rises on a louder signal within about half a
  millisecond, and falls, after a loud moment, at the rate *DECAY* sets.
- *RANGE* (the default) follows the signal's highest and lowest points. A new high or low is
  taken at once, so the gain never lets a peak overshoot full scale; between them, both fall
  towards each other at *DECAY*'s rate. The signal is crushed about the middle of that range
  rather than about zero, so a signal with an offset is crushed on its swing.

*DECAY* is the time either takes to let go by 20 dB, from 1 ms to 5 s; its default is 115 ms.
A short *DECAY* follows every swell and dip, so the crush stays even through a fade; a long one
holds the gain where the loudest moment set it, so quiet parts after a loud one are crushed
less. The knob is geometric: its centre reads 71 ms.

#let ag = data("autogain")
#figure(
  plot(height: 50mm, x: (min: 0, max: 400, ticks: (0, 60, 100, 200, 300, 400), title: "Time (ms)"),
    y: (min: 0, max: 30, ticks: (0, 10, 20, 30), title: "Gain (dB)"),
    series: (
      (data: column(ag, "peak20"), color: ramp.at("3").at(0), label: "PEAK, DECAY 20 ms", width: 1pt),
      (data: column(ag, "peak115"), color: ramp.at("3").at(1), label: "PEAK, 115 ms", width: 1pt),
      (data: column(ag, "peak500"), color: ramp.at("3").at(2), label: "PEAK, 500 ms", width: 1pt),
      (data: column(ag, "range115"), color: cat.at(1), label: "RANGE, 115 ms", width: 1pt),
    ),
    notes: ((x: 30, y: 4, body: [loud], anchor: "center"),
      (x: 230, y: 4, body: [20 dB quieter], anchor: "center"))),
  caption: [The gain auto gain applies over a loud passage (a 200 Hz sine at 0.8 for 60 ms),
    then one 20 dB quieter, starting from silence at its 46 dB ceiling (off the top). The
    longer *DECAY*, the longer the gain takes to rise to about the quiet passage's 22 dB; at
    20 ms, PEAK lets go enough between peaks to ripple with every cycle.],
)

#let os = data("offset")
#figure(
  plot(height: 44mm, x: (min: 0, max: 3.33, ticks: (0, 1, 2, 3), title: "Time (ms)"),
    y: (min: 0, max: 1, ticks: (0, 0.25, 0.5, 0.75, 1), title: "Level"),
    series: (
      (data: column(os, "input"), color: ink-3, label: "Input", width: 0.8pt, dash: "dashed"),
      (data: column(os, "peak"), color: cat.at(0), label: "PEAK", width: 1pt),
      (data: column(os, "range"), color: cat.at(1), label: "RANGE", width: 1pt),
    )),
  caption: [A signal with an offset (0.5 + 0.1 sin, 300 Hz) at 1 bit with auto gain, *DECAY*
    100 ms. PEAK crushes about zero, so the whole wave lands on one step; RANGE crushes about
    the wave's middle and keeps its shape.],
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

*STEREO* sets how the two channels share the crush.

- *MONO* sums left and right, crushes the sum once and sends it to both. Either channel alone
  reaches both outputs.
- *SEP* crushes each channel on its own. With *AUTO* on, each has its own auto gain, so a channel
  that is quieter than the other is raised further; the stereo image can shift as the balance
  changes.
- *LINK* (the default) crushes each channel on its own with one auto gain for both, so the image
  holds still. With *DETECT* on RANGE it follows both channels' highest and lowest points
  together; on PEAK it follows the level of their sum.

With *AUTO* off, SEP and LINK sound the same.

*MIX* blends the crushed signal with the dry one, from 0% (dry) to 100% (wet, the default).
The two are always aligned to the sample, so a partial mix never combs.

All of Beverley's continuous controls glide to new values over about ten milliseconds, so turning a
knob or automating it never clicks.

= Starting points

These settings are starting points, not presets: each changes only what it names from the
defaults.

/ Lo-fi drums: *BIT DEPTH* 6 bits, *CRUSH* SYM, *AUTO* on, *DETECT* PEAK, *DECAY* 20 ms, *MIX*
  50%. The crush follows each hit's envelope, so tails grit up as they fade, and the dry signal
  keeps the transients.
/ Square fuzz: *BIT DEPTH* 1 bit, *CRUSH* ASYM, *AUTO* on. Every wave becomes a square at its
  own level, like a fuzz box that never runs out of gain.
/ Soft grain: *BIT DEPTH* 8 bits, *SMOOTHING* 50%. A faint, warm graininess with little of the
  high fizz.
/ Gated crunch: *BIT DEPTH* 3 bits, *CRUSH* SYM, *GAIN* 0 dB. Quiet passages fall to silence
  between notes, loud ones crunch.
/ Fragile tails: *BIT DEPTH* 4 bits, *GAMMA* 0.3, *CRUSH* SYM. The steps crowd round the quiet
  part of each wave, so sustains and reverb tails break up while attacks stay clean.
/ Spitting attacks: *BIT DEPTH* 4 bits, *GAMMA* 3, *GAIN* 12 dB. Peaks break up and clip while
  the body of the sound stays smoother.

= Reference

== Controls

#figure(
  {
    let rows = csv("data/params.csv").slice(1)
    table(columns: 4, align: (left, left, left, left),
      [*Control*], [*From*], [*To*], [*Default*],
      ..rows.flatten())
  },
  caption: [Every control's range and default, as the plugin shows them. Hosts list them under
    these names.],
)

#let tp = data("tapers")
#figure(
  {
    let rows = tp.rows
    let fmt(v, d) = str(calc.round(v, digits: d))
    table(columns: 7, align: (left, right, right, right, right, right, right),
      [*Travel*], [*GAIN* (dB)], [*GAMMA*], [*BIT DEPTH*], [*SMOOTHING* (%)], [*DECAY* (ms)],
      [*MIX* (%)],
      ..rows.map(r => (
        ("0", "¼", "½", "¾", "1").at(int(r.at(0) * 4)),
        fmt(r.at(1), 1), fmt(r.at(2), 2), fmt(r.at(3), 1), fmt(r.at(4), 1), fmt(r.at(5), 1),
        fmt(r.at(6), 0),
      )).flatten())
  },
  caption: [What each knob reads at five points of its travel.],
)

== The plugin

- Formats: CLAP (id `labs.vulpus.beverley`) and VST3, for macOS (Apple silicon and Intel),
  Windows and Linux. The macOS builds are signed and notarized.
- Audio: one stereo input and output. A mono input crushes in mono.
- Latency: none, at every setting and sample rate.
- Licence: free, under the MIT or Apache-2.0 licence, at your option. The source is at
  #link("https://github.com/Vulpus-Labs/vxn-free").
- The VST3 builds use free-audio's clap-wrapper and Steinberg's VST3 SDK, both MIT-licensed.
  VST is a registered trademark of Steinberg Media Technologies GmbH.
