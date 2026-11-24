// Vulpus Labs manuals: the page, the type, labelled faces and plots.
//
// A manual imports this file from the repository root, as
//   #import "/docs/manual/vulpus-manual.typ": *
// and is compiled with `typst compile --root <repo>`. Paths in Typst resolve against the file
// that names them, so a manual reads its own data, images and JSON and passes them in: this
// file never opens a file itself. It sets type in Avenir Next and Charter, which macOS has.
//
// Licence: under the repository's licence (MIT OR Apache-2.0).

// --- Colour and type ---------------------------------------------------------------------------

#let ink = rgb("#0b0b0b")
#let ink-2 = rgb("#52514e")
#let ink-3 = rgb("#8a8984")
#let rule = rgb("#d9d8d3")
#let grid-ink = rgb("#e6e5e0")
#let wash = rgb("#f4f3ef")
#let accent = rgb("#c8202a")

#let sans = ("Avenir Next", "Helvetica Neue", "Helvetica")
#let serif = ("Charter", "Iowan Old Style", "New York")

// Series colours: the dataviz reference palette. Categorical slots in fixed order, for settings
// that are different in kind; the blue ramp for one control stepped through its travel.
#let cat = (rgb("#2a78d6"), rgb("#eb6834"), rgb("#1baf7a"), rgb("#eda100"))
#let ramp = (
  "2": (rgb("#86b6ef"), rgb("#1c5cab")),
  "3": (rgb("#86b6ef"), rgb("#2a78d6"), rgb("#0d366b")),
  "4": (rgb("#86b6ef"), rgb("#3987e5"), rgb("#1c5cab"), rgb("#0d366b")),
  "5": (rgb("#86b6ef"), rgb("#5598e7"), rgb("#2a78d6"), rgb("#1c5cab"), rgb("#0d366b")),
)

// --- The document --------------------------------------------------------------------------------

#let manual(title: "", kind: "", version: "", body) = {
  set document(title: title + " manual", author: "Vulpus Labs")
  set page(
    paper: "a4",
    margin: (top: 22mm, bottom: 20mm, inside: 20mm, outside: 20mm),
    header: context {
      if counter(page).get().first() > 1 {
        set text(font: sans, size: 7.5pt, fill: ink-3, tracking: 0.04em)
        grid(columns: (1fr, auto), upper(title + " · " + kind), upper("Vulpus Labs"))
        v(-1mm)
        line(length: 100%, stroke: 0.4pt + rule)
      }
    },
    footer: context {
      if counter(page).get().first() > 1 {
        set text(font: sans, size: 7.5pt, fill: ink-3)
        align(center, counter(page).display())
      }
    },
  )
  set text(font: serif, size: 9.6pt, fill: ink, lang: "en", region: "gb", hyphenate: true)
  set par(justify: true, leading: 0.62em, spacing: 0.95em)
  set heading(numbering: none)

  show heading: set text(font: sans, fill: ink)
  show heading.where(level: 1): it => {
    set text(size: 17pt, weight: "bold")
    block(above: 10mm, below: 5mm, sticky: true, it.body)
  }
  show heading.where(level: 2): it => {
    set text(size: 11.5pt, weight: "semibold")
    block(above: 7mm, below: 3mm, sticky: true, it.body)
  }
  show heading.where(level: 3): it => {
    set text(size: 9.6pt, weight: "semibold")
    block(above: 4.5mm, below: 2mm, sticky: true, it.body)
  }

  // Control and jack names: *RESO* sets them as the panel does.
  show strong: it => text(font: sans, weight: "semibold", size: 0.9em, tracking: 0.02em, it.body)

  set table(stroke: none, inset: (x: 2.2mm, y: 1.5mm), align: left + horizon)
  show table.cell.where(y: 0): set text(font: sans, size: 7.5pt, weight: "semibold", fill: ink-2,
    tracking: 0.04em)
  set table(
    stroke: (x, y) => (
      top: if y == 0 { 0.6pt + ink } else if y == 1 { 0.4pt + ink-2 } else { 0.3pt + rule },
      bottom: 0.6pt + ink,
    ),
  )

  set figure(gap: 2.5mm)
  show figure.caption: it => {
    set text(font: sans, size: 7.8pt, fill: ink-2)
    set par(justify: false)
    block(width: 100%, align(left, [*#it.supplement #context it.counter.display(it.numbering).* #it.body]))
  }
  show figure.where(kind: image): set figure(supplement: "Figure")

  set list(indent: 1mm, body-indent: 2mm, marker: text(fill: ink-3, sym.bullet))

  body
}

// Small sans text: notes beside figures, tooltips quoted.
#let note(body) = text(font: sans, size: 8pt, fill: ink-2, body)

// A short summary at the head of a section, set apart from the reference text.
#let summary(body) = block(
  width: 100%, inset: (x: 3.5mm, y: 3mm), fill: wash, radius: 1mm, above: 2mm, below: 4mm,
  text(size: 9.4pt, body),
)

// --- Callouts on the face --------------------------------------------------------------------------

// A numbered badge: white, ringed in red, as legible on the dark face as on the page.
#let badge(n, size: 4.2mm) = box(
  width: size, height: size, baseline: 0.22 * size,
  circle(radius: size / 2, fill: white, stroke: 0.7pt + accent, inset: 0pt,
    align(center + horizon,
      text(font: sans, weight: "bold", size: size * 0.52, fill: ink, str(n)))),
)

// Where a badge sits against a control's box, in panel units: one of the corners or edges, "c"
// for its centre, then nudged by `d`.
#let _anchor(c, side, d) = {
  let (l, t, w, h) = (c.left, c.top, c.width, c.height)
  let x = if side.contains("l") { l } else if side.contains("r") { l + w } else { l + w / 2 }
  let y = if side.starts-with("t") { t } else if side.starts-with("b") { t + h } else { t + h / 2 }
  (x + d.at(0), y + d.at(1))
}

// The face, or a crop of it, with numbered badges.
//
//   img        the face's image, as bytes (read(path, encoding: none))
//   positions  control-positions.json, decoded
//   callouts   an array of (n, id) or (n, id, side) or (n, id, side, (dx, dy)); id is a control's
//              key in the positions, or a box (left:, top:, width:, height:) for anything else
//   crop       (left, top, width, height) in panel units, or none for the whole face
//   width      the printed width
#let face(img, positions, callouts: (), crop: none, width: 100%, badge-size: 4.2mm) = layout(size => {
  let pw = positions.panelWidthPx
  let ph = positions.panelHeightPx
  let (cl, ct, cw, ch) = if crop == none { (0, 0, pw, ph) } else { crop }
  let w = if type(width) == relative or type(width) == ratio { size.width * width } else { width }
  let s = w / cw
  box(width: w, height: ch * s, clip: true, radius: 1.2mm, {
    // A block of the image's own size: an image alone in a box shorter than itself is centred
    // in it, whatever the placement says.
    place(top + left, dx: -cl * s, dy: -ct * s,
      block(width: pw * s, height: ph * s, image(img, width: pw * s, height: ph * s)))
    for co in callouts {
      let n = co.at(0)
      let id = co.at(1)
      let side = co.at(2, default: "tl")
      let d = co.at(3, default: (0, 0))
      let c = if type(id) == str { positions.controls.at(id) } else { id }
      let (x, y) = _anchor(c, side, d)
      place(dx: (x - cl) * s - badge-size / 2, dy: (y - ct) * s - badge-size / 2,
        badge(n, size: badge-size))
    }
  })
})

// The key to a labelled face: badge, name, what it is.
#let key(..rows, columns: 1) = {
  let cells = rows.pos().map(r => (
    align(right, badge(r.at(0), size: 3.8mm)),
    text(font: sans, weight: "semibold", size: 8pt, r.at(1)),
    text(size: 8.6pt, r.at(2)),
  ))
  set par(justify: false)
  grid(
    columns: (5mm, auto, 1fr) * columns, column-gutter: (1.6mm, 2.4mm, 5mm) * columns,
    row-gutter: 1.9mm, align: horizon,
    ..cells.flatten(),
  )
}

// --- Plots -----------------------------------------------------------------------------------------

// A CSV, decoded with its header: (header, rows) where each row is an array of floats or none.
#let table-of(raw) = {
  let header = raw.at(0)
  let rows = raw.slice(1).map(r => r.map(v => if v.trim() == "" { none } else { float(v) }))
  (header: header, rows: rows)
}

// One column of a decoded CSV, by name, against its first column: an array of (x, y).
#let column(t, name) = {
  let i = t.header.position(h => h == name)
  assert(i != none, message: "no column " + name)
  t.rows.map(r => (r.at(0), r.at(i)))
}

#let _frac(a, v) = {
  if a.at("log", default: false) {
    (calc.ln(v) - calc.ln(a.min)) / (calc.ln(a.max) - calc.ln(a.min))
  } else {
    (v - a.min) / (a.max - a.min)
  }
}

#let _tick-label(v) = {
  if type(v) == array { v.at(1) } else if calc.abs(v) >= 1000 and calc.rem(v, 1000) == 0 {
    str(int(v / 1000)) + "k"
  } else if v == calc.round(v) { str(int(v)).replace("-", "−") } else { str(v).replace("-", "−") }
}
#let _tick-value(v) = if type(v) == array { v.at(0) } else { v }

// A line plot on one pair of axes.
//
//   x, y     axes: (min:, max:, log: bool, ticks: (v or (v, label), ...), title:)
//   series   (data: ((x, y), ...), color:, label:, width:, dash:, kind: "line" | "stem" | "dot")
//   notes    labels on the plot: (x:, y:, body:, anchor: "left" | "right" | "center")
//   legend   whether to key the series above the plot
#let plot(x: (:), y: (:), series: (), notes: (), width: 100%, height: 52mm, legend: auto) = layout(size => {
  let w = if type(width) == relative or type(width) == ratio { size.width * width } else { width }
  let gutter-l = if y.at("title", default: none) != none { 12.5mm } else { 9mm }
  let gutter-b = 8.5mm
  let pw = w - gutter-l - 1.5mm
  let ph = height - gutter-b
  let px(v) = _frac(x, v) * pw
  let py(v) = (1 - _frac(y, v)) * ph
  let show-legend = if legend == auto { series.filter(s => s.at("label", default: none) != none).len() > 1 } else { legend }

  set text(font: sans, size: 7pt, fill: ink-2)
  block(breakable: false, {
  if show-legend {
    let keys = series.filter(s => s.at("label", default: none) != none).map(s => box({
      box(width: 4.5mm, height: 0.6em, baseline: -0.05em,
        align(horizon, line(length: 100%, stroke: (paint: s.color, thickness: 1.3pt,
          dash: s.at("dash", default: none)))))
      h(1.2mm)
      s.label
    }))
    block(below: 1.8mm, inset: (left: gutter-l), keys.join(h(4mm)))
  }

  box(width: w, height: height, {
    // Grid and tick labels.
    for t in x.at("ticks", default: ()) {
      let v = _tick-value(t)
      place(dx: gutter-l + px(v), dy: 0pt, line(length: ph, angle: 90deg, stroke: 0.35pt + grid-ink))
      place(dx: gutter-l + px(v) - 8mm, dy: ph + 1.4mm, box(width: 16mm, align(center, _tick-label(t))))
    }
    for t in y.at("ticks", default: ()) {
      let v = _tick-value(t)
      place(dx: gutter-l, dy: py(v), line(length: pw, stroke: 0.35pt + grid-ink))
      place(dx: gutter-l - 10mm, dy: py(v) - 0.5em, box(width: 8.6mm, height: 1em,
        align(right + horizon, _tick-label(t))))
    }
    // The frame: a baseline and a left axis.
    place(dx: gutter-l, dy: ph, line(length: pw, stroke: 0.5pt + ink-3))
    place(dx: gutter-l, dy: 0pt, line(length: ph, angle: 90deg, stroke: 0.5pt + ink-3))
    if x.at("title", default: none) != none {
      place(dx: gutter-l, dy: ph + 5.2mm, box(width: pw, align(center, x.title)))
    }
    if y.at("title", default: none) != none {
      place(dx: 0pt, dy: 0pt, rotate(-90deg, reflow: true, box(width: ph, align(center, y.title))))
    }

    // The data, clipped to the axes.
    place(dx: gutter-l, dy: 0pt, box(width: pw, height: ph, clip: true, {
      for s in series {
        let stroke = (paint: s.color, thickness: s.at("width", default: 1.15pt), cap: "round",
          join: "round", dash: s.at("dash", default: none))
        if s.at("kind", default: "line") == "stem" {
          for (vx, vy) in s.data {
            if vy == none { continue }
            let yy = calc.max(y.min, vy)
            place(dx: px(vx), dy: py(y.min), line(end: (0pt, py(yy) - py(y.min)), stroke: stroke))
            if vy >= y.min {
              place(dx: px(vx) - 1.3pt, dy: py(yy) - 1.3pt, circle(radius: 1.3pt, fill: s.color,
                stroke: 0.6pt + white))
            }
          }
        } else if s.at("kind", default: "line") == "dot" {
          for (vx, vy) in s.data {
            if vy == none { continue }
            place(dx: px(vx) - 2pt, dy: py(vy) - 2pt, circle(radius: 2pt, fill: s.color,
              stroke: 0.7pt + white))
          }
        } else {
          let parts = ()
          let pen = false
          for (vx, vy) in s.data {
            if vx == none or vy == none or (x.at("log", default: false) and vx <= 0) {
              pen = false
              continue
            }
            let yy = calc.max(calc.min(vy, y.max + (y.max - y.min)), y.min - (y.max - y.min))
            let p = (px(vx), py(yy))
            parts.push(if pen { curve.line(p) } else { curve.move(p) })
            pen = true
          }
          place(curve(stroke: stroke, ..parts))
        }
      }
    }))
    for n in notes {
      let anchor = n.at("anchor", default: "left")
      let bx = gutter-l + px(n.x)
      let by = py(n.y)
      let body = box(fill: white.transparentize(15%), inset: 0.6mm, text(fill: ink, n.body))
      if anchor == "left" {
        place(dx: bx, dy: by - 0.6em, body)
      } else if anchor == "right" {
        place(dx: bx - 30mm, dy: by - 0.6em, box(width: 30mm, align(right, body)))
      } else {
        place(dx: bx - 15mm, dy: by - 0.6em, box(width: 30mm, align(center, body)))
      }
    }
  })
  })
})

// Ticks for a frequency axis, 20 Hz to 20 kHz.
#let audio-ticks = (20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000)
