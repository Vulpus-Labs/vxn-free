//! The params extension, through clack-host.

mod common;

use clack_extensions::params::{ParamInfoBuffer, PluginParams};
use clack_host::prelude::*;
use common::{events, load, run, sine, value};
use glow_up::PARAMS;

#[test]
fn every_param_is_listed_with_its_id_name_and_range() {
    let (_entry, mut plugin) = load();
    let mut handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    assert_eq!(params.count(&mut handle) as usize, PARAMS.len());
    let mut buf = ParamInfoBuffer::new();
    let mut ids = Vec::new();
    for (i, spec) in PARAMS.iter().enumerate() {
        let info = params.get_info(&mut handle, i as u32, &mut buf).unwrap();
        assert_eq!(info.id, ClapId::new(spec.id), "{}", spec.name);
        assert_eq!(info.name, spec.name.as_bytes());
        assert_eq!(
            (info.min_value, info.max_value, info.default_value),
            (
                f64::from(spec.min),
                f64::from(spec.max),
                f64::from(spec.default)
            ),
            "{}",
            spec.name
        );
        ids.push(spec.id);
    }
    assert_eq!(
        ids,
        (1..=17).collect::<Vec<u32>>(),
        "ids never change or move"
    );
}

#[test]
fn values_read_in_their_units() {
    let (_entry, mut plugin) = load();
    let mut handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    let mut text = [0u8; 64];
    for (id, value, shown) in [
        (1, 25.0, "25 %"),
        (2, 1.0, "2"),
        (6, 0.0, "1"),
        (7, 50.0, "50 cents"),
        (11, 2.0, "2048"),
        (12, 6.0, "6.0 dB"),
        (14, 1.0, "IRR"),
        (17, 1.0, "Stereo"),
    ] {
        let got = params
            .value_to_text(&mut handle, ClapId::new(id), value, &mut text)
            .unwrap();
        assert_eq!(std::str::from_utf8(got).unwrap(), shown, "param {id}");
    }
}

#[test]
fn automation_reaches_audio_and_the_model() {
    let (_entry, mut plugin) = load();
    // A sine at 440 Hz and its octave, quieter.
    let n = 48_000;
    let a = sine(0.4, 440.0 / 48_000.0, n);
    let b = sine(0.05, 880.0 / 48_000.0, n);
    let x: Vec<f32> = a.iter().zip(&b).map(|(p, q)| p + q).collect();
    // Both amounts off: the input passes untouched.
    let (dry, _) = run(
        &mut plugin,
        48_000.0,
        &events(&[(1, 0.0), (4, 0.0)]),
        &x,
        &x,
    );
    assert_eq!(dry, x);
    assert_eq!(value(&mut plugin, 1), 0.0);
    // A at full: the octave is boosted.
    let (wet, _) = run(&mut plugin, 48_000.0, &events(&[(1, 100.0)]), &x, &x);
    let tail = |y: &[f32]| y[24_000..].iter().map(|s| s * s).sum::<f32>();
    assert!(tail(&wet) > 1.05 * tail(&x), "{} {}", tail(&wet), tail(&x));
}
