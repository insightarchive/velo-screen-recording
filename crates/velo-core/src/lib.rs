//! Platform-independent pieces of the Velo recording engine.
//!
//! Everything that touches Windows (capture, WASAPI, input hooks) lives in the app crate and
//! is deliberately thin. The logic that decides *what* gets recorded, in what timeline and in
//! what format lives here so it can be tested on any OS.

pub mod clock;
pub mod config;
pub mod mixer;
pub mod project;

pub use clock::Clock;
pub use config::{CaptureMode, StartConfig};
pub use mixer::{Levels, Mixer, SAMPLE_RATE};
pub use project::Project;
