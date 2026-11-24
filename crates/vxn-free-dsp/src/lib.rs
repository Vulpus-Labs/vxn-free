//! DSP shared by the vxn-free plugins: Beverley, Glow-Up and Charm.
//!
//! # Rules
//!
//! - Audio is `f32` at ±1.0 full scale; every coefficient is computed from
//!   the host's sample rate, never a fixed one.
//! - Nothing here allocates or locks once a processor is constructed: the
//!   engines call into it on the audio thread.
//! - It depends on no host or UI crate, and builds for
//!   `wasm32-unknown-unknown`, so a web build of a plugin stays possible.

pub mod crossover;
pub mod delay;
pub mod peak;
pub mod smooth;
pub mod svf;

pub use crossover::Crossover;
pub use delay::{DelayLine, SideBuffer};
pub use peak::{PeakFollower, RangeFollower};
pub use smooth::{Glide, Ramp};
pub use svf::{Svf, SvfOut};
