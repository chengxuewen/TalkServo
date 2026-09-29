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
