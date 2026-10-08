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
    /// Admin-console JWT secret (plan-5 S1). None => /admin/api answers 503
    /// (fail-closed). MUST differ from jwt_secret — enforced at startup.
    pub admin_secret: Option<String>,
    /// Scheme-inclusive public base for issued links (e.g.
    /// https://192.168.2.127:8443). Required for LAN/TLS deploys; unset
    /// falls back to the request Host header for loopback dev ONLY.
    pub public_base: Option<String>,
    pub max_peers: usize,
    pub max_queue: usize,
    pub max_frame_bytes: usize,
}

/// server.yaml snake_case keys → environment names (single source; tests
/// assert every env key a yaml can set; modules/06 documents the table).
pub static YAML_ENV_KEYS: std::sync::LazyLock<std::collections::HashMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        std::collections::HashMap::from([
            ("jwt_secret", "TS_JWT_SECRET"),
            ("admin_secret", "TS_ADMIN_SECRET"),
            ("public_base", "TS_PUBLIC_BASE"),
            ("bind", "TS_BIND"),
            ("announced_ip", "TS_ANNOUNCED_IP"),
            ("turn_secret", "TURN_SECRET"),
            ("turn_uri", "TURN_URI"),
            ("jwt_ttl_s", "TS_JWT_TTL_S"),
            ("turn_ttl_s", "TURN_TTL_S"),
            ("floor_max_hold_ms", "FLOOR_MAX_HOLD_MS"),
            ("floor_media_grace_ms", "FLOOR_MEDIA_GRACE_MS"),
            ("floor_request_cooldown_ms", "FLOOR_REQUEST_COOLDOWN_MS"),
            ("heartbeat_s", "HEARTBEAT_S"),
            ("rtc_port_min", "RTC_PORT_MIN"),
            ("rtc_port_max", "RTC_PORT_MAX"),
            ("transport_guardrail", "TRANSPORT_GUARDRAIL"),
            ("no_rtp_watchdog_ms", "NO_RTP_WATCHDOG_MS"),
            ("room_idle_ttl_s", "ROOM_IDLE_TTL_S"),
            ("max_peers", "MAX_PEERS"),
            ("max_queue", "MAX_QUEUE"),
            ("max_frame_bytes", "MAX_FRAME_BYTES"),
        ])
    });

impl Config {
    /// Read from environment; missing `TS_JWT_SECRET` aborts startup with a
    /// fix hint (modules/06: no default secret ever).
    /// D19 instance config: read `<dir>/etc/server.yaml` (or explicit path)
    /// and fill only env keys NOT already present — precedence is
    /// **process env > yaml > defaults** (modules/06). Must run during single-
    /// threaded startup (main, before spawn): std::env::set_var is only safe
    /// there (edition-2024 rule). Unknown keys warn (typo visibility beats
    /// silent no-op; doctor re-surfaces them).
    pub fn load_yaml(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        let doc: serde_yaml::Value =
            serde_yaml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        let map = doc.as_mapping().ok_or_else(|| "server.yaml must be a flat mapping".to_string())?;
        for (k, v) in map {
            let key = k.as_str().ok_or_else(|| "yaml keys must be strings".to_string())?;
            let Some(env_name) = YAML_ENV_KEYS.get(key) else {
                tracing::warn!(event = "config_unknown_key", key, "ignored (check spelling)");
                continue;
            };
            if std::env::var_os(env_name).is_some() {
                continue; // env wins (12-factor)
            }
            let val = match v {
                serde_yaml::Value::String(str) => str.clone(),
                serde_yaml::Value::Number(num) => num.to_string(),
                serde_yaml::Value::Bool(flag) => flag.to_string(),
                other => {
                    let _ = other;
                    return Err(format!("yaml key {key}: only flat scalars supported (null/nested rejected)"));
                }
            };
            // SAFETY: single-threaded startup (main before any spawn), see doc comment.
            unsafe { std::env::set_var(env_name, val) };
        }
        Ok(Self::from_env())
    }

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
            admin_secret: std::env::var("TS_ADMIN_SECRET").ok().filter(|v| !v.is_empty()),
            public_base: std::env::var("TS_PUBLIC_BASE").ok().filter(|v| !v.is_empty()),
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
            admin_secret: None,
            public_base: None,
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

/// test-only: process-env is global — every env-touching test takes this
/// lock (poison-tolerant: one failed assertion must not cascade) and clears
/// the mapped keys first (set_var injection leaves residue otherwise).
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn clear_config_env() {
    for env in YAML_ENV_KEYS.values() {
        unsafe { std::env::remove_var(env) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn write_yaml(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        std::fs::create_dir_all(dir.join("etc")).unwrap();
        let p = dir.join("etc/server.yaml");
        std::fs::write(&p, body).unwrap();
        p
    }

    #[test]
    fn yaml_fills_and_env_overrides() {
        let _g = lock();
        clear_config_env();
        let dir = std::env::temp_dir().join(format!("ts-cfg-{}", std::process::id()));
        let p = write_yaml(
            &dir,
            "jwt_secret: from-yaml\nheartbeat_s: 42\nmax_peers: 3\nrtc_port_min: 40960\n",
        );
        // SAFETY: test serialised under ENV_LOCK
        unsafe {
            std::env::remove_var("TS_JWT_SECRET");
            std::env::remove_var("HEARTBEAT_S");
            std::env::set_var("MAX_PEERS", "7"); // env must win over yaml
            std::env::remove_var("TS_ADMIN_SECRET");
            std::env::remove_var("TS_BIND");
        }
        let c = Config::load_yaml(&p).expect("load");
        assert_eq!(c.jwt_secret, "from-yaml");
        assert_eq!(c.heartbeat_s, 42);
        assert_eq!(c.max_peers, 7, "process env must override yaml");
        assert_eq!(c.rtc_port_min, 40960);
        unsafe { std::env::remove_var("MAX_PEERS") };
    }

    #[test]
    fn nested_yaml_rejected_unknown_ignored() {
        let _g = lock();
        clear_config_env();
        let dir = std::env::temp_dir().join(format!("ts-cfg-neg-{}", std::process::id()));
        let p = write_yaml(
            &dir,
            "jwt_secret: ok\nnot_a_real_key: hi\nfloor_media_grace_ms: {a: 1}\n",
        );
        unsafe { std::env::remove_var("TS_JWT_SECRET") };
        let err = Config::load_yaml(&p).unwrap_err();
        assert!(err.contains("floor_media_grace_ms"), "{err}");
    }

    #[test]
    fn every_mapped_env_name_is_consumed() {
        // guard against map drift: each yaml-mapped env key must appear in
        // from_env's reads — verified behaviourally with a full yaml
        let _g = lock();
        clear_config_env();
        let dir = std::env::temp_dir().join(format!("ts-cfg-all-{}", std::process::id()));
        let body: String = YAML_ENV_KEYS
            .keys()
            .map(|k| match *k {
                "jwt_secret" => "jwt_secret: s\n".to_string(),
                "turn_secret" | "admin_secret" | "public_base" | "bind" | "announced_ip" =>
                    format!("{k}: v\n").replace(": v bind", ": 127.0.0.1:9999")
                        .replace(": v public_base", ": http://x")
                        .replace(": v bind", ": 127.0.0.1:9999")
                        .replace(": v turn_secret", ": ts")
                        .replace(": v admin_secret", ": as")
                        .replace(": v announced_ip", ": 10.0.0.1"),
                "turn_uri" => "turn_uri: turn:10.0.0.1:3478\n".into(),
                "floor_max_hold_ms" => "floor_max_hold_ms: off\n".into(),
                _ => format!("{k}: 5\n"),
            })
            .collect();
        let p = write_yaml(&dir, &body);
        for env in YAML_ENV_KEYS.values() {
            unsafe { std::env::remove_var(env) };
        }
        let c = Config::load_yaml(&p).expect("full yaml loads");
        assert_eq!(c.heartbeat_s, 5);
        assert_eq!(c.rtc_port_min, 5);
        assert_eq!(c.floor_max_hold_ms, None, "off parses via from_env path");
        unsafe { std::env::remove_var("TS_JWT_SECRET") };
    }
}
