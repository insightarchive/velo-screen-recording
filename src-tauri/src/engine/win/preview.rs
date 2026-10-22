//! A low-rate JPEG preview of the selected display or window, streamed to the UI before a
//! recording starts (and while choosing a source), so "Preview" shows the real screen instead of
//! a placeholder.

use super::capture::Error;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use velo_core::CaptureMode;
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{ImageEncoder, ImageEncoderPixelFormat, ImageFormat};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings,
    SecondaryWindowSettings, Settings,
};

/// ~8 fps: plenty to feel live, cheap enough to run continuously while a user is just browsing sources.
const INTERVAL: Duration = Duration::from_millis(120);

pub struct Flags {
    app: AppHandle,
}

pub struct Handler {
    app: AppHandle,
    encoder: ImageEncoder,
    last: Instant,
}

impl GraphicsCaptureApiHandler for Handler {
    type Flags = Flags;
    type Error = Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let encoder = ImageEncoder::new(ImageFormat::Jpeg, ImageEncoderPixelFormat::Rgba8)?;
        Ok(Self { app: ctx.flags.app, encoder, last: Instant::now() - INTERVAL })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _control: InternalCaptureControl) -> Result<(), Self::Error> {
        if self.last.elapsed() < INTERVAL {
            return Ok(());
        }
        self.last = Instant::now();

        let buffer = frame.buffer()?;
        let mut scratch = Vec::new();
        let bytes = buffer.as_nopadding_buffer(&mut scratch);
        let jpeg = self.encoder.encode(bytes, buffer.width(), buffer.height())?;
        let data_url = format!("data:image/jpeg;base64,{}", BASE64.encode(jpeg));
        let _ = self.app.emit(
            "preview-frame",
            serde_json::json!({ "dataUrl": data_url, "width": buffer.width(), "height": buffer.height() }),
        );
        Ok(())
    }
}

pub struct Preview {
    control: CaptureControl<Handler, Error>,
}

impl Preview {
    pub fn stop(self) {
        let _ = self.control.stop();
    }
}

fn settings<T>(item: T, flags: Flags) -> Settings<Flags, T>
where
    T: TryInto<windows_capture::settings::GraphicsCaptureItemType>,
{
    Settings::new(
        item,
        CursorCaptureSettings::WithCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Custom(INTERVAL),
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        flags,
    )
}

// ---------- One-shot thumbnails, for the source picker list ----------

struct ThumbFlags {
    tx: std::sync::mpsc::Sender<String>,
}

struct ThumbHandler {
    tx: std::sync::mpsc::Sender<String>,
    encoder: ImageEncoder,
    sent: bool,
}

impl GraphicsCaptureApiHandler for ThumbHandler {
    type Flags = ThumbFlags;
    type Error = Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let encoder = ImageEncoder::new(ImageFormat::Jpeg, ImageEncoderPixelFormat::Rgba8)?;
        Ok(Self { tx: ctx.flags.tx, encoder, sent: false })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, control: InternalCaptureControl) -> Result<(), Self::Error> {
        if self.sent {
            return Ok(());
        }
        self.sent = true;
        let buffer = frame.buffer()?;
        let mut scratch = Vec::new();
        let bytes = buffer.as_nopadding_buffer(&mut scratch);
        let jpeg = self.encoder.encode(bytes, buffer.width(), buffer.height())?;
        let _ = self.tx.send(format!("data:image/jpeg;base64,{}", BASE64.encode(jpeg)));
        control.stop();
        Ok(())
    }
}

fn thumb_settings<T>(item: T, flags: ThumbFlags) -> Settings<ThumbFlags, T>
where
    T: TryInto<windows_capture::settings::GraphicsCaptureItemType>,
{
    Settings::new(
        item,
        CursorCaptureSettings::WithoutCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        flags,
    )
}

/// A single JPEG snapshot of a display or window, for the source picker list (not live).
pub fn thumbnail(mode: CaptureMode, index: usize) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let flags = ThumbFlags { tx };
    let control = match mode {
        CaptureMode::Display => {
            let monitors = Monitor::enumerate().map_err(|e| e.to_string())?;
            let monitor = *monitors.get(index).ok_or("That display is no longer available.")?;
            ThumbHandler::start_free_threaded(thumb_settings(monitor, flags))
                .map_err(|e| format!("Could not capture a thumbnail: {e}"))?
        }
        CaptureMode::Window => {
            let windows = super::sources::capturable_windows();
            let window = *windows.get(index).ok_or("That window is no longer available.")?;
            ThumbHandler::start_free_threaded(thumb_settings(window, flags))
                .map_err(|e| format!("Could not capture a thumbnail: {e}"))?
        }
        CaptureMode::Region | CaptureMode::Camera => return Err("This capture mode is coming soon.".into()),
    };

    let data_url = rx
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Timed out waiting for a frame.".to_string());
    let _ = control.stop();
    data_url
}

pub fn start(app: AppHandle, mode: CaptureMode, index: usize) -> Result<Preview, String> {
    let flags = Flags { app };
    match mode {
        CaptureMode::Display => {
            let monitors = Monitor::enumerate().map_err(|e| e.to_string())?;
            let monitor = *monitors.get(index).ok_or("That display is no longer available.")?;
            let control =
                Handler::start_free_threaded(settings(monitor, flags)).map_err(|e| format!("Could not start preview: {e}"))?;
            Ok(Preview { control })
        }
        CaptureMode::Window => {
            let windows = super::sources::capturable_windows();
            let window = *windows.get(index).ok_or("That window is no longer available.")?;
            let control =
                Handler::start_free_threaded(settings(window, flags)).map_err(|e| format!("Could not start preview: {e}"))?;
            Ok(Preview { control })
        }
        CaptureMode::Region | CaptureMode::Camera => Err("This capture mode is coming soon.".into()),
    }
}
