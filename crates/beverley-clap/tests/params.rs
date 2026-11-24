//! The params extension, through clack-host.

mod common;

use beverley::PARAMS;
use clack_extensions::params::{ParamInfoBuffer, PluginParams};
use clack_host::prelude::*;
use common::{events, load, run, sine, value};

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
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
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
        (1, 6.0, "6.0 dB"),
        (3, 0.0, "1.00"),
        (4, 8.0, "8.0 bits"),
        (5, 20.0, "20 %"),
        (6, 1.0, "Symmetric"),
        (8, 0.0, "Mono"),
        (9, 115.0, "115 ms"),
        (10, 0.0, "Peak"),
    ] {
        let got = params
            .value_to_text(&mut handle, ClapId::new(id), value, &mut text)
            .unwrap();
        assert_eq!(std::str::from_utf8(got).unwrap(), shown);
    }
    assert_eq!(
        params.text_to_value(&mut handle, ClapId::new(4), c"6 bits"),
        Some(6.0)
    );
}

#[test]
fn automation_reaches_audio_and_the_model() {
    let (_entry, mut plugin) = load();
    let x = sine(0.7, 0.01, 4800);
    let (before, _) = run(&mut plugin, 48_000.0, &events(&[]), &x, &x);
    assert_ne!(before[4000..], x[4000..]);
    // MIX to 0: it glides to the dry signal.
    let (after, _) = run(&mut plugin, 48_000.0, &events(&[(7, 0.0)]), &x, &x);
    assert!(
        after[4000..]
            .iter()
            .zip(&x[4000..])
            .all(|(a, b)| (a - b).abs() < 1e-3)
    );
    assert_eq!(value(&mut plugin, 7), 0.0);
    // The next activation starts there, with no glide.
    let (next, _) = run(&mut plugin, 48_000.0, &events(&[]), &x, &x);
    assert_eq!(next, x);
}
