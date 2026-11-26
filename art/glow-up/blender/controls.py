"""Glow-Up's controls, modelled and rendered top-down as skin frames for
`crates/glow-up/glow-up.svg`, in the style of a modern rack unit:

- the knob: machined aluminium, a finely knurled skirt under a smooth
  band, a chamfered top with concentric turning marks, and a line cut in
  it and filled with the panel's amber. A cycled skin:
  `knob_pointer.png` (the line alone, at 12 o'clock), then
  `knob_body_N.png`, the body turned evenly across one knurl period
  (`KNURL_PERIOD`). The same frames serve the large knobs and the small
  trims;
- the key: a soft-touch black key with a rounded square face, an amber
  LED behind a small lens at its top. `key_0.png` off, `key_1.png` lit.

    /Applications/Blender.app/Contents/MacOS/Blender -b -P controls.py -- \\
        --out ../../../crates/glow-up/art/controls

Options (after `--`):
    --out DIR     where to write the frames
    --only NAME   knob or key
    --inspect     45° product shots instead of frames

The studio (lights, camera, materials) is Beverley's,
`art/beverley/blender/studio.py`, so the families are lit alike.

Licence: original work for vxn-free, under the repository's licence
(MIT OR Apache-2.0).
"""

import argparse
import math
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "beverley", "blender"))
import studio  # noqa: E402

# The knurl: this many ridges, so the body repeats every 6°.
KNURLS = 60
KNURL_PERIOD = 360.0 / KNURLS
# Body frames across one knurl period.
STEPS = 4
# Rendered sizes, px: at least twice the largest use at 2× scale.
KNOB_PX = 320
KEY_PX = 128
# Scene units across each frame: the knob is 2 units across, the key 1.
KNOB_SPAN = 2.5
KEY_SPAN = 1.3
# The panel's amber, #f2a33a (faceplate.py's ACCENT), linear.
AMBER = (0.888, 0.366, 0.0423)


def args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser()
    p.add_argument("--out", required=True)
    p.add_argument("--only")
    p.add_argument("--inspect", action="store_true")
    return p.parse_args(argv)


def aluminium(name, roughness=0.28, turned=False):
    """Bead-blasted or turned aluminium. Turned: anisotropic about the
    knob's axis, so the top shows the turning marks' radial highlight."""
    m = studio.material(name, (0.58, 0.585, 0.6), metallic=1.0, roughness=roughness)
    bsdf = m.node_tree.nodes["Principled BSDF"]
    if turned:
        nodes, links = m.node_tree.nodes, m.node_tree.links
        bsdf.inputs["Anisotropic"].default_value = 0.8
        tangent = nodes.new("ShaderNodeTangent")
        tangent.direction_type = "RADIAL"
        tangent.axis = "Z"
        links.new(tangent.outputs["Tangent"], bsdf.inputs["Tangent"])
    return studio.bumped(m, scale=400.0, strength=0.02)


def inspect(s, out, name, target, dist):
    """A product shot from 45° above and 30° round to the right, on a
    graphite floor, instead of a frame."""
    s.render.resolution_x = s.render.resolution_y = 600
    s.render.film_transparent = False
    floor = bpy.data.objects["floor"]
    floor.is_shadow_catcher = False
    floor.data.materials[0] = studio.material("bench", (0.03, 0.032, 0.036), roughness=0.5)
    cam = s.camera
    cam.data.type = "PERSP"
    cam.data.lens = 85
    e, a = math.radians(45), math.radians(30)
    tx, ty, tz = target
    cam.location = (tx + dist * math.cos(e) * math.sin(a),
                    ty - dist * math.cos(e) * math.cos(a), tz + dist * math.sin(e))
    aim = bpy.data.objects.new("look", None)
    bpy.context.collection.objects.link(aim)
    aim.location = target
    track = cam.constraints.new("TRACK_TO")
    track.target = aim
    track.track_axis = "TRACK_NEGATIVE_Z"
    track.up_axis = "UP_Y"
    studio.render(s, os.path.join(out, f"{name}_inspect.png"))


# ---- knob ------------------------------------------------------------------


def build_knob():
    """The knob, 2 units across; the pointer last."""
    knurled = aluminium("knurled", roughness=0.35)
    band = aluminium("band", roughness=0.26)
    top = aluminium("top", roughness=0.42, turned=True)
    fill = studio.material("amber fill", AMBER, roughness=0.35, emission=AMBER, strength=0.4)
    parts = [
        studio.prism("skirt", studio.fluted(1.0, 0.035, KNURLS, per=8), 0.0, 0.34, knurled,
                     bevel=0.03, segments=3),
        studio.prism("band", studio.circle(0.96), 0.32, 0.6, band, bevel=0.1, segments=6,
                     taper=0.96),
        studio.prism("top", studio.circle(0.8), 0.59, 0.62, top, bevel=0.02),
        studio.prism("pointer", studio.rounded_rect(0.08, 0.62, 0.04), 0.62, 0.625, fill),
    ]
    parts[-1].location.y = 0.5
    return parts, studio.pivot(parts, "knob")


def knob(out, look):
    s = studio.scene(KNOB_PX, KNOB_PX, KNOB_SPAN)
    parts, turn = build_knob()
    studio.light_only(parts)
    pointer = parts[-1]
    if look:
        turn.rotation_euler.z = -math.radians(50)
        inspect(s, out, "knob", (0, 0, 0.3), 7.0)
        return
    pointer.hide_render = True
    for k in range(STEPS):
        turn.rotation_euler.z = -math.radians(KNURL_PERIOD * k / STEPS)
        studio.render(s, os.path.join(out, f"knob_body_{k}.png"))
    pointer.hide_render = False
    for part in parts[:-1]:
        part.visible_camera = False
    bpy.data.objects["floor"].hide_render = True
    turn.rotation_euler.z = 0
    studio.render(s, os.path.join(out, "knob_pointer.png"))


# ---- key -------------------------------------------------------------------


def build_key():
    """A soft-touch key 0.9 across in a thin black collar, with an LED lens
    near its top edge. Returns the parts, the lens, and its dark and lit
    materials."""
    collar = studio.material("collar", (0.008, 0.008, 0.009), roughness=0.3, specular=0.5)
    rubber = studio.bumped(studio.material("soft touch", (0.025, 0.025, 0.027), roughness=0.62,
                                           specular=0.35), scale=300.0, strength=0.03)
    dark = studio.material("led off", (0.12, 0.05, 0.01), roughness=0.1, coat=0.8)
    lit = studio.material("led on", AMBER, roughness=0.1, coat=0.8, emission=AMBER,
                          strength=9.0)
    rim = studio.prism("collar", studio.rounded_rect(1.0, 1.0, 0.2), 0.0, 0.08, collar,
                       bevel=0.02)
    cap = studio.prism("key", studio.rounded_rect(0.84, 0.84, 0.16), 0.04, 0.26, rubber,
                       bevel=0.07, segments=6)
    lens = studio.prism("lens", studio.circle(0.07, 48), 0.26, 0.275, dark, bevel=0.01)
    lens.location.y = 0.22
    return [rim, cap, lens], lens, dark, lit


def key(out, look):
    s = studio.scene(KEY_PX, KEY_PX, KEY_SPAN)
    parts, lens, dark, lit = build_key()
    studio.light_only(parts)
    if look:
        lens.data.materials[0] = lit
        inspect(s, out, "key", (0, 0, 0.12), 4.0)
        return
    for i, m in enumerate((dark, lit)):
        lens.data.materials[0] = m
        studio.render(s, os.path.join(out, f"key_{i}.png"))


def main():
    a = args()
    out = os.path.abspath(a.out)
    os.makedirs(out, exist_ok=True)
    for name, make in (("knob", knob), ("key", key)):
        if a.only in (None, name):
            make(out, a.inspect)


main()
