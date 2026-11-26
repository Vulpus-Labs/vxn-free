//! The faceplate for the manual, as the editor draws it at 2×, with a
//! note playing so the scope shows something. Beside it,
//! `positions.json`: every control's box, for the numbered badges.

use std::f64::consts::TAU;
use std::fmt::Write as _;
use std::path::Path;

use glow_up::engine::GlowUpEngine;
use glow_up::faceplate::{CONTROLS, SIZE};
use silkscreen::ParamIndex;
use silkscreen::bridge::Engine;
use silkscreen_harness::Harness;

const SCALE: f32 = 2.0;
const RATE: f64 = 48_000.0;

pub fn write(out: &Path) {
    let faces = out.join("faces");
    std::fs::create_dir_all(&faces).expect("create faces/");
    playing(&[]).save_png(faces.join("init.png"));
    positions(out);
}

/// The editor after a second of a G3 saw (196 Hz) through an engine
/// with `values`, the scope fed a frame every 10 ms as a host would.
fn playing(values: &[(ParamIndex, f32)]) -> Harness {
    let mut e = GlowUpEngine::new(RATE as f32);
    for &(p, v) in values {
        e.set_param(p, v);
    }
    e.reset();
    let mut h = Harness::new(glow_up::editor, SCALE);
    for &(p, v) in values {
        h.host_change(p, v);
    }
    let step = (0.01 * RATE) as usize;
    let mut n = 0usize;
    for _ in 0..100 {
        let mut x: Vec<f32> = (n..n + step)
            .map(|i| {
                let t = i as f64 / RATE;
                (1..60)
                    .take_while(|&k| 196.0 * f64::from(k) < 16_000.0)
                    .map(|k| 0.3 / f64::from(k) * (TAU * 196.0 * f64::from(k) * t).sin())
                    .sum::<f64>() as f32
            })
            .collect();
        n += step;
        e.process(&mut [&mut x]);
        let mut frame = Vec::new();
        e.frame(&mut |f| frame = f.to_vec());
        h.engine_frame(&frame);
        h.advance(10.0);
    }
    // Away from every control, so no hover or tooltip shows.
    h.move_to(silkscreen::core::Point::new(1.0, 1.0));
    h
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
