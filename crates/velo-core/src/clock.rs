use std::time::{Duration, Instant};

/// The recording timeline. It runs while recording and stops while paused, so everything
/// stamped with it (input events, audio) lines up with the video that has paused time cut out.
#[derive(Debug, Clone)]
pub struct Clock {
    start: Instant,
    paused_total: Duration,
    paused_at: Option<Instant>,
}

impl Clock {
    pub fn new(now: Instant) -> Self {
        Self { start: now, paused_total: Duration::ZERO, paused_at: None }
    }

    pub fn is_paused(&self) -> bool {
        self.paused_at.is_some()
    }

    pub fn pause(&mut self, now: Instant) {
        self.paused_at.get_or_insert(now);
    }

    pub fn resume(&mut self, now: Instant) {
        if let Some(at) = self.paused_at.take() {
            self.paused_total += now.saturating_duration_since(at);
        }
    }

    /// Time recorded so far, not counting paused time.
    pub fn active(&self, now: Instant) -> Duration {
        let end = self.paused_at.unwrap_or(now);
        end.saturating_duration_since(self.start).saturating_sub(self.paused_total)
    }

    pub fn active_ms(&self, now: Instant) -> u64 {
        self.active(now).as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_time_is_not_counted() {
        let t0 = Instant::now();
        let mut c = Clock::new(t0);
        assert_eq!(c.active_ms(t0 + Duration::from_secs(2)), 2_000);

        c.pause(t0 + Duration::from_secs(2));
        assert!(c.is_paused());
        assert_eq!(c.active_ms(t0 + Duration::from_secs(10)), 2_000, "frozen while paused");

        c.resume(t0 + Duration::from_secs(10));
        assert_eq!(c.active_ms(t0 + Duration::from_secs(13)), 5_000);
    }

    #[test]
    fn double_pause_and_resume_are_harmless() {
        let t0 = Instant::now();
        let mut c = Clock::new(t0);
        c.pause(t0 + Duration::from_secs(1));
        c.pause(t0 + Duration::from_secs(5));
        c.resume(t0 + Duration::from_secs(6));
        c.resume(t0 + Duration::from_secs(9));
        assert_eq!(c.active_ms(t0 + Duration::from_secs(7)), 2_000);
    }
}
