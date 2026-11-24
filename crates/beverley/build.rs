//! Renders `beverley.svg` to PNGs at each scale and generates the
//! `faceplate` module that `src/lib.rs` includes.
//!
//! The SVG and the art it uses come from `art/beverley/`: `faceplate.py`
//! writes the SVG, and Blender renders the panel's relief maps
//! (`art/panel/`) and the controls' skin frames (`art/controls/`).

/// The knob's grip repeats every flute: 24 of them.
const FLUTE_PERIOD: f32 = 15.0;
/// Knob body frames across one flute period.
const KNOB_STEPS: usize = 6;

fn main() {
    // The panel maps the SVG places: build.rs must name linked images.
    println!("cargo::rerun-if-changed=art/panel/shade.png");
    println!("cargo::rerun-if-changed=art/panel/sheen.png");

    let knob: Vec<String> = std::iter::once("knob_pointer".to_owned())
        .chain((0..KNOB_STEPS).map(|k| format!("knob_body_{k}")))
        .map(|n| frame(&n))
        .collect();
    let toggle2 = frames(&["toggle2_0", "toggle2_1"]);
    let toggle3 = frames(&["toggle3_0", "toggle3_1", "toggle3_2"]);
    let button = frames(&["button_0", "button_1"]);

    silkscreen_build::Faceplate::new("beverley.svg")
        // The legends are Jost, the face the editor's own text uses.
        .font_dir(silkscreen_build::TEXT_FONT_DIR)
        // A cycled knob: the pointer turns exactly, the fluted body
        // repeats every flute.
        .skin("knob", &refs(&knob))
        .skin_cycle("knob", FLUTE_PERIOD)
        .skin("toggle2", &refs(&toggle2))
        .skin("toggle3", &refs(&toggle3))
        .skin("button", &refs(&button))
        .build();
}

fn frame(name: &str) -> String {
    format!("art/controls/{name}.png")
}

fn frames(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| frame(n)).collect()
}

fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}
