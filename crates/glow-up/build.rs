//! Renders `glow-up.svg` (the plain faceplate, ticket 0010) to PNGs at
//! each scale and generates the `faceplate` module that `src/lib.rs`
//! includes.

fn main() {
    silkscreen_build::Faceplate::new("glow-up.svg")
        // The legends are Jost, the face the editor's own text uses.
        .font_dir(silkscreen_build::TEXT_FONT_DIR)
        .build();
}
