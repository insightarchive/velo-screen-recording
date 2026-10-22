use crate::types::{InputEvent, InputKind, ZoomConfig, ZoomSegment};

/// Moments worth zooming into: a click is a point in time, typing is a span.
struct Interest {
    start_ms: u64,
    end_ms: u64,
}

fn interests(events: &[InputEvent], cfg: &ZoomConfig) -> Vec<Interest> {
    let mut out: Vec<Interest> = events
        .iter()
        .filter(|e| matches!(e.kind, InputKind::Down { .. }))
        .map(|e| Interest { start_ms: e.t_ms, end_ms: e.t_ms })
        .collect();

    // Typing: runs of key presses where each key follows the previous within the window.
    let keys: Vec<u64> = events.iter().filter(|e| matches!(e.kind, InputKind::Key)).map(|e| e.t_ms).collect();
    let mut run_start = 0;
    for i in 0..=keys.len() {
        let run_ends = i == keys.len() || (i > run_start && keys[i] - keys[i - 1] > cfg.typing_window_ms);
        if run_ends && i > run_start {
            if i - run_start >= cfg.typing_min_keys {
                out.push(Interest { start_ms: keys[run_start], end_ms: keys[i - 1] });
            }
            run_start = i;
        }
    }
    out.sort_by_key(|i| i.start_ms);
    out
}

/// Builds the automatic zoom segments for a recording. The editor can then adjust them.
pub fn plan(events: &[InputEvent], duration_ms: u64, cfg: &ZoomConfig) -> Vec<ZoomSegment> {
    let scale = cfg.scale.clamp(1.1, 6.0);
    let mut segments: Vec<ZoomSegment> = Vec::new();

    for it in interests(events, cfg) {
        let start = it.start_ms.saturating_sub(cfg.lead_ms);
        let end = (it.end_ms + cfg.hold_ms).min(duration_ms.max(it.end_ms));
        match segments.last_mut() {
            Some(prev) if start <= prev.end_ms + cfg.merge_gap_ms => prev.end_ms = prev.end_ms.max(end),
            _ => segments.push(ZoomSegment { start_ms: start, end_ms: end, scale, anchor: None }),
        }
    }
    segments
}
