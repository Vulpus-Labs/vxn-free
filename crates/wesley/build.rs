//! Renders `wesley.svg` (the plain faceplate, ticket 0034) to PNGs at each
//! scale and generates the `faceplate` module that `src/lib.rs` includes.

fn main() {
    silkscreen_build::Faceplate::new("wesley.svg")
        // The legends are Jost, the face the editor's own text uses.
        .font_dir(silkscreen_build::TEXT_FONT_DIR)
        .build();
}
