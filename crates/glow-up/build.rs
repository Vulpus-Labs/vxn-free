//! Renders `glow-up.svg` to PNGs at each scale and generates the
//! `faceplate` module that `src/lib.rs` includes.
//!
//! The SVG and the art it uses come from `art/glow-up/`: `faceplate.py`
//! writes the SVG, Beverley's `blender/panel.py` renders the panel's relief
//! maps (`art/panel/`), and `blender/controls.py` the controls' skin frames
//! (`art/controls/`).

/// The knob's knurl repeats every ridge: 60 of them.
const KNURL_PERIOD: f32 = 6.0;
/// Knob body frames across one knurl period.
const KNOB_STEPS: usize = 4;

fn main() {
    // The panel maps the SVG places: build.rs must name linked images.
    println!("cargo::rerun-if-changed=art/panel/shade.png");
    println!("cargo::rerun-if-changed=art/panel/sheen.png");

    let knob: Vec<String> = std::iter::once("knob_pointer".to_owned())
        .chain((0..KNOB_STEPS).map(|k| format!("knob_body_{k}")))
        .map(|n| frame(&n))
        .collect();
    let key = [frame("key_0"), frame("key_1")];

    silkscreen_build::Faceplate::new("glow-up.svg")
        // The legends are Jost, the face the editor's own text uses.
        .font_dir(silkscreen_build::TEXT_FONT_DIR)
        // A cycled knob: the pointer turns exactly, the knurled body
        // repeats every ridge.
        .skin("knob", &refs(&knob))
        .skin_cycle("knob", KNURL_PERIOD)
        // The same knob, smaller, for the trims: a skin renders at one
        // size.
        .skin("knob_small", &refs(&knob))
        .skin_cycle("knob_small", KNURL_PERIOD)
        .skin("key", &refs(&key))
        .build();
}

fn frame(name: &str) -> String {
    format!("art/controls/{name}.png")
}

fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}
