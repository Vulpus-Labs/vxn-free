//! The flock display (ticket 0019), driven by telemetry frames.
//!
//! Snapshots use frames built here by hand, not rendered by the engine, so
//! they don't move with the engine's rounding on each platform. What the
//! engine puts in a frame is checked separately, by property.

use charm::engine::{CharmEngine, PAN_GUARD};
use charm::faceplate::ids;
use charm::flock::{FLOOR, max_velocity};
use charm::observe::{BandObs, FRAME_WORDS, HIGH, MID, Observation, VoiceObs};
use charm::{AUTO_WIDTH, BLOOM, DUCK, WIDTH};
use silkscreen::ParamIndex;
use silkscreen::bridge::{Engine, EngineSide};
use silkscreen_harness::{Harness, assert_snapshot};

const SPREAD: f32 = 0.010;
const DETUNE: f64 = 12.0;

/// A repeatable spread of numbers in 0.05–0.95; `axis` picks an
/// independent sequence.
fn scatter(n: usize, axis: usize) -> f32 {
    const STEPS: [f32; 3] = [0.618_034, 0.754_878, 0.569_840];
    ((n as f32 * STEPS[axis] + 0.13 * (axis + 1) as f32).fract() * 0.9) + 0.05
}

/// A frame of eight voices a band, placed by `voice(band, i)`.
fn frame(width: [f32; 2], voice: impl Fn(usize, usize) -> VoiceObs) -> Vec<f32> {
    let mut obs = Observation {
        tick: 100,
        spread: SPREAD,
        max_velocity: max_velocity(DETUNE) as f32,
        ..Observation::default()
    };
    for (b, band) in obs.bands.iter_mut().enumerate() {
        *band = BandObs {
            active: 8,
            width: width[b],
            coherence: 1.0,
            voices: std::array::from_fn(|i| {
                if i < 8 {
                    voice(b, i)
                } else {
                    VoiceObs::default()
                }
            }),
            ..BandObs::default()
        };
    }
    let mut out = [0.0; FRAME_WORDS];
    obs.to_frame(&mut out);
    out.to_vec()
}

/// A voice at `pan` (a share of `width`) and `height` (0 floor, 1 top),
/// moving at `speed` of the limit.
fn voice(pan: f32, width: f32, height: f32, speed: f32, shape: f32) -> VoiceObs {
    VoiceObs {
        delay: FLOOR as f32 + height * SPREAD,
        pan: (pan * width).clamp(-PAN_GUARD, PAN_GUARD),
        velocity: speed * max_velocity(DETUNE) as f32,
        gain: 1.0 / 8f32.sqrt(),
        shape,
    }
}

fn spread_voice(b: usize, i: usize, width: f32, shape: impl Fn(f32) -> f32) -> VoiceObs {
    let n = b * 8 + i;
    let height = scatter(n, 1);
    voice(
        2.0 * scatter(n, 0) - 1.0,
        width,
        height,
        2.0 * scatter(n, 2) - 1.0,
        shape(height),
    )
}

fn shown(frame: &[f32]) -> Harness {
    let mut h = Harness::new(charm::editor, 1.0);
    h.engine_frame(frame);
    h.advance(16.0);
    h
}

#[test]
fn a_sustained_tone_shows_the_flock_wandering() {
    let f = frame([0.7, 0.7], |b, i| spread_voice(b, i, 0.7, |_| 1.0));
    let mut h = shown(&f);
    assert_snapshot!(h, "display_sustained_1x");
}

#[test]
fn a_wide_input_closes_auto_widths_walls_in() {
    // Coherence near 0: each band gets a quarter of WIDTH, or a little
    // more.
    let f = frame([0.2, 0.26], |b, i| {
        spread_voice(b, i, [0.2, 0.26][b], |_| 1.0)
    });
    let mut h = shown(&f);
    assert_snapshot!(h, "display_wide_walls_1x");
}

#[test]
fn a_ducked_onset_shows_as_a_pulse_climbing_the_flock() {
    // The onset has reached voices up to 40 % of SPREAD: they are ducked
    // to 30 %; those above haven't heard it yet.
    let f = frame([0.7, 0.7], |b, i| {
        spread_voice(b, i, 0.7, |h| if h < 0.4 { 0.3 } else { 1.0 })
    });
    let mut h = shown(&f);
    assert_snapshot!(h, "display_ducked_1x");
}

#[test]
fn an_out_of_date_frame_is_not_drawn() {
    let empty = Harness::new(charm::editor, 1.0).snapshot().clone();
    let f = frame([0.7, 0.7], |b, i| spread_voice(b, i, 0.7, |_| 1.0));
    let mut h = shown(&f);
    assert_ne!(*h.snapshot(), empty, "a fresh frame is drawn");
    h.advance(charm::display::STALE_MS);
    assert!(h.editor().needs_render(), "going stale repaints");
    assert_eq!(*h.snapshot(), empty, "a stale frame is not");
    // A new frame brings it back.
    h.engine_frame(&f).advance(16.0);
    assert_ne!(*h.snapshot(), empty);
}

#[test]
fn a_frame_of_the_wrong_size_is_not_drawn() {
    let empty = Harness::new(charm::editor, 1.0).snapshot().clone();
    let mut h = shown(&[1.0; 7]);
    assert_eq!(*h.snapshot(), empty);
}

#[test]
fn only_the_display_repaints_for_a_frame() {
    let f = frame([0.7, 0.7], |b, i| spread_voice(b, i, 0.7, |_| 1.0));
    let mut h = shown(&f);
    h.snapshot();
    let g = frame([0.7, 0.7], |b, i| spread_voice(b, i + 1, 0.7, |_| 1.0));
    h.engine_frame(&g).advance(16.0);
    h.snapshot();
    let display = charm::faceplate::LAYOUT
        .control(ids::FLOCK)
        .rect
        .to_physical(1.0);
    assert!(
        h.last_damage()
            .iter()
            .all(|d| display.intersect(d) == Some(*d))
    );
    assert!(!h.last_damage().is_empty());
}

// What the engine puts in its frames.

fn noise(len: usize, mut seed: u32) -> Vec<f32> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect()
}

fn run(params: &[(ParamIndex, f32)], left: &[f32], right: &[f32]) -> Vec<Observation> {
    let mut e = CharmEngine::new(48_000.0);
    for &(p, v) in params {
        e.set_param(p, v);
    }
    e.reset();
    let mut out = Vec::new();
    for (l, r) in left.chunks(64).zip(right.chunks(64)) {
        let (mut l, mut r) = (l.to_vec(), r.to_vec());
        e.process(&mut [&mut l, &mut r]);
        e.frame(&mut |f| out.push(Observation::from_frame(f).unwrap()));
    }
    out
}

#[test]
fn the_engines_frames_place_every_voice_inside_the_plot() {
    let x = noise(48_000, 3);
    for obs in run(&[(DUCK, 0.0)], &x, &x) {
        assert!((obs.spread - 0.010).abs() < 1e-6, "SPREAD's default");
        assert!(obs.max_velocity > 0.0);
        for band in &obs.bands {
            for v in &band.voices[..usize::from(band.active)] {
                let up = (v.delay - FLOOR as f32) / obs.spread;
                assert!((-1e-3..=1.001).contains(&up), "{up}");
                assert!(v.pan.abs() <= band.width.min(PAN_GUARD) + 1e-6);
                assert!(v.velocity.abs() <= obs.max_velocity * 1.5);
                assert!((v.shape - 1.0).abs() < 1e-6, "DUCK 0: {}", v.shape);
            }
        }
    }
}

#[test]
fn wide_input_closes_the_walls_and_mono_input_opens_them() {
    let (l, r) = (noise(96_000, 5), noise(96_000, 9));
    let params = [(WIDTH, 80.0), (AUTO_WIDTH, 1.0), (DUCK, 0.0)];
    let wide = run(&params, &l, &r);
    let mono = run(&params, &l, &l);
    let last = |o: &[Observation], b: usize| o.last().unwrap().bands[b].width;
    for b in [MID, HIGH] {
        assert!(
            last(&wide, b) < 0.8 * 0.4,
            "wide band {b}: {}",
            last(&wide, b)
        );
        assert!(
            last(&mono, b) > 0.8 * 0.95,
            "mono band {b}: {}",
            last(&mono, b)
        );
    }
    let off = run(&[(WIDTH, 80.0), (AUTO_WIDTH, 0.0)], &l, &r);
    assert!(
        (last(&off, MID) - 0.8).abs() < 1e-3,
        "AUTO WIDTH off: WIDTH"
    );
}

#[test]
fn an_onset_ducks_the_voices_it_reaches_first() {
    // Silence, then a burst: the onset reaches the nearest voices first.
    let mut x = vec![0.0; 24_000];
    x.extend(noise(24_000, 11).iter().map(|s| s * 1.5));
    let obs = run(&[(DUCK, 100.0), (BLOOM, 0.0)], &x, &x);
    let ducked = obs
        .iter()
        .flat_map(|o| o.bands.iter())
        .flat_map(|b| &b.voices[..usize::from(b.active)])
        .map(|v| v.shape)
        .fold(1.0_f32, f32::min);
    assert!(ducked < 0.5, "the deepest duck seen: {ducked}");
}

/// The plugin's path: the editor over a real bridge sized by `UI.frame`,
/// the engine publishing through `write_block` as the CLAP processor does.
#[test]
fn the_editor_over_a_real_bridge_draws_the_engines_flock() {
    let defaults: Vec<f32> = charm::PARAMS.iter().map(|p| p.default).collect();
    let (ui, mut end) = silkscreen_bridge::bridge_with_frame(&defaults, 1024, charm::UI.frame);
    let mut harness = Harness::with_link(charm::editor, Box::new(ui), 1.0);
    let empty = harness.snapshot().clone();
    let mut engine = CharmEngine::new(48_000.0);
    end.seed_engine(&mut engine);
    let input = noise(4800, 21);
    for block in input.chunks(480) {
        let (mut left, mut right) = (block.to_vec(), block.to_vec());
        engine.process(&mut [&mut left, &mut right]);
        end.telemetry().write_block(&mut engine, &[&left, &right]);
        harness.advance(10.0);
    }
    let (frame, generation) = harness.editor().state().engine_frame();
    assert_eq!(frame.len(), FRAME_WORDS, "generation {generation}");
    assert_ne!(*harness.snapshot(), empty, "the flock is drawn");
}
