use crate::engine::{SourceInfo, Sources};
use wasapi::{initialize_mta, DeviceEnumerator, Direction};
use windows_capture::monitor::Monitor;
use windows_capture::window::Window;

/// Windows worth offering for capture. The UI and `start` must agree on this list and its order.
pub fn capturable_windows() -> Vec<Window> {
    Window::enumerate()
        .unwrap_or_default()
        .into_iter()
        .filter(|w| w.is_valid() && w.title().is_ok_and(|t| !t.trim().is_empty() && t != "Velo Recorder"))
        .collect()
}

pub fn list_sources() -> Sources {
    let primary = Monitor::primary().ok().map(|m| m.as_raw_hmonitor() as usize);
    let displays = Monitor::enumerate()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let (w, h) = (m.width().unwrap_or(0), m.height().unwrap_or(0));
            let main = if Some(m.as_raw_hmonitor() as usize) == primary { " (main)" } else { "" };
            SourceInfo { name: m.name().unwrap_or_else(|_| format!("Display {}", i + 1)), detail: format!("{w} x {h}{main}") }
        })
        .collect();

    let windows = capturable_windows()
        .iter()
        .map(|w| SourceInfo { name: w.title().unwrap_or_default(), detail: w.process_name().unwrap_or_default() })
        .collect();

    Sources { displays, windows, mics: device_names(Direction::Capture, "Default microphone"), speakers: device_names(Direction::Render, "Default speakers") }
}

fn device_names(direction: Direction, default_label: &str) -> Vec<String> {
    let _ = initialize_mta();
    let mut names = vec![default_label.to_string()];
    let Ok(enumerator) = DeviceEnumerator::new() else { return names };
    let Ok(collection) = enumerator.get_device_collection(&direction) else { return names };
    let count = collection.get_nbr_devices().unwrap_or(0);
    for i in 0..count {
        if let Some(name) = collection.get_device_at_index(i).ok().and_then(|d| d.get_friendlyname().ok()) {
            names.push(name);
        }
    }
    names
}
