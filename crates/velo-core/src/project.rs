use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use velo_director::{InputEvent, Size, ZoomConfig, ZoomSegment};

/// Saved next to every recording as `<name>.velo.json`.
///
/// The video is untouched footage. This file holds everything needed to re-render it with zoom,
/// a smooth cursor and a background, so edits are non-destructive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    /// File name of the video, relative to this file.
    pub video: String,
    pub screen: Size,
    pub fps: u32,
    pub duration_ms: u64,
    /// Input events are stamped from the clock start; the first video frame arrives a little later.
    /// Subtract this from event times to line them up with the video.
    pub video_offset_ms: u64,
    /// True when the cursor was left out of the video so it can be redrawn.
    pub cursor_hidden: bool,
    pub studio: bool,
    pub system_audio: bool,
    pub mic: bool,
    pub zoom: Vec<ZoomSegment>,
    pub zoom_config: ZoomConfig,
    pub events: Vec<InputEvent>,
}

pub const VERSION: u32 = 1;

impl Project {
    /// Builds the project and plans the automatic zoom from the recorded events.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video: String,
        screen: Size,
        fps: u32,
        duration_ms: u64,
        video_offset_ms: u64,
        cursor_hidden: bool,
        studio: bool,
        system_audio: bool,
        mic: bool,
        mut events: Vec<InputEvent>,
    ) -> Self {
        // Event times become video times, and anything before the first frame is dropped.
        events.retain(|e| e.t_ms >= video_offset_ms);
        for e in &mut events {
            e.t_ms -= video_offset_ms;
        }
        let zoom_config = ZoomConfig::default();
        let zoom = velo_director::plan(&events, duration_ms, &zoom_config);
        Self {
            version: VERSION,
            video,
            screen,
            fps,
            duration_ms,
            video_offset_ms,
            cursor_hidden,
            studio,
            system_audio,
            mic,
            zoom,
            zoom_config,
            events,
        }
    }

    /// Where the project file for a video lives.
    pub fn path_for(video: &Path) -> PathBuf {
        video.with_extension("velo.json")
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        // Write then rename so a crash never leaves a half-written project beside a good video.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, path)
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(std::io::Error::other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velo_director::{Button, InputKind};

    fn click(t: u64) -> InputEvent {
        InputEvent { t_ms: t, x: 10.0, y: 20.0, kind: InputKind::Down { button: Button::Left } }
    }

    fn sample(events: Vec<InputEvent>) -> Project {
        Project::new("a.mp4".into(), Size { w: 1920.0, h: 1080.0 }, 60, 10_000, 120, true, true, true, true, events)
    }

    #[test]
    fn event_times_are_shifted_onto_the_video_timeline() {
        let p = sample(vec![click(100), click(3_120)]);
        assert_eq!(p.events.len(), 1, "an event before the first frame is dropped");
        assert_eq!(p.events[0].t_ms, 3_000);
    }

    #[test]
    fn zoom_is_planned_from_the_shifted_events() {
        let p = sample(vec![click(3_120)]);
        assert_eq!(p.zoom.len(), 1);
        assert_eq!(p.zoom[0].start_ms, 3_000 - p.zoom_config.lead_ms);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("velo-core-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mp4");
        let path = Project::path_for(&video);
        assert!(path.to_string_lossy().ends_with("clip.velo.json"));
        let p = sample(vec![click(5_000)]);
        p.save(&path).unwrap();
        assert_eq!(Project::load(&path).unwrap(), p);
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
