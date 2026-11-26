//! Glow-Up as a stereo CLAP audio effect: the editor and engine of
//! `glow-up` behind `silkscreen-clap`. `cargo silkscreen bundle
//! glow-up-clap --install` builds the `.clap` and puts it where hosts look.

use std::ffi::CStr;

use glow_up::engine::GlowUpEngine;
use silkscreen::PluginUi;
use silkscreen_clap::{SilkscreenPlugin, export_clap, features};

#[derive(Debug)]
pub struct GlowUpPlugin;

impl SilkscreenPlugin for GlowUpPlugin {
    /// Never changed once released: hosts save projects under it.
    const ID: &'static str = "labs.vulpus.glow-up";
    const NAME: &'static str = "Glow-Up";
    const VERSION: &'static str = glow_up::UI.version;
    const FEATURES: &'static [&'static CStr] = &[features::AUDIO_EFFECT, features::FILTER];
    const BUILD: &'static str = env!("SILKSCREEN_BUILD_ID");
    const AUDIO_INPUT: bool = true;
    const USER_PRESETS: bool = true;
    /// None, at any setting and rate (ADR 0005, point 3).
    const LATENCY: u32 = GlowUpEngine::LATENCY;
    const UI: PluginUi = glow_up::UI;

    type Engine = GlowUpEngine;

    fn engine(sample_rate: f32, _max_frames: u32) -> GlowUpEngine {
        GlowUpEngine::new(sample_rate)
    }
}

export_clap!(GlowUpPlugin);
