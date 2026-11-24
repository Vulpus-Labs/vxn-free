//! DSP shared by the vxn-free plugins, Beverley and Glow-Up.
//!
//! # Rules
//!
//! - Audio is `f32` at ±1.0 full scale; every coefficient is computed from
//!   the host's sample rate, never a fixed one.
//! - Nothing here allocates or locks once a processor is constructed: the
//!   engines call into it on the audio thread.
//! - It depends on no host or UI crate, and builds for
//!   `wasm32-unknown-unknown`, so a web build of a plugin stays possible.

pub mod peak;
pub mod smooth;

pub use peak::PeakFollower;
pub use smooth::{Glide, Ramp};
