# Glow-Up's faceplate art

Glow-Up looks like a modern rack unit: graphite anodised aluminium with a
horizontal brush, light legends and fine scales, machined aluminium knobs
with a dark polished-metal line, soft-touch keys with amber LEDs, and black hex
button-head rack screws. Beverley and Wesley are its 1990s siblings.

Everything here is original, drawn and modelled for vxn-free, under the
repository's licence (MIT OR Apache-2.0). The lettering is Jost (SIL OFL
1.1), the face silkscreen embeds.

## The parts

- **Print**: `faceplate.py` writes `crates/glow-up/glow-up.svg` and the
  relief's geometry, `crates/glow-up/art/panel/geometry.json`. Don't edit
  what it writes: change it and run `python3 faceplate.py` here.
- **Panel relief**: Beverley's `blender/panel.py` renders it (about
  1 min 40 s), with hex button-head screws (`"screw": "hex"`):

  ```sh
  cd ../beverley/blender
  /Applications/Blender.app/Contents/MacOS/Blender -b -P panel.py -- \
      --geometry ../../../crates/glow-up/art/panel/geometry.json \
      --out ../../../crates/glow-up/art/panel
  ```

- **Controls**: `blender/controls.py` models the knob and the key and
  renders them into `crates/glow-up/art/controls/` (about 40 s);
  `--inspect` renders 45° product shots instead.

  ```sh
  cd blender
  /Applications/Blender.app/Contents/MacOS/Blender -b -P controls.py -- \
      --out ../../../crates/glow-up/art/controls
  ```

Both use Beverley's `blender/studio.py`, so the families are lit alike.
They need Blender 5.2 (made with 5.2.2). After either, re-bless the
editor's snapshots (`SILKSCREEN_BLESS=1 cargo test -p glow-up`) and look
at them.
