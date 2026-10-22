use crate::cursor::{frame_count, CursorTrack};
use crate::spring::Spring;
use crate::types::{Camera, InputEvent, Point, Size, ZoomConfig, ZoomSegment};

/// Computes the camera for every video frame from the zoom segments and cursor movement.
///
/// Inside a segment the camera zooms toward the cursor and only pans once the cursor leaves
/// a dead zone, so small hand movements do not shake the picture. Outside segments it eases
/// back to the full screen. The visible area never leaves the screen.
pub fn camera_path(
    segments: &[ZoomSegment],
    events: &[InputEvent],
    screen: Size,
    duration_ms: u64,
    fps: u32,
    cfg: &ZoomConfig,
) -> Vec<Camera> {
    let track = CursorTrack::new(events);
    let dt = 1.0 / fps as f64;
    let full = Camera::full(screen);

    let (mut cx, mut cy, mut sc) = (Spring::new(full.cx), Spring::new(full.cy), Spring::new(1.0));
    let mut target = Point::new(full.cx, full.cy);
    let mut was_active = false;

    (0..frame_count(duration_ms, fps))
        .map(|i| {
            let t = (i as f64 * dt * 1000.0) as u64;
            let seg = segments.iter().find(|s| t >= s.start_ms && t < s.end_ms);

            let target_scale = match seg {
                Some(s) => s.scale.max(1.0),
                None => 1.0,
            };

            match seg {
                Some(s) => {
                    let cursor = s.anchor.or_else(|| track.at(t)).unwrap_or(Point::new(full.cx, full.cy));
                    if !was_active {
                        // Aim straight at where the action is when a zoom begins.
                        target = cursor;
                    } else if s.anchor.is_some() {
                        target = cursor;
                    } else {
                        let half_w = screen.w / sc.pos / 2.0 * cfg.deadzone;
                        let half_h = screen.h / sc.pos / 2.0 * cfg.deadzone;
                        target.x = target.x.clamp(cursor.x - half_w, cursor.x + half_w);
                        target.y = target.y.clamp(cursor.y - half_h, cursor.y + half_h);
                    }
                }
                None => target = Point::new(full.cx, full.cy),
            }
            was_active = seg.is_some();

            sc.step(target_scale, cfg.zoom_omega, dt);
            cx.step(target.x, cfg.pan_omega, dt);
            cy.step(target.y, cfg.pan_omega, dt);

            let scale = sc.pos.max(1.0);
            let half_w = screen.w / scale / 2.0;
            let half_h = screen.h / scale / 2.0;
            Camera {
                cx: cx.pos.clamp(half_w, screen.w - half_w),
                cy: cy.pos.clamp(half_h, screen.h - half_h),
                scale,
            }
        })
        .collect()
}
