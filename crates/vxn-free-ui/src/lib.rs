//! Editor widgets the vxn-free plugins share. They run in the editor, on
//! the main thread; engines never depend on this crate.

pub mod crush_view;

pub use crush_view::{ChainParams, CrushStyle, CrushView};
