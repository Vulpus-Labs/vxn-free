//! Charm as a stereo CLAP audio effect: the editor and engine of `charm`
//! behind `silkscreen-clap`. `cargo silkscreen bundle charm-clap
//! --install` builds the `.clap` and puts it where hosts look.

use std::ffi::CStr;

use charm::engine::CharmEngine;
use silkscreen::PluginUi;
use silkscreen_clap::{SilkscreenPlugin, export_clap, features};

#[derive(Debug)]
pub struct CharmPlugin;

impl SilkscreenPlugin for CharmPlugin {
    /// Never changed once released: hosts save projects under it.
    const ID: &'static str = "labs.vulpus.charm";
    const NAME: &'static str = "Charm";
    const VERSION: &'static str = charm::UI.version;
    const FEATURES: &'static [&'static CStr] =
        &[features::AUDIO_EFFECT, features::CHORUS, features::STEREO];
    const BUILD: &'static str = env!("SILKSCREEN_BUILD_ID");
    const AUDIO_INPUT: bool = true;
    const USER_PRESETS: bool = true;
    /// None, at any setting and rate (ADR 0002, point 2).
    const LATENCY: u32 = CharmEngine::LATENCY;
    const UI: PluginUi = charm::UI;

    type Engine = CharmEngine;

    fn engine(sample_rate: f32, _max_frames: u32) -> CharmEngine {
        CharmEngine::new(sample_rate)
    }
}

export_clap!(CharmPlugin);
