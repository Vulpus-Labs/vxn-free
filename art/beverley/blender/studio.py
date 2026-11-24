"""The studio Beverley's art is rendered in: Cycles on the CPU with a
fixed seed, an orthographic camera looking straight down, three area
lights from the top left, a sky seen only in reflections, and a floor
that catches the parts' drop shadows on a transparent background.

`controls.py` and `panel.py` import it; run those, not this.

Licence: original work for vxn-free, under the repository's licence
(MIT OR Apache-2.0).
"""

import math

import bmesh
import bpy


# ---- materials -------------------------------------------------------------


def material(name, color, metallic=0.0, roughness=0.5, coat=0.0, specular=0.5,
             emission=None, strength=0.0):
    """A Principled material; `color` and `emission` in linear RGB."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = m.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Coat Weight"].default_value = coat
    bsdf.inputs["Specular IOR Level"].default_value = specular
    if emission:
        bsdf.inputs["Emission Color"].default_value = (*emission, 1.0)
        bsdf.inputs["Emission Strength"].default_value = strength
    return m


def bumped(m, scale, strength, detail=8.0):
    """Add a fine noise bump to `m`: moulded or painted texture."""
    nodes, links = m.node_tree.nodes, m.node_tree.links
    coord = nodes.new("ShaderNodeTexCoord")
    noise = nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = scale
    noise.inputs["Detail"].default_value = detail
    bump = nodes.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = strength
    links.new(coord.outputs["Object"], noise.inputs["Vector"])
    links.new(noise.outputs["Fac"], bump.inputs["Height"])
    links.new(bump.outputs["Normal"], nodes["Principled BSDF"].inputs["Normal"])
    return m


# ---- solids ----------------------------------------------------------------


def prism(name, outline, z0, z1, mat=None, bevel=0.0, segments=3, taper=1.0,
          bottom_bevel=False):
    """A solid over the closed 2D `outline` from height z0 to z1, its top
    scaled by `taper`. Its top edges are rounded over by `bevel` (and its
    bottom edges too with `bottom_bevel`)."""
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    low = [bm.verts.new((x, y, z0)) for x, y in outline]
    high = [bm.verts.new((x * taper, y * taper, z1)) for x, y in outline]
    n = len(outline)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((low[i], low[j], high[j], high[i]))
    bm.faces.new(list(reversed(low)))
    bm.faces.new(high)
    weight = bm.edges.layers.float.new("bevel_weight_edge")
    tops = set(high)
    for edge in bm.edges:
        on_top = all(v in tops for v in edge.verts)
        on_bottom = not any(v in tops for v in edge.verts)
        if on_top or (bottom_bevel and on_bottom):
            edge[weight] = 1.0
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    if mat:
        obj.data.materials.append(mat)
    if bevel > 0:
        mod = obj.modifiers.new("bevel", "BEVEL")
        mod.width = bevel
        mod.segments = segments
        mod.limit_method = "WEIGHT"
    for poly in obj.data.polygons:
        poly.use_smooth = True
    # Keep flat faces flat: smooth shading alone would dome a thin disc.
    split = obj.modifiers.new("crisp", "EDGE_SPLIT")
    split.split_angle = math.radians(35)
    return obj


def circle(r, n=128):
    return [(r * math.cos(2 * math.pi * i / n), r * math.sin(2 * math.pi * i / n))
            for i in range(n)]


def polygon(r, sides, rotation=0.0):
    """A regular polygon with its corners at radius `r`."""
    return [(r * math.cos(rotation + 2 * math.pi * i / sides),
             r * math.sin(rotation + 2 * math.pi * i / sides)) for i in range(sides)]


def rounded_rect(w, h, r, n=10):
    """A `w` × `h` rectangle about the origin with corners of radius `r`."""
    r = min(r, w / 2, h / 2)
    pts = []
    for cx, cy, a0 in ((w / 2 - r, h / 2 - r, 0), (-w / 2 + r, h / 2 - r, 90),
                       (-w / 2 + r, -h / 2 + r, 180), (w / 2 - r, -h / 2 + r, 270)):
        for i in range(n + 1):
            a = math.radians(a0 + 90 * i / n)
            p = (cx + r * math.cos(a), cy + r * math.sin(a))
            # Where the corners meet (r is half a side), skip the repeat:
            # a zero-length edge breaks booleans.
            if not pts or math.dist(p, pts[-1]) > 1e-7:
                pts.append(p)
    if math.dist(pts[0], pts[-1]) <= 1e-7:
        pts.pop()
    return pts


def fluted(r, depth, count, per=12):
    """A circle of radius `r` with `count` rounded grooves `depth` deep."""
    pts = []
    n = count * per
    for i in range(n):
        a = 2 * math.pi * i / n
        groove = (0.5 + 0.5 * math.cos(count * a)) ** 3
        rr = r - depth * groove
        pts.append((rr * math.cos(a), rr * math.sin(a)))
    return pts


def pivot(parts, name):
    """An empty that `parts` hang from, to turn or tilt them together."""
    empty = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(empty)
    for part in parts:
        part.parent = empty
    return empty


# ---- the studio ------------------------------------------------------------


def _sky(s):
    """A sky seen only by glossy rays: a bright band of softboxes round the
    middle, dark at the horizon and zenith, so curved metal and gloss
    mirror bright lines while flat faces keep their colour. Diffuse
    surfaces see almost none of it."""
    world = bpy.data.worlds.new("sky")
    world.use_nodes = True
    nodes, links = world.node_tree.nodes, world.node_tree.links
    coord = nodes.new("ShaderNodeTexCoord")
    xyz = nodes.new("ShaderNodeSeparateXYZ")
    ramp = nodes.new("ShaderNodeValToRGB")
    stops = [(0.0, 0.02), (0.35, 0.3), (0.6, 0.95), (0.8, 0.5), (1.0, 0.05)]
    els = ramp.color_ramp.elements
    while len(els) < len(stops):
        els.new(0.5)
    for el, (pos, v) in zip(els, stops):
        el.position = pos
        el.color = (v, v, v, 1)
    links.new(coord.outputs["Generated"], xyz.inputs["Vector"])
    links.new(xyz.outputs["Z"], ramp.inputs["Fac"])
    links.new(ramp.outputs["Color"], nodes["Background"].inputs["Color"])
    path = nodes.new("ShaderNodeLightPath")
    mix = nodes.new("ShaderNodeMix")
    mix.name = "sky strength"
    mix.data_type = "FLOAT"
    mix.inputs["A"].default_value = 0.02
    mix.inputs["B"].default_value = 1.0
    links.new(path.outputs["Is Glossy Ray"], mix.inputs["Factor"])
    links.new(mix.outputs["Result"], nodes["Background"].inputs["Strength"])
    s.world = world


def _light(name, size, energy, location):
    data = bpy.data.lights.new(name, "AREA")
    data.size = size
    data.energy = energy
    obj = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(obj)
    obj.location = location
    target = bpy.data.objects.get("aim")
    if target is None:
        target = bpy.data.objects.new("aim", None)
        bpy.context.collection.objects.link(target)
    track = obj.constraints.new("TRACK_TO")
    track.target = target
    return obj


def scene(width, height, span, samples=128):
    """A fresh scene rendering `width` × `height` px, the camera's view
    `span` units across (its longer side), looking down the Z axis."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    s = bpy.context.scene
    s.render.engine = "CYCLES"
    s.cycles.device = "CPU"
    s.cycles.samples = samples
    s.cycles.seed = 7
    s.cycles.use_denoising = True
    s.render.film_transparent = True
    s.render.resolution_x = width
    s.render.resolution_y = height
    s.render.resolution_percentage = 100
    s.render.image_settings.file_format = "PNG"
    s.render.image_settings.color_mode = "RGBA"
    s.render.image_settings.color_depth = "8"
    s.view_settings.view_transform = "Standard"
    _sky(s)

    cam = bpy.data.cameras.new("camera")
    cam.type = "ORTHO"
    cam.ortho_scale = span
    obj = bpy.data.objects.new("camera", cam)
    bpy.context.collection.objects.link(obj)
    obj.location = (0, 0, 20)
    s.camera = obj

    # The key rakes from the top left, the fill lifts the right, and a
    # soft light overhead keeps flat tops from going dark.
    _light("key", 3.0, 520, (-3.5, 4.0, 6.0))
    _light("fill", 6.0, 70, (4.0, -2.5, 5.0))
    _light("top", 4.0, 60, (-0.5, 0.5, 7.0))

    bpy.ops.mesh.primitive_plane_add(size=60)
    floor = bpy.context.active_object
    floor.name = "floor"
    floor.is_shadow_catcher = True
    floor.data.materials.append(material("floor", (0.5, 0.5, 0.5), roughness=1.0))
    _drop_shadow_sun(floor)
    return s


def _drop_shadow_sun(floor):
    """A soft sun from the top left that lights only the floor: each part
    blocks it, casting a drop shadow down and to the right onto the
    catcher, without changing how the part itself is lit."""
    data = bpy.data.lights.new("sun", "SUN")
    data.energy = 3.0
    data.angle = math.radians(14)
    obj = bpy.data.objects.new("sun", data)
    bpy.context.collection.objects.link(obj)
    obj.rotation_euler = (math.radians(32), 0, math.radians(215))
    only = bpy.data.collections.new("floor only")
    only.objects.link(floor)
    obj.light_linking.receiver_collection = only


def light_only(parts):
    """The key and fill light only `parts`: their long raking shadows
    would otherwise run off the frame. The sun casts the drop shadow."""
    only = bpy.data.collections.new("parts only")
    for part in parts:
        only.objects.link(part)
    for name in ("key", "fill"):
        bpy.data.objects[name].light_linking.receiver_collection = only


def render(s, path):
    s.render.filepath = path
    bpy.ops.render.render(write_still=True)
