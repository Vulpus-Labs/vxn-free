//! Wesley: Beverley in three bands (ADR 0003). Its parameters and its
//! engine.
//!
//! The input is split into LOW, MID and HIGH by two Linkwitz-Riley
//! crossovers whose bands sum flat (`vxn_free_dsp::ThreeBand`). Each band
//! has Beverley's crush chain with its own settings; STEREO and auto
//! gain's DETECT and DECAY are shared. LOW is off by default, so out of
//! the box the bass passes clean and MID and HIGH are crushed. The editor
//! arrives with the plugin (ticket 0034).

use silkscreen::ParamIndex;
use silkscreen::model::{ParamSpec, Taper};

pub mod engine;

/// The three bands, low to high.
pub const BANDS: usize = 3;
pub const LOW: usize = 0;
pub const MID: usize = 1;
pub const HIGH: usize = 2;

/// Each band's params, in order: band `b`'s param `p` is at index
/// `b * PER_BAND + p` ([`band_param`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BandParam {
    Gain = 0,
    AutoGain = 1,
    Gamma = 2,
    BitDepth = 3,
    Smoothing = 4,
    CrushMode = 5,
    Mix = 6,
    Level = 7,
    On = 8,
    Solo = 9,
}

/// Params a band has.
pub const PER_BAND: u32 = 10;

impl BandParam {
    /// Every band param, in index order.
    pub const ALL: [BandParam; PER_BAND as usize] = [
        BandParam::Gain,
        BandParam::AutoGain,
        BandParam::Gamma,
        BandParam::BitDepth,
        BandParam::Smoothing,
        BandParam::CrushMode,
        BandParam::Mix,
        BandParam::Level,
        BandParam::On,
        BandParam::Solo,
    ];
}

/// Band `band`'s `param`.
pub const fn band_param(band: usize, param: BandParam) -> ParamIndex {
    ParamIndex(band as u32 * PER_BAND + param as u32)
}

/// Which band param `index` is, if it is one.
pub fn which_band_param(index: ParamIndex) -> Option<(usize, BandParam)> {
    let i = index.0;
    (i < BANDS as u32 * PER_BAND).then(|| {
        (
            (i / PER_BAND) as usize,
            BandParam::ALL[(i % PER_BAND) as usize],
        )
    })
}

const SHARED: u32 = BANDS as u32 * PER_BAND;
pub const LOW_XOVER: ParamIndex = ParamIndex(SHARED);
pub const HIGH_XOVER: ParamIndex = ParamIndex(SHARED + 1);
pub const STEREO: ParamIndex = ParamIndex(SHARED + 2);
pub const DETECT: ParamIndex = ParamIndex(SHARED + 3);
pub const DECAY: ParamIndex = ParamIndex(SHARED + 4);
pub const MIX: ParamIndex = ParamIndex(SHARED + 5);

/// GAMMA's knob runs from −1 to 1; the power it sets is ten to that.
fn gamma_from_knob(knob: f32) -> f32 {
    10f32.powf(knob)
}

fn gamma_to_knob(gamma: f32) -> f32 {
    gamma.max(1.0e-6).log10()
}

/// One band's params, `name` prefixed (`"Low"`), ids from `id`. Ranges
/// and defaults are Beverley's but for AUTO GAIN, on: a band is quieter
/// than the whole, and auto gain brings it to full scale before it is
/// crushed (ADR 0003, point 4). Drafts until ticket 0035.
macro_rules! band {
    ($name:literal, $id:literal, $on:literal) => {
        [
            ParamSpec::new(concat!($name, " Gain"), 0.0, 20.0, 0.0)
                .unit("dB")
                .decimals(1)
                .id($id),
            ParamSpec::toggle(concat!($name, " Auto Gain"), true).id($id + 1),
            ParamSpec::new(concat!($name, " Gamma"), -1.0, 1.0, 0.0)
                .bipolar()
                .display(gamma_from_knob, gamma_to_knob)
                .decimals(2)
                .id($id + 2),
            ParamSpec::new(concat!($name, " Bit Depth"), 1.0, 16.0, 4.0)
                .taper(Taper::Exp { mid: 4.0 })
                .unit("bits")
                .decimals(1)
                .id($id + 3),
            ParamSpec::new(concat!($name, " Smoothing"), 0.0, 100.0, 0.0)
                .taper(Taper::Exp { mid: 20.0 })
                .unit("%")
                .decimals(0)
                .id($id + 4),
            ParamSpec::toggle(concat!($name, " Crush Mode"), false)
                .labels(&["Asymmetric", "Symmetric"])
                .id($id + 5),
            // The band's crushed signal against its own dry signal.
            ParamSpec::new(concat!($name, " Mix"), 0.0, 100.0, 100.0)
                .unit("%")
                .decimals(0)
                .id($id + 6),
            // The band's output.
            ParamSpec::new(concat!($name, " Level"), -24.0, 12.0, 0.0)
                .unit("dB")
                .decimals(1)
                .id($id + 7),
            // Off passes the band uncrushed.
            ParamSpec::toggle(concat!($name, " On"), $on).id($id + 8),
            // Any band soloed mutes the rest: for setting up by ear.
            ParamSpec::toggle(concat!($name, " Solo"), false).id($id + 9),
        ]
    };
}

const LOW_PARAMS: [ParamSpec; PER_BAND as usize] = band!("Low", 1, false);
const MID_PARAMS: [ParamSpec; PER_BAND as usize] = band!("Mid", 11, true);
const HIGH_PARAMS: [ParamSpec; PER_BAND as usize] = band!("High", 21, true);

/// Params in all.
pub const PARAM_COUNT: usize = BANDS * PER_BAND as usize + 6;

/// The parameter table: index `i` is `ParamIndex(i)`. Each `id` is what
/// presets and host projects store, so never change or reuse one.
pub static PARAMS: [ParamSpec; PARAM_COUNT] = {
    let mut all = [LOW_PARAMS[0]; PARAM_COUNT];
    let mut i = 0;
    while i < PER_BAND as usize {
        all[i] = LOW_PARAMS[i];
        all[PER_BAND as usize + i] = MID_PARAMS[i];
        all[2 * PER_BAND as usize + i] = HIGH_PARAMS[i];
        i += 1;
    }
    let s = SHARED as usize;
    // Below this, LOW; up to HIGH XOVER, MID. Geometric over the travel.
    all[s] = ParamSpec::new("Low Xover", 40.0, 1000.0, 200.0)
        .taper(Taper::Exp { mid: 200.0 })
        .unit("Hz")
        .decimals(0)
        .id(31);
    // Held at least an octave above LOW XOVER.
    all[s + 1] = ParamSpec::new("High Xover", 500.0, 12_000.0, 2500.0)
        .taper(Taper::Exp { mid: 2449.49 })
        .unit("Hz")
        .decimals(0)
        .id(32);
    // How the two channels are crushed, as Beverley's STEREO.
    all[s + 2] = ParamSpec::stepped("Stereo", &["Mono", "Separate", "Linked"], 2).id(33);
    // How auto gain follows each band, as Beverley's DETECT.
    all[s + 3] = ParamSpec::toggle("Detect", true)
        .labels(&["Peak", "Range"])
        .id(34);
    // Auto gain lets go 20 dB in this time, as Beverley's DECAY.
    all[s + 4] = ParamSpec::new("Decay", 1.0, 5000.0, 115.0)
        .taper(Taper::Exp { mid: 70.710_68 })
        .unit("ms")
        .decimals(0)
        .id(35);
    // The summed bands against the input split and summed uncrushed.
    all[s + 5] = ParamSpec::new("Mix", 0.0, 100.0, 100.0)
        .unit("%")
        .decimals(0)
        .id(36);
    all
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_in_order() {
        let ids: Vec<u32> = PARAMS.iter().map(|p| p.id).collect();
        assert_eq!(ids, (1..=36).collect::<Vec<u32>>());
    }

    #[test]
    fn band_params_index_round_trip() {
        for band in 0..BANDS {
            for p in BandParam::ALL {
                let i = band_param(band, p);
                assert_eq!(which_band_param(i), Some((band, p)));
            }
        }
        assert_eq!(which_band_param(LOW_XOVER), None);
        assert_eq!(
            PARAMS[band_param(MID, BandParam::BitDepth).0 as usize].name,
            "Mid Bit Depth"
        );
    }

    #[test]
    fn low_is_off_by_default() {
        let on = |b| PARAMS[band_param(b, BandParam::On).0 as usize].default;
        assert_eq!((on(LOW), on(MID), on(HIGH)), (0.0, 1.0, 1.0));
    }
}
