//! The state extension: save, change, load.

mod common;

use clack_extensions::state::PluginState;
use clack_host::prelude::*;
use common::{events, load, run, sine, value};
use silkscreen_clap::test_host::Host;

fn save(plugin: &mut PluginInstance<Host>) -> Vec<u8> {
    let mut handle = plugin.plugin_handle();
    let state = handle.get_extension::<PluginState>().unwrap();
    let mut bytes = Vec::new();
    state.save(&mut handle, &mut bytes).unwrap();
    bytes
}

fn restore(plugin: &mut PluginInstance<Host>, bytes: &[u8]) {
    let mut handle = plugin.plugin_handle();
    let state = handle.get_extension::<PluginState>().unwrap();
    state.load(&mut handle, &mut &bytes[..]).unwrap();
}

#[test]
fn a_saved_state_comes_back_with_its_sound() {
    let (_entry, mut plugin) = load();
    let x = sine(0.7, 0.01, 2400);
    let first: &[(u32, f64)] = &[
        (1, 6.0),
        (2, 1.0),
        (3, 0.4),
        (4, 5.5),
        (5, 30.0),
        (6, 1.0),
        (7, 80.0),
        (8, 1.0),
        (9, 400.0),
        (10, 0.0),
    ];
    run(&mut plugin, 48_000.0, &events(first), &x, &x);
    let saved = save(&mut plugin);
    let sound = run(&mut plugin, 48_000.0, &events(&[]), &x, &x);

    let second: Vec<(u32, f64)> = first.iter().map(|&(id, _)| (id, 1.0)).collect();
    run(&mut plugin, 48_000.0, &events(&second), &x, &x);
    assert_eq!(value(&mut plugin, 9), 1.0);

    restore(&mut plugin, &saved);
    for &(id, v) in first {
        assert_eq!(value(&mut plugin, id) as f32, v as f32, "param {id}");
    }
    // Loaded while inactive, the next activation starts at the state.
    assert_eq!(run(&mut plugin, 48_000.0, &events(&[]), &x, &x), sound);
}
