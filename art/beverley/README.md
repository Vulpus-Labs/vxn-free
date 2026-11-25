# Beverley's faceplate art

Beverley looks like a two-unit, 1990s rack effects unit: charcoal
textured paint, white legends, black fluted knobs with a white line,
chrome miniature toggles, a grey push-button with a red LED, and rack
screws on nylon cup washers.

Everything here is original, drawn and modelled for vxn-free, under the
repository's licence (MIT OR Apache-2.0). Nothing is taken from Voltage
Modular's skins or art made with them. The lettering is Jost (SIL OFL
1.1), the face silkscreen embeds, and the faceplate's build rasterises
it.

## The parts

- **Print**: `faceplate.py` writes `crates/beverley/beverley.svg` (paint,
  legends, scales, the flat colours of slots, screws and washers, and the
  controls' markers) and the relief's geometry,
  `crates/beverley/art/panel/geometry.json`. Don't edit what it writes:
  change it and run `python3 faceplate.py` here.
- **Panel relief**: `blender/panel.py` builds the panel (its face in
  sections split by engraved grooves, the rack slots, the display's
  recessed window, the screws and washers) and renders two maps into `crates/beverley/art/panel/`. The
  SVG multiplies `shade.png` over the print and screens `sheen.png` over
  it. Re-render after the geometry changes (about 40 s):

  ```sh
  cd blender
  /Applications/Blender.app/Contents/MacOS/Blender -b -P panel.py -- \
      --geometry ../../../crates/beverley/art/panel/geometry.json \
      --out ../../../crates/beverley/art/panel
  ```

- **Controls**: `blender/controls.py` models the knob, the toggle and the
  button and renders them top-down into `crates/beverley/art/controls/`
  (about 40 s). `crates/beverley/build.rs` registers them as skins: the
  knob cycled across its 15° flute period, the toggles with two and three
  positions, the button off and lit. `--inspect` renders 45° product
  shots of each instead, to check the look.

  ```sh
  cd blender
  /Applications/Blender.app/Contents/MacOS/Blender -b -P controls.py -- \
      --out ../../../crates/beverley/art/controls
  ```

Both renders use `blender/studio.py`'s lights, so the panel and the
controls are lit from the same top left. They need Blender 5.2 (made
with 5.2.2). After either, re-bless the editor's snapshots
(`SILKSCREEN_BLESS=1 cargo test -p beverley`) and look at them.
