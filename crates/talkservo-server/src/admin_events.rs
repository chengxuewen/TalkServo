//! Admin-plane push channel (plan-9 W1).
//!
//! Why a second endpoint and not `/ws`: `AdminClaims` deliberately carries no
//! `room` field (see `auth.rs`), and `/ws` decodes the room-mandatory `Claims`, so
//! an admin token structurally cannot join a room session — and must not be able
//! to, even if both secrets were ever configured equal. That audience isolation is
//! what makes `/admin/ws` the *non-breaking* shape: the signaling contract stays
//! at its 27 variants (plan-7 §4 untouched), and this plane evolves behind its own
//! `v`.
//!
//! Topology is one producer, two sinks. `AuditTx::emit` is the only producer of
//! activity, and it drives:
//!   * the JSONL writer — the ledger; `seq` is stamped there and stays there;
//!   * this hub — a HINT only. A row's content never crosses the socket
//!     (plan-9 R3=B), so the Logs table keeps exactly one code path that can put a
//!     line into it, and the two sinks cannot race over sequence numbers.
//!
//! A hint reaching the hub means "at least one tab is connected and got it". An
//! event with no subscribers is not a loss (nobody has the console open), so
//! `dropped` counts only tabs we had to evict for falling behind the bound —
//! otherwise every audited action in a closed-console deployment would look like a
//! failure.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;

use crate::ws::App;

/// Admin-plane contract version — independent of the signaling `v: 1`.
pub const ADMIN_EVENT_V: u32 = 1;

/// Per-subscriber backlog before it is judged slow. Bounded on purpose: a console
/// tab that stops reading must lose *its own* socket, never grow the server or
/// delay the ledger (plan-9 §3.3).
pub const HUB_CAPACITY: usize = 64;

/// Coalescing window for the shared rooms snapshot: one burst of activity costs
/// one fan-out, not one per tab. A debounce is a silent surface by definition, so
/// `tests/admin_events.rs` measures it instead of trusting it.
pub const ROOMS_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(100);

/// How long an unauthenticated socket may take to present its credential.
pub const AUTH_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

/// Cap on the auth frame itself (a JWT is ~200 B; the room is for headroom, not
/// for hosting a file upload).
pub const AUTH_FRAME_MAX_BYTES: usize = 4096;

/// `/admin/ws` connect budget per minute, on its OWN window (plan-9 R5=A). It is
/// deliberately separate from the REST `admin_rate`: reconnect backoff from a few
/// tabs is normal traffic and must never be able to bounce the operator out of
/// login or token issuance with a 401 that points at the wrong cause. Still
/// bounded, because a wedged page looping `new WebSocket()` is the misconfig.
pub const ADMIN_WS_CONNECTS_PER_MIN: u32 = 120;

/// Server → console frames.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdminEvent {
    /// first frame after the credential is accepted; `dropped` is the hub's
    /// eviction count, so "a tab stopped keeping up" is visible on the next
    /// connect instead of being silent.
    /// `debounce_ms` is the server's live coalescing window, advertised so the
    /// console never hard-codes a number this file can change under it.
    Hello {
        v: u32,
        dropped: u64,
        debounce_ms: u64,
    },
    /// "a line reached the ledger" — content comes from `/admin/api/audit`.
    AuditHint { v: u32 },
    /// the `/admin/api/rooms/live` body, pushed instead of polled. Monitor applies
    /// it immediately; the reconcile poll bounds any loss, so a missed snapshot is
    /// a delay, never a lie.
    RoomsLive {
        v: u32,
        rooms: Vec<serde_json::Value>,
    },
    /// answer to a client `ping`. A browser cannot send WS control pings (the
    /// WebSocket API exposes none), so liveness has to be an application frame or
    /// the client cannot tell a dead socket from a quiet room.
    Pong { v: u32 },
}

/// Console → server frames. `Auth` must be FIRST and ONLY-FIRST: a browser cannot
/// put a header on a WebSocket, and the token must not ride the query string
/// either (access logs and browser history would keep a copy of a credential —
/// the same rule that keeps tokens out of audit lines).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdminIn {
    Auth {
        token: String,
    },
    /// accepted and ignored: both streams are always on, so there is nothing to
    /// negotiate yet. Present so a client need not guess whether to send it.
    Subscribe,
    Ping,
}

/// Bounded fan-out shared by every `/admin/ws` subscriber.
#[derive(Debug)]
pub struct EventHub {
    tx: broadcast::Sender<AdminEvent>,
    dropped: AtomicU64,
}

impl Default for EventHub {
    fn default() -> Self {
        Self {
            tx: broadcast::channel(HUB_CAPACITY).0,
            dropped: AtomicU64::new(0),
        }
    }
}

impl EventHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AdminEvent> {
        self.tx.subscribe()
    }

    /// Tabs this server had to evict for falling behind `HUB_CAPACITY`.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn note_eviction(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn hello(&self) -> AdminEvent {
        AdminEvent::Hello {
            v: ADMIN_EVENT_V,
            dropped: self.dropped(),
            debounce_ms: ROOMS_DEBOUNCE.as_millis() as u64,
        }
    }

    pub fn pong(&self) -> AdminEvent {
        AdminEvent::Pong { v: ADMIN_EVENT_V }
    }

    /// Fire-and-forget from `AuditTx::emit`: the audited write is already queued by
    /// the time this runs, and a console that cannot keep up must never be able to
    /// block, delay or fail it.
    pub fn hint(&self) {
        self.publish(AdminEvent::AuditHint { v: ADMIN_EVENT_V });
    }

    pub fn rooms(&self, rooms: Vec<serde_json::Value>) {
        self.publish(AdminEvent::RoomsLive {
            v: ADMIN_EVENT_V,
            rooms,
        });
    }

    fn publish(&self, ev: AdminEvent) {
        // tokio's broadcast reports lag to the RECEIVER, never to the sender:
        // `send` only fails once every receiver is gone — and that case is "nobody is
        // watching", not "we owed someone an event". So nothing is counted here: a
        // tab that falls behind counts itself when it observes `Lagged` (the socket
        // task and the refresher both do), and the number surfaces in the next Hello.
        let _ = self.tx.send(ev);
    }
}

/// One snapshot task for the whole admin plane: activity -> coalesce -> a single
/// fan-out -> `RoomsLive` to every tab. It subscribes like any client, so its own
/// backlog cannot starve a console tab (broadcast receivers are independent) and
/// it can never amplify a burst into one fan-out per tab.
pub fn spawn_rooms_refresher(hub: Arc<EventHub>, app: Arc<App>) {
    tokio::spawn(async move {
        let mut rx = hub.subscribe();
        loop {
            match rx.recv().await {
                // only ledger activity means the room table may have moved; our
                // own RoomsLive echoes back here and must not re-trigger it
                Ok(AdminEvent::AuditHint { .. }) => {}
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    hub.note_eviction();
                    continue;
                }
                Err(_) => break, // hub dropped: nothing left to push to
            }
            absorb(&mut rx);
            tokio::time::sleep(ROOMS_DEBOUNCE).await;
            absorb(&mut rx);
            hub.rooms(crate::admin::rooms_body(&app).await);
            absorb(&mut rx);
        }
    });
}

/// Drop the refresher's own backlog. Hints collapse into one snapshot; a `Pong`
/// is answered per-connection in `admin.rs` and never needs the hub, so nothing
/// a console waits for is discarded here.
fn absorb(rx: &mut broadcast::Receiver<AdminEvent>) {
    while rx.try_recv().is_ok() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_with_no_tab_open_is_not_a_loss() {
        let hub = EventHub::new();
        for _ in 0..5 {
            hub.hint();
        }
        // If this ever counts, `dropped` stops meaning "we owed a tab an event" and
        // the console warning becomes noise on every quiet deployment.
        assert_eq!(hub.dropped(), 0);
    }

    #[test]
    fn a_subscriber_sees_every_frame_and_keeps_its_own_cursor() {
        let hub = EventHub::new();
        let mut rx = hub.subscribe();
        hub.hint();
        hub.rooms(vec![serde_json::json!({ "room": "r1" })]);
        assert!(matches!(rx.try_recv(), Ok(AdminEvent::AuditHint { .. })));
        assert!(matches!(rx.try_recv(), Ok(AdminEvent::RoomsLive { .. })));
        assert!(rx.try_recv().is_err());
        assert_eq!(hub.dropped(), 0);
    }

    #[test]
    fn a_tab_that_falls_behind_lags_and_then_is_counted() {
        let hub = EventHub::new();
        let mut slow = hub.subscribe();
        // one frame past the bound: the publisher never blocks and never counts,
        // because lag is reported to the receiver — the only side that knows
        for _ in 0..=HUB_CAPACITY {
            hub.hint();
        }
        assert_eq!(hub.dropped(), 0, "publishing is fire-and-forget");
        assert!(
            matches!(
                slow.try_recv(),
                Err(broadcast::error::TryRecvError::Lagged(_))
            ),
            "the slow tab must learn it lagged rather than replay stale frames"
        );
        // the socket's own Lagged arm (admin.rs) is what turns that into a visible
        // number, and Hello carries it to the next connection
        hub.note_eviction();
        assert!(matches!(
            hub.hello(),
            AdminEvent::Hello {
                v: ADMIN_EVENT_V,
                dropped: 1,
                ..
            }
        ));
    }

    #[test]
    fn absorb_collapses_a_burst_into_one_snapshot() {
        // The refresher's coalescing is precisely this drain. Proving it here is
        // deterministic; timing a live burst over HTTP is not — and without it, a
        // join wave would cost one fan-out per hint.
        let hub = EventHub::new();
        let mut rx = hub.subscribe();
        for _ in 0..5 {
            hub.hint();
        }
        absorb(&mut rx);
        assert!(
            rx.try_recv().is_err(),
            "the burst must be absorbed, not replayed one snapshot per hint"
        );
        assert_eq!(hub.dropped(), 0, "coalescing is not the same as losing");
    }

    #[test]
    fn hello_advertises_the_window_the_server_actually_runs() {
        match EventHub::new().hello() {
            AdminEvent::Hello { debounce_ms, .. } => assert_eq!(
                debounce_ms,
                ROOMS_DEBOUNCE.as_millis() as u64,
                "the advertised number is the compiled number"
            ),
            other => panic!("expected hello, got {other:?}"),
        }
    }

    #[test]
    fn hello_carries_the_cumulative_eviction_count() {
        let hub = EventHub::new();
        hub.note_eviction();
        hub.note_eviction();
        assert!(matches!(
            hub.hello(),
            AdminEvent::Hello {
                v: ADMIN_EVENT_V,
                dropped: 2,
                ..
            }
        ));
    }

    #[test]
    fn wire_shape_is_snake_case_tagged_with_the_plane_version() {
        let json = serde_json::to_string(&AdminEvent::AuditHint { v: ADMIN_EVENT_V }).unwrap();
        assert_eq!(json, r#"{"type":"audit_hint","v":1}"#);
        let live = serde_json::to_string(&AdminEvent::RoomsLive {
            v: 1,
            rooms: vec![],
        })
        .unwrap();
        assert!(live.contains(r#""type":"rooms_live""#), "{live}");
        assert!(live.contains(r#""rooms":[]"#), "{live}");
        // inbound frames the console sends must parse with the same naming law
        let auth: AdminIn = serde_json::from_str(r#"{"type":"auth","token":"x"}"#).unwrap();
        assert!(matches!(auth, AdminIn::Auth { token } if token == "x"));
        assert!(serde_json::from_str::<AdminIn>(r#"{"type":"ping"}"#).is_ok());
        assert!(serde_json::from_str::<AdminIn>(r#"{"type":"subscribe"}"#).is_ok());
    }
}
