//! The params extension, through clack-host.

mod common;

use clack_extensions::params::{ParamInfoBuffer, PluginParams};
use clack_host::prelude::*;
use common::{events, load, run, sine, value};
use vxn_free_dsp::ThreeBand;
use wesley::PARAMS;

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
        (1..=36).collect::<Vec<u32>>(),
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
        (14, 6.5, "6.5 bits"),
        (18, 3.0, "3.0 dB"),
        (31, 200.0, "200 Hz"),
        (32, 2500.0, "2500 Hz"),
        (35, 300.0, "300 ms"),
        (36, 50.0, "50 %"),
    ] {
        let got = params
            .value_to_text(&mut handle, ClapId::new(id), value, &mut text)
            .unwrap();
        assert_eq!(std::str::from_utf8(got).unwrap(), shown, "param {id}");
    }
    assert_eq!(
        params.text_to_value(&mut handle, ClapId::new(31), c"300 Hz"),
        Some(300.0)
    );
}

#[test]
fn automation_reaches_audio_and_the_model() {
    let (_entry, mut plugin) = load();
    let x = sine(0.5, 0.02, 9600);
    let mut split = ThreeBand::new(200.0, 2500.0, 48_000.0);
    let clean: Vec<f32> = x.iter().map(|&s| split.split(s).iter().sum()).collect();
    // At the defaults MID and HIGH crush the sine (960 Hz, in MID).
    let (before, _) = run(&mut plugin, 48_000.0, &events(&[]), &x, &x);
    let far = |y: &[f32]| {
        y[8000..]
            .iter()
            .zip(&clean[8000..])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max)
    };
    assert!(far(&before) > 0.01);
    // MID and HIGH off: the split passes clean, after their glide.
    let (after, _) = run(
        &mut plugin,
        48_000.0,
        &events(&[(19, 0.0), (29, 0.0)]),
        &x,
        &x,
    );
    assert!(far(&after) < 1e-5, "{}", far(&after));
    assert_eq!(value(&mut plugin, 19), 0.0);
}
