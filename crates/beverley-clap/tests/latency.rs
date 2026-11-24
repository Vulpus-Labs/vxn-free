//! The latency the plugin reports is the latency it has: checked against
//! an impulse and a step through the whole plugin at each common rate.

mod common;

use beverley::engine::BeverleyEngine;
use clack_extensions::latency::PluginLatency;
use clack_host::prelude::*;
use common::{events, load, run};
use silkscreen_clap::test_host::{Host, HostAudio};

/// The latency the plugin reports while active at `rate`.
fn reported(plugin: &mut PluginInstance<Host>, rate: f64) -> u32 {
    let config = PluginAudioConfiguration {
        sample_rate: rate,
        min_frames_count: 1,
        max_frames_count: 512,
    };
    let processor = plugin.activate(|_, _| HostAudio, config).unwrap();
    let mut handle = plugin.plugin_handle();
    let latency = handle.get_extension::<PluginLatency>().unwrap();
    let samples = latency.get(&mut handle);
    plugin.deactivate(processor);
    samples
}

#[test]
fn the_reported_latency_is_the_measured_one() {
    const AT: usize = 100;
    let settings: &[&[(u32, f64)]] = &[
        &[],
        &[(2, 1.0), (8, 1.0)],
        &[(4, 1.0), (6, 1.0), (8, 0.0)],
        &[(4, 12.5), (5, 60.0), (7, 50.0)],
        &[(7, 0.0)],
    ];
    for rate in [44_100.0, 48_000.0, 96_000.0] {
        let (_entry, mut plugin) = load();
        let latency = reported(&mut plugin, rate);
        assert_eq!(latency, BeverleyEngine::LATENCY, "at {rate} Hz");
        for &setting in settings {
            for input in ["impulse", "step"] {
                let mut x = vec![0.0f32; 400];
                match input {
                    "impulse" => x[AT] = 0.8,
                    _ => x[AT..].fill(0.8),
                }
                let (l, r) = run(&mut plugin, rate, &events(setting), &x, &x);
                for y in [l, r] {
                    let first = y.iter().position(|&s| s != 0.0);
                    assert_eq!(
                        first,
                        Some(AT + latency as usize),
                        "{input} at {rate} Hz, {setting:?}"
                    );
                }
            }
        }
    }
}
