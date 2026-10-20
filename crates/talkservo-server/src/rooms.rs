//! Room registry (plan-7 S2, D23-①): `etc/rooms.yaml` - the rooms-registry
//! borrowing of the Mumble/Zello "channels have owners & config" lesson.
//!
//! Same runtime shape as [`crate::accounts`]: program-maintained via the admin
//! REST, atomic 0600 write, HOT (read per room-spawn - no cache), env override
//! `TS_ROOMS_FILE`, derived `<instance>/etc/rooms.yaml` from the config path.
//! Absent file/row = today's behavior (implicit rooms, global limits, Hybrid
//! default) - the registry configures rooms, it never gates existence. `locked`
//! is the one field that gates JOIN (wire Error{code:"room_locked"}, no new
//! variant).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use talkservo_core::ids::RoomId;
use talkservo_core::wire::FloorMode;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RoomRow {
    pub room: String,
    /// free-text label (Zello creator / MCPTT squad). No runtime power here:
    /// acting authority rides token mod-claims (S4) and dispatch role.
    #[serde(default)]
    pub owner: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<FloorMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_peers: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_queue: Option<usize>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// Resolved per-room startup spec (registry row merged over global defaults).
#[derive(Clone, Debug, PartialEq)]
pub struct RoomSpec {
    pub mode: FloorMode,
    pub max_peers: usize,
    pub max_queue: usize,
    pub locked: bool,
}

pub fn path(cfg: &crate::config::Config) -> Option<PathBuf> {
    cfg.rooms_file.clone()
}

/// Load rows; missing file = empty; unreadable/corrupt lines are logged and the
/// whole file treated empty (fail-open to legacy behavior mirrors accounts).
pub fn load(cfg: &crate::config::Config) -> Vec<RoomRow> {
    let Some(p) = path(cfg) else {
        return Vec::new();
    };
    match std::fs::read_to_string(&p) {
        Ok(text) => match serde_yaml::from_str::<Vec<RoomRow>>(&text) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(event = "rooms_registry_unreadable", "using defaults: {e}");
                Vec::new()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            tracing::error!(event = "rooms_registry_read_failed", "{e}");
            Vec::new()
        }
    }
}

pub fn save(cfg: &crate::config::Config, rows: &[RoomRow]) -> Result<(), String> {
    let p = path(cfg).ok_or("rooms registry path not configured")?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let body = serde_yaml::to_string(rows).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("yaml.tmp");
    std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn find(cfg: &crate::config::Config, room: &str) -> Option<RoomRow> {
    load(cfg).into_iter().find(|r| r.room == room)
}

/// True when the registry says this room refuses new joins.
pub fn is_locked(cfg: &crate::config::Config, room: &RoomId) -> bool {
    find(cfg, room.0.as_ref()).is_some_and(|r| r.locked)
}

/// Resolve the spawn-time spec: row over global defaults.
pub fn resolve(cfg: &crate::config::Config, room: &RoomId) -> RoomSpec {
    let row = find(cfg, room.0.as_ref());
    RoomSpec {
        mode: row
            .as_ref()
            .and_then(|r| r.mode)
            .unwrap_or(FloorMode::Hybrid),
        max_peers: row
            .as_ref()
            .and_then(|r| r.max_peers)
            .unwrap_or(cfg.max_peers),
        max_queue: row
            .as_ref()
            .and_then(|r| r.max_queue)
            .unwrap_or(cfg.max_queue),
        locked: row.is_some_and(|r| r.locked),
    }
}

/// admin.rs mirror of auth::valid_id-style room-name check (URL-path safe).
pub fn valid_room(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub fn validate(row: &RoomRow) -> Result<(), String> {
    if !valid_room(&row.room) {
        return Err("room name must be 1-64 chars of [A-Za-z0-9._-]".into());
    }
    if row.owner.len() > 64 {
        return Err("owner label too long (<=64)".into());
    }
    if row.max_peers.is_some_and(|v| v == 0 || v > 1024) {
        return Err("max_peers out of range (1..=1024)".into());
    }
    if row.max_queue.is_some_and(|v| v > 1024) {
        return Err("max_queue out of range (0..=1024)".into());
    }
    Ok(())
}

/// Upsert by room name (create or full-replace). Returns index.
pub fn upsert(cfg: &crate::config::Config, row: RoomRow) -> Result<usize, String> {
    validate(&row)?;
    let mut rows = load(cfg);
    let idx = match rows.iter().position(|r| r.room == row.room) {
        Some(i) => {
            rows[i] = row;
            i
        }
        None => {
            rows.push(row);
            rows.len() - 1
        }
    };
    save(cfg, &rows)?;
    Ok(idx)
}

pub fn remove(cfg: &crate::config::Config, room: &str) -> Result<bool, String> {
    let mut rows = load(cfg);
    let before = rows.len();
    rows.retain(|r| r.room != room);
    if rows.len() == before {
        return Ok(false);
    }
    save(cfg, &rows)?;
    Ok(true)
}

/// Safe projection for the admin list (never leaks more than ops fields).
pub fn list_view(cfg: &crate::config::Config) -> Vec<serde_json::Value> {
    load(cfg)
        .iter()
        .map(|r| {
            serde_json::json!({
                "room": r.room,
                "owner": r.owner,
                "mode": r.mode,
                "max_peers": r.max_peers,
                "max_queue": r.max_queue,
                "locked": r.locked,
            })
        })
        .collect()
}

/// load + serde roundtrip of the file shape operators hand-edit.
pub fn read_raw(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_in(dir: &Path) -> crate::config::Config {
        let mut c = crate::config::Config::for_test("t");
        c.rooms_file = Some(dir.join("rooms.yaml"));
        c.max_peers = 10;
        c.max_queue = 8;
        c
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ts-rooms-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn empty_dir_resolves_to_legacy_defaults() {
        let dir = tmpdir("defaults");
        let cfg = cfg_in(&dir);
        let spec = resolve(&cfg, &RoomId::from("any"));
        assert_eq!(spec.mode, FloorMode::Hybrid);
        assert_eq!(spec.max_peers, 10);
        assert!(!spec.locked);
        assert!(load(&cfg).is_empty());
    }

    #[test]
    fn upsert_roundtrip_hot_read() {
        let dir = tmpdir("roundtrip");
        let cfg = cfg_in(&dir);
        let row = RoomRow {
            room: "ops-01".into(),
            owner: "squad-a".into(),
            mode: Some(FloorMode::Open),
            max_peers: Some(4),
            max_queue: None,
            locked: false,
        };
        assert_eq!(upsert(&cfg, row.clone()).unwrap(), 0);
        let spec = resolve(&cfg, &RoomId::from("ops-01"));
        assert_eq!(spec.mode, FloorMode::Open);
        assert_eq!(spec.max_peers, 4);
        assert_eq!(spec.max_queue, 8, "global default fills absent field");
        // upsert again = replace, not duplicate
        let updated = RoomRow {
            locked: true,
            ..row
        };
        assert_eq!(upsert(&cfg, updated).unwrap(), 0);
        assert_eq!(load(&cfg).len(), 1);
        assert!(is_locked(&cfg, &RoomId::from("ops-01")));
        // file mode 0600 where unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(dir.join("rooms.yaml")).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validation_rejects_bad_shapes() {
        let dir = tmpdir("validate");
        let _cfg = cfg_in(&dir);
        let base = RoomRow {
            room: "ok".into(),
            owner: String::new(),
            mode: None,
            max_peers: None,
            max_queue: None,
            locked: false,
        };
        assert!(validate(&base).is_ok());
        assert!(
            validate(&RoomRow {
                room: "bad name!".into(),
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate(&RoomRow {
                max_peers: Some(0),
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate(&RoomRow {
                max_peers: Some(9999),
                ..base.clone()
            })
            .is_err()
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_file_fails_open_to_defaults() {
        let dir = tmpdir("corrupt");
        std::fs::write(dir.join("rooms.yaml"), ": : not yaml [[").unwrap();
        let cfg = cfg_in(&dir);
        assert!(load(&cfg).is_empty());
        assert_eq!(resolve(&cfg, &RoomId::from("x")).max_peers, 10);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remove_reports_absence() {
        let dir = tmpdir("remove");
        let cfg = cfg_in(&dir);
        upsert(
            &cfg,
            RoomRow {
                room: "gone".into(),
                owner: String::new(),
                mode: None,
                max_peers: None,
                max_queue: None,
                locked: false,
            },
        )
        .unwrap();
        assert!(remove(&cfg, "gone").unwrap());
        assert!(!remove(&cfg, "gone").unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }
}
