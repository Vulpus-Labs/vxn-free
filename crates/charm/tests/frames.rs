//! Charm's telemetry frame (ticket 0022): one per flock tick, read whole
//! by the editor while the engine runs.

use std::collections::HashMap;
use std::sync::mpsc;

use charm::engine::CharmEngine;
use charm::observe::{FRAME_WORDS, Observation};
use charm::{DETUNE, TURBULENCE, UI};
use silkscreen::bridge::Engine;
use silkscreen_bridge::bridge_with_frame;

/// White noise, repeatable.
fn noise(len: usize, mut seed: u32) -> Vec<f32> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect()
}

fn engine() -> CharmEngine {
    let mut e = CharmEngine::new(48_000.0);
    e.set_param(DETUNE, 40.0);
    e.set_param(TURBULENCE, 100.0);
    e.reset();
    e
}

#[test]
fn the_plugin_declares_room_for_its_frame() {
    assert_eq!(UI.frame, FRAME_WORDS);
}

#[test]
fn a_frame_is_published_once_per_tick_with_the_latest_observation() {
    let mut e = engine();
    let mut frames = Vec::new();
    let take = |e: &mut CharmEngine| {
        let mut got = None;
        e.frame(&mut |f| got = Some(f.to_vec()));
        got
    };
    let input = noise(256, 1);
    let (mut l, mut r) = (input.clone(), input.clone());
    e.process(&mut [&mut l[..100], &mut r[..100]]);
    frames.push(take(&mut e).expect("a tick ran"));
    assert!(take(&mut e).is_none(), "nothing new without a tick");
    // 100 samples are into the fourth 32-sample tick: 28 more finish it,
    // and a fifth starts with the next sample.
    e.process(&mut [&mut l[100..128], &mut r[100..128]]);
    assert!(take(&mut e).is_none(), "the same tick");
    e.process(&mut [&mut l[128..129], &mut r[128..129]]);
    frames.push(take(&mut e).expect("a new tick"));
    let ticks: Vec<u64> = frames
        .iter()
        .map(|f| Observation::from_frame(f).unwrap().tick)
        .collect();
    assert_eq!(ticks, [4, 5]);
    let latest = Observation::from_frame(&frames[1]).unwrap();
    let obs = e.observation();
    assert_eq!(latest.bands[0].voices, obs.bands[0].voices);
    assert_eq!(latest.bands[1].active, obs.bands[1].active);
}

/// The engine runs on its own thread, publishing through a real bridge as
/// a host does; the editor's end polls as fast as it can. Every frame it
/// takes must be one the engine published, whole: the frame the engine
/// encoded for that tick, word for word.
#[test]
fn frames_read_while_the_engine_runs_are_always_complete() {
    const BLOCKS: usize = 3000;
    const BLOCK: usize = 64;
    let (mut ui, mut end) = bridge_with_frame(&[], 16, UI.frame);
    let (sent, published) = mpsc::channel::<(u64, Vec<f32>)>();
    let audio = std::thread::spawn(move || {
        let mut e = engine();
        let input = noise(BLOCK * BLOCKS, 7);
        for block in input.chunks_exact(BLOCK) {
            let (mut l, mut r) = (block.to_vec(), block.to_vec());
            e.process(&mut [&mut l, &mut r]);
            end.telemetry().write_block(&mut e, &[&l, &r]);
            // What the frame must hold for this tick (encoded again here,
            // off the tap's path).
            let mut frame = [0.0; FRAME_WORDS];
            e.observation().to_frame(&mut frame);
            sent.send((e.observation().tick, frame.to_vec())).unwrap();
        }
    });
    let reader = silkscreen::EngineLink::telemetry(&mut ui).unwrap();
    let mut taken = Vec::new();
    while !audio.is_finished() {
        if reader.poll_frame() {
            taken.push(reader.frame().to_vec());
        }
    }
    audio.join().unwrap();
    if reader.poll_frame() {
        taken.push(reader.frame().to_vec());
    }
    let by_tick: HashMap<u64, Vec<f32>> = published.into_iter().collect();
    assert!(
        taken.len() > 10,
        "frames were read while it ran: {}",
        taken.len()
    );
    let mut last = 0;
    for frame in &taken {
        let obs = Observation::from_frame(frame).expect("a frame of FRAME_WORDS");
        let want = by_tick.get(&obs.tick).expect("a tick the engine published");
        // Bit for bit: NaN-free, but -0.0 and 0.0 must match too.
        let bits = |f: &[f32]| f.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        assert_eq!(
            bits(frame),
            bits(want),
            "frame for tick {} is torn",
            obs.tick
        );
        assert!(obs.tick > last, "frames go forward");
        last = obs.tick;
    }
    assert_eq!(last, (BLOCKS * BLOCK / 32) as u64, "the last tick, at rest");
}
