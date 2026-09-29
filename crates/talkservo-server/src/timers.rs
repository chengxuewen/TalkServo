//! Timer decisions (plan-2 T4). All state lives in the room's single-writer
//! task; these are pure helpers + the wiring loop. Intervals from Config
//! (modules/06). Tests drive the same code paths with short intervals.

use crate::config::Config;
use crate::obs;
use talkservo_core::ids::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// E11 watch state per peer (room-task-owned).
#[derive(Default)]
pub struct WatchState {
    /// PeerId → first-seen-silent instant (None = currently active).
    silent_since: HashMap<PeerId, Instant>,
}

impl WatchState {
    /// Probe result feeds here: `active` clears grace, `silent` starts/keeps it.
    /// Returns peers whose grace has fully elapsed → MediaDown candidates.
    pub fn probe(
        &mut self,
        results: impl IntoIterator<Item = (PeerId, bool)>, // (peer, is_active)
        grace: Duration,
        now: Instant,
    ) -> Vec<PeerId> {
        let mut down = Vec::new();
        for (peer, is_active) in results {
            if is_active {
                self.silent_since.remove(&peer);
            } else {
                let since = *self.silent_since.entry(peer.clone()).or_insert(now);
                if now.duration_since(since) >= grace {
                    down.push(peer);
                }
            }
        }
        down
    }

    pub fn forget(&mut self, peer: &PeerId) {
        self.silent_since.remove(peer);
    }
}

/// Proactive TokenRefresh: push at expiry − 5 min (modules/02 §1; plan-2 #5).
pub fn refresh_due(jwt_issued_at_s: u64, cfg: &Config, now_s: u64) -> bool {
    let lead = 300u64.min(cfg.jwt_ttl_s);
    now_s >= jwt_issued_at_s + cfg.jwt_ttl_s.saturating_sub(lead)
}

/// Heartbeat cadence for the sweeper loop.
pub fn heartbeat_interval(cfg: &Config) -> Duration {
    Duration::from_secs(cfg.heartbeat_s)
}

/// Max-hold for the floor holder (dispatch-mode cap; None = off).
pub fn max_hold_interval(cfg: &Config) -> Option<Duration> {
    cfg.floor_max_hold_ms.map(Duration::from_millis)
}

/// Acceptance-#7/#10 metric line.
pub fn log_grant_latency(room: &str, peer: &str, generation: u64, ms: u64) {
    obs::latency(room, peer, generation, "grant_latency_ms", ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config::for_test("t")
    }

    #[test]
    fn watch_state_grace_then_down() {
        let mut w = WatchState::default();
        let t0 = Instant::now();
        let grace = Duration::from_millis(2000);

        // first silent probe starts grace, not yet down
        let down = w.probe([(PeerId::from("a"), false)], grace, t0);
        assert!(down.is_empty());

        // active probe clears grace for a; b starts its own
        let down = w.probe(
            [(PeerId::from("a"), true), (PeerId::from("b"), false)],
            grace,
            t0 + Duration::from_millis(100),
        );
        assert!(down.is_empty());

        // b silent past grace → down; a re-silenced but its grace restarted at
        // the clear point, so only elapsed-since-restart counts
        let down = w.probe(
            [(PeerId::from("a"), false), (PeerId::from("b"), false)],
            grace,
            t0 + Duration::from_millis(2100),
        );
        assert_eq!(down, vec![PeerId::from("b")]);

        w.forget(&PeerId::from("a"));
        let down = w.probe(
            [(PeerId::from("a"), false)],
            grace,
            t0 + Duration::from_millis(2200),
        );
        assert!(down.is_empty(), "grace restarted after forget");
    }

    #[test]
    fn refresh_due_at_five_minute_lead() {
        let c = cfg();
        let issued = 1_000_000;
        assert!(!refresh_due(issued, &c, issued + 100));
        assert!(refresh_due(issued, &c, issued + c.jwt_ttl_s - 300));
        assert!(refresh_due(issued, &c, issued + c.jwt_ttl_s));
    }

    #[test]
    fn intervals_from_config() {
        let c = cfg();
        assert_eq!(heartbeat_interval(&c), Duration::from_secs(30));
        assert_eq!(max_hold_interval(&c), Some(Duration::from_millis(45_000)));
        // open mode: FLOOR_MAX_HOLD_MS=off → None
        let mut open = c.clone();
        open.floor_max_hold_ms = None;
        assert_eq!(max_hold_interval(&open), None);
    }
}
