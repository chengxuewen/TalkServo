//! Observability bootstrap (modules/05 §Obs) — JSON logs, closed event vocab.
//! No metrics server in PoC (ponytail): acceptance reads measurements from logs.

use serde_json::json;

/// Closed event vocabulary (modules/05 §Obs — parsers rely on this set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Join,
    Leave,
    FloorRequest,
    Grant,
    Deny,
    Taken,
    Release,
    IdleTimeout,
    ModeChange,
    TransportCreate,
    TransportFail,
    ProduceOk,
    ConsumeOk,
    WorkerExited,
    MediaRestartDone,
    RateLimited,
}

impl Event {
    pub fn as_str(&self) -> &'static str {
        match self {
            Event::Join => "join",
            Event::Leave => "leave",
            Event::FloorRequest => "floor_request",
            Event::Grant => "grant",
            Event::Deny => "deny",
            Event::Taken => "taken",
            Event::Release => "release",
            Event::IdleTimeout => "idle_timeout",
            Event::ModeChange => "mode_change",
            Event::TransportCreate => "transport_create",
            Event::TransportFail => "transport_fail",
            Event::ProduceOk => "produce_ok",
            Event::ConsumeOk => "consume_ok",
            Event::WorkerExited => "worker_exited",
            Event::MediaRestartDone => "media_restart_done",
            Event::RateLimited => "rate_limited",
        }
    }
}

/// Emit one protocol-event log line with the mandatory room/peer/gen fields
/// (modules/05 §1 principle: every line carries them).
pub fn event(room: &str, peer: &str, generation: u64, ev: Event, extra: serde_json::Value) {
    let line = json!({
        "level": "info",
        "event": ev.as_str(),
        "room": room,
        "peer": peer,
        "gen": generation,
        "detail": extra,
    });
    println!("{line}");
}

/// Denial/warn-class line (E8/#1 security surface — warn level per policy).
pub fn warn_event(room: &str, peer: &str, generation: u64, ev: Event, detail: &str) {
    let line = json!({
        "level": "warn",
        "event": ev.as_str(),
        "room": room,
        "peer": peer,
        "gen": generation,
        "detail": detail,
    });
    eprintln!("{line}");
}

/// Latency measurement log (acceptance #7/#10 read these).
pub fn latency(room: &str, peer: &str, generation: u64, metric: &str, ms: u64) {
    let line = json!({
        "level": "info",
        "event": "latency",
        "metric": metric,
        "ms": ms,
        "room": room,
        "peer": peer,
        "gen": generation,
    });
    println!("{line}");
    // R-F21: tests assert against this ring — stdout capture is unreliable
    // under the test harness, the ring is deterministic. Bounded: latency
    // lines are low-frequency (one per grant), 1k entries never overflow
    // meaningfully; tests drain it.
    if let Some(mut ring) = LATENCY_RING.lock().ok().filter(|r| r.len() < 1024) {
        ring.push(LatencyRecord {
            metric: metric.to_string(),
            room: room.to_string(),
            peer: peer.to_string(),
            ms,
        });
    }
}

/// R-F21: capture ring for latency lines (test assertion surface; also
/// readable by an ops sidecar — bounded at 1k, low-frequency producer).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct LatencyRecord {
    pub metric: String,
    pub room: String,
    pub peer: String,
    pub ms: u64,
}

#[doc(hidden)]
static LATENCY_RING: std::sync::Mutex<Vec<LatencyRecord>> = std::sync::Mutex::new(Vec::new());

/// Drain records matching `metric` from the ring (test helper).
#[doc(hidden)]
pub fn drain_latency(metric: &str) -> Vec<LatencyRecord> {
    let mut ring = LATENCY_RING.lock().expect("ring");
    let matched: Vec<LatencyRecord> = ring.drain(..).filter(|r| r.metric == metric).collect();
    matched
}

/// Init the tracing subscriber (JSON to stdout, env-filter level).
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(std::io::stdout)
        .init();
}
