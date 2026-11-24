"""Beverley's controls, modelled and rendered top-down as skin frames for
`crates/beverley/beverley.svg`, in the style of a 1990s rack effects unit:

- the knob: black moulded plastic, a fluted grip on a thin flange, a grey
  cap with a line in the title stripe's orange. A cycled skin: `knob_pointer.png` (the line
  alone, at 12 o'clock), then `knob_body_N.png`, the rest turned evenly
  across one flute period (`FLUTE_PERIOD`);
- the toggle: a flat grey plastic bat, fanning out from its pivot and
  bevelled, its rim painted the title stripe's orange, in a black
  plastic boss. `toggle2_{0,1}.png` down and up;
  `toggle3_{0,1,2}.png` down, middle (seen end-on) and up;
- the button: a grey key in a black bezel, with a red LED in the key.
  `button_0.png` off, `button_1.png` lit.

    /Applications/Blender.app/Contents/MacOS/Blender -b -P controls.py -- \\
        --out ../../../crates/beverley/art/controls

Options (after `--`):
    --out DIR     where to write the frames
    --only NAME   knob, toggle or button
    --inspect     45° product shots of each control instead of frames

Licence: original work for vxn-free, under the repository's licence
(MIT OR Apache-2.0).
"""

import argparse
import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import studio  # noqa: E402

# The knob's grip has this many flutes, so it repeats every 15°.
FLUTES = 24
FLUTE_PERIOD = 360.0 / FLUTES
# Body frames across one flute period.
STEPS = 6
# Rendered sizes, px: at least twice the controls' largest use (2× scale).
KNOB_PX = 256
TOGGLE_PX = 192
BUTTON_PX = 128
# Scene units across each frame. The knob is 2 units across, the toggle's
# boss 1.25 tall, the button's bezel 1.
KNOB_SPAN = 2.5
TOGGLE_SPAN = 2.6
BUTTON_SPAN = 1.3
# How far the bat leans from upright, either way.
TOGGLE_TILT = 32.0
# The title stripe's orange, #d9572b (faceplate.py's ACCENT), linear.
ACCENT = (0.693, 0.0953, 0.0242)
# An LED's red, linear.
LED_RED = (1.0, 0.03, 0.01)


def args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    p = argparse.ArgumentParser()
    p.add_argument("--out", required=True)
    p.add_argument("--only")
    p.add_argument("--inspect", action="store_true")
    return p.parse_args(argv)


def plastic(name, grey, roughness=0.42):
    """Moulded plastic with a fine matte texture."""
    m = studio.material(name, (grey, grey, grey * 1.04), roughness=roughness, specular=0.45)
    return studio.bumped(m, scale=180.0, strength=0.06)


def chrome(name="chrome", roughness=0.07):
    return studio.material(name, (0.92, 0.92, 0.94), metallic=1.0, roughness=roughness)


def inspect(s, out, name, target, dist):
    """A product shot from 45° above and 30° round to the right, on a dark
    grey floor, instead of a frame."""
    s.render.resolution_x = s.render.resolution_y = 600
    s.render.film_transparent = False
    floor = bpy.data.objects["floor"]
    floor.is_shadow_catcher = False
    floor.data.materials[0] = studio.material("bench", (0.06, 0.06, 0.065), roughness=0.7)
    cam = s.camera
    cam.data.type = "PERSP"
    cam.data.lens = 85
    e, a = math.radians(45), math.radians(30)
    tx, ty, tz = target
    cam.location = (tx + dist * math.cos(e) * math.sin(a), ty - dist * math.cos(e) * math.cos(a),
                    tz + dist * math.sin(e))
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
    black = plastic("knob plastic", 0.018)
    cap = plastic("cap plastic", 0.09, roughness=0.35)
    ink = studio.material("orange ink", ACCENT, roughness=0.5)
    parts = [
        studio.prism("flange", studio.circle(1.0), 0.0, 0.09, black, bevel=0.03),
        studio.prism("grip", studio.fluted(0.88, 0.055, FLUTES), 0.06, 0.6, black,
                     bevel=0.04, segments=4, taper=0.94),
        studio.prism("cap", studio.circle(0.68), 0.58, 0.64, cap, bevel=0.03),
        studio.prism("pointer", studio.rounded_rect(0.09, 0.46, 0.04), 0.64, 0.645, ink),
    ]
    parts[-1].location.y = 0.39
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
        # Clockwise, seen from above, as the pointer turns up.
        turn.rotation_euler.z = -math.radians(FLUTE_PERIOD * k / STEPS)
        studio.render(s, os.path.join(out, f"knob_body_{k}.png"))
    # The pointer alone, lit where it lies: the rest shades it unseen.
    pointer.hide_render = False
    for part in parts[:-1]:
        part.visible_camera = False
    bpy.data.objects["floor"].hide_render = True
    turn.rotation_euler.z = 0
    studio.render(s, os.path.join(out, "knob_pointer.png"))


# ---- bat toggle --------------------------------------------------------------


def fan(base, tip, length, round_tip=0.08, n=8):
    """A paddle's outline in the XY plane: `base` wide at y = 0, fanning
    out to `tip` wide at y = `length`, its tip corners rounded."""
    r = round_tip
    pts = [(-base / 2, 0.0), (base / 2, 0.0)]
    # Up the right side to where the tip's corner starts, round it, along
    # the tip, round the left corner, and back down.
    for cx, a0 in ((tip / 2 - r, 0.0), (-tip / 2 + r, 90.0)):
        for i in range(n + 1):
            a = math.radians(a0 + 90.0 * i / n)
            pts.append((cx + r * math.cos(a), length - r + r * math.sin(a)))
    return pts


def rimmed(m, rim_color):
    """Paint the rim of a flat part made by `studio.prism`: its sides and
    bevels take `rim_color`, its two broad faces (normal along the
    object's Z) keep their own."""
    nodes, links = m.node_tree.nodes, m.node_tree.links
    bsdf = nodes["Principled BSDF"]
    geometry = nodes.new("ShaderNodeNewGeometry")
    to_object = nodes.new("ShaderNodeVectorTransform")
    to_object.vector_type = "NORMAL"
    to_object.convert_from = "WORLD"
    to_object.convert_to = "OBJECT"
    xyz = nodes.new("ShaderNodeSeparateXYZ")
    facing = nodes.new("ShaderNodeMath")
    facing.operation = "ABSOLUTE"
    # Fully rim below 0.9 of the face's normal, fully face above 0.97: the
    # paint's edge sits on the bevel, just short of the face.
    edge = nodes.new("ShaderNodeMapRange")
    edge.inputs["From Min"].default_value = 0.9
    edge.inputs["From Max"].default_value = 0.97
    edge.inputs["To Min"].default_value = 1.0
    edge.inputs["To Max"].default_value = 0.0
    mix = nodes.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    mix.inputs["A"].default_value = bsdf.inputs["Base Color"].default_value[:]
    mix.inputs["B"].default_value = (*rim_color, 1.0)
    links.new(geometry.outputs["Normal"], to_object.inputs["Vector"])
    links.new(to_object.outputs["Vector"], xyz.inputs["Vector"])
    links.new(xyz.outputs["Z"], facing.inputs[0])
    links.new(facing.outputs["Value"], edge.inputs["Value"])
    links.new(edge.outputs["Result"], mix.inputs["Factor"])
    links.new(mix.outputs["Result"], bsdf.inputs["Base Color"])
    return m


def build_toggle():
    """A plastic toggle: a rounded boss with a dark throat, and a flat bat
    that fans out from its pivot, wide across the panel and thin along
    its throw, its edges bevelled. Returns the parts and the pivot, which
    tilts the bat about X."""
    boss_m = plastic("toggle boss", 0.014, roughness=0.36)
    bat_m = rimmed(plastic("toggle bat", 0.24, roughness=0.36), ACCENT)
    throat_m = studio.material("throat", (0.004, 0.004, 0.005), roughness=0.8, specular=0.1)
    boss = studio.prism("boss", studio.rounded_rect(0.95, 1.25, 0.22), 0.0, 0.16, boss_m,
                        bevel=0.05, segments=4)
    throat = studio.prism("throat", studio.rounded_rect(0.62, 0.3, 0.1), 0.16, 0.165, throat_m)
    # The paddle's outline lies in XY; standing it up turns its length
    # into height and its thickness into depth along the throw.
    bat = studio.prism("bat", fan(0.26, 0.62, 1.0), -0.06, 0.06, bat_m, bevel=0.035,
                       segments=4, bottom_bevel=True)
    bat.rotation_euler.x = math.radians(90)
    lever = studio.pivot([bat], "lever")
    lever.location.z = 0.1
    return [boss, throat, bat], lever


def toggle(out, look):
    s = studio.scene(TOGGLE_PX, TOGGLE_PX, TOGGLE_SPAN)
    parts, lever = build_toggle()
    studio.light_only(parts)
    if look:
        lever.rotation_euler.x = -math.radians(TOGGLE_TILT)
        inspect(s, out, "toggle", (0, 0, 0.5), 7.0)
        return
    # Rotating about X by −t leans the bat towards +Y: up the panel.
    down, middle, up = TOGGLE_TILT, 0.0, -TOGGLE_TILT
    for name, tilts in (("toggle2", (down, up)), ("toggle3", (down, middle, up))):
        for i, t in enumerate(tilts):
            lever.rotation_euler.x = math.radians(t)
            studio.render(s, os.path.join(out, f"{name}_{i}.png"))


# ---- button ------------------------------------------------------------------


def build_button():
    """A square key in a bezel 1 unit across, with an LED lens set in the
    key's upper half. Returns the parts, the lens, and its dark and lit
    materials."""
    bezel = plastic("bezel", 0.012, roughness=0.38)
    key = plastic("key", 0.16, roughness=0.5)
    dark = studio.material("led off", (0.09, 0.006, 0.004), roughness=0.15, coat=0.6)
    lit = studio.material("led on", LED_RED, roughness=0.15, coat=0.6, emission=LED_RED,
                          strength=6.0)
    rim = studio.prism("bezel", studio.rounded_rect(1.0, 1.0, 0.12), 0.0, 0.1, bezel, bevel=0.03)
    cap = studio.prism("key", studio.rounded_rect(0.76, 0.76, 0.08), 0.05, 0.3, key,
                       bevel=0.06, segments=4)
    lens = studio.prism("lens", studio.rounded_rect(0.3, 0.11, 0.05), 0.3, 0.315, dark,
                        bevel=0.01)
    lens.location.y = 0.2
    return [rim, cap, lens], lens, dark, lit


def button(out, look):
    s = studio.scene(BUTTON_PX, BUTTON_PX, BUTTON_SPAN)
    parts, lens, dark, lit = build_button()
    studio.light_only(parts)
    if look:
        lens.data.materials[0] = lit
        inspect(s, out, "button", (0, 0, 0.15), 4.0)
        return
    for i, m in enumerate((dark, lit)):
        lens.data.materials[0] = m
        studio.render(s, os.path.join(out, f"button_{i}.png"))


def main():
    a = args()
    out = os.path.abspath(a.out)
    os.makedirs(out, exist_ok=True)
    for name, make in (("knob", knob), ("toggle", toggle), ("button", button)):
        if a.only in (None, name):
            make(out, a.inspect)


main()
