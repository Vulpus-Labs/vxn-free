"""Beverley's panel relief, rendered as two maps the faceplate blends over
its print (`faceplate.py` writes both the print and the relief's
geometry):

- `shade.png`, multiplied: the relief's light and shade. White where the
  panel's face is lit flat on; darker on its bevelled edges, in the rack
  slots, the engraved dividers and the display's window, and in the
  shadows the screws and washers cast.
- `sheen.png`, screened: the light the relief reflects. The bevels and
  the screws catch the key light; the paint has a soft satin sheen.

The relief is lit as the controls are (`studio.py`), but by suns along the
studio's light directions, so the light falls the same way along the
whole panel. The maps are smooth and denoised; the paint's grain is the
print's.

    /Applications/Blender.app/Contents/MacOS/Blender -b -P panel.py -- \\
        --geometry ../../../crates/beverley/art/panel/geometry.json \\
        --out ../../../crates/beverley/art/panel

Options (after `--`):
    --geometry FILE  the relief, from faceplate.py
    --out DIR        where to write the maps
    --samples N      Cycles samples (default 128)
    --scale N        pixels per faceplate unit (default 2)

Licence: original work for vxn-free, under the repository's licence
(MIT OR Apache-2.0).
"""

import argparse
import json
import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import studio  # noqa: E402

# Scene units per faceplate unit (px at 1×).
U = 0.01
# The panel is PLATE thick, its front edges rounded over BEVEL.
PLATE = 3.0
BEVEL = 1.4
# Engraved dividers: GROOVE wide, GROOVE_DEPTH deep.
GROOVE = 1.2
GROOVE_DEPTH = 0.7
# The display's window: a recess WINDOW_DEPTH deep, its corners rounded
# WINDOW_R.
WINDOW_DEPTH = 1.6
WINDOW_R = 3.0
# A nylon cup washer, and a pan-head screw in it.
WASHER_R = 7.5
WASHER_H = 1.3
HEAD_R = 5.0
HEAD_H = 2.0
# The studio's light positions (studio.scene): here, directions for suns.
LIGHTS = {"key": ((-3.5, 4.0, 6.0), 6.0), "fill": ((4.0, -2.5, 5.0), 1.1),
          "top": ((-0.5, 0.5, 7.0), 1.0)}
# Exposures: the shade map's flat face comes out white, the sheen map dark
# but for what the relief reflects.
SHADE_EXPOSURE = -1.12
SHEEN_EXPOSURE = -3.8


def args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser()
    p.add_argument("--geometry", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--samples", type=int, default=128)
    p.add_argument("--scale", type=float, default=2.0)
    return p.parse_args(argv)


def suns():
    """Suns where the studio has area lights, pointing the same way."""
    for name in ("key", "fill", "top"):
        old = bpy.data.objects[name]
        bpy.data.objects.remove(old)
    for name, (position, strength) in LIGHTS.items():
        data = bpy.data.lights.new(name, "SUN")
        data.energy = strength
        data.angle = math.radians(20)
        obj = bpy.data.objects.new(name, data)
        bpy.context.collection.objects.link(obj)
        # A sun shines down its own −Z: point that from the light's
        # position towards the origin.
        obj.rotation_euler = Vector(position).to_track_quat("Z", "Y").to_euler()


def to_scene(x, y, size):
    w, h = size
    return (x - w / 2) * U, (h / 2 - y) * U


def cut_by(obj, **operand):
    """Cut `obj` with a boolean difference, then split its sharp edges
    (split first, the mesh is no longer closed). The manifold solver: the
    exact one left a slot cutter's top standing on the first section."""
    obj.modifiers.remove(obj.modifiers["crisp"])
    boolean = obj.modifiers.new("cut", "BOOLEAN")
    boolean.operation = "DIFFERENCE"
    boolean.solver = "MANIFOLD"
    for key, value in operand.items():
        setattr(boolean, key, value)
    split = obj.modifiers.new("crisp", "EDGE_SPLIT")
    split.split_angle = math.radians(35)


def build(geom):
    size = geom["size"]
    w, h = size
    gap = geom["gap"]
    top = PLATE * U
    parts = []

    # The rack behind: a floor under everything, its shadow the panel's.
    bpy.ops.mesh.primitive_plane_add(size=1)
    rack = bpy.context.active_object
    rack.name = "rack"
    rack.scale = (w * U * 1.2, h * U * 2, 1)
    rack.location.z = -0.4 * U
    parts.append(rack)

    # Cutters: slots right through, grooves part way.
    cuts = bpy.data.collections.new("cuts")
    bpy.context.scene.collection.children.link(cuts)
    cuts.hide_render = True

    def cutter(outline, cx, cy, z0, angle=0.0):
        c = studio.prism("cut", outline, z0, top + 1.0)
        bpy.context.collection.objects.unlink(c)
        cuts.objects.link(c)
        c.location.x, c.location.y = to_scene(cx, cy, size)
        c.rotation_euler.z = angle
        c.hide_render = True

    for x, y, sw, sh in geom["slots"]:
        cutter(studio.rounded_rect(sw * U, sh * U, sh / 2 * U), x + sw / 2, y + sh / 2, -1.0)
    # Display windows: recessed part way into the face.
    for x, y, sw, sh in geom.get("windows", []):
        cutter(studio.rounded_rect(sw * U, sh * U, WINDOW_R * U), x + sw / 2, y + sh / 2,
               top - WINDOW_DEPTH * U)

    # The face is in sections, a groove's width apart, over a backing
    # plate a groove's depth down: each section's rounded edge is the
    # groove's lip. Cutting the grooves into one face instead leaves its
    # long top face shading in facets.
    xs = sorted(x for x, _, _ in geom["grooves"])
    bounds = [-0.02 * w] + [e for x in xs for e in (x - GROOVE / 2, x + GROOVE / 2)] + [1.02 * w]
    face_h = (h - 2 * gap) * U
    back = studio.prism("backing", studio.rounded_rect(w * U * 1.04, face_h, 1 * U), 0.0,
                        top - GROOVE_DEPTH * U)
    sections = [back]
    for x0, x1 in zip(bounds[::2], bounds[1::2]):
        section = studio.prism("face", studio.rounded_rect((x1 - x0) * U, face_h, 1 * U),
                               0.0, top, bevel=BEVEL * U, segments=4)
        section.location.x = ((x0 + x1) / 2 - w / 2) * U
        sections.append(section)
    for section in sections:
        cut_by(section, operand_type="COLLECTION", collection=cuts)
        # Flat shading keeps the long faces flat; the bevels carry the
        # light.
        for poly in section.data.polygons:
            poly.use_smooth = False
        parts.append(section)

    # Each screw's own slot cutter, kept out of the panel's cuts.
    screw_cuts = bpy.data.collections.new("screw cuts")
    bpy.context.scene.collection.children.link(screw_cuts)
    screw_cuts.hide_render = True
    washers, heads = [], []
    for sx, sy in geom["screws"]:
        x, y = to_scene(sx, sy, size)
        # The washers sit in the slots, on the rack rail behind the panel's
        # face; their cups stand proud of it.
        washer = studio.prism("washer", studio.circle(WASHER_R * U, 64), -0.4 * U,
                              top + WASHER_H * U, bevel=WASHER_H * 0.8 * U, segments=5)
        washer.location.x, washer.location.y = x, y
        head = studio.prism("head", studio.circle(HEAD_R * U, 64), top, top + (WASHER_H + HEAD_H) * U,
                            bevel=HEAD_H * 0.9 * U, segments=6)
        head.location.x, head.location.y = x, y
        slot = studio.prism("slot", studio.rounded_rect(7.4 * U, 1.3 * U, 0.6 * U),
                            top + (WASHER_H + HEAD_H * 0.4) * U, top + 1.0)
        bpy.context.collection.objects.unlink(slot)
        screw_cuts.objects.link(slot)
        slot.hide_render = True
        slot.location.x, slot.location.y = x, y
        slot.rotation_euler.z = -math.atan2(2.8, 6.8)
        cut_by(head, object=slot)
        washers.append(washer)
        heads.append(head)
    return parts, washers, heads


def paint_all(objs, m):
    for obj in objs:
        obj.data.materials.clear()
        obj.data.materials.append(m)


def main():
    a = args()
    with open(a.geometry) as f:
        geom = json.load(f)
    w, h = geom["size"]
    s = studio.scene(round(w * a.scale), round(h * a.scale), w * U, samples=a.samples)
    s.render.film_transparent = False
    s.render.image_settings.color_mode = "RGB"
    # No dither: the maps are smooth, and dither's noise won't compress.
    s.render.dither_intensity = 0.0
    bpy.data.objects["floor"].hide_render = True
    bpy.data.objects["sun"].hide_render = True
    suns()
    parts, washers, heads = build(geom)

    white = studio.material("shade", (1.0, 1.0, 1.0), roughness=1.0, specular=0.0)
    satin = studio.material("paint sheen", (0.0, 0.0, 0.0), roughness=0.4, specular=0.5)
    nylon = studio.material("nylon sheen", (0.0, 0.0, 0.0), roughness=0.25, specular=0.6)
    steel = studio.material("steel sheen", (0.35, 0.35, 0.36), metallic=1.0, roughness=0.3)
    out = os.path.abspath(a.out)
    os.makedirs(out, exist_ok=True)
    for name, materials, exposure in (
        ("shade", (white, white, white), SHADE_EXPOSURE),
        ("sheen", (satin, nylon, steel), SHEEN_EXPOSURE),
    ):
        for objs, m in zip((parts, washers, heads), materials):
            paint_all(objs, m)
        s.view_settings.exposure = exposure
        studio.render(s, os.path.join(out, f"{name}.png"))


main()
