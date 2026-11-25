//! A flock of chorus voices (ADR 0002, point 3).
//!
//! Each voice is a point in (delay, pan). Its velocity on the delay axis is
//! its pitch shift: a voice whose delay grows at d′ seconds per second
//! plays at ratio 1 − d′. DETUNE is therefore a speed limit, and the flock
//! never moves a voice faster than it.
//!
//! The voices steer by the boids rules, worked out in a unit square (delay
//! from the floor to the top of SPREAD, pan from left to right):
//!
//! - cohesion pulls each voice towards the flock's centre;
//! - separation pushes apart voices that come close (two voices at one
//!   delay double up and comb);
//! - alignment matches each voice's velocity to the flock's;
//! - turbulence is a smoothed random force per voice, so the motion never
//!   settles and has no period;
//! - soft walls turn voices back before the edges.
//!
//! Every force is scaled by the speed limit squared, so changing DETUNE
//! plays the same motion faster or slower rather than changing its shape.
//! The flock steps once per control tick; the engine slides each voice's
//! delay linearly across the tick's samples, so within a tick the delay's
//! rate of change is the voice's velocity exactly.

use crate::MAX_VOICES;

/// The shortest delay any voice has, seconds: longer than the analysis's
/// lag plus a sample for the interpolator, at every rate, so the analysis
/// has seen each sample before a voice plays it (ADR 0002, point 6). The
/// engine checks it when built.
pub const FLOOR: f64 = 0.0075;

/// Cohesion's pull at 100 %.
const COHESION: f64 = 4.0;
/// Separation's push at 100 %, at zero distance.
const SEPARATION: f64 = 80.0;
/// Separation acts within this distance in the unit square, for two
/// voices; it shrinks as 1/√N so a full flock still fits.
const SEPARATION_REACH: f64 = 0.5;
/// Alignment's pull towards the flock's mean velocity.
const ALIGNMENT: f64 = 1.0;
/// Drag on each voice's velocity.
const DRAG: f64 = 0.5;
/// Turbulence's push at 100 %.
const TURBULENCE: f64 = 8.0;
/// Turbulence's noise changes over this share of the time a voice takes
/// to cross the square at full speed.
const NOISE_TIME: f64 = 0.25;
/// Walls push back within this distance of an edge: far enough in that a
/// voice at full speed can turn round within the acceleration limit.
const WALL_MARGIN: f64 = 0.3;
/// The walls' push at the edge.
const WALL: f64 = 30.0;
/// The steering is smoothed over this share of the time a voice takes to
/// cross the square at full speed, so pitch eases in and out of its
/// changes rather than jerking (a voice's acceleration is how fast its
/// pitch changes).
const ACCEL_TIME: f64 = 0.15;
/// The most a voice may accelerate along its delay, as a share of the
/// speed limit squared: at 2, a voice goes from still to full speed in half
/// a crossing time.
const MAX_ACCEL: f64 = 2.0;
/// The same for pan, which carries no pitch: crowding is resolved mostly
/// across the stereo field, quickly, rather than in pitch.
const MAX_PAN_ACCEL: f64 = 10.0;

/// The fastest a voice pans, in widths per second, whatever the delay
/// axis's speed limit: with a narrow SPREAD that limit is fast in the unit
/// square, and panning that fast would be heard as tremolo.
const PAN_SPEED: f64 = 0.5;

/// What a flock steps by. Cohesion, separation and turbulence are 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlockSettings {
    /// Voices in use, 1 to [`MAX_VOICES`].
    pub voices: usize,
    /// The speed limit on the delay axis, seconds per second: DETUNE's
    /// 1 − 2^(−cents/1200).
    pub max_velocity: f64,
    /// Delays run from [`FLOOR`] to `FLOOR + spread`, seconds.
    pub spread: f64,
    pub cohesion: f64,
    pub separation: f64,
    pub turbulence: f64,
}

/// The speed limit for `cents` of detune. A voice slowing its delay at
/// this rate plays `cents` flat; one shortening it plays slightly less
/// than `cents` sharp. Neither exceeds `cents`.
pub fn max_velocity(cents: f64) -> f64 {
    1.0 - (-cents / 1200.0).exp2()
}

/// One voice.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Voice {
    /// Seconds.
    pub delay: f64,
    /// −1 to 1, before WIDTH.
    pub pan: f64,
    /// Seconds per second.
    pub delay_velocity: f64,
    /// Pan units per second.
    pub pan_velocity: f64,
    noise: [f64; 2],
    /// The smoothed steering, (delay, pan), in the unit square.
    accel: [f64; 2],
}

/// A flock of up to [`MAX_VOICES`] voices.
#[derive(Clone, Debug)]
pub struct Flock {
    voices: [Voice; MAX_VOICES],
    /// Velocities in the unit square, (delay, pan).
    unit_velocity: [[f64; 2]; MAX_VOICES],
    seed: u32,
    rng: u32,
}

/// A step of xorshift32: uniform on −1..1.
fn uniform(state: &mut u32) -> f64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    f64::from(x >> 8) * (2.0 / f64::from(1u32 << 24)) - 1.0
}

/// The walls' push at `at` (0 to 1) along one axis, before scaling.
fn wall(at: f64) -> f64 {
    if at < WALL_MARGIN {
        WALL * (WALL_MARGIN - at) / WALL_MARGIN
    } else if at > 1.0 - WALL_MARGIN {
        -WALL * (at - (1.0 - WALL_MARGIN)) / WALL_MARGIN
    } else {
        0.0
    }
}

/// Smooth and limit one voice's steering `a`, in place: on the delay
/// axis smoothed (by `keep` a step) so pitch glides and never jerks, and
/// limited; pan, which carries no pitch, only limited. Then, on each axis,
/// once a voice moving towards an edge has it within the stopping
/// distance at the limit, brake at the limit: the hard stop at the edge
/// would otherwise snap the pitch.
fn steer(
    smoothed: &mut [f64; 2],
    a: &mut [f64; 2],
    velocity: [f64; 2],
    at: [f64; 2],
    accel: f64,
    keep: f64,
    dt: f64,
) {
    for k in 0..2 {
        let most = accel * if k == 0 { MAX_ACCEL } else { MAX_PAN_ACCEL };
        smoothed[k] = if k == 0 {
            keep * smoothed[k] + (1.0 - keep) * a[k]
        } else {
            a[k]
        };
        a[k] = smoothed[k].clamp(-most, most);
        let speed = velocity[k];
        let room = if speed > 0.0 { 1.0 - at[k] } else { at[k] };
        // Allow for one more step at the limit towards the edge before the
        // brake bites: crowding can push a voice that hard (seen with 64
        // voices in 1 ms of SPREAD).
        let ahead = speed.abs() + most * dt;
        let stopping = ahead * ahead / (2.0 * most) + 2.0 * ahead * dt;
        // Only a voice moving towards an edge brakes. One at rest on the
        // floor has no room and no stopping distance either, and braking
        // it (signum(0.0) is 1, so downwards) pinned it there for good.
        if speed != 0.0 && stopping >= room {
            a[k] = -speed.signum() * most;
            smoothed[k] = a[k];
        }
    }
}

impl Flock {
    /// A flock at rest in its starting positions, its turbulence seeded
    /// with `seed` (non-zero).
    pub fn new(seed: u32, spread: f64) -> Self {
        let mut flock = Self {
            voices: [Voice::default(); MAX_VOICES],
            unit_velocity: [[0.0; 2]; MAX_VOICES],
            seed: seed.max(1),
            rng: seed.max(1),
        };
        flock.reset(spread);
        flock
    }

    /// Back to the starting positions, at rest, with the turbulence
    /// reseeded: the same settings then give the same motion.
    pub fn reset(&mut self, spread: f64) {
        self.rng = self.seed;
        for (i, v) in self.voices.iter_mut().enumerate() {
            // Golden-ratio steps: however many voices are in use, the
            // first few already cover the range.
            let at = 0.05 + 0.9 * (0.5 + 0.618_034 * i as f64).fract();
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            *v = Voice {
                // Spread up the delay range, alternating sides.
                delay: FLOOR + spread * at,
                pan: side * (0.2 + 0.6 * at),
                ..Voice::default()
            };
        }
        self.unit_velocity = [[0.0; 2]; MAX_VOICES];
    }

    /// Every voice, in use or not.
    pub fn voices(&self) -> &[Voice; MAX_VOICES] {
        &self.voices
    }

    /// Advance by `dt` seconds.
    // Vector arithmetic over two axes and a handful of voices reads most
    // plainly with indices and the usual short names.
    #[allow(
        clippy::many_single_char_names,
        clippy::similar_names,
        clippy::needless_range_loop
    )]
    pub fn step(&mut self, s: &FlockSettings, dt: f64) {
        let n = s.voices.clamp(1, MAX_VOICES);
        let spread = s.spread.max(1.0e-4);
        // The speed limit in the unit square, per second.
        let umax = s.max_velocity / spread;
        let accel = umax * umax;

        // Positions in the unit square.
        let mut q = [[0.0f64; 2]; MAX_VOICES];
        for (qi, v) in q.iter_mut().zip(&self.voices).take(n) {
            *qi = [(v.delay - FLOOR) / spread, 0.5 * (v.pan + 1.0)];
        }
        let inv_n = 1.0 / n as f64;
        let mut centre = [0.0f64; 2];
        let mut mean_v = [0.0f64; 2];
        for i in 0..n {
            for k in 0..2 {
                centre[k] += q[i][k] * inv_n;
                mean_v[k] += self.unit_velocity[i][k] * inv_n;
            }
        }
        let reach = SEPARATION_REACH / (n as f64).sqrt();
        let noise_keep = (-dt * umax / NOISE_TIME).exp();
        // One-pole smoothing narrows white noise by √((1 − k)/(1 + k));
        // undo that, so TURBULENCE means the same at any tick rate.
        let noise_norm = ((1.0 + noise_keep) / (1.0 - noise_keep).max(1.0e-9)).sqrt();

        for i in 0..n {
            let mut a = [0.0f64; 2];
            // Cohesion.
            for k in 0..2 {
                a[k] += accel * COHESION * s.cohesion * (centre[k] - q[i][k]);
            }
            // Separation.
            for j in 0..n {
                if j == i {
                    continue;
                }
                let dx = q[i][0] - q[j][0];
                let dy = q[i][1] - q[j][1];
                let dist2 = dx * dx + dy * dy;
                // Most pairs are out of reach: no root for them.
                if dist2 < reach * reach {
                    let dist = dist2.sqrt();
                    let push = accel * SEPARATION * s.separation * (1.0 - dist / reach);
                    if dist > 1.0e-6 {
                        a[0] += push * dx / dist;
                        a[1] += push * dy / dist;
                    } else {
                        // Coincident: part them along delay, by index.
                        a[0] += if i < j { -push } else { push };
                    }
                }
            }
            // Alignment and drag.
            for k in 0..2 {
                a[k] += umax
                    * (ALIGNMENT * (mean_v[k] - self.unit_velocity[i][k])
                        - DRAG * self.unit_velocity[i][k]);
            }
            // Turbulence: smoothed noise.
            let v = &mut self.voices[i];
            for k in 0..2 {
                let fresh = uniform(&mut self.rng);
                v.noise[k] = noise_keep * v.noise[k] + (1.0 - noise_keep) * fresh;
                a[k] += accel * TURBULENCE * s.turbulence * v.noise[k] * noise_norm;
            }
            // Walls.
            for (ak, push) in a.iter_mut().zip(q[i]) {
                *ak += accel * wall(push);
            }

            // Smooth and limit the steering, braking for edges in time.
            let keep = (-dt * umax / ACCEL_TIME).exp();
            steer(
                &mut v.accel,
                &mut a,
                self.unit_velocity[i],
                q[i],
                accel,
                keep,
                dt,
            );

            // Integrate, with the speed limit on each axis.
            let uv = &mut self.unit_velocity[i];
            uv[0] = (uv[0] + a[0] * dt).clamp(-umax, umax);
            let pmax = umax.min(PAN_SPEED);
            uv[1] = (uv[1] + a[1] * dt).clamp(-pmax, pmax);
            let vmax = s.max_velocity;
            let vd = (uv[0] * spread).clamp(-vmax, vmax);
            let (lo, hi) = (FLOOR, FLOOR + spread);
            let mut d = v.delay + vd * dt;
            if d > hi {
                // Back inside, but no faster than the limit allows.
                d = hi.max(v.delay - vmax * dt);
                uv[0] = uv[0].min(0.0);
            } else if d < lo {
                d = lo.min(v.delay + vmax * dt);
                uv[0] = uv[0].max(0.0);
            }
            v.delay_velocity = (d - v.delay) / dt;
            v.delay = d;

            let vp = 2.0 * uv[1];
            let mut p = v.pan + vp * dt;
            if p.abs() > 1.0 {
                p = p.clamp(-1.0, 1.0);
                uv[1] = 0.0;
            }
            v.pan_velocity = (p - v.pan) / dt;
            v.pan = p;
        }
        for (v, uv) in self.voices.iter_mut().zip(&mut self.unit_velocity).skip(n) {
            // Voices not in use hold still while they fade.
            v.delay_velocity = 0.0;
            v.pan_velocity = 0.0;
            *uv = [0.0; 2];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 32.0 / 48_000.0;

    fn settings(
        voices: usize,
        cents: f64,
        spread_ms: f64,
        c: f64,
        s: f64,
        t: f64,
    ) -> FlockSettings {
        FlockSettings {
            voices,
            max_velocity: max_velocity(cents),
            spread: spread_ms * 0.001,
            cohesion: c,
            separation: s,
            turbulence: t,
        }
    }

    #[test]
    fn the_speed_limit_is_detune() {
        // 1 − v flat at the limit is exactly `cents` down.
        let v = max_velocity(12.0);
        assert!((1200.0 * (1.0 - v).log2() + 12.0).abs() < 1.0e-3);
        // Shortening at the same rate is a little under `cents` up.
        assert!(1200.0 * (1.0 + v).log2() < 12.0);
    }

    #[test]
    fn no_voice_outruns_detune_or_leaves_its_range() {
        for (voices, cents, spread, c, s, t) in [
            (8, 40.0, 1.0, 1.0, 1.0, 1.0),
            (64, 40.0, 1.0, 1.0, 1.0, 1.0),
            (64, 12.0, 25.0, 0.5, 0.5, 0.4),
            (16, 40.0, 150.0, 0.5, 0.5, 1.0),
            (32, 40.0, 2000.0, 0.5, 0.5, 1.0),
            (8, 1.0, 25.0, 0.0, 1.0, 1.0),
            (2, 12.0, 10.0, 1.0, 0.0, 1.0),
            (5, 25.0, 3.0, 0.3, 0.7, 0.4),
        ] {
            let set = settings(voices, cents, spread, c, s, t);
            let mut flock = Flock::new(7, set.spread);
            for _ in 0..30_000 {
                flock.step(&set, DT);
                for v in &flock.voices()[..voices] {
                    assert!(
                        v.delay_velocity.abs() <= set.max_velocity * 1.000_01,
                        "{v:?}"
                    );
                    assert!(
                        v.delay >= FLOOR && v.delay <= FLOOR + set.spread + 1.0e-7,
                        "{v:?}"
                    );
                    assert!(v.pan.abs() <= 1.0);
                }
            }
        }
    }

    #[test]
    fn pitch_glides_and_never_jumps() {
        // A voice's velocity is its pitch; its acceleration is how fast its
        // pitch changes, which must stay within the limit, walls included.
        for (voices, cents, spread, c, s, t) in [
            (8, 40.0, 1.0, 1.0, 1.0, 1.0),
            (64, 40.0, 1.0, 1.0, 1.0, 1.0),
            (64, 12.0, 25.0, 0.5, 0.5, 0.4),
            (16, 40.0, 150.0, 0.5, 0.5, 1.0),
            (32, 40.0, 2000.0, 0.5, 0.5, 1.0),
            (4, 12.0, 10.0, 0.5, 0.5, 0.4),
            (2, 25.0, 3.0, 0.0, 1.0, 1.0),
        ] {
            let set = settings(voices, cents, spread, c, s, t);
            let mut flock = Flock::new(9, set.spread);
            let limit = MAX_ACCEL * set.max_velocity * set.max_velocity / set.spread;
            let mut last = [0.0f64; MAX_VOICES];
            for step in 0..30_000 {
                flock.step(&set, DT);
                for (v, l) in flock.voices()[..voices].iter().zip(&mut last) {
                    let change = (v.delay_velocity - *l).abs() / DT;
                    // The first steps start from rest at the start positions.
                    if step > 0 {
                        // At an edge, discrete braking can leave a sliver of
                        // speed for the hard stop to take: allow half again,
                        // far from the hundredfold jumps this guards against.
                        assert!(
                            change <= limit * 1.5,
                            "{cents} cents, {spread} ms: {change} > {limit} at step {step}"
                        );
                    }
                    *l = v.delay_velocity;
                }
            }
        }
    }

    #[test]
    fn it_keeps_moving_with_turbulence() {
        let set = settings(4, 12.0, 10.0, 0.5, 0.5, 0.4);
        let mut flock = Flock::new(3, set.spread);
        let mut sum = 0.0f64;
        let steps = 30_000;
        for _ in 0..steps {
            flock.step(&set, DT);
            sum += flock.voices()[0].delay_velocity.abs();
        }
        // On average at least a fifth of the speed limit.
        let mean = sum / f64::from(steps);
        assert!(mean > 0.2 * set.max_velocity, "{mean}");
    }

    #[test]
    fn a_voice_at_rest_on_an_edge_leaves_it() {
        // SPREAD swept up and down puts voices on the floor at rest; the
        // walls must lift them off (they once stayed there for good).
        let set = settings(4, 12.0, 10.0, 0.5, 0.5, 0.4);
        let mut flock = Flock::new(7, set.spread);
        for (i, v) in flock.voices.iter_mut().enumerate() {
            v.delay = FLOOR;
            v.pan = if i % 2 == 0 { -1.0 } else { 1.0 };
            v.accel = [0.0; 2];
        }
        flock.unit_velocity = [[0.0; 2]; MAX_VOICES];
        for _ in 0..(2.0 / DT) as usize {
            flock.step(&set, DT);
        }
        for v in &flock.voices()[..4] {
            let up = (v.delay - FLOOR) / set.spread;
            assert!(up > 0.05, "a voice still on the floor: {up}");
            assert!(v.pan.abs() < 0.99, "a voice still at the side: {}", v.pan);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_motion() {
        let set = settings(6, 20.0, 12.0, 0.5, 0.5, 0.8);
        let mut a = Flock::new(11, set.spread);
        let mut b = Flock::new(11, set.spread);
        for _ in 0..5000 {
            a.step(&set, DT);
            b.step(&set, DT);
        }
        assert_eq!(a.voices(), b.voices());
        b.reset(set.spread);
        let mut c = Flock::new(11, set.spread);
        for _ in 0..100 {
            b.step(&set, DT);
            c.step(&set, DT);
        }
        assert_eq!(b.voices(), c.voices());
    }
}
