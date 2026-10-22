use super::{Finished, LevelsSink, SourceInfo, Sources};
use velo_core::{CaptureMode, StartConfig};

const UNSUPPORTED: &str = "Recording is only available on Windows.";

pub fn list_sources() -> Sources {
    let s = |n: &str, d: &str| SourceInfo { name: n.into(), detail: d.into() };
    Sources {
        displays: vec![s("Display 1", "1920 x 1080 (main)")],
        windows: vec![s("Preview window", "demo")],
        mics: vec!["Default microphone".into()],
        speakers: vec!["Default speakers".into()],
    }
}

pub struct Recording;

impl Recording {
    pub fn start(_: StartConfig, _: LevelsSink) -> Result<Self, String> {
        Err(UNSUPPORTED.into())
    }
    pub fn pause(&self) {}
    pub fn resume(&self) {}
    pub fn is_paused(&self) -> bool {
        false
    }
    pub fn elapsed_ms(&self) -> u64 {
        0
    }
    pub fn finish(self) -> Result<Finished, String> {
        Err(UNSUPPORTED.into())
    }
}

pub struct Preview;

impl Preview {
    pub fn stop(self) {}
}

pub fn start_preview(_app: tauri::AppHandle, _mode: CaptureMode, _index: usize) -> Result<Preview, String> {
    Err(UNSUPPORTED.into())
}

pub fn capture_thumbnail(_mode: CaptureMode, _index: usize) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

pub fn video_thumbnail(_path: &str) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

pub fn render_project(
    _project: &velo_core::Project,
    _video_path: &std::path::Path,
    _out_path: &std::path::Path,
    _trim_start_ms: u64,
    _trim_end_ms: u64,
    _on_progress: impl Fn(f32),
) -> Result<(), String> {
    Err(UNSUPPORTED.into())
}
