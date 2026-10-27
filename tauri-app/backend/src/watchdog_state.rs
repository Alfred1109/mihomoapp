use std::time::{Duration, Instant};

const RESTART_WINDOW: Duration = Duration::from_secs(60);
const MAX_RESTART_ATTEMPTS: u32 = 5;

/// Runtime intent and recovery policy are kept together so a manual stop cannot
/// race with a recovery decision or overwrite the user's recovery preference.
#[derive(Debug)]
pub(super) struct WatchdogState {
    pub(super) process_id: Option<u32>,
    pub(super) auto_restart: bool,
    desired_running: bool,
    restart_count: u32,
    window_start: Option<Instant>,
}

impl Default for WatchdogState {
    fn default() -> Self {
        Self {
            process_id: None,
            auto_restart: true,
            desired_running: false,
            restart_count: 0,
            window_start: None,
        }
    }
}

impl WatchdogState {
    pub(super) fn started(&mut self, pid: u32) {
        self.process_id = Some(pid);
        self.desired_running = true;
        self.reset_attempts();
    }

    pub(super) fn stopped(&mut self) -> Option<u32> {
        let process_id = self.process_id.take();
        self.desired_running = false;
        self.reset_attempts();
        process_id
    }

    fn reset_attempts(&mut self) {
        self.restart_count = 0;
        self.window_start = None;
    }

    pub(super) fn reserve_restart(
        &mut self,
        healthy: bool,
        process_exited: bool,
        now: Instant,
    ) -> Option<u32> {
        if healthy && !process_exited {
            self.reset_attempts();
            return None;
        }
        // PID zero represents a system service. Its service manager owns recovery;
        // spawning a standalone process here would create a second lifecycle owner.
        if !process_exited
            || !self.desired_running
            || !self.auto_restart
            || self.process_id == Some(0)
        {
            return None;
        }
        if self
            .window_start
            .is_none_or(|start| now.duration_since(start) >= RESTART_WINDOW)
        {
            self.window_start = Some(now);
            self.restart_count = 0;
        }
        if self.restart_count >= MAX_RESTART_ATTEMPTS {
            return None;
        }
        self.restart_count += 1;
        Some(self.restart_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_preserves_user_preference_but_suppresses_recovery() {
        let mut state = WatchdogState::default();
        state.started(12);
        assert_eq!(state.stopped(), Some(12));
        assert_eq!(state.process_id, None);
        assert!(state.auto_restart);
        assert_eq!(state.reserve_restart(false, true, Instant::now()), None);
        state.started(13);
        assert_eq!(state.reserve_restart(false, true, Instant::now()), Some(1));
        state.auto_restart = false;
        state.stopped();
        state.started(14);
        assert_eq!(state.reserve_restart(false, true, Instant::now()), None);
    }

    #[test]
    fn retries_persistent_failure_with_a_bounded_window() {
        let mut state = WatchdogState::default();
        state.started(12);
        let now = Instant::now();
        for attempt in 1..=MAX_RESTART_ATTEMPTS {
            assert_eq!(state.reserve_restart(false, true, now), Some(attempt));
        }
        assert_eq!(state.reserve_restart(false, true, now), None);
        assert_eq!(
            state.reserve_restart(false, true, now + RESTART_WINDOW),
            Some(1)
        );
    }

    #[test]
    fn observed_health_resets_failure_budget() {
        let mut state = WatchdogState::default();
        state.started(12);
        let now = Instant::now();
        assert_eq!(state.reserve_restart(false, true, now), Some(1));
        assert_eq!(state.reserve_restart(true, false, now), None);
        assert_eq!(state.reserve_restart(false, true, now), Some(1));
    }

    #[test]
    fn unhealthy_api_does_not_restart_a_live_owned_process() {
        let mut state = WatchdogState::default();
        state.started(12);
        let now = Instant::now();
        for _ in 0..10 {
            assert_eq!(state.reserve_restart(false, false, now), None);
        }
        assert_eq!(state.process_id, Some(12));
        assert_eq!(state.reserve_restart(false, true, now), Some(1));
    }

    #[test]
    fn never_recovers_unmanaged_or_service_processes() {
        let mut state = WatchdogState::default();
        let now = Instant::now();
        assert_eq!(state.reserve_restart(false, true, now), None);
        state.started(0);
        assert_eq!(state.reserve_restart(false, true, now), None);
    }
}
