use serde::{Deserialize, Serialize};

/// Size of the captured screen in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputKind {
    Move,
    Down { button: Button },
    Up { button: Button },
    Scroll { dx: f64, dy: f64 },
    Key,
}

/// One recorded input event. `x`/`y` is always the cursor position at that moment,
/// even for key events, so the camera knows where the user was looking.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InputEvent {
    pub t_ms: u64,
    pub x: f64,
    pub y: f64,
    #[serde(flatten)]
    pub kind: InputKind,
}

/// A time range where the camera zooms in. Segments are plain data so the editor can
/// move, resize, retarget or delete them, and the camera path is rebuilt from them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ZoomSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub scale: f64,
    /// `None` follows the cursor. `Some` pins the camera to a fixed point.
    pub anchor: Option<Point>,
}

/// Where the camera looks: the centre of the visible area and the zoom factor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub cx: f64,
    pub cy: f64,
    pub scale: f64,
}

impl Camera {
    pub fn full(screen: Size) -> Self {
        Self { cx: screen.w / 2.0, cy: screen.h / 2.0, scale: 1.0 }
    }

    /// The part of the screen currently visible.
    pub fn viewport(&self, screen: Size) -> Rect {
        let w = screen.w / self.scale;
        let h = screen.h / self.scale;
        Rect { x: self.cx - w / 2.0, y: self.cy - h / 2.0, w, h }
    }
}

/// Tunables for [`crate::plan`] and [`crate::camera_path`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ZoomConfig {
    /// Zoom factor used for new segments.
    pub scale: f64,
    /// How long before an action the camera starts moving in.
    pub lead_ms: u64,
    /// How long the camera stays after the last action.
    pub hold_ms: u64,
    /// Segments closer than this are merged into one continuous zoom.
    pub merge_gap_ms: u64,
    /// Keys needed within `typing_window_ms` to count as typing.
    pub typing_min_keys: usize,
    pub typing_window_ms: u64,
    /// Fraction of the half-viewport the cursor may roam before the camera follows (0..1).
    pub deadzone: f64,
    /// Camera centre responsiveness (rad/s, higher is snappier).
    pub pan_omega: f64,
    /// Zoom responsiveness (rad/s).
    pub zoom_omega: f64,
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            scale: 2.0,
            lead_ms: 450,
            hold_ms: 1800,
            merge_gap_ms: 1200,
            typing_min_keys: 3,
            typing_window_ms: 1200,
            deadzone: 0.45,
            pan_omega: 7.0,
            zoom_omega: 6.0,
        }
    }
}
