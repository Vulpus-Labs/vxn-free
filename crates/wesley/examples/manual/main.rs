//! Wesley's manual data: the faceplate as the editor draws it
//! (`faces.rs`) and measurements taken by running the engine
//! (`measure.rs`). `crates/wesley/manual/build.sh` runs it, then
//! typesets the PDF.
//!
//! ```sh
//! cargo run --release -p wesley --example manual -- crates/wesley/manual/data
//! ```

mod faces;
mod measure;

fn main() {
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --release -p wesley --example manual -- OUT_DIR");
        std::process::exit(2);
    };
    let out = std::path::Path::new(&out);
    std::fs::create_dir_all(out).expect("create OUT_DIR");
    faces::write(out);
    measure::write(out);
}
