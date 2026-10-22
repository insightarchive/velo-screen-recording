//! The recording engine. Windows has the real implementation (Graphics Capture, WASAPI, input
//! hooks); other platforms get a stub so the app still builds and the UI can be developed anywhere.

use serde::Serialize;
use std::sync::Arc;
use velo_core::Levels;
use velo_director::Size;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{capture_thumbnail, list_sources, render_project, start_preview, video_thumbnail, Preview, Recording};

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::{capture_thumbnail, list_sources, render_project, start_preview, video_thumbnail, Preview, Recording};

#[derive(Debug, Clone, Serialize)]
pub struct SourceInfo {
    pub name: String,
    pub detail: String,
}

/// Everything the UI can pick from. Indexes into these lists are what `StartConfig` carries.
#[derive(Debug, Clone, Serialize)]
pub struct Sources {
    pub displays: Vec<SourceInfo>,
    pub windows: Vec<SourceInfo>,
    pub mics: Vec<String>,
    pub speakers: Vec<String>,
}

/// What the UI gets back when a recording ends.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finished {
    pub path: String,
    pub project_path: Option<String>,
    pub seconds: u64,
    pub size_mb: f64,
    pub screen: Size,
}

pub type LevelsSink = Arc<dyn Fn(Levels) + Send + Sync>;
