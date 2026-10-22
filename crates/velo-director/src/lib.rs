//! Auto-zoom "director" for Velo.
//!
//! While recording, Velo saves the raw video and, separately, a timeline of mouse and key
//! events. This crate turns that timeline into zoom segments ([`plan`]) and then into a
//! smooth per-frame camera ([`camera_path`]), so zoom is non-destructive and editable.
//! It is plain Rust with no platform code, so it is fully unit-tested on any OS.

mod camera;
mod cursor;
mod plan;
mod spring;
mod types;

pub use camera::camera_path;
pub use cursor::{smooth_cursor, CursorTrack};
pub use plan::plan;
pub use spring::Spring;
pub use types::*;
