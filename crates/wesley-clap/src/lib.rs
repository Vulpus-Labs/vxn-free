//! Wesley as a stereo CLAP audio effect: the editor and engine of `wesley`
//! behind `silkscreen-clap`. `cargo silkscreen bundle wesley-clap
//! --install` builds the `.clap` and puts it where hosts look.

use std::ffi::CStr;

use silkscreen::PluginUi;
use silkscreen_clap::{SilkscreenPlugin, export_clap, features};
use wesley::engine::WesleyEngine;

#[derive(Debug)]
pub struct WesleyPlugin;

impl SilkscreenPlugin for WesleyPlugin {
    /// Never changed once released: hosts save projects under it.
    const ID: &'static str = "labs.vulpus.wesley";
    const NAME: &'static str = "Wesley";
    const VERSION: &'static str = wesley::UI.version;
    const FEATURES: &'static [&'static CStr] = &[
        features::AUDIO_EFFECT,
        features::DISTORTION,
        features::STEREO,
    ];
    const BUILD: &'static str = env!("SILKSCREEN_BUILD_ID");
    const AUDIO_INPUT: bool = true;
    const USER_PRESETS: bool = true;
    /// None, at any setting and rate (ADR 0004, point 2).
    const LATENCY: u32 = WesleyEngine::LATENCY;
    const UI: PluginUi = wesley::UI;

    type Engine = WesleyEngine;

    fn engine(sample_rate: f32, _max_frames: u32) -> WesleyEngine {
        WesleyEngine::new(sample_rate)
    }
}

export_clap!(WesleyPlugin);
