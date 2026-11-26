//! Glow-Up's manual data: the faceplate as the editor draws it, with a
//! sound playing (`faces.rs`), and measurements taken by running the
//! engine (`measure.rs`). `crates/glow-up/manual/build.sh` runs it, then
//! typesets the PDF.
//!
//! ```sh
//! cargo run --release -p glow-up --example manual -- crates/glow-up/manual/data
//! ```

mod faces;
mod measure;

fn main() {
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --release -p glow-up --example manual -- OUT_DIR");
        std::process::exit(2);
    };
    let out = std::path::Path::new(&out);
    std::fs::create_dir_all(out).expect("create OUT_DIR");
    faces::write(out);
    measure::write(out);
}
