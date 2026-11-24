//! The faceplate for the manual, as the editor draws it at 2×: at rest,
//! and with AUTO on (which lights DETECT and DECAY and dims GAIN). Beside
//! them, `positions.json`: every control's box, for the numbered badges.

use std::fmt::Write as _;
use std::path::Path;

use beverley::faceplate::{CONTROLS, SIZE, ids};
use silkscreen_harness::Harness;

const SCALE: f32 = 2.0;

pub fn write(out: &Path) {
    let faces = out.join("faces");
    std::fs::create_dir_all(&faces).expect("create faces/");
    let mut h = Harness::new(beverley::editor, SCALE);
    h.save_png(faces.join("init.png"));
    h.click(ids::AUTO_GAIN);
    // Away from every control, so no hover or tooltip shows.
    h.move_to(silkscreen::core::Point::new(1.0, 1.0))
        .advance(50.0);
    h.save_png(faces.join("auto.png"));
    positions(out);
}

/// `positions.json`: the panel's size and each control's box, in panel
/// units, keyed by the control's name in the SVG.
fn positions(out: &Path) {
    let mut json = String::new();
    let _ = write!(
        json,
        "{{\n  \"panelWidthPx\": {},\n  \"panelHeightPx\": {},\n  \"controls\": {{",
        SIZE.w, SIZE.h
    );
    let rows: Vec<String> = CONTROLS
        .iter()
        .map(|c| {
            format!(
                "\n    \"{}\": {{\"left\": {}, \"top\": {}, \"width\": {}, \"height\": {}}}",
                c.name, c.rect.x, c.rect.y, c.rect.w, c.rect.h
            )
        })
        .collect();
    json.push_str(&rows.join(","));
    json.push_str("\n  }\n}\n");
    std::fs::write(out.join("positions.json"), json).expect("write positions.json");
}
