//! Server configuration (modules/06 §config) — env-driven, fail-fast.

/// All keys with defaults; `TS_JWT_SECRET` is the only mandatory one.
#[derive(Debug, Clone)]
pub struct Config {
    pub jwt_secret: String,
    /// TURN static-auth secret (coturn `use-auth-secret`). None = TURN creds
    /// disabled (Welcome carries empty uris) — PoC dev runs without coturn.
    pub turn_secret: Option<String>,
    /// TURN URI template delivered in Welcome (e.g. "turn:host:3478").
    pub turn_uri: String,
    pub jwt_ttl_s: u64,
    pub turn_ttl_s: u64,
    /// `off` disables the max-hold timer (open mode); `Some(ms)` for dispatch.
    pub floor_max_hold_ms: Option<u64>,
    pub floor_media_grace_ms: u64,
    pub floor_request_cooldown_ms: u64,
    pub heartbeat_s: u64,
    pub rtc_port_min: u16,
    pub rtc_port_max: u16,
    pub transport_guardrail: usize,
    pub no_rtp_watchdog_ms: u64,
    pub room_idle_ttl_s: u64,
    pub max_peers: usize,
    pub max_queue: usize,
    pub max_frame_bytes: usize,
}

impl Config {
    /// Read from environment; missing `TS_JWT_SECRET` aborts startup with a
    /// fix hint (modules/06: no default secret ever).
    pub fn from_env() -> Self {
        let jwt_secret = std::env::var("TS_JWT_SECRET").unwrap_or_else(|_| {
            eprintln!(
                "FATAL: TS_JWT_SECRET is not set. Generate one with: openssl rand -base64 48"
            );
            std::process::exit(1);
        });
        Self {
            jwt_secret,
            turn_secret: std::env::var("TURN_SECRET").ok(),
            turn_uri: std::env::var("TURN_URI").unwrap_or_default(),
            jwt_ttl_s: env_u64("TS_JWT_TTL_S", 3600),
            turn_ttl_s: env_u64("TURN_TTL_S", 3600),
            floor_max_hold_ms: match std::env::var("FLOOR_MAX_HOLD_MS").as_deref() {
                Ok("off") | Err(_) => None,
                Ok(v) => v.parse().ok(),
            },
            floor_media_grace_ms: env_u64("FLOOR_MEDIA_GRACE_MS", 2000),
            floor_request_cooldown_ms: env_u64("FLOOR_REQUEST_COOLDOWN_MS", 500),
            heartbeat_s: env_u64("HEARTBEAT_S", 30),
            rtc_port_min: env_u64("RTC_PORT_MIN", 40000) as u16,
            rtc_port_max: env_u64("RTC_PORT_MAX", 40100) as u16,
            transport_guardrail: env_u64("TRANSPORT_GUARDRAIL", 50) as usize,
            no_rtp_watchdog_ms: env_u64("NO_RTP_WATCHDOG_MS", 2000),
            room_idle_ttl_s: env_u64("ROOM_IDLE_TTL_S", 600),
            max_peers: env_u64("MAX_PEERS", 10) as usize,
            max_queue: env_u64("MAX_QUEUE", 8) as usize,
            max_frame_bytes: env_u64("MAX_FRAME_BYTES", 65536) as usize,
        }
    }

    /// Test/dev constructor: explicit values, no env reads.
    pub fn for_test(jwt_secret: impl Into<String>) -> Self {
        Self {
            jwt_secret: jwt_secret.into(),
            turn_secret: None,
            turn_uri: String::new(),
            jwt_ttl_s: 3600,
            turn_ttl_s: 3600,
            floor_max_hold_ms: Some(45_000),
            floor_media_grace_ms: 2_000,
            floor_request_cooldown_ms: 500,
            heartbeat_s: 30,
            rtc_port_min: 40000,
            rtc_port_max: 40100,
            transport_guardrail: 50,
            no_rtp_watchdog_ms: 2_000,
            room_idle_ttl_s: 600,
            max_peers: 10,
            max_queue: 8,
            max_frame_bytes: 65_536,
        }
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
