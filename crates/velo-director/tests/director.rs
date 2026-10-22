use velo_director::*;

const SCREEN: Size = Size { w: 1920.0, h: 1080.0 };

fn click(t_ms: u64, x: f64, y: f64) -> InputEvent {
    InputEvent { t_ms, x, y, kind: InputKind::Down { button: Button::Left } }
}
fn mv(t_ms: u64, x: f64, y: f64) -> InputEvent {
    InputEvent { t_ms, x, y, kind: InputKind::Move }
}
fn key(t_ms: u64) -> InputEvent {
    InputEvent { t_ms, x: 500.0, y: 500.0, kind: InputKind::Key }
}

#[test]
fn no_activity_means_no_zoom() {
    let events: Vec<_> = (0..50).map(|i| mv(i * 100, 100.0 + i as f64, 200.0)).collect();
    assert!(plan(&events, 5_000, &ZoomConfig::default()).is_empty());
}

#[test]
fn a_click_creates_a_segment_with_lead_and_hold() {
    let cfg = ZoomConfig::default();
    let segs = plan(&[click(3_000, 800.0, 400.0)], 10_000, &cfg);
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].start_ms, 3_000 - cfg.lead_ms);
    assert_eq!(segs[0].end_ms, 3_000 + cfg.hold_ms);
    assert_eq!(segs[0].anchor, None);
}

#[test]
fn a_click_near_the_start_never_underflows() {
    let segs = plan(&[click(100, 10.0, 10.0)], 10_000, &ZoomConfig::default());
    assert_eq!(segs[0].start_ms, 0);
}

#[test]
fn nearby_clicks_merge_and_distant_clicks_do_not() {
    let cfg = ZoomConfig::default();
    let merged = plan(&[click(3_000, 1.0, 1.0), click(4_500, 2.0, 2.0)], 20_000, &cfg);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].end_ms, 4_500 + cfg.hold_ms);

    let apart = plan(&[click(3_000, 1.0, 1.0), click(12_000, 2.0, 2.0)], 20_000, &cfg);
    assert_eq!(apart.len(), 2);
    assert!(apart[0].end_ms < apart[1].start_ms);
}

#[test]
fn segment_end_is_clamped_to_the_recording() {
    let segs = plan(&[click(9_500, 1.0, 1.0)], 10_000, &ZoomConfig::default());
    assert_eq!(segs[0].end_ms, 10_000);
}

#[test]
fn typing_zooms_but_a_stray_key_does_not() {
    let cfg = ZoomConfig::default();
    let typing: Vec<_> = (0..6).map(|i| key(2_000 + i * 150)).collect();
    let segs = plan(&typing, 20_000, &cfg);
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].end_ms, 2_000 + 5 * 150 + cfg.hold_ms);

    assert!(plan(&[key(1_000), key(9_000)], 20_000, &cfg).is_empty());
}

#[test]
fn unsorted_events_give_the_same_plan() {
    let a = [click(8_000, 1.0, 1.0), click(2_000, 2.0, 2.0)];
    let b = [click(2_000, 2.0, 2.0), click(8_000, 1.0, 1.0)];
    assert_eq!(plan(&a, 20_000, &ZoomConfig::default()), plan(&b, 20_000, &ZoomConfig::default()));
}

#[test]
fn camera_starts_full_zooms_toward_the_click_and_returns() {
    let cfg = ZoomConfig::default();
    let events = vec![mv(0, 1500.0, 300.0), click(3_000, 1500.0, 300.0), mv(9_000, 1500.0, 300.0)];
    let segs = plan(&events, 10_000, &cfg);
    let path = camera_path(&segs, &events, SCREEN, 10_000, 60, &cfg);
    assert_eq!(path.len(), 600);

    assert_eq!(path[0], Camera::full(SCREEN));
    let mid = path[(3.6 * 60.0) as usize];
    assert!(mid.scale > 1.8, "zoomed in, scale={}", mid.scale);
    assert!(mid.cx > 1300.0 && mid.cy < 400.0, "aimed at the click, got {mid:?}");
    let end = path[599];
    assert!(end.scale < 1.05, "eased back out, scale={}", end.scale);
}

#[test]
fn camera_never_leaves_the_screen_even_for_corner_clicks() {
    let cfg = ZoomConfig::default();
    let corners = [(1.0, 1.0), (1919.0, 1.0), (1.0, 1079.0), (1919.0, 1079.0)];
    for (i, (x, y)) in corners.into_iter().enumerate() {
        let events = vec![mv(0, x, y), click(1_000, x, y)];
        let segs = plan(&events, 6_000, &cfg);
        for cam in camera_path(&segs, &events, SCREEN, 6_000, 60, &cfg) {
            let v = cam.viewport(SCREEN);
            let eps = 1e-6;
            assert!(v.x >= -eps && v.y >= -eps, "corner {i}: {v:?}");
            assert!(v.x + v.w <= SCREEN.w + eps && v.y + v.h <= SCREEN.h + eps, "corner {i}: {v:?}");
        }
    }
}

#[test]
fn small_cursor_movement_does_not_pan_but_large_movement_does() {
    let cfg = ZoomConfig::default();
    let segs = [ZoomSegment { start_ms: 0, end_ms: 10_000, scale: 2.0, anchor: None }];

    let small: Vec<_> = (0..100).map(|i| mv(i * 100, 960.0 + (i % 5) as f64 * 8.0, 540.0)).collect();
    let path = camera_path(&segs, &small, SCREEN, 10_000, 60, &cfg);
    let settled = &path[300..];
    let spread = settled.iter().map(|c| c.cx).fold(f64::MIN, f64::max) - settled.iter().map(|c| c.cx).fold(f64::MAX, f64::min);
    assert!(spread < 2.0, "camera stayed still, spread={spread}");

    let big = vec![mv(0, 600.0, 540.0), mv(5_000, 600.0, 540.0), mv(6_000, 1400.0, 540.0), mv(10_000, 1400.0, 540.0)];
    let path = camera_path(&segs, &big, SCREEN, 10_000, 60, &cfg);
    assert!(path[599].cx > path[200].cx + 300.0, "camera followed the cursor right");
}

#[test]
fn an_anchored_segment_ignores_the_cursor() {
    let cfg = ZoomConfig::default();
    let segs = [ZoomSegment { start_ms: 0, end_ms: 6_000, scale: 2.5, anchor: Some(Point::new(400.0, 300.0)) }];
    let events = vec![mv(0, 1700.0, 900.0), mv(6_000, 1700.0, 900.0)];
    let path = camera_path(&segs, &events, SCREEN, 6_000, 60, &cfg);
    let last = path[359];
    assert!(last.cx < 600.0 && last.cy < 450.0, "{last:?}");
}

#[test]
fn camera_motion_is_smooth() {
    let cfg = ZoomConfig::default();
    let events = vec![mv(0, 200.0, 200.0), click(1_000, 200.0, 200.0), mv(3_000, 1700.0, 900.0), click(3_100, 1700.0, 900.0)];
    let segs = plan(&events, 8_000, &cfg);
    let path = camera_path(&segs, &events, SCREEN, 8_000, 60, &cfg);
    let max_jump = path.windows(2).map(|w| (w[1].cx - w[0].cx).abs().max((w[1].cy - w[0].cy).abs())).fold(0.0, f64::max);
    assert!(max_jump < 60.0, "no teleporting between frames, max jump={max_jump}");
}

#[test]
fn camera_path_is_deterministic() {
    let cfg = ZoomConfig::default();
    let events = vec![mv(0, 10.0, 10.0), click(2_000, 900.0, 500.0)];
    let segs = plan(&events, 6_000, &cfg);
    assert_eq!(
        camera_path(&segs, &events, SCREEN, 6_000, 60, &cfg),
        camera_path(&segs, &events, SCREEN, 6_000, 60, &cfg)
    );
}

#[test]
fn spring_converges_without_overshoot() {
    let mut s = Spring::new(0.0);
    let mut peak: f64 = 0.0;
    for _ in 0..600 {
        s.step(100.0, 8.0, 1.0 / 60.0);
        peak = peak.max(s.pos);
    }
    assert!((s.pos - 100.0).abs() < 0.01);
    assert!(peak <= 100.0 + 1e-9, "critically damped, peak={peak}");
}

#[test]
fn spring_is_stable_with_huge_timesteps() {
    let mut s = Spring::new(0.0);
    s.step(50.0, 40.0, 5.0);
    assert!(s.pos.is_finite() && s.pos > 0.0 && s.pos <= 50.0);
}

#[test]
fn cursor_track_interpolates_between_samples() {
    let track = CursorTrack::new(&[mv(0, 0.0, 0.0), mv(1_000, 100.0, 200.0)]);
    let p = track.at(500).unwrap();
    assert!((p.x - 50.0).abs() < 1e-9 && (p.y - 100.0).abs() < 1e-9);
    assert_eq!(track.at(5_000).unwrap(), Point::new(100.0, 200.0));
    assert!(CursorTrack::new(&[]).at(0).is_none());
}

#[test]
fn smoothed_cursor_tracks_the_real_one_closely() {
    let events: Vec<_> = (0..=100).map(|i| mv(i * 50, i as f64 * 10.0, 300.0)).collect();
    let smooth = smooth_cursor(&events, 5_000, 60, 30.0);
    assert_eq!(smooth.len(), 300);
    let i = 240;
    let real = i as f64 / 60.0 * 200.0;
    assert!((smooth[i].x - real).abs() < 40.0, "lag small: {} vs {}", smooth[i].x, real);
}

#[test]
fn events_round_trip_through_json_for_the_ipc_layer() {
    let ev = click(10, 1.0, 2.0);
    let json = serde_json::to_string(&ev).unwrap();
    assert!(json.contains("\"kind\":\"down\""), "{json}");
    assert_eq!(serde_json::from_str::<InputEvent>(&json).unwrap(), ev);
}
