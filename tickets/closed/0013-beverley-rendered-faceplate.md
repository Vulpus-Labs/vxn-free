---
id: "0013"
title: "Beverley's faceplate rendered in Blender, as 1990s rack gear"
priority: medium
created: 2026-11-24
epic: E002
depends: ["0012"]
---

## Summary

Replace the flat vector faceplate with a rendered one in the style of a
1990s rack effects unit: Blender-modelled controls as skins, and the
panel's relief rendered as maps blended over the vector print.

## Acceptance criteria

- [x] Scripts in `art/beverley/`: a generator for the SVG and the
      relief's geometry, and Blender scripts for the panel maps and the
      control frames, each runnable with one command.
- [x] Controls: a fluted knob as a cycled skin (period ≥ 15°), two- and
      three-position bat toggles, a push-button with an LED, each frame
      at least twice its largest use at 2×.
- [x] Panel: shade and sheen maps from a relief of the face, rack slots,
      screws on cup washers and engraved dividers, lit as the controls
      are.
- [x] Original art only; snapshots re-blessed.

## Close-out (2026-11-24)

- `art/beverley/faceplate.py` writes `beverley.svg` and the relief's
  `geometry.json`. The print is textured charcoal paint (a faint
  turbulence speckle), white legends, an orange title stripe, and the flat
  colours of the rack slots, screws and nylon washers. Knob markers grew
  to 64 px for the rendered knob and its drop shadow; toggles are 40 px
  and the button 30 px. The vector wells and screws of 0004 are gone.
- `art/beverley/blender/studio.py`: Cycles on the CPU with a fixed seed,
  an orthographic top-down camera, key, fill and top area lights from the
  top left, a sky seen only by glossy rays, and a shadow catcher with a
  sun for drop shadows.
- `blender/controls.py`, about 40 s:
  - Knob: black moulded plastic with 24 flutes, a grey cap and a white
    line. Cycled across 15°, with 6 body frames and the pointer, at 256
    px (2× its 2× use).
  - Toggle: a flat grey plastic bat that fans out from its pivot, with
    bevelled edges, in a black plastic boss with a dark throat. Two and
    three positions (the middle seen end-on), at 192 px; the markers are
    46 px.
  - Grey key in a black bezel, with a red LED off and lit, at 128 px.
  - `--inspect` renders 45° product shots.
- `blender/panel.py`, about 40 s at 2× (2180 × 300): the shade and sheen
  maps. Two things learned:
  - Cutting the grooves into one long face left its top shading in big
    diagonal facets. The face is built instead as sections a groove
    apart, over a backing plate, and each section's bevel forms the
    groove's lip.
  - With the edge split before the boolean, the exact solver closed the
    split mesh with a cutter's faces and left a slot-shaped post
    standing. Splitting after the boolean wasn't enough. The manifold
    solver cuts correctly.
  - Suns stand in for the studio's area lights, so the light falls the
    same way along the whole panel.
- `crates/beverley/build.rs` registers the skins and reruns on the maps.
  The art is 0.9 MB of PNGs. Snapshots re-blessed; the editor tests
  still pass unchanged.

## Follow-up (2026-11-24)

The first toggle was chrome: a round bat in a bushing, with a hex nut and
washer. It was replaced by the plastic paddle above, which suits the
panel better. Black on black didn't read from above, so the bat is grey.
It is a little darker thrown up than down: its visible face then turns
away from the key light.

Then the bats' rims (sides and bevels, by the normal in object space) and
the knobs' pointers were painted the title stripe's orange, #d9572b,
which `controls.py` holds as `ACCENT` alongside `faceplate.py`'s.
