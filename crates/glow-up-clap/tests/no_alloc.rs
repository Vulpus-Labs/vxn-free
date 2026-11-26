//! The plugin's process call must not allocate: host events, the bridge
//! and the engine, all through clack-host.

use clack_host::events::event_types::ParamValueEvent;
use clack_host::prelude::*;
use clack_host::utils::Cookie;
use glow_up_clap::GlowUpPlugin;
use silkscreen_bridge::testing::{CountingAllocator, allocations};
use silkscreen_clap::test_host::{self, HostAudio};

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator;

const FRAMES: usize = 256;

#[test]
fn process_never_allocates() {
    let (_entry, mut plugin) = test_host::load::<GlowUpPlugin>();
    let config = PluginAudioConfiguration {
        sample_rate: 48_000.0,
        min_frames_count: 1,
        max_frames_count: FRAMES as u32,
    };
    let processor = plugin.activate(|_, _| HostAudio, config).unwrap();
    let mut processor = processor.start_processing().unwrap();

    let mut in_bufs = [
        (0..FRAMES)
            .map(|i| (i as f32 * 0.07).sin())
            .collect::<Vec<_>>(),
        vec![0.3f32; FRAMES],
    ];
    let mut out_bufs = [vec![0f32; FRAMES], vec![0f32; FRAMES]];
    let mut in_ports = AudioPorts::with_capacity(2, 1);
    let mut out_ports = AudioPorts::with_capacity(2, 1);
    let mut input_events = EventBuffer::with_capacity(16);
    for (t, id, v) in [
        (0, 17, 1.0),
        (10, 9, 60.0),
        (64, 8, 80.0),
        (100, 11, 0.0),
        (150, 14, 0.0),
        (200, 1, 80.0),
    ] {
        input_events.push(&ParamValueEvent::new(
            t,
            ClapId::new(id),
            Pckn::match_all(),
            v,
            Cookie::empty(),
        ));
    }
    let mut output_events = EventBuffer::with_capacity(16);
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

    // Warm up once (host-side buffers may grow on first use), then count.
    let input = input_events.as_input();
    processor
        .process(
            &ins,
            &mut outs,
            &input,
            &mut output_events.as_output(),
            None,
            None,
        )
        .unwrap();
    output_events.clear();
    let n = allocations(|| {
        for _ in 0..8 {
            processor
                .process(
                    &ins,
                    &mut outs,
                    &input,
                    &mut output_events.as_output(),
                    None,
                    None,
                )
                .unwrap();
        }
    });
    assert_eq!(n, 0);
}
