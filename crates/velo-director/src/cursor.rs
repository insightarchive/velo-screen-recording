use crate::spring::Spring;
use crate::types::{InputEvent, InputKind, Point};

/// Cursor position at time `t_ms`, linearly interpolated between recorded samples.
pub struct CursorTrack {
    samples: Vec<(u64, Point)>,
}

impl CursorTrack {
    pub fn new(events: &[InputEvent]) -> Self {
        let mut samples: Vec<(u64, Point)> = events.iter().map(|e| (e.t_ms, Point::new(e.x, e.y))).collect();
        samples.sort_by_key(|s| s.0);
        Self { samples }
    }

    pub fn at(&self, t_ms: u64) -> Option<Point> {
        let first = self.samples.first()?;
        if t_ms <= first.0 {
            return Some(first.1);
        }
        let i = self.samples.partition_point(|s| s.0 <= t_ms);
        let (t0, p0) = self.samples[i - 1];
        match self.samples.get(i) {
            Some(&(t1, p1)) if t1 > t0 => {
                let k = (t_ms - t0) as f64 / (t1 - t0) as f64;
                Some(Point::new(p0.x + (p1.x - p0.x) * k, p0.y + (p1.y - p0.y) * k))
            }
            _ => Some(p0),
        }
    }
}

/// Re-draws the cursor path one point per video frame, with jitter removed.
/// `omega` around 30 keeps the cursor within a frame or two of the real position.
pub fn smooth_cursor(events: &[InputEvent], duration_ms: u64, fps: u32, omega: f64) -> Vec<Point> {
    let moves: Vec<InputEvent> = events
        .iter()
        .copied()
        .filter(|e| matches!(e.kind, InputKind::Move | InputKind::Down { .. } | InputKind::Up { .. }))
        .collect();
    let track = CursorTrack::new(&moves);
    let Some(start) = track.at(0) else { return Vec::new() };
    let dt = 1.0 / fps as f64;
    let frames = frame_count(duration_ms, fps);
    let (mut x, mut y) = (Spring::new(start.x), Spring::new(start.y));
    (0..frames)
        .map(|i| {
            let t = (i as f64 * dt * 1000.0) as u64;
            let target = track.at(t).unwrap_or(start);
            x.step(target.x, omega, dt);
            y.step(target.y, omega, dt);
            Point::new(x.pos, y.pos)
        })
        .collect()
}

pub(crate) fn frame_count(duration_ms: u64, fps: u32) -> usize {
    (duration_ms as f64 / 1000.0 * fps as f64).ceil() as usize
}
