use serde::{Deserialize, Serialize};
use velo_director::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    #[default]
    Display,
    Window,
    Region,
    Camera,
}

/// What the UI sends when the user presses Record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StartConfig {
    pub mode: CaptureMode,
    pub display: usize,
    pub window: usize,
    pub region: Option<Rect>,
    pub system: bool,
    pub mic: bool,
    pub mic_device: usize,
    /// Microphone volume, 0 to 100.
    pub mic_gain: u32,
    pub camera: bool,
    pub cursor: bool,
    pub fps: u32,
    /// "Low", "Balanced" or "High".
    pub bitrate: String,
    /// Studio mode keeps the raw video plus input events so zoom and effects stay editable.
    pub studio: bool,
    pub hide_window: bool,
    pub folder: String,
    /// File name pattern with `{date}` and `{time}`.
    pub pattern: String,
}

impl Default for StartConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::Display,
            display: 0,
            window: 0,
            region: None,
            system: true,
            mic: true,
            mic_device: 0,
            mic_gain: 80,
            camera: false,
            cursor: true,
            fps: 60,
            bitrate: "Balanced".into(),
            studio: true,
            hide_window: true,
            folder: String::new(),
            pattern: "Velo {date} {time}".into(),
        }
    }
}

impl StartConfig {
    /// Frame rate clamped to what the encoder and capture API handle well.
    pub fn fps(&self) -> u32 {
        self.fps.clamp(15, 120)
    }

    /// Target video bitrate. Studio mode would normally record near-lossless footage for a later
    /// re-encode, but that export pipeline doesn't exist yet, so for now it only gets a modest
    /// bump over "High" rather than the full near-lossless rate, to keep files a reasonable size.
    pub fn bitrate_bps(&self, width: u32, height: u32) -> u32 {
        let bits_per_pixel = if self.studio {
            0.11
        } else {
            match self.bitrate.as_str() {
                "Low" => 0.05,
                "High" => 0.16,
                _ => 0.09,
            }
        };
        let raw = width as f64 * height as f64 * self.fps() as f64 * bits_per_pixel;
        raw.clamp(4_000_000.0, 120_000_000.0) as u32
    }

    /// File name without extension, with the pattern expanded and unsafe characters removed.
    pub fn file_stem(&self, date: &str, time: &str) -> String {
        let name = self.pattern.replace("{date}", date).replace("{time}", time);
        let clean: String = name
            .chars()
            .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { '-' } else { c })
            .collect();
        let clean = clean.trim().trim_end_matches('.').to_string();
        if clean.is_empty() { format!("Velo {date} {time}") } else { clean }
    }
}

/// H.264 needs even dimensions.
pub fn even(n: u32) -> u32 {
    n & !1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_accepts_what_the_ui_sends_and_fills_the_rest() {
        let c: StartConfig = serde_json::from_str(r#"{"mode":"window","window":2,"fps":30,"mic":false}"#).unwrap();
        assert_eq!(c.mode, CaptureMode::Window);
        assert_eq!((c.window, c.fps, c.mic), (2, 30, false));
        assert!(c.system && c.studio, "defaults apply to missing fields");
    }

    #[test]
    fn bitrate_scales_with_pixels_and_is_clamped() {
        let mut c = StartConfig { studio: false, ..Default::default() };
        let hd = c.bitrate_bps(1920, 1080);
        assert!((10_000_000..20_000_000).contains(&hd), "{hd}");
        assert!(c.bitrate_bps(3840, 2160) > hd);
        assert_eq!(c.bitrate_bps(100, 100), 4_000_000);
        c.studio = true;
        assert!(c.bitrate_bps(1920, 1080) > hd, "studio footage is higher quality");
        assert_eq!(c.bitrate_bps(7680, 4320), 120_000_000);
    }

    #[test]
    fn file_names_are_safe() {
        let c = StartConfig { pattern: "Demo: {date}/{time}?".into(), ..Default::default() };
        assert_eq!(c.file_stem("2026-10-21", "17-04-11"), "Demo- 2026-10-21-17-04-11-");
        let empty = StartConfig { pattern: "  ".into(), ..Default::default() };
        assert_eq!(empty.file_stem("d", "t"), "Velo d t");
    }

    #[test]
    fn dimensions_are_made_even() {
        assert_eq!((even(1919), even(1080), even(1)), (1918, 1080, 0));
    }

    #[test]
    fn fps_is_clamped() {
        assert_eq!(StartConfig { fps: 500, ..Default::default() }.fps(), 120);
        assert_eq!(StartConfig { fps: 1, ..Default::default() }.fps(), 15);
    }
}
