//! Turns a raw recording plus its project (zoom plan, cursor track) into a finished video:
//! crops/scales each frame to the auto-zoom camera, redraws the cursor when it was left out of
//! the raw footage, and encodes the result. Runs entirely on this thread; the caller (a Tauri
//! command) is expected to run it off the main thread and report `on_progress` back to the UI.

use super::decode::Decoder;
use std::path::Path;
use velo_core::{config::even, Project};
use velo_director::{camera_path, smooth_cursor, Point, Rect};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder, VideoSettingsSubType,
};

/// Renders `project` (whose raw video lives at `video_path`) to `out_path`, keeping only
/// `[trim_start_ms, trim_end_ms)` of it. `on_progress` is called with a 0.0..=1.0 fraction after
/// each frame.
///
/// Video only for now: the raw recording's audio is not yet carried into the export. Background,
/// frame padding/shadow and cursor styling aren't applied yet either — only trim, the auto-zoom
/// camera and (when the raw cursor was hidden) a redrawn cursor.
pub fn render(
    project: &Project,
    video_path: &Path,
    out_path: &Path,
    trim_start_ms: u64,
    trim_end_ms: u64,
    on_progress: impl Fn(f32),
) -> Result<(), String> {
    let mut decoder = Decoder::open(video_path)?;
    let screen = project.screen;
    let fps = project.fps.clamp(1, 120);
    let trim_end_ms = trim_end_ms.min(project.duration_ms).max(trim_start_ms + 1);
    let all_frames = camera_path(&project.zoom, &project.events, screen, project.duration_ms, fps, &project.zoom_config);
    let all_cursor =
        project.cursor_hidden.then(|| smooth_cursor(&project.events, project.duration_ms, fps, 30.0));

    let frame_ms = 1000.0 / fps as f64;
    let first = ((trim_start_ms as f64) / frame_ms).floor() as usize;
    let last = (((trim_end_ms as f64) / frame_ms).ceil() as usize).min(all_frames.len());
    let frames = &all_frames[first.min(all_frames.len())..last];

    let (out_w, out_h) = (even(screen.w.round() as u32).max(2), even(screen.h.round() as u32).max(2));
    let mut encoder = VideoEncoder::new(
        VideoSettingsBuilder::new(out_w, out_h).sub_type(VideoSettingsSubType::H264).bitrate(12_000_000).frame_rate(fps),
        AudioSettingsBuilder::default().disabled(true),
        ContainerSettingsBuilder::default(),
        out_path,
    )
    .map_err(|e| e.to_string())?;

    let (src_w, src_h) = (decoder.width(), decoder.height());
    let total = frames.len().max(1);
    let mut canvas = vec![0u8; (out_w as usize) * (out_h as usize) * 4];
    for (out_i, cam) in frames.iter().enumerate() {
        let src_i = first + out_i;
        let src_t_100ns = (src_i as f64 * frame_ms * 10_000.0) as i64;
        let out_t_100ns = (out_i as f64 * frame_ms * 10_000.0) as i64;
        let src = decoder.frame_at(src_t_100ns)?;
        let view = cam.viewport(screen);
        resample(src, src_w, src_h, view, out_w, out_h, &mut canvas);
        if let Some(p) = all_cursor.as_ref().and_then(|t| t.get(src_i)) {
            draw_cursor(&mut canvas, out_w, out_h, *p, view);
        }

        encoder.send_frame_buffer(&canvas, out_t_100ns).map_err(|e| e.to_string())?;
        on_progress((out_i + 1) as f32 / total as f32);
    }
    encoder.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// Nearest-neighbour crop+scale of `view` (in source pixels) from `src` onto the full `out`
/// canvas. Good enough for a zoomed-in screen recording; a fancier filter can follow later.
fn resample(src: &[u8], src_w: u32, src_h: u32, view: Rect, out_w: u32, out_h: u32, out: &mut [u8]) {
    let (src_w, src_h) = (src_w as i64, src_h as i64);
    let scale_x = view.w / out_w as f64;
    let scale_y = view.h / out_h as f64;
    for y in 0..out_h {
        let sy = ((view.y + y as f64 * scale_y) as i64).clamp(0, src_h - 1);
        let out_row = (y as usize) * (out_w as usize) * 4;
        let src_row = (sy as usize) * (src_w as usize) * 4;
        for x in 0..out_w {
            let sx = ((view.x + x as f64 * scale_x) as i64).clamp(0, src_w - 1);
            let si = src_row + (sx as usize) * 4;
            let di = out_row + (x as usize) * 4;
            out[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
}

/// Burns a simple filled circle at the cursor's position, scaled the same way the camera zoomed
/// the frame, for recordings where the real cursor was left out of the raw footage.
fn draw_cursor(out: &mut [u8], out_w: u32, out_h: u32, cursor: Point, view: Rect) {
    let scale = out_w as f64 / view.w;
    let cx = (cursor.x - view.x) * scale;
    let cy = (cursor.y - view.y) * scale;
    let r = (9.0 * scale).clamp(4.0, 40.0);
    let (x0, x1) = ((cx - r).floor().max(0.0) as i64, (cx + r).ceil().min(out_w as f64) as i64);
    let (y0, y1) = ((cy - r).floor().max(0.0) as i64, (cy + r).ceil().min(out_h as f64) as i64);
    for y in y0..y1 {
        for x in x0..x1 {
            let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r {
                continue;
            }
            let i = ((y as usize) * (out_w as usize) + x as usize) * 4;
            // A thin dark ring with a white fill reads on any background.
            let (b, g, red) = if dist > r - 1.6 { (20u8, 20u8, 20u8) } else { (255u8, 255u8, 255u8) };
            out[i] = b;
            out[i + 1] = g;
            out[i + 2] = red;
            out[i + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::engine::win::Recording;
    use crate::engine::LevelsSink;
    use std::sync::Arc;
    use std::time::Duration;
    use velo_core::{CaptureMode, Project, StartConfig};

    /// Exercises the real pipeline end to end: captures ~2s of the real screen, then renders it.
    /// Not run by default (`cargo test -- --ignored`) since it needs a live desktop session.
    #[test]
    #[ignore]
    fn captures_and_renders_a_real_clip() {
        let cfg = StartConfig {
            mode: CaptureMode::Display,
            studio: true,
            cursor: true,
            system: false,
            mic: false,
            camera: false,
            fps: 30,
            folder: std::env::temp_dir().to_string_lossy().into_owned(),
            ..Default::default()
        };
        let levels: LevelsSink = Arc::new(|_| {});
        let recording = Recording::start(cfg, levels).expect("recording should start");
        std::thread::sleep(Duration::from_secs(2));
        let finished = recording.finish().expect("recording should finish");
        assert!(finished.seconds >= 1, "expected at least 1s captured, got {}", finished.seconds);

        let project_path = finished.project_path.clone().expect("project should have been saved");
        let project = Project::load(std::path::Path::new(&project_path)).expect("project should load");
        let out_path = std::env::temp_dir().join("velo-render-test-output.mp4");

        render(&project, std::path::Path::new(&finished.path), &out_path, 0, finished.seconds * 1000, |_| {})
            .expect("render should succeed");

        let meta = std::fs::metadata(&out_path).expect("output file should exist");
        assert!(meta.len() > 1000, "output file suspiciously small: {} bytes", meta.len());

        let _ = std::fs::remove_file(&finished.path);
        let _ = std::fs::remove_file(&project_path);
        let _ = std::fs::remove_file(&out_path);
    }
}
