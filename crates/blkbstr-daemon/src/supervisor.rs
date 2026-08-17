//! Restart policy for an engine that exits by itself.
//!
//! Separated from [`crate::engine`] because the decision — restart, or give up and stay down — is
//! the part worth testing, and it needs no process, no clock and no filesystem to test.
//!
//! Giving up matters as much as restarting. Each restart rewrites the nftables ruleset, so an
//! engine that dies immediately on a bad config would otherwise thrash the firewall forever while
//! the user watches their network flap.

use std::time::{Duration, Instant};

/// Restarts allowed inside [`WINDOW`] before the supervisor stops trying.
const MAX_RESTARTS: usize = 5;
const WINDOW: Duration = Duration::from_secs(60);
/// Delay before each restart, indexed by how many have already happened in the window. Capped
/// rather than unbounded: a user waiting on their network is not served by a five-minute backoff.
const BACKOFF: [Duration; MAX_RESTARTS] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(15),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Restart, once this much time has passed since the exit.
    RestartAfter(Duration),
    /// Too many restarts too quickly. The engine stays down and the reason stands.
    GiveUp { restarts: usize, window: Duration },
}

/// Restart history within the sliding window.
#[derive(Debug, Default)]
pub struct Supervisor {
    restarts: Vec<Instant>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets the history. Called on a deliberate start or stop, so a user fixing their config is
    /// not still being judged on the crash loop from before the fix.
    pub fn reset(&mut self) {
        self.restarts.clear();
    }

    pub fn restarts_in_window(&mut self, now: Instant) -> usize {
        self.restarts.retain(|t| now.duration_since(*t) < WINDOW);
        self.restarts.len()
    }

    /// Call when the engine has been found dead. Records the attempt when it says to restart.
    pub fn on_exit(&mut self, now: Instant) -> Decision {
        let recent = self.restarts_in_window(now);
        if recent >= MAX_RESTARTS {
            return Decision::GiveUp {
                restarts: recent,
                window: WINDOW,
            };
        }
        self.restarts.push(now);
        Decision::RestartAfter(BACKOFF[recent])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backs_off_further_on_each_restart() {
        let mut s = Supervisor::new();
        let t = Instant::now();
        let delays: Vec<_> = (0..MAX_RESTARTS).map(|_| s.on_exit(t)).collect();
        assert_eq!(
            delays,
            BACKOFF
                .iter()
                .map(|d| Decision::RestartAfter(*d))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn gives_up_after_a_crash_loop() {
        let mut s = Supervisor::new();
        let t = Instant::now();
        for _ in 0..MAX_RESTARTS {
            s.on_exit(t);
        }
        assert!(matches!(s.on_exit(t), Decision::GiveUp { .. }));
    }

    #[test]
    fn an_old_crash_does_not_count_against_a_later_one() {
        let mut s = Supervisor::new();
        let long_ago = Instant::now();
        for _ in 0..MAX_RESTARTS {
            s.on_exit(long_ago);
        }
        // An engine that ran fine for an hour and then died gets the full allowance again.
        let now = long_ago + WINDOW + Duration::from_secs(1);
        assert_eq!(s.on_exit(now), Decision::RestartAfter(BACKOFF[0]));
    }

    #[test]
    fn a_deliberate_start_forgives_the_previous_loop() {
        let mut s = Supervisor::new();
        let t = Instant::now();
        for _ in 0..MAX_RESTARTS {
            s.on_exit(t);
        }
        s.reset();
        assert_eq!(s.on_exit(t), Decision::RestartAfter(BACKOFF[0]));
    }
}
