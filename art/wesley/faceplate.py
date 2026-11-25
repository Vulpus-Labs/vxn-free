"""Writes Wesley's faceplate, `crates/wesley/wesley.svg`, and the relief
Beverley's `blender/panel.py` renders, `crates/wesley/art/panel/geometry.json`.

Wesley is Beverley's sibling (ADR 0004): the same 1990s rack style, the
same knobs and switches (Beverley's skins, `crates/beverley/art/controls/`),
in wine red where Beverley is dark teal-blue. Each band is a column, as on
the plain faceplate it replaced, holding Beverley's strip folded into
rows; a row along the bottom holds the title and the shared controls. The
engraved grooves part the face into a grid (`sections` in the geometry):

    [LOW  ON SOLO]               [MID ...]        [HIGH ...]
    [1 CYCLE LCD ]
    [GAIN AUTO DETECT DECAY]
    [GAMMA BIT DEPTH SMOOTHING]
    [CRUSH LEVEL MIX]
    [title]                      [LOW X HIGH X]   [STEREO MIX]

    python3 faceplate.py            (from this folder)

Then re-render the relief (a minute or two):

    cd ../beverley/blender
    /Applications/Blender.app/Contents/MacOS/Blender -b -P panel.py -- \\
        --geometry ../../../crates/wesley/art/panel/geometry.json \\
        --out ../../../crates/wesley/art/panel

Don't edit what it writes; change this and run it again.

Licence: original work for vxn-free, under the repository's licence
(MIT OR Apache-2.0).
"""

import json
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.join(HERE, "..", "..", "crates", "wesley")

GAP = 3
EAR = 36
# A column a band; under them, a row for the title and shared controls.
BAND_W = 300
W = EAR + 3 * BAND_W + EAR
# Where the bottom row starts (its groove), and its height.
BOTTOM = 552
H = BOTTOM + 148
GROOVE = 1.2
# Rows within a column: (label line, control centre line).
ROWS_Y = [(190, 252), (320, 382), (450, 512), (BOTTOM + 30, BOTTOM + 92)]
KR = 25
KNOB_BOX = 64
TOGGLE_BOX = 46
BUTTON_BOX = 30
# A wine red: Wesley's colour (Beverley's panel is a dark teal-blue).
PAINT = "#561019"
INK = "#f1ece2"
DIM = "#c9b3ae"
ACCENT = "#e2b25a"  # the title stripe: warm gold on the red
# Each band's crush display, as Beverley's window.
WIN_W, WIN_H = 160, 84
WIN_Y = 76
LCD_INSET = 5
BANDS = (("low", "LOW"), ("mid", "MID"), ("high", "HIGH"))

out = []
o = out.append
geometry = {"size": [W, H], "gap": GAP, "slots": [], "screws": [], "grooves": [],
            "windows": [], "sections": []}
controls = []


def polar(cx, cy, r, deg):
    a = math.radians(deg)
    return cx + r * math.sin(a), cy - r * math.cos(a)


def text(x, y, s, size, anchor="middle", fill=INK, spacing=0):
    o(f'  <text x="{x:.2f}" y="{y:.2f}" font-size="{size}" text-anchor="{anchor}" '
      f'letter-spacing="{spacing}" fill="{fill}">{s}</text>')


def scale(cx, cy, ticks, labels):
    """A knob's printed scale: 270° of ticks, majors where labelled."""
    for p in ticks:
        deg = -135 + 270 * p
        major = any(abs(p - q) < 1e-6 for q in labels)
        x1, y1 = polar(cx, cy, KR + 6, deg)
        x2, y2 = polar(cx, cy, KR + (11 if major else 9), deg)
        o(f'  <line x1="{x1:.2f}" y1="{y1:.2f}" x2="{x2:.2f}" y2="{y2:.2f}" '
          f'stroke="{INK if major else DIM}" stroke-width="{1.4 if major else 1}" '
          f'stroke-linecap="round"/>')
    for p, s in labels.items():
        deg = -135 + 270 * p
        x, y = polar(cx, cy, KR + 20, deg)
        anchor = "middle"
        if deg < -20:
            anchor, x = "end", x + 4
        if deg > 20:
            anchor, x = "start", x - 4
        text(x, y + 3, s, 8, anchor=anchor, fill=DIM)


def knob(name, cx, row, label, ticks, labels):
    ly, y = ROWS_Y[row]
    text(cx, ly, label, 10.5, spacing=1.6)
    scale(cx, y, ticks, labels)
    controls.append(("knob", name, cx, y, KNOB_BOX, "knob"))


def bat(name, cx, row, label, top, bottom, skin="toggle2", middle=None):
    ly, y = ROWS_Y[row]
    text(cx, ly, label, 10.5, spacing=1.6)
    text(cx, y - 22, top, 8, fill=DIM)
    text(cx, y + 29, bottom, 8, fill=DIM)
    if middle:
        text(cx + 17, y + 3, middle, 8, anchor="start", fill=DIM)
    controls.append(("toggle", name, cx, y, TOGGLE_BOX, skin))


def button(name, cx, row, label, y=None):
    """A push-button with its LED: in a row, labelled above; given `y`,
    labelled small underneath."""
    if y is None:
        ly, y = ROWS_Y[row]
        text(cx, ly, label, 10.5, spacing=1.6)
    else:
        text(cx, y + 28, label, 8, fill=DIM, spacing=1.2)
    controls.append(("button", name, cx, y, BUTTON_BOX, "button"))


def window(name, cx):
    """A display: a recess the relief cuts part way into the face, a dark
    bezel printed in it, and the LCD the editor's widget draws."""
    x, y = cx - WIN_W / 2, WIN_Y
    geometry["windows"].append([x, y, WIN_W, WIN_H])
    o(f'  <rect x="{x}" y="{y}" width="{WIN_W}" height="{WIN_H}" rx="3" fill="#0b0b0c"/>')
    o(f'  <rect x="{x + LCD_INSET - 1}" y="{y + LCD_INSET - 1}" '
      f'width="{WIN_W - 2 * LCD_INSET + 2}" height="{WIN_H - 2 * LCD_INSET + 2}" '
      f'fill="#3a4430"/>')
    controls.append(("canvas", name, x + LCD_INSET, y + LCD_INSET,
                     (WIN_W - 2 * LCD_INSET, WIN_H - 2 * LCD_INSET), None))


def slot_and_screw(cx, cy):
    """A rack slot with a screw in it, on a nylon cup washer, off-centre
    as screws sit in slots."""
    sw, sh = 20, 11
    geometry["slots"].append([cx - sw / 2, cy - sh / 2, sw, sh])
    o(f'  <rect x="{cx - sw / 2}" y="{cy - sh / 2}" width="{sw}" height="{sh}" rx="{sh / 2}" '
      f'fill="#0a0a0b"/>')
    sx = cx - 2.5
    geometry["screws"].append([sx, cy])
    o(f'  <circle cx="{sx}" cy="{cy}" r="7.5" fill="#cfccc2"/>')
    o(f'  <circle cx="{sx}" cy="{cy}" r="5" fill="#7d7f83"/>')
    o(f'  <line x1="{sx - 3.4}" y1="{cy - 1.4}" x2="{sx + 3.4}" y2="{cy + 1.4}" '
      f'stroke="#2c2d30" stroke-width="1.4" stroke-linecap="round"/>')


def groove(x1, y1, x2, y2):
    """An engraved line: the print's dark line where the relief's groove
    falls (the relief's own grooves come from `sections`)."""
    o(f'  <line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#1d0508" stroke-width="1.2"/>')


def grid(xs, ys):
    """The face's sections: a grid parted at grooves `xs` (down) and `ys`
    (across), the outer ones running past the panel's edges as
    Beverley's do."""
    def spans(cuts, lo, hi):
        edges = [lo] + [e for c in cuts for e in (c - GROOVE / 2, c + GROOVE / 2)] + [hi]
        return list(zip(edges[::2], edges[1::2]))
    for y0, y1 in spans(ys, GAP, H - GAP):
        for x0, x1 in spans(xs, -0.02 * W, 1.02 * W):
            geometry["sections"].append([x0, y0, x1, y1])


def rule(y, x0, x1):
    """A printed line under a column's header."""
    o(f'  <line x1="{x0}" y1="{y}" x2="{x1}" y2="{y}" stroke="#2a080c" stroke-width="1.2"/>')


def lin(n):
    return [i / (n - 1) for i in range(n)]


def exp_pos(v, lo, hi):
    """Where `v` sits on a geometric knob from `lo` to `hi`."""
    return math.log(v / lo) / math.log(hi / lo)


o(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">')
o('''  <!-- Wesley's faceplate, written by art/wesley/faceplate.py: don't edit
       it, run that. The print, then Blender's relief maps over it
       (art/beverley/blender/panel.py); each shape in the "controls" layer
       marks one control, drawn from Beverley's skins. -->
  <defs>
    <!-- Textured paint: a fine speckle over the flat colour. -->
    <filter id="paint" x="0" y="0" width="1" height="1">
      <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="7"/>
      <feColorMatrix type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0.5 0 0 0 -0.2"/>
    </filter>
  </defs>''')
o(f'  <rect width="{W}" height="{H}" fill="#08080a"/>')
o(f'  <rect x="0" y="{GAP}" width="{W}" height="{H - 2 * GAP}" fill="{PAINT}"/>')
o(f'  <rect x="0" y="{GAP}" width="{W}" height="{H - 2 * GAP}" filter="url(#paint)" '
  f'opacity="0.06"/>')
for x in (EAR / 2, W - EAR / 2):
    for y in (28, H / 2, H - 28):
        slot_and_screw(x, y)

o('  <g font-family="\'Jost*\'" font-weight="500">')
COL = [EAR + k * BAND_W for k in range(4)]
grid(COL[1:3], [BOTTOM])
for x in COL[1:3]:
    groove(x, 20, x, H - 20)
groove(20, BOTTOM, W - 20, BOTTOM)

# The band columns, each Beverley's strip folded into rows.
for k, (key, name) in enumerate(BANDS):
    x0 = COL[k]
    cx = x0 + BAND_W / 2
    text(cx, 40, name, 13, spacing=3)
    button(f"{key}_on", x0 + 46, 0, "ON", y=34)
    button(f"{key}_solo", x0 + BAND_W - 46, 0, "SOLO", y=34)
    window(f"{key}_display", cx)
    rule(172, x0 + 8, x0 + BAND_W - 8)
    knob(f"{key}_gain", x0 + 56, 0, "GAIN", lin(11), {0: "0", 0.5: "10", 1: "20 dB"})
    button(f"{key}_auto_gain", x0 + 122, 0, "AUTO")
    bat(f"{key}_detect", x0 + 176, 0, "DETECT", "RANGE", "PEAK")
    knob(f"{key}_decay", x0 + 246, 0, "DECAY", lin(9), {0: "1 ms", 0.5: "70", 1: "5 s"})
    knob(f"{key}_gamma", x0 + 54, 1, "GAMMA", lin(9), {0: "0.1", 0.5: "1", 1: "10"})
    knob(f"{key}_bit_depth", cx, 1, "BIT DEPTH", lin(9),
         {0: "1", 0.25: "2", 0.5: "4", 0.75: "8", 1: "16"})
    knob(f"{key}_smoothing", x0 + BAND_W - 54, 1, "SMOOTHING", lin(9),
         {0: "0", 0.5: "20", 1: "100 %"})
    bat(f"{key}_crush_mode", x0 + 54, 2, "CRUSH", "SYM", "ASYM")
    knob(f"{key}_level", cx, 2, "LEVEL", lin(10), {0: "-24", 24 / 36: "0", 1: "+12"})
    knob(f"{key}_mix", x0 + BAND_W - 54, 2, "MIX", lin(11), {0: "DRY", 1: "WET"})

# The bottom row: the title block, as Beverley's; then the crossovers;
# then STEREO and the overall MIX.
top = BOTTOM
o(f'  <rect x="{EAR + 14}" y="{top + 22}" width="5" height="106" fill="{ACCENT}"/>')
text(EAR + 27, top + 68, "WESLEY", 24, anchor="start", spacing=2.5)
text(EAR + 28, top + 86, "3-BAND CRUSHER", 9.5, anchor="start", fill=ACCENT, spacing=3.2)
text(EAR + 28, top + 126, "VULPUS LABS", 8, anchor="start", fill=DIM, spacing=2.4)
knob("low_xover", COL[1] + 92, 3, "LOW X", lin(9),
     {0: "40", exp_pos(200, 40, 1000): "200", 1: "1k"})
knob("high_xover", COL[2] - 92, 3, "HIGH X", lin(9),
     {0: "500", exp_pos(2500, 500, 12000): "2.5k", 1: "12k"})
bat("stereo", COL[2] + 92, 3, "STEREO", "LINK", "MONO", skin="toggle3", middle="SEP")
knob("mix", COL[3] - 92, 3, "MIX", lin(11), {0: "DRY", 1: "WET"})
o('  </g>')

# Blender's relief over the print: light and shadow multiplied, the
# reflected light screened.
for name, blend in (("shade", "multiply"), ("sheen", "screen")):
    o(f'  <image href="art/panel/{name}.png" x="0" y="0" width="{W}" height="{H}" '
      f'preserveAspectRatio="none" style="mix-blend-mode:{blend}"/>')

o('\n  <g id="controls">')
for kind, name, cx, cy, box, skin in controls:
    if kind == "canvas":
        w, h = box
        o(f'    <rect id="canvas.{name}" x="{cx}" y="{cy}" width="{w}" height="{h}" fill="#f0f"/>')
    else:
        o(f'    <rect id="{kind}.{name}" x="{cx - box / 2}" y="{cy - box / 2}" width="{box}" '
          f'height="{box}" data-skin="{skin}" fill="#f0f"/>')
o('  </g>\n</svg>')

with open(os.path.join(CRATE, "wesley.svg"), "w") as f:
    f.write("\n".join(out) + "\n")
os.makedirs(os.path.join(CRATE, "art", "panel"), exist_ok=True)
with open(os.path.join(CRATE, "art", "panel", "geometry.json"), "w") as f:
    json.dump(geometry, f, indent=1)
    f.write("\n")
