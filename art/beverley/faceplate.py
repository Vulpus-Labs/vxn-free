"""Writes Beverley's faceplate, `crates/beverley/beverley.svg`, and the
relief `blender/panel.py` renders, `crates/beverley/art/panel/geometry.json`.

The faceplate is a two-unit, 1990s rack panel. The SVG is the print:
paint, legends, scales, and the flat colour of the rack slots, screws and
washers. Over it go two maps that Blender renders from the relief (the
panel's bevelled edges, the slots, the screws on their cup washers and
the engraved dividers): `shade.png`, multiplied, and `sheen.png`,
screened. The controls are skins, rendered by `blender/controls.py`.

    python3 faceplate.py            (from this folder)

Don't edit what it writes; change this and run it again. Re-render the
panel maps when the geometry changes.

Licence: original work for vxn-free, under the repository's licence
(MIT OR Apache-2.0).
"""

import json
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.join(HERE, "..", "..", "crates", "beverley")

W, H = 1290, 150
# The panel stands this far in from the top and bottom of the view; the
# gap shows the rack behind.
GAP = 3
EAR = 36
KY = 92            # control centre line
KR = 25            # knob radius as rendered (the marker is larger)
KNOB_BOX = 64      # knob marker: the rendered frame, drop shadow and all
TOGGLE_BOX = 46
BUTTON_BOX = 30
PAINT = "#232427"
INK = "#ecebe4"     # white legends
DIM = "#9d9c96"     # scales
ACCENT = "#d9572b"  # the 90s signal orange of the title stripe

out = []
o = out.append
geometry = {"size": [W, H], "gap": GAP, "slots": [], "screws": [], "grooves": [],
            "windows": []}


def polar(cx, cy, r, deg):
    """The point `r` from (cx, cy), `deg` clockwise from 12 o'clock."""
    a = math.radians(deg)
    return cx + r * math.sin(a), cy - r * math.cos(a)


def text(x, y, s, size, anchor="middle", fill=INK, spacing=0):
    o(f'  <text x="{x:.2f}" y="{y:.2f}" font-size="{size}" text-anchor="{anchor}" '
      f'letter-spacing="{spacing}" fill="{fill}">{s}</text>')


def scale(cx, ticks, labels):
    """A knob's printed scale: 270° of ticks, majors where labelled."""
    for p in ticks:
        deg = -135 + 270 * p
        major = p in labels
        x1, y1 = polar(cx, KY, KR + 6, deg)
        x2, y2 = polar(cx, KY, KR + (11 if major else 9), deg)
        o(f'  <line x1="{x1:.2f}" y1="{y1:.2f}" x2="{x2:.2f}" y2="{y2:.2f}" '
          f'stroke="{INK if major else DIM}" stroke-width="{1.4 if major else 1}" '
          f'stroke-linecap="round"/>')
    for p, s in labels.items():
        deg = -135 + 270 * p
        x, y = polar(cx, KY, KR + 20, deg)
        anchor = "middle"
        if deg < -20:
            anchor, x = "end", x + 4
        if deg > 20:
            anchor, x = "start", x - 4
        text(x, y + 3, s, 8, anchor=anchor, fill=DIM)


def knob(cx, label, ticks, labels):
    text(cx, 30, label, 10.5, spacing=1.6)
    scale(cx, ticks, labels)


def bat(cx, label, top, bottom, middle=None):
    text(cx, 30, label, 10.5, spacing=1.6)
    text(cx, KY - 22, top, 8, fill=DIM)
    text(cx, KY + 29, bottom, 8, fill=DIM)
    if middle:
        text(cx + 17, KY + 3, middle, 8, anchor="start", fill=DIM)


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


def divider(x):
    """An engraved line between groups: the relief cuts the groove."""
    geometry["grooves"].append([x, 20, 130])
    o(f'  <line x1="{x}" y1="20" x2="{x}" y2="130" stroke="#141517" stroke-width="1.2"/>')


# The crush display's window (ticket 0039): an LCD set into a recess, the
# canvas inside its bezel.
WIN_X, WIN_Y, WIN_W, WIN_H = 828, 40, 160, 84
LCD_INSET = 5


def window():
    """The display's window: a recess the relief cuts part way into the
    face, a dark bezel printed in it, and the LCD's glass (drawn by the
    editor's widget over the canvas marker)."""
    geometry["windows"].append([WIN_X, WIN_Y, WIN_W, WIN_H])
    o(f'  <rect x="{WIN_X}" y="{WIN_Y}" width="{WIN_W}" height="{WIN_H}" rx="3" '
      f'fill="#0b0b0c"/>')
    o(f'  <rect x="{WIN_X + LCD_INSET - 1}" y="{WIN_Y + LCD_INSET - 1}" '
      f'width="{WIN_W - 2 * LCD_INSET + 2}" height="{WIN_H - 2 * LCD_INSET + 2}" '
      f'fill="#3a4430"/>')


def lin(n):
    return [i / (n - 1) for i in range(n)]


o(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">')
o('''  <!-- Beverley's faceplate, written by art/beverley/faceplate.py: don't
       edit it, run that. The print, then Blender's relief maps over it
       (art/beverley/blender/panel.py); each shape in the "controls" layer
       marks one control, drawn from its skin (blender/controls.py). -->
  <defs>
    <!-- Textured paint: a fine speckle over the flat colour. -->
    <filter id="paint" x="0" y="0" width="1" height="1">
      <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="3"/>
      <feColorMatrix type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0.5 0 0 0 -0.2"/>
    </filter>
  </defs>''')
# The rack behind, then the panel's paint.
o(f'  <rect width="{W}" height="{H}" fill="#08080a"/>')
o(f'  <rect x="0" y="{GAP}" width="{W}" height="{H - 2 * GAP}" fill="{PAINT}"/>')
o(f'  <rect x="0" y="{GAP}" width="{W}" height="{H - 2 * GAP}" filter="url(#paint)" '
  f'opacity="0.06"/>')
for x in (EAR / 2, W - EAR / 2):
    for y in (28, H - 28):
        slot_and_screw(x, y)

o('  <g font-family="\'Jost*\'" font-weight="500">')
# The title block.
o(f'  <rect x="{EAR + 14}" y="22" width="5" height="106" fill="{ACCENT}"/>')
text(EAR + 27, 68, "BEVERLEY", 24, anchor="start", spacing=2.5)
text(EAR + 28, 86, "BITCRUSHER", 9.5, anchor="start", fill=ACCENT, spacing=3.2)
text(EAR + 28, 126, "VULPUS LABS", 8, anchor="start", fill=DIM, spacing=2.4)

divider(200)
knob(250, "GAIN", lin(11), {0: "0", 0.5: "10", 1: "20 dB"})
text(312, 30, "AUTO", 10.5, spacing=1.6)
bat(372, "DETECT", "RANGE", "PEAK")
knob(442, "DECAY", lin(9), {0: "1 ms", 0.5: "70", 1: "5 s"})
divider(500)
knob(552, "GAMMA", lin(9), {0: "0.1", 0.5: "1", 1: "10"})
knob(650, "BIT DEPTH", lin(9), {0: "1", 0.25: "2", 0.5: "4", 0.75: "8", 1: "16"})
knob(748, "SMOOTHING", lin(9), {0: "0", 0.5: "20", 1: "100 %"})
divider(808)
text(WIN_X + WIN_W / 2, 30, "1 CYCLE", 10.5, spacing=1.6)
window()
divider(1008)
bat(1046, "CRUSH", "SYM", "ASYM")
bat(1108, "STEREO", "LINK", "MONO", "SEP")
divider(1150)
knob(1192, "MIX", lin(11), {0: "DRY", 1: "WET"})
o('  </g>')

# Blender's relief over the print: light and shadow multiplied, the
# reflected light screened.
for name, blend in (("shade", "multiply"), ("sheen", "screen")):
    o(f'  <image href="art/panel/{name}.png" x="0" y="0" width="{W}" height="{H}" '
      f'preserveAspectRatio="none" style="mix-blend-mode:{blend}"/>')

o('''
  <g id="controls">''')


def marker(kind, name, cx, box, skin):
    o(f'    <rect id="{kind}.{name}" x="{cx - box / 2}" y="{KY - box / 2}" width="{box}" '
      f'height="{box}" data-skin="{skin}" fill="#f0f"/>')


for cx, n in ((250, "gain"), (442, "decay"), (552, "gamma"), (650, "bit_depth"),
              (748, "smoothing"), (1192, "mix")):
    marker("knob", n, cx, KNOB_BOX, "knob")
marker("button", "auto_gain", 312, BUTTON_BOX, "button")
marker("toggle", "detect", 372, TOGGLE_BOX, "toggle2")
marker("toggle", "crush_mode", 1046, TOGGLE_BOX, "toggle2")
marker("toggle", "stereo", 1108, TOGGLE_BOX, "toggle3")
o(f'    <rect id="canvas.display" x="{WIN_X + LCD_INSET}" y="{WIN_Y + LCD_INSET}" '
  f'width="{WIN_W - 2 * LCD_INSET}" height="{WIN_H - 2 * LCD_INSET}" fill="#f0f"/>')
o('  </g>\n</svg>')

with open(os.path.join(CRATE, "beverley.svg"), "w") as f:
    f.write("\n".join(out) + "\n")
os.makedirs(os.path.join(CRATE, "art", "panel"), exist_ok=True)
with open(os.path.join(CRATE, "art", "panel", "geometry.json"), "w") as f:
    json.dump(geometry, f, indent=1)
    f.write("\n")
