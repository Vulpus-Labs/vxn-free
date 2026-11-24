//! Beverley as a stereo CLAP audio effect: the editor and engine of
//! `beverley` behind `silkscreen-clap`. `cargo silkscreen bundle
//! beverley-clap --install` builds the `.clap` and puts it where hosts
//! look.

use std::ffi::CStr;

use beverley::engine::BeverleyEngine;
use silkscreen::PluginUi;
use silkscreen_clap::{SilkscreenPlugin, export_clap, features};

#[derive(Debug)]
pub struct BeverleyPlugin;

impl SilkscreenPlugin for BeverleyPlugin {
    /// Never changed once released: hosts save projects under it.
    const ID: &'static str = "labs.vulpus.beverley";
    const NAME: &'static str = "Beverley";
    const VERSION: &'static str = beverley::UI.version;
    const FEATURES: &'static [&'static CStr] = &[
        features::AUDIO_EFFECT,
        features::DISTORTION,
        features::STEREO,
    ];
    const BUILD: &'static str = env!("SILKSCREEN_BUILD_ID");
    const AUDIO_INPUT: bool = true;
    const USER_PRESETS: bool = true;
    /// None, at any setting and rate (ticket 0003).
    const LATENCY: u32 = BeverleyEngine::LATENCY;
    const UI: PluginUi = beverley::UI;

    type Engine = BeverleyEngine;

    fn engine(sample_rate: f32, _max_frames: u32) -> BeverleyEngine {
        BeverleyEngine::new(sample_rate)
    }
}

export_clap!(BeverleyPlugin);
