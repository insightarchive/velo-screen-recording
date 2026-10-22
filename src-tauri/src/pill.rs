//! The small floating bar shown while recording. It is its own always-on-top window and is
//! excluded from screen capture, so it never appears in the video.

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "pill";
const WIDTH: f64 = 410.0;
const HEIGHT: f64 = 92.0;

pub fn show(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("pill.html".into()))
        .title("Velo Recorder")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .shadow(false)
        .build();
    let Ok(window) = built else { return };

    // Top centre of the main display.
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let x = monitor.position().x as f64 + (monitor.size().width as f64 - WIDTH * scale) / 2.0;
        let y = monitor.position().y as f64 + 20.0 * scale;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    exclude_from_capture(&window);
}

pub fn close(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
}

const LAUNCHER_LABEL: &str = "launcher";
/// Collapsed: just the bar. The source picker grows the window from JS (`resize_window`) when opened.
const LAUNCHER_WIDTH: f64 = 480.0;
const LAUNCHER_HEIGHT: f64 = 92.0;

/// Opens the launcher, or closes it if it's already open. Does nothing while a recording is live;
/// the recording bar owns that spot then.
pub fn toggle_launcher(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    if app.get_webview_window(LAUNCHER_LABEL).is_some() {
        close_launcher(app);
    } else {
        show_launcher(app);
    }
}

fn show_launcher(app: &AppHandle) {
    let built = WebviewWindowBuilder::new(app, LAUNCHER_LABEL, WebviewUrl::App("launcher.html".into()))
        .title("Velo Recorder")
        .inner_size(LAUNCHER_WIDTH, LAUNCHER_HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .focused(true)
        .shadow(false)
        .build();
    let Ok(window) = built else { return };

    // Top centre of the main display, same spot as the recording bar.
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let x = monitor.position().x as f64 + (monitor.size().width as f64 - LAUNCHER_WIDTH * scale) / 2.0;
        let y = monitor.position().y as f64 + 20.0 * scale;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    exclude_from_capture(&window);
}

pub fn close_launcher(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LAUNCHER_LABEL) {
        let _ = w.close();
    }
}

const COUNTDOWN_LABEL: &str = "countdown";
const COUNTDOWN_SIZE: f64 = 320.0;

/// A click-through overlay with a big number, centred on the screen, so the countdown before
/// recording is visible wherever you are instead of tucked in a small pill. Built hidden and only
/// shown once it's click-through, so there's no window under the cursor that can catch input.
pub fn show_countdown(app: &AppHandle) {
    if app.get_webview_window(COUNTDOWN_LABEL).is_some() {
        return;
    }
    let Ok(Some(monitor)) = app.primary_monitor() else { return };
    let built = WebviewWindowBuilder::new(app, COUNTDOWN_LABEL, WebviewUrl::App("countdown.html".into()))
        .title("Velo Recorder")
        .inner_size(COUNTDOWN_SIZE, COUNTDOWN_SIZE)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .shadow(false)
        .visible(false)
        .build();
    let Ok(window) = built else { return };

    let scale = monitor.scale_factor();
    let x = monitor.position().x as f64 + (monitor.size().width as f64 - COUNTDOWN_SIZE * scale) / 2.0;
    let y = monitor.position().y as f64 + (monitor.size().height as f64 - COUNTDOWN_SIZE * scale) / 2.0;
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.set_ignore_cursor_events(true);
    exclude_from_capture(&window);
    let _ = window.show();
}

pub fn close_countdown(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(COUNTDOWN_LABEL) {
        let _ = w.close();
    }
}

#[cfg(windows)]
fn exclude_from_capture(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};
    // Tauri may use a different `windows` version, so pass the handle across as a plain pointer.
    if let Ok(hwnd) = window.hwnd() {
        let raw = hwnd.0;
        let _ = unsafe { SetWindowDisplayAffinity(HWND(raw), WDA_EXCLUDEFROMCAPTURE) };
    }
}

#[cfg(not(windows))]
fn exclude_from_capture(_: &tauri::WebviewWindow) {}
