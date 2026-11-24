//! Renders `charm.svg` (the plain faceplate, ticket 0020) to PNGs at each
//! scale and generates the `faceplate` module that `src/lib.rs` includes.

fn main() {
    silkscreen_build::Faceplate::new("charm.svg")
        // The legends are Jost, the face the editor's own text uses.
        .font_dir(silkscreen_build::TEXT_FONT_DIR)
        .build();
}
