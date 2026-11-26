//! Running Glow-Up through silkscreen-clap's test host.

#![allow(dead_code)]

use clack_extensions::params::PluginParams;
use clack_host::events::event_types::ParamValueEvent;
use clack_host::prelude::*;
use clack_host::utils::Cookie;
use glow_up_clap::GlowUpPlugin;
use silkscreen_clap::test_host::{self, Host, HostAudio};

pub fn load() -> (PluginEntry, PluginInstance<Host>) {
    test_host::load::<GlowUpPlugin>()
}

/// A param change at sample `time`; `id` is the CLAP id (the spec's `id`).
pub fn param(time: u32, id: u32, value: f64) -> ParamValueEvent {
    ParamValueEvent::new(
        time,
        ClapId::new(id),
        Pckn::match_all(),
        value,
        Cookie::empty(),
    )
}

pub fn events(changes: &[(u32, f64)]) -> EventBuffer {
    let mut buf = EventBuffer::new();
    for &(id, value) in changes {
        buf.push(&param(0, id, value));
    }
    buf
}

/// What the host reads for param `id`.
pub fn value(plugin: &mut PluginInstance<Host>, id: u32) -> f64 {
    let mut handle = plugin.plugin_handle();
    let params = handle.get_extension::<PluginParams>().unwrap();
    params.get_value(&mut handle, ClapId::new(id)).unwrap()
}

/// Activate at `rate`, process `left` and `right` as one block with
/// `events`, deactivate: the two output channels.
pub fn run(
    plugin: &mut PluginInstance<Host>,
    rate: f64,
    events: &EventBuffer,
    left: &[f32],
    right: &[f32],
) -> (Vec<f32>, Vec<f32>) {
    let frames = left.len();
    let config = PluginAudioConfiguration {
        sample_rate: rate,
        min_frames_count: 1,
        max_frames_count: frames as u32,
    };
    let processor = plugin.activate(|_, _| HostAudio, config).unwrap();
    let mut processor = processor.start_processing().unwrap();
    let mut in_bufs = [left.to_vec(), right.to_vec()];
    let mut out_bufs = [vec![0f32; frames], vec![0f32; frames]];
    let mut in_ports = AudioPorts::with_capacity(2, 1);
    let mut out_ports = AudioPorts::with_capacity(2, 1);
    let ins = in_ports.with_input_buffers([AudioPortBuffer {
        channels: AudioPortBufferType::f32_input_only(
            in_bufs.iter_mut().map(InputChannel::variable),
        ),
        latency: 0,
    }]);
    let mut outs = out_ports.with_output_buffers([AudioPortBuffer {
        channels: AudioPortBufferType::f32_output_only(out_bufs.iter_mut().map(Vec::as_mut_slice)),
        latency: 0,
    }]);
    let mut output = EventBuffer::new();
    processor
        .process(
            &ins,
            &mut outs,
            &events.as_input(),
            &mut output.as_output(),
            None,
            None,
        )
        .unwrap();
    plugin.deactivate(processor.stop_processing());
    let [l, r] = out_bufs;
    (l, r)
}

pub fn sine(amp: f32, cycles_per_sample: f32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| amp * (std::f32::consts::TAU * cycles_per_sample * i as f32).sin())
        .collect()
}
