//! The filter bank (ADR 0005, points 5–8): band-passes boosting what is
//! already in the input at boost A's and boost B's ratio of each peak, from
//! resonances fixed on a grid (FOCUS 0) to filters on each peak's exact
//! ratio, steered by audio-rate partial trackers (FOCUS 1).
//!
//! # Once a frame ([`Bank::frame`])
//!
//! 1. **Candidates.** Each peak's two ratios, walked in frequency order
//!    (the peaks come ascending), each with a salience: its boost's amount
//!    × the peak's prominence × TONE's tilt, capped at 1, × the bound
//!    towards Nyquist, which fades boosts from 0.25 fs to nothing at
//!    0.42 fs.
//! 2. **Targets.** Toward the grid end, each grid point is driven by the
//!    candidates near it, weighted by the band-pass's own response at the
//!    distance, summed and capped at 1; its centre is pulled by FOCUS
//!    toward its candidates' mean, weighted by drive. At TRACK, candidates closer than the
//!    tracking window merge, taking the larger salience.
//! 3. **The twelve kept.** The most salient, a target a filter already
//!    holds counting the steal margin more, amplitude breaking ties.
//! 4. **Trackers** are matched to the frame's peaks by their FFT anchors,
//!    and snapped back when they stray.
//! 5. **Assignment.** A filter keeps the kept target nearest it within the
//!    window; without one it waits about 100 ms, its boost fading, then
//!    releases. A new target,
//!    kept for enough frames running, takes a free filter, or chokes the
//!    quietest tail, or steals the weakest active filter if it beats it by
//!    the margin and that filter has held long enough. A steal fades the
//!    filter out over [`CHOKE_SECONDS`] before it moves: a sounding filter
//!    is never retuned.
//!
//! # Every sample ([`Bank::tick`])
//!
//! Each tracker's band-pass, then each filter's: its output is
//! `gain · k0 · bp`, scaled by WIDTH's damping `k0` always, so nothing
//! jumps when a filter enters or leaves oscillation. Below oscillation
//! RESONANCE lowers the live damping only as far as `k0 / 4`; past it the
//! level is held by damping. In stereo each channel has its own filters,
//! following the same targets with the same gains.
//!
//! Nothing allocates after [`Bank::new`].

use vxn_free_dsp::{DetectedPeak, PartialTracker, Resonator};

/// Filters sounding at once.
pub const M: usize = 12;
/// Filters in all: as many spares as active ones, for tails.
pub const POOL: usize = 2 * M;
/// Samples a control block: filters are steered once a block.
pub const BLOCK: usize = 32;
/// A filter's most gain at its centre: 1 + 4, +14 dB.
pub const BOOST_MAX: f64 = 4.0;

const STEAL_MARGIN: f64 = 0.15;
const ATTACK_FAST: f64 = 0.005;
const ATTACK_SLOW: f64 = 0.150;
const DECAY_FAST: f64 = 0.080;
const DECAY_SLOW: f64 = 0.640;
const HOLD_FAST: f64 = 0.040;
const HOLD_SLOW: f64 = 0.400;
const PERSIST_FAST: u32 = 1;
const PERSIST_SLOW: u32 = 5;
/// A steal's fade.
pub const CHOKE_SECONDS: f64 = 0.010;
/// A release, to −60 dB.
pub const RELEASE_SECONDS: f64 = 0.15;
const TRACK_CENTS: f64 = 50.0;
/// How long a filter waits for a target that has gone, its boost decaying,
/// before it releases. A chord's weaker partials drop out of the analysis
/// for a frame or two as they beat; a frame's grace released and restarted
/// their filters many times a second (ticket 0043).
const GRACE_SECONDS: f64 = 0.1;
/// An oscillation's output, at full boost, as a share of its source peak.
const OSC_SHARE: f64 = 0.25;
const GROWTH: f64 = 0.05;
/// A filter's centre moves at most this a block.
const SLEW_CENTS: f64 = 10.0;
const NYQUIST_FROM: f64 = 0.25;
const NYQUIST_TO: f64 = 0.42;
/// A grid point responds to a candidate within about two bandwidths.
const DRIVE_LEAST: f64 = 0.2;
/// A new target needs its peak this prominent to start a filter; a filter
/// keeps its target at any prominence. The hysteresis stops peaks just
/// over THRESHOLD, which come and go as a chord's partials beat, from
/// starting and releasing filters many times a second.
const START_PROMINENCE: f64 = 0.25;
/// A tracker rougher than this, cents/s² ([`PartialTracker::roughness`]),
/// doesn't steer: a neighbouring partial is in its band, and its readings
/// jump about. Its filter follows the FFT instead. Up to [`ROUGH_FROM_HZ`]
/// a ±30 cent vibrato at 5.5 Hz reads under 65 000 and a ±100 cent one at
/// 7 Hz about 190 000; neighbours read 570 000 and up. Above it the
/// crossings' own error grows, and the limit with it, as the square of the
/// frequency.
const ROUGH_MOST: f64 = 300_000.0;
const ROUGH_FROM_HZ: f64 = 880.0;

/// What a filter is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Free,
    /// Boosting its target.
    Active,
    /// Its target gone, ringing down.
    Releasing,
    /// Stolen, fading out before it moves.
    Choking,
}

/// The controls the bank reads, set once a block.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Each boost's amount, 0 to 1.
    pub amount: [f64; 2],
    /// Each boost's ratio, MULT over DIV.
    pub ratio: [f64; 2],
    /// 0 to 2; a filter oscillates where this × its salience reaches 1.
    pub resonance: f64,
    /// 0 (GRID) to 1 (TRACK).
    pub focus: f64,
    /// TONE's tilt, dB an octave about 1 kHz.
    pub tone_db_per_oct: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            amount: [0.0; 2],
            ratio: [2.0, 1.5],
            resonance: 0.0,
            focus: 0.0,
            tone_db_per_oct: 0.0,
        }
    }
}

/// A target: where a boost is wanted, how much, and where from.
#[derive(Clone, Copy, Debug, Default)]
struct Target {
    hz: f64,
    salience: f64,
    /// The source peak's amplitude.
    amp: f64,
    /// The grid point, or the target itself at TRACK.
    base: f64,
    /// Where the boost aims before FOCUS pulls it toward the grid: the
    /// candidate, or a grid point's drivers' mean.
    aim: f64,
    /// The source peak's index in the frame.
    peak: usize,
    /// The source peak's prominence; at the grid end, the strongest
    /// driver's.
    prominence: f64,
    ratio: f64,
}

/// One filter of the pool.
#[derive(Clone, Copy, Debug)]
pub struct Filter {
    res: [Resonator; 2],
    pub state: State,
    /// Its centre now, Hz.
    pub hz: f64,
    hz_target: f64,
    /// Its output gain now, 0 to [`BOOST_MAX`].
    pub gain: f64,
    gain_target: f64,
    prio: f64,
    src_amp: f64,
    base: f64,
    ratio: f64,
    tracker: Option<usize>,
    missed: u32,
    hold_until: u64,
    choke_dec: f64,
    pending: Target,
    pending_tracker: Option<usize>,
    /// Whether it is self-oscillating.
    pub oscillating: bool,
    started_at: u64,
    choked_at: u64,
}

impl Filter {
    /// Its band-pass's damping now (the first channel's): WIDTH's, or
    /// lowered by RESONANCE, at or below zero while it sings.
    pub fn damping(&self) -> f64 {
        self.res[0].damping()
    }
}

/// Counts the tests read.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Moves of a sounding filter's centre by more than half its band in a
    /// block.
    pub retunes_while_audible: u32,
    pub steals: u32,
    /// The soonest a stolen filter had held its target, samples.
    pub steal_soonest: u64,
    /// The shortest choke, samples.
    pub choke_shortest: u64,
    /// Tails choked early while a free filter stood ready.
    pub tail_chokes_with_free: u32,
}

/// The bank. Build it with [`Bank::new`] off the audio thread.
#[derive(Debug)]
pub struct Bank {
    rate: f64,
    pub settings: Settings,
    k0: f64,
    k_min: f64,

    // Targets.
    targets: Box<[Target]>,
    count: usize,
    rank: Box<[f64]>,
    order: Box<[usize]>,
    claimed: Box<[bool]>,
    streak: Box<[u32]>,
    prev_hz: [f64; M],
    prev_streak: [u32; M],
    prev_count: usize,
    g_drive: Box<[f64]>,
    /// Each grid point's drive-weighted sum of its drivers' log frequency.
    g_log: Box<[f64]>,
    g_best: Box<[f64]>,
    g_target: Box<[Target]>,

    // The frame's peaks.
    gap: Box<[f64]>,
    peak_tracker: Box<[Option<usize>]>,

    filters: [Filter; POOL],
    trackers: [PartialTracker; POOL],
    users: [u32; POOL],

    attack_step: f64,
    decay_step: f64,
    hold_samples: u64,
    persist_frames: u32,
    /// Each block's share of the way to the FFT's target, for a filter
    /// whose tracker doesn't steer: a time constant of half a hop, so a
    /// frame's change becomes a glide across it, not a step.
    fft_glide: f64,
    /// Frames a filter waits for a lost target: [`GRACE_SECONDS`] at the
    /// hop.
    grace_frames: u32,
    speed: f64,
    release_mul: f64,
    choke_step: f64,
    slew: f64,

    now: u64,
    pub stats: Stats,
}

impl Bank {
    /// At `sample_rate`, for frames of up to `max_peaks` peaks and grids of
    /// up to `max_grid` points.
    pub fn new(sample_rate: f64, max_peaks: usize, max_grid: usize) -> Self {
        let max_targets = 2 * max_peaks + max_grid;
        let filter = Filter {
            res: [Resonator::new(sample_rate, 1000.0); 2],
            state: State::Free,
            hz: 1000.0,
            hz_target: 1000.0,
            gain: 0.0,
            gain_target: 0.0,
            prio: 0.0,
            src_amp: 0.0,
            base: 1000.0,
            ratio: 1.0,
            tracker: None,
            missed: 0,
            hold_until: 0,
            choke_dec: 0.0,
            pending: Target::default(),
            pending_tracker: None,
            oscillating: false,
            started_at: 0,
            choked_at: 0,
        };
        let mut bank = Self {
            rate: sample_rate,
            settings: Settings::default(),
            k0: 0.0289,
            k_min: 0.0289 / 4.0,
            targets: vec![Target::default(); max_targets].into_boxed_slice(),
            count: 0,
            rank: vec![0.0; max_targets].into_boxed_slice(),
            order: vec![0; max_targets].into_boxed_slice(),
            claimed: vec![false; max_targets].into_boxed_slice(),
            streak: vec![0; max_targets].into_boxed_slice(),
            prev_hz: [0.0; M],
            prev_streak: [0; M],
            prev_count: 0,
            g_drive: vec![0.0; max_grid].into_boxed_slice(),
            g_log: vec![0.0; max_grid].into_boxed_slice(),
            g_best: vec![0.0; max_grid].into_boxed_slice(),
            g_target: vec![Target::default(); max_grid].into_boxed_slice(),
            gap: vec![0.0; max_peaks].into_boxed_slice(),
            peak_tracker: vec![None; max_peaks].into_boxed_slice(),
            filters: [filter; POOL],
            trackers: [PartialTracker::new(sample_rate); POOL],
            users: [0; POOL],
            attack_step: 0.0,
            decay_step: 0.0,
            hold_samples: 0,
            persist_frames: 1,
            grace_frames: 1,
            fft_glide: 0.3,
            speed: -1.0,
            release_mul: (-(1000f64.ln()) / (RELEASE_SECONDS * sample_rate)).exp(),
            choke_step: 1.0 / (CHOKE_SECONDS * sample_rate),
            slew: SLEW_CENTS / 1200.0 * 2f64.ln(),
            now: 0,
            stats: Stats {
                steal_soonest: u64::MAX,
                choke_shortest: u64::MAX,
                ..Stats::default()
            },
        };
        bank.set_speed(0.8);
        bank
    }

    /// WIDTH, each boost's bandwidth in cents: the nominal damping.
    pub fn set_width_cents(&mut self, cents: f64) {
        self.k0 = 2.0 * (2f64.ln() * cents / 2400.0).sinh();
        self.k_min = self.k0 / 4.0;
    }

    /// The nominal damping, WIDTH's.
    pub fn nominal_damping(&self) -> f64 {
        self.k0
    }

    /// SPEED, 0 (slow and smooth) to 1 (fast and twitchy): the boosts'
    /// attack and decay, a filter's hold before it can be stolen, and the
    /// frames a new target must be kept before it gets a filter, together.
    /// Geometric between the ends.
    pub fn set_speed(&mut self, speed: f64) {
        if speed == self.speed {
            return;
        }
        self.speed = speed;
        let u = 1.0 - speed;
        let geo = |fast: f64, slow: f64| fast * (slow / fast).powf(u);
        let rate = self.rate;
        self.attack_step = 1.0 - (-1.0 / (geo(ATTACK_FAST, ATTACK_SLOW) * rate)).exp();
        self.decay_step = 1.0 - (-1.0 / (geo(DECAY_FAST, DECAY_SLOW) * rate)).exp();
        self.hold_samples = (geo(HOLD_FAST, HOLD_SLOW) * rate) as u64;
        self.persist_frames =
            PERSIST_FAST + (f64::from(PERSIST_SLOW - PERSIST_FAST) * u).round() as u32;
    }

    /// The analysis's hop, samples: frames come this far apart.
    pub fn set_hop(&mut self, hop: usize) {
        self.grace_frames = ((GRACE_SECONDS * self.rate / hop as f64).round() as u32).max(1);
        self.fft_glide = 1.0 - (-(BLOCK as f64) / (0.5 * hop as f64)).exp();
    }

    /// A filter's hold before it can be stolen, samples.
    pub fn hold_samples(&self) -> u64 {
        self.hold_samples
    }

    /// The pool.
    pub fn filters(&self) -> &[Filter; POOL] {
        &self.filters
    }

    /// The trackers.
    pub fn trackers(&self) -> &[PartialTracker; POOL] {
        &self.trackers
    }

    /// Samples since the bank was made.
    pub fn now(&self) -> u64 {
        self.now
    }

    /// The tracking window's half-width at `hz`: half the bandwidth, at
    /// least [`TRACK_CENTS`].
    fn window(&self, hz: f64) -> f64 {
        (0.5 * self.k0).max(2f64.powf(TRACK_CENTS / 1200.0) - 1.0) * hz
    }

    /// The bound towards Nyquist at `hz`.
    pub fn nyquist_bound(&self, hz: f64) -> f64 {
        let (lo, hi) = (NYQUIST_FROM * self.rate, NYQUIST_TO * self.rate);
        if hz <= lo {
            1.0
        } else if hz >= hi {
            0.0
        } else {
            0.5 + 0.5 * (std::f64::consts::PI * (hz - lo) / (hi - lo)).cos()
        }
    }

    // ---- Once a frame -------------------------------------------------

    /// A frame's `peaks` (ascending) are in; `grid` is GRID's points
    /// (ascending). Steers the bank.
    pub fn frame(&mut self, peaks: &[DetectedPeak], grid: &[f64]) {
        let peaks = &peaks[..peaks.len().min(self.gap.len())];
        for (i, p) in peaks.iter().enumerate() {
            let hz = f64::from(p.hz);
            let mut d = f64::MAX;
            if i > 0 {
                d = hz - f64::from(peaks[i - 1].hz);
            }
            if let Some(next) = peaks.get(i + 1) {
                d = d.min(f64::from(next.hz) - hz);
            }
            self.gap[i] = if d == f64::MAX { hz } else { d };
            self.peak_tracker[i] = None;
        }
        self.candidates(peaks);
        if self.settings.focus < 1.0 {
            self.grid_targets(grid);
        } else {
            self.merge();
        }
        self.keep_top_m();
        self.match_trackers(peaks);
        self.assign(peaks);
        self.free_trackers();
        self.prev_count = self.count;
        for j in 0..self.count {
            self.prev_hz[j] = self.targets[self.order[j]].hz;
            self.prev_streak[j] = self.streak[j];
        }
    }

    /// Each peak's two ratios, in frequency order.
    fn candidates(&mut self, peaks: &[DetectedPeak]) {
        let s = self.settings;
        let mut walk = [0usize; 2];
        self.count = 0;
        while walk[0] < peaks.len() || walk[1] < peaks.len() {
            let mut b = 0;
            let mut hz = f64::MAX;
            for (m, &w) in walk.iter().enumerate() {
                if w < peaks.len() {
                    let f = s.ratio[m] * f64::from(peaks[w].hz);
                    if f < hz {
                        hz = f;
                        b = m;
                    }
                }
            }
            let p = walk[b];
            walk[b] += 1;
            let amount = s.amount[b];
            if amount <= 0.0 {
                continue;
            }
            let bound = self.nyquist_bound(hz);
            if bound <= 0.0 {
                continue;
            }
            let tilt = 10f64.powf(s.tone_db_per_oct * (hz / 1000.0).log2() / 20.0);
            let peak = peaks[p];
            let salience = (amount * f64::from(peak.prominence) * tilt).min(1.0) * bound;
            if salience <= 1e-3 {
                continue;
            }
            self.targets[self.count] = Target {
                hz,
                salience,
                amp: f64::from(peak.amplitude),
                base: hz,
                aim: hz,
                peak: p,
                prominence: f64::from(peak.prominence),
                ratio: s.ratio[b],
            };
            self.count += 1;
        }
    }

    /// Toward the grid end: grid points driven by the candidates near
    /// them, each centre pulled toward its strongest by FOCUS.
    // The band-pass's response is written as the maths has it.
    #[allow(clippy::many_single_char_names)]
    fn grid_targets(&mut self, grid: &[f64]) {
        let n = grid.len().min(self.g_drive.len());
        let grid = &grid[..n];
        self.g_drive[..n].fill(0.0);
        self.g_log[..n].fill(0.0);
        self.g_best[..n].fill(0.0);
        let q = 1.0 / self.k0;
        for t in &self.targets[..self.count] {
            let at = grid.partition_point(|&g| g < t.hz);
            let mut drive = |j: usize| -> bool {
                let x = grid[j] / t.hz - t.hz / grid[j];
                let w = 1.0 / (1.0 + q * q * x * x).sqrt();
                if w < DRIVE_LEAST {
                    return false;
                }
                let c = t.salience * w;
                self.g_drive[j] += c;
                self.g_log[j] += c * t.hz.ln();
                if c > self.g_best[j] {
                    self.g_best[j] = c;
                    self.g_target[j] = *t;
                }
                true
            };
            for j in (0..at).rev() {
                if !drive(j) {
                    break;
                }
            }
            for j in at..n {
                if !drive(j) {
                    break;
                }
            }
        }
        let focus = self.settings.focus;
        let mut out = 0;
        for (j, &g) in grid.iter().enumerate() {
            if self.g_drive[j] <= 1e-3 {
                continue;
            }
            // Pulled toward its drivers' weighted mean, not the strongest:
            // two near partials taking turns as the strongest would make the
            // centre jump between them.
            let best = self.g_target[j];
            let toward = self.g_log[j] / self.g_drive[j];
            self.targets[out] = Target {
                hz: (g.ln() + focus * (toward - g.ln())).exp(),
                aim: toward.exp(),
                salience: self.g_drive[j].min(1.0),
                base: g,
                ..best
            };
            out += 1;
        }
        self.count = out;
    }

    /// TRACK: candidates closer than the tracking window are one target,
    /// with the larger salience.
    fn merge(&mut self) {
        let mut out = 0;
        for i in 0..self.count {
            let t = self.targets[i];
            if out > 0 && t.hz - self.targets[out - 1].hz < self.window(self.targets[out - 1].hz) {
                let o = &mut self.targets[out - 1];
                let w = o.salience + t.salience;
                o.hz = (o.hz * o.salience + t.hz * t.salience) / w;
                o.amp = o.amp.max(t.amp);
                o.prominence = o.prominence.max(t.prominence);
                if t.salience > o.salience {
                    o.peak = t.peak;
                    o.ratio = t.ratio;
                }
                o.base = o.hz;
                o.aim = o.hz;
                o.salience = o.salience.max(t.salience);
            } else {
                self.targets[out] = t;
                out += 1;
            }
        }
        self.count = out;
    }

    fn keep_top_m(&mut self) {
        for i in 0..self.count {
            self.order[i] = i;
            self.rank[i] = self.targets[i].salience + 1e-4 * self.targets[i].amp.min(1.0);
        }
        // Incumbency: the target each active filter holds counts more.
        for k in 0..POOL {
            if self.filters[k].state != State::Active {
                continue;
            }
            let f = self.filters[k].hz_target;
            let mut best = None;
            let mut bd = self.window(f);
            for (j, t) in self.targets[..self.count].iter().enumerate() {
                let d = (t.hz - f).abs();
                if d < bd {
                    bd = d;
                    best = Some(j);
                }
            }
            if let Some(j) = best {
                self.rank[j] += STEAL_MARGIN;
            }
        }
        let keep = M.min(self.count);
        for i in 0..keep {
            let mut best = i;
            for j in i + 1..self.count {
                if self.rank[self.order[j]] > self.rank[self.order[best]] {
                    best = j;
                }
            }
            self.order.swap(i, best);
        }
        self.count = keep;
        self.claimed[..keep].fill(false);
    }

    /// The frames running a kept target at `hz` was kept before: the last
    /// frame's nearest within half a bandwidth.
    fn last_streak(&self, hz: f64) -> u32 {
        let mut bd = 0.5 * self.k0 * hz;
        let mut best = 0;
        for j in 0..self.prev_count {
            let d = (self.prev_hz[j] - hz).abs();
            if d < bd {
                bd = d;
                best = self.prev_streak[j];
            }
        }
        best
    }

    fn assign(&mut self, peaks: &[DetectedPeak]) {
        for j in 0..self.count {
            self.streak[j] = 1 + self.last_streak(self.targets[self.order[j]].hz);
        }
        for i in 0..POOL {
            if self.filters[i].state != State::Active {
                continue;
            }
            let centre = self.filters[i].hz_target;
            let mut bd = self.window(centre);
            let mut best = None;
            for j in 0..self.count {
                if self.claimed[j] {
                    continue;
                }
                let d = (self.targets[self.order[j]].hz - centre).abs();
                if d < bd {
                    bd = d;
                    best = Some(j);
                }
            }
            if let Some(j) = best {
                self.claimed[j] = true;
                let t = self.targets[self.order[j]];
                // Keep the tracker it has while that still lands on the
                // target: near partials take turns as a target's strongest
                // source, and swapping trackers with them made the filter
                // jump between them.
                let kept = self.filters[i].tracker.filter(|&k| {
                    let tr = &self.trackers[k];
                    tr.active()
                        && (self.filters[i].ratio * tr.frequency() - t.aim).abs()
                            < self.window(t.aim)
                });
                let tracker = kept.or_else(|| self.tracker_for(t.peak, peaks));
                let f = &mut self.filters[i];
                f.hz_target = t.hz;
                f.missed = 0;
                f.prio = t.salience;
                f.gain_target = BOOST_MAX * t.salience;
                f.src_amp = t.amp;
                if kept.is_some() {
                    f.base = t.base;
                } else if tracker.is_some() {
                    f.tracker = tracker;
                    f.ratio = t.ratio;
                    f.base = t.base;
                }
            } else if self.filters[i].missed < self.grace_frames {
                // Waiting for it to come back, fading, first to be stolen.
                let f = &mut self.filters[i];
                f.missed += 1;
                f.gain_target = 0.0;
                f.prio = 0.0;
            } else {
                self.filters[i].state = State::Releasing;
                self.filters[i].prio = 0.0;
            }
        }
        for j in 0..self.count {
            if self.claimed[j] || self.streak[j] < self.persist_frames {
                continue;
            }
            let t = self.targets[self.order[j]];
            if t.prominence < START_PROMINENCE {
                continue;
            }
            let busy = self
                .filters
                .iter()
                .filter(|f| matches!(f.state, State::Active | State::Choking))
                .count();
            if busy < M {
                if let Some(free) = self.filters.iter().position(|f| f.state == State::Free) {
                    let tracker = self.tracker_for(t.peak, peaks);
                    self.start(free, t, tracker);
                    continue;
                }
                let tail = (0..POOL)
                    .filter(|&i| self.filters[i].state == State::Releasing)
                    .min_by(|&a, &b| self.filters[a].gain.total_cmp(&self.filters[b].gain));
                if let Some(i) = tail {
                    let tracker = self.tracker_for(t.peak, peaks);
                    self.choke(i, t, tracker);
                }
                continue;
            }
            let weakest = (0..POOL)
                .filter(|&i| {
                    let f = &self.filters[i];
                    f.state == State::Active && self.now >= f.hold_until
                })
                .min_by(|&a, &b| self.filters[a].prio.total_cmp(&self.filters[b].prio));
            if let Some(i) = weakest
                && t.salience > self.filters[i].prio + STEAL_MARGIN
            {
                self.stats.steals += 1;
                self.stats.steal_soonest = self
                    .stats
                    .steal_soonest
                    .min(self.now - self.filters[i].started_at);
                let tracker = self.tracker_for(t.peak, peaks);
                self.choke(i, t, tracker);
            }
        }
    }

    fn start(&mut self, i: usize, t: Target, tracker: Option<usize>) {
        let (k0, hold, now) = (self.k0, self.hold_samples, self.now);
        let f = &mut self.filters[i];
        f.tracker = tracker;
        f.ratio = t.ratio;
        f.base = t.base;
        f.hz = t.hz;
        f.hz_target = t.hz;
        f.missed = 0;
        for r in &mut f.res {
            r.reset();
            r.set_frequency(t.hz);
            r.set_damping(k0);
        }
        f.gain = 0.0;
        f.gain_target = BOOST_MAX * t.salience;
        f.prio = t.salience;
        f.src_amp = t.amp;
        f.hold_until = now + hold;
        f.started_at = now;
        f.oscillating = false;
        f.state = State::Active;
    }

    fn choke(&mut self, i: usize, t: Target, tracker: Option<usize>) {
        let (step, now) = (self.choke_step, self.now);
        let f = &mut self.filters[i];
        f.pending = t;
        f.pending_tracker = tracker;
        f.choke_dec = f.gain.max(1e-6) * step;
        f.prio = t.salience;
        f.choked_at = now;
        f.state = State::Choking;
    }

    // ---- Trackers -----------------------------------------------------

    /// The tracker on peak `p`, started if it has none. None at the grid
    /// end, where nothing moves.
    fn tracker_for(&mut self, p: usize, peaks: &[DetectedPeak]) -> Option<usize> {
        if self.settings.focus <= 0.0 {
            return None;
        }
        if let Some(k) = self.peak_tracker[p] {
            return Some(k);
        }
        let k = self.trackers.iter().position(|t| !t.active())?;
        let damping = self.tracker_damping(p, peaks);
        self.trackers[k].start(f64::from(peaks[p].hz), damping);
        self.peak_tracker[p] = Some(k);
        Some(k)
    }

    /// A tracker's band: 30 % of the gap to the neighbouring peaks, as a
    /// share of its centre.
    fn tracker_damping(&self, p: usize, peaks: &[DetectedPeak]) -> f64 {
        (0.3 * self.gap[p] / f64::from(peaks[p].hz)).clamp(0.02, 0.25)
    }

    /// Each tracker takes the peak nearest its anchor (the FFT's view of
    /// its own peak, so a stray tracker can't drag it), and is snapped back
    /// when nearer a neighbour than its own peak, or when it isn't hearing
    /// its partial and is stale by more than the tracking window.
    fn match_trackers(&mut self, peaks: &[DetectedPeak]) {
        for k in 0..POOL {
            if !self.trackers[k].active() {
                continue;
            }
            let anchor = self.trackers[k].anchor();
            let mut bd = self.window(anchor);
            let mut best = None;
            for (p, peak) in peaks.iter().enumerate() {
                if self.peak_tracker[p].is_some() {
                    continue;
                }
                let d = (f64::from(peak.hz) - anchor).abs();
                if d < bd {
                    bd = d;
                    best = Some(p);
                }
            }
            let Some(p) = best else { continue };
            self.peak_tracker[p] = Some(k);
            let hz = f64::from(peaks[p].hz);
            let damping = self.tracker_damping(p, peaks);
            let window = self.window(hz);
            let t = &mut self.trackers[k];
            t.anchor_to(hz);
            t.set_damping(damping);
            let off = (hz - t.frequency()).abs();
            if off > 0.5 * self.gap[p] || (!t.present() && off > window) {
                t.snap(hz);
            }
        }
    }

    fn free_trackers(&mut self) {
        self.users = [0; POOL];
        for f in &self.filters {
            if f.state != State::Free
                && let Some(k) = f.tracker
            {
                self.users[k] += 1;
            }
        }
        for (t, &users) in self.trackers.iter_mut().zip(&self.users) {
            if t.active() && users == 0 {
                t.stop();
            }
        }
    }

    // ---- Every sample -------------------------------------------------

    /// One sample: `mid` is what was analysed (the trackers hear it), `x`
    /// each channel's input, `channels` 1 or 2. Returns each channel's
    /// filter sum, `Σ gain · k0 · bp`.
    #[inline]
    pub fn tick(&mut self, mid: f64, x: [f64; 2], channels: usize) -> [f64; 2] {
        let control = self.now.is_multiple_of(BLOCK as u64);
        for t in &mut self.trackers {
            t.tick(mid, self.now, control);
        }
        let mut y = [0.0; 2];
        for i in 0..POOL {
            let state = self.filters[i].state;
            if state == State::Free {
                continue;
            }
            if control {
                self.control(i, channels);
            }
            let f = &mut self.filters[i];
            let mut bp = [0.0; 2];
            for c in 0..channels {
                bp[c] = f.res[c].next(x[c]);
            }
            match state {
                State::Active => {
                    let step = if f.gain_target > f.gain {
                        self.attack_step
                    } else {
                        self.decay_step
                    };
                    f.gain += step * (f.gain_target - f.gain);
                }
                State::Releasing => {
                    f.gain *= self.release_mul;
                    if f.gain < 1e-3 * BOOST_MAX {
                        f.state = State::Free;
                        f.gain = 0.0;
                        f.oscillating = false;
                        for r in &mut f.res {
                            r.reset();
                        }
                    }
                }
                State::Choking => {
                    f.gain = (f.gain - f.choke_dec).max(0.0);
                    if f.gain <= 0.0 {
                        self.stats.choke_shortest =
                            self.stats.choke_shortest.min(self.now - f.choked_at);
                        let (t, tracker) = (f.pending, f.pending_tracker);
                        self.start(i, t, tracker);
                    }
                }
                State::Free => {}
            }
            let g = self.filters[i].gain * self.k0;
            for c in 0..channels {
                y[c] += g * bp[c];
            }
        }
        self.now += 1;
        y
    }

    /// A filter's control block: its centre, then its damping.
    fn control(&mut self, i: usize, channels: usize) {
        let (k0, k_min, focus, slew) = (self.k0, self.k_min, self.settings.focus, self.slew);
        let fft_glide = self.fft_glide;
        let resonance = self.settings.resonance;
        let f = &mut self.filters[i];
        if f.state != State::Active {
            for r in &mut f.res {
                r.set_damping(k0);
            }
            f.oscillating = false;
            return;
        }
        let settled = f.tracker.filter(|&k| {
            let t = &self.trackers[k];
            let over = (t.frequency() / ROUGH_FROM_HZ).max(1.0);
            t.settled() && t.roughness() < ROUGH_MOST * over * over
        });
        let mut moved = None;
        if let Some(k) = settled {
            // The tracker gives the ratio now; FOCUS pulls the grid base
            // toward it; a light glide and a slew turn a catch-up into a
            // glide, never a step.
            let exact = f.ratio * self.trackers[k].frequency();
            let base = if focus < 1.0 { f.base } else { exact };
            let centre = (base.ln() + focus * (exact.ln() - base.ln())).exp();
            let step = (0.3 * (centre / f.hz).ln()).clamp(-slew, slew);
            let nf = f.hz * step.exp();
            if (nf - f.hz).abs() > 0.5 * k0 * f.hz && f.gain > 1e-6 {
                self.stats.retunes_while_audible += 1;
            }
            moved = Some(nf);
        } else if f.hz != f.hz_target && focus > 0.0 {
            let step = (fft_glide * (f.hz_target / f.hz).ln()).clamp(-slew, slew);
            moved = Some(f.hz * step.exp());
        }
        if let Some(nf) = moved {
            f.hz = nf;
            for r in &mut f.res {
                r.set_frequency(nf);
            }
        }
        let r = resonance * f.prio;
        if r >= 1.0 {
            let target = OSC_SHARE * f.src_amp / k0;
            for res in &mut f.res[..channels] {
                if res.level() < 1e-6 * target {
                    res.seed(1e-3 * target);
                }
                let k = res.holding_damping(target, GROWTH, BLOCK, f.hz, k0);
                res.set_damping(k);
            }
            f.oscillating = true;
        } else {
            let k = k_min.max(k0 * (1.0 - r));
            for res in &mut f.res {
                res.set_damping(k);
            }
            f.oscillating = false;
        }
    }

    /// Every filter silent and every tracker free.
    pub fn clear(&mut self) {
        for f in &mut self.filters {
            f.state = State::Free;
            f.gain = 0.0;
            f.tracker = None;
            f.oscillating = false;
            for r in &mut f.res {
                r.reset();
            }
        }
        for t in &mut self.trackers {
            t.stop();
        }
        self.prev_count = 0;
        self.count = 0;
    }
}
