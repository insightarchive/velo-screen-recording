use super::Shared;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use velo_core::{config::even, CaptureMode, StartConfig};
use velo_director::{Point, Size};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO};
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder, VideoSettingsSubType,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings,
    SecondaryWindowSettings, Settings,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct Flags {
    shared: Arc<Shared>,
    path: std::path::PathBuf,
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u32,
    audio: bool,
}

pub struct Handler {
    shared: Arc<Shared>,
}

impl GraphicsCaptureApiHandler for Handler {
    type Flags = Flags;
    type Error = Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let f = ctx.flags;
        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(f.width, f.height)
                .sub_type(VideoSettingsSubType::H264)
                .bitrate(f.bitrate)
                .frame_rate(f.fps),
            AudioSettingsBuilder::default().disabled(!f.audio),
            ContainerSettingsBuilder::default(),
            &f.path,
        )?;
        *f.shared.encoder.lock() = Some(encoder);
        Ok(Self { shared: f.shared })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _control: InternalCaptureControl) -> Result<(), Self::Error> {
        if self.shared.stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        if let Some(encoder) = self.shared.encoder.lock().as_mut() {
            self.shared.mark_first_frame();
            encoder.send_frame(frame)?;
        }
        Ok(())
    }
}

pub struct Started {
    pub control: CaptureControl<Handler, Error>,
    pub screen: Size,
    /// Screen position of the captured area's top-left corner, to make mouse positions relative to it.
    pub origin: Point,
}

pub fn start(cfg: &StartConfig, shared: &Arc<Shared>, path: &Path) -> Result<Started, String> {
    // Studio mode would normally leave the cursor out so the editor can redraw it smoothly, but
    // that redraw pipeline doesn't exist yet, so burn it in whenever the user wants a cursor at all.
    let cursor = if cfg.cursor { CursorCaptureSettings::WithCursor } else { CursorCaptureSettings::WithoutCursor };
    let make_flags = |w: u32, h: u32| {
        let (width, height) = (even(w), even(h));
        Flags {
            shared: shared.clone(),
            path: path.to_path_buf(),
            width,
            height,
            fps: cfg.fps(),
            bitrate: cfg.bitrate_bps(width, height),
            audio: cfg.system || cfg.mic,
        }
    };

    match cfg.mode {
        CaptureMode::Display => {
            let monitors = Monitor::enumerate().map_err(|e| e.to_string())?;
            let monitor = *monitors.get(cfg.display).ok_or("That display is no longer available.")?;
            let (w, h) = (monitor.width().map_err(err)?, monitor.height().map_err(err)?);
            let origin = monitor_origin(&monitor);
            let settings = settings(monitor, cursor, make_flags(w, h));
            let control = Handler::start_free_threaded(settings).map_err(|e| format!("Could not start capture: {e}"))?;
            Ok(Started { control, screen: Size { w: w as f64, h: h as f64 }, origin })
        }
        CaptureMode::Window => {
            let windows = super::sources::capturable_windows();
            let window = *windows.get(cfg.window).ok_or("That window is no longer available.")?;
            let rect = window.rect().map_err(err)?;
            let (w, h) = ((rect.right - rect.left).max(2) as u32, (rect.bottom - rect.top).max(2) as u32);
            let origin = Point::new(rect.left as f64, rect.top as f64);
            let settings = settings(window, cursor, make_flags(w, h));
            let control = Handler::start_free_threaded(settings).map_err(|e| format!("Could not start capture: {e}"))?;
            Ok(Started { control, screen: Size { w: w as f64, h: h as f64 }, origin })
        }
        CaptureMode::Region | CaptureMode::Camera => Err("This capture mode is coming soon.".into()),
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn settings<T>(item: T, cursor: CursorCaptureSettings, flags: Flags) -> Settings<Flags, T>
where
    T: TryInto<windows_capture::settings::GraphicsCaptureItemType>,
{
    Settings::new(
        item,
        cursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        flags,
    )
}

fn monitor_origin(monitor: &Monitor) -> Point {
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let ok = unsafe { GetMonitorInfoW(HMONITOR(monitor.as_raw_hmonitor()), &mut info) }.as_bool();
    if ok { Point::new(info.rcMonitor.left as f64, info.rcMonitor.top as f64) } else { Point::new(0.0, 0.0) }
}
