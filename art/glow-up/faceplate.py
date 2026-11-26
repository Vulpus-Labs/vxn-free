"""Writes Glow-Up's faceplate, `crates/glow-up/glow-up.svg`, and the relief
Beverley's `blender/panel.py` renders, `crates/glow-up/art/panel/geometry.json`.

Glow-Up is a modern rack unit, where Beverley and Wesley are 1990s ones:
graphite anodised aluminium with a horizontal brush, light legends and
fine scales, a warm amber for the title and the lights, machined
aluminium knobs and soft-touch keys with LEDs (`blender/controls.py`),
black hex button-head rack screws. No engraved grooves: the sections are
parted by printed hairlines. The tracking display is the centrepiece:

    [title  ] [BOOST A ] [  display           ] [BOOST B ] [FILTER         ] [OUTPUT ]
    [       ] [ AMOUNT ] [                    ] [ AMOUNT ] [RESONANCE FOCUS] [  MIX  ]
    [MODE   ] [NUM/DEN ] [WINDOW THRESH  GRID ] [NUM/DEN ] [WIDTH TONE SPEED][DRY CUT]

    python3 faceplate.py            (from this folder)

Then re-render the relief (a minute or two):

    cd ../beverley/blender
    /Applications/Blender.app/Contents/MacOS/Blender -b -P panel.py -- \\
        --geometry ../../../crates/glow-up/art/panel/geometry.json \\
        --out ../../../crates/glow-up/art/panel

Don't edit what it writes; change this and run it again.

Licence: original work for vxn-free, under the repository's licence
(MIT OR Apache-2.0).
"""

import json
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.join(HERE, "..", "..", "crates", "glow-up")

GAP = 3
EAR = 36
# The sections' left edges, then the right ear's.
TITLE, BOOST_A, DISPLAY, BOOST_B, FILTER, OUTPUT, RIGHT = (
    EAR, 236, 436, 876, 1076, 1356, 1476)
W = RIGHT + EAR
H = 330
# Rows: the big knobs' centre line, the small controls', and the header.
HEAD_Y = 40
BIG_Y = 126
SMALL_Y = 258
# Markers: the rendered frame (knob 0.8 of it, shadow round it).
BIG_BOX = 80
SMALL_BOX = 48
KEY_BOX = 30
# Graphite anodised aluminium.
PAINT = "#25272b"
INK = "#e7e5e0"
DIM = "#8e9096"
FAINT = "#4a4d53"
ACCENT = "#f2a33a"  # amber: the title and the lights
# The display's window.
WIN = (DISPLAY + 20, 24, 400, 184)
GLASS_INSET = 6

out = []
o = out.append
geometry = {"size": [W, H], "gap": GAP, "slots": [], "screws": [], "grooves": [],
            "windows": [], "sections": [[-0.02 * W, GAP, 1.02 * W, H - GAP]],
            "screw": "hex"}
controls = []


def polar(cx, cy, r, deg):
    a = math.radians(deg)
    return cx + r * math.sin(a), cy - r * math.cos(a)


def text(x, y, s, size, anchor="middle", fill=INK, spacing=0):
    o(f'  <text x="{x:.2f}" y="{y:.2f}" font-size="{size}" text-anchor="{anchor}" '
      f'letter-spacing="{spacing}" fill="{fill}">{s}</text>')


def scale(cx, cy, r, ticks, labels, size=7.5):
    """A knob's printed scale: 270° of fine ticks, majors where labelled,
    the labels round the outside."""
    for p in ticks:
        deg = -135 + 270 * p
        major = any(abs(p - q) < 1e-6 for q in labels)
        x1, y1 = polar(cx, cy, r + 4, deg)
        x2, y2 = polar(cx, cy, r + (9 if major else 7), deg)
        o(f'  <line x1="{x1:.2f}" y1="{y1:.2f}" x2="{x2:.2f}" y2="{y2:.2f}" '
          f'stroke="{INK if major else DIM}" stroke-width="{1.2 if major else 0.8}" '
          f'stroke-linecap="round"/>')
    for p, s in labels.items():
        deg = -135 + 270 * p
        x, y = polar(cx, cy, r + 16, deg)
        anchor = "middle"
        if deg < -30:
            anchor, x = "end", x + 5
        if deg > 30:
            anchor, x = "start", x - 5
        text(x, y + 3, s, size, anchor=anchor, fill=DIM)


def knob(name, cx, cy, label, ticks, labels, big=True):
    box = BIG_BOX if big else SMALL_BOX
    r = 0.4 * box
    scale(cx, cy, r, ticks, labels)
    text(cx, cy + r + 24, label, 9 if big else 8, fill=INK, spacing=1.6)
    controls.append(("knob", name, cx, cy, box, "knob" if big else "knob_small"))


def key(name, cx, cy, label):
    """A soft-touch key with its LED, labelled underneath."""
    text(cx, cy + KEY_BOX / 2 + 13, label, 7.5, fill=DIM, spacing=1.2)
    controls.append(("button", name, cx, cy, KEY_BOX, "key"))


def header(x0, x1, s):
    """A section's name, left-aligned, with an amber tick before it."""
    o(f'  <rect x="{x0 + 14}" y="{HEAD_Y - 8}" width="3" height="10" fill="{ACCENT}"/>')
    text(x0 + 22, HEAD_Y, s, 9, anchor="start", spacing=2.4)


def hairline(x):
    """A section's edge: a dark line with a faint light one beside it, as
    a machined step catches the light."""
    o(f'  <line x1="{x}" y1="18" x2="{x}" y2="{H - 18}" stroke="#141518" stroke-width="1"/>')
    o(f'  <line x1="{x + 1}" y1="18" x2="{x + 1}" y2="{H - 18}" stroke="#34373c" '
      f'stroke-width="1"/>')


def slot_and_screw(cx, cy):
    """A rack slot with a black hex button-head screw in it, off-centre."""
    sw, sh = 20, 11
    geometry["slots"].append([cx - sw / 2, cy - sh / 2, sw, sh])
    o(f'  <rect x="{cx - sw / 2}" y="{cy - sh / 2}" width="{sw}" height="{sh}" rx="{sh / 2}" '
      f'fill="#08090a"/>')
    sx = cx - 2.5
    geometry["screws"].append([sx, cy])
    o(f'  <circle cx="{sx}" cy="{cy}" r="6.5" fill="#1b1c1f"/>')
    hexagon = " ".join(f"{sx + 2.4 * math.cos(math.radians(30 + 60 * i)):.2f},"
                       f"{cy + 2.4 * math.sin(math.radians(30 + 60 * i)):.2f}" for i in range(6))
    o(f'  <polygon points="{hexagon}" fill="#060607"/>')


def window():
    """The display: a recess the relief cuts into the face, a black bezel
    printed in it, and the glass the editor's widget draws."""
    x, y, w, h = WIN
    geometry["windows"].append([x, y, w, h])
    o(f'  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="#060607"/>')
    g = GLASS_INSET
    o(f'  <rect x="{x + g - 1}" y="{y + g - 1}" width="{w - 2 * g + 2}" '
      f'height="{h - 2 * g + 2}" rx="2" fill="#0b0d10"/>')
    controls.append(("canvas", "display", x + g, y + g, (w - 2 * g, h - 2 * g), None))


def lin(n):
    return [i / (n - 1) for i in range(n)]


def exp_pos(v, lo, hi):
    """Where `v` sits on a geometric knob from `lo` to `hi`."""
    return math.log(v / lo) / math.log(hi / lo)


def boost(x0, which):
    """A boost's section: its AMOUNT, and its ratio as NUM over DEN."""
    cx = x0 + 100
    k = which.lower()
    header(x0, x0 + 200, f"BOOST {which}")
    knob(f"{k}_amount", cx, BIG_Y, "AMOUNT", lin(11), {0: "0", 0.5: "50", 1: "100"})
    steps = {i / 7: str(i + 1) for i in range(8)}
    knob(f"{k}_num", cx - 46, SMALL_Y, "NUM", lin(8), steps, big=False)
    knob(f"{k}_den", cx + 46, SMALL_Y, "DEN", lin(8), steps, big=False)
    # The ratio's bar between them.
    o(f'  <line x1="{cx + 5}" y1="{SMALL_Y - 9}" x2="{cx - 5}" y2="{SMALL_Y + 9}" '
      f'stroke="{DIM}" stroke-width="1.4" stroke-linecap="round"/>')


o(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">')
o('''  <!-- Glow-Up's faceplate, written by art/glow-up/faceplate.py: don't edit
       it, run that. The print, then Blender's relief maps over it
       (art/beverley/blender/panel.py); each shape in the "controls" layer
       marks one control, drawn from Glow-Up's skins
       (art/glow-up/blender/controls.py). -->
  <defs>
    <!-- Brushed anodising: long, fine horizontal streaks. -->
    <filter id="brush" x="0" y="0" width="1" height="1">
      <feTurbulence type="fractalNoise" baseFrequency="0.0025 0.9" numOctaves="3" seed="11"/>
      <feColorMatrix type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  1.4 0 0 0 -0.55"/>
    </filter>
    <!-- The title's glow: amber light fading out round it. -->
    <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#f2a33a" stop-opacity="0.22"/>
      <stop offset="1" stop-color="#f2a33a" stop-opacity="0"/>
    </radialGradient>
  </defs>''')
o(f'  <rect width="{W}" height="{H}" fill="#08080a"/>')
o(f'  <rect x="0" y="{GAP}" width="{W}" height="{H - 2 * GAP}" fill="{PAINT}"/>')
o(f'  <rect x="0" y="{GAP}" width="{W}" height="{H - 2 * GAP}" filter="url(#brush)" '
  f'opacity="0.07"/>')
for x in (EAR / 2, W - EAR / 2):
    for y in (30, H - 30):
        slot_and_screw(x, y)

o('  <g font-family="\'Jost*\'" font-weight="500">')
for x in (BOOST_A, DISPLAY, BOOST_B, FILTER, OUTPUT):
    hairline(x)

# The title.
o(f'  <ellipse cx="{TITLE + 98}" cy="84" rx="96" ry="46" fill="url(#glow)"/>')
text(TITLE + 20, 92, "GLOW-UP", 30, anchor="start", spacing=3)
o(f'  <rect x="{TITLE + 21}" y="104" width="34" height="2" fill="{ACCENT}"/>')
text(TITLE + 21, 124, "HARMONIC FILTER", 9, anchor="start", fill=ACCENT, spacing=3)
text(TITLE + 21, H - 22, "VULPUS LABS", 7.5, anchor="start", fill=DIM, spacing=2.4)
text(TITLE + 21, SMALL_Y - 32, "MODE", 8, anchor="start", spacing=1.6)
key("mono", TITLE + 40, SMALL_Y, "MONO")
key("stereo", TITLE + 100, SMALL_Y, "STEREO")

boost(BOOST_A, "A")

# The display and what shapes the analysis.
window()
knob("window", DISPLAY + 60, SMALL_Y, "WINDOW", lin(3), {0: "512", 0.5: "1k", 1: "2k"},
     big=False)
knob("threshold", DISPLAY + 156, SMALL_Y, "THRESHOLD", lin(7),
     {0: "0", 0.5: "15", 1: "30 dB"}, big=False)
text(DISPLAY + 320, SMALL_Y - 32, "GRID", 8, spacing=1.6)
key("grid_lin", DISPLAY + 260, SMALL_Y, "LIN")
key("grid_irr", DISPLAY + 320, SMALL_Y, "IRR")
key("grid_tet", DISPLAY + 380, SMALL_Y, "TET")

boost(BOOST_B, "B")

# The filters' character.
header(FILTER, OUTPUT, "FILTER")
knob("resonance", FILTER + 76, BIG_Y, "RESONANCE", lin(11), {0: "0", 0.5: "SING", 1: "100"})
knob("focus", FILTER + 204, BIG_Y, "FOCUS", lin(11), {0: "GRID", 1: "TRACK"})
knob("width", FILTER + 50, SMALL_Y, "WIDTH", lin(9),
     {0: "10", 0.5: "45", 1: "200"}, big=False)
knob("tone", FILTER + 140, SMALL_Y, "TONE", lin(9), {0: "-6", 0.5: "0", 1: "+6"},
     big=False)
knob("speed", FILTER + 230, SMALL_Y, "SPEED", lin(9), {0: "SLOW", 1: "FAST"}, big=False)

# The output.
header(OUTPUT, RIGHT, "OUTPUT")
knob("mix", OUTPUT + 60, BIG_Y, "MIX", lin(11), {0: "DRY", 1: "WET"})
key("dry_cut", OUTPUT + 60, SMALL_Y, "DRY CUT")
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

with open(os.path.join(CRATE, "glow-up.svg"), "w") as f:
    f.write("\n".join(out) + "\n")
os.makedirs(os.path.join(CRATE, "art", "panel"), exist_ok=True)
with open(os.path.join(CRATE, "art", "panel", "geometry.json"), "w") as f:
    json.dump(geometry, f, indent=1)
    f.write("\n")
