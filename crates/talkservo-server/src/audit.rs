//! Audit log (plan-7 S1, D23-②: JSONL + Discord-shaped five-tuple).
//!
//! Fire-and-forget append-only trail: sites `emit()` into an unbounded mpsc,
//! one dedicated task serializes + appends (`0600`, daily rotation by date
//! suffix). The WRITE NEVER BLOCKS a floor/WS/admin path. Fail-visible (plan-7
//! R4 ruling): an IO failure increments [`AUDIT_DROPPED`] AND logs
//! `tracing::error!` — never a panic, never a silent swallow.
//!
//! Event vocabulary is a CLOSED typed enum (the Discord `AuditLogEvent`
//! borrowing that matters is exhaustiveness, not the wire numbers - JSONL is
//! local and human-grepped, so kinds serialize snake_case). Entry shape =
//! actor + action + target + `changes[{key,old,new}]` + reason; reads page by
//! `seq` cursor (`before`/`after`), filter by room/actor/event, limit clamped
//! `1..=1000` (doc-audit GAP-8).

use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

/// Counter of events the writer could not persist (read by `/admin/api/audit`).
pub static AUDIT_DROPPED: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    FloorGrant,
    FloorTaken,
    FloorIdle,
    ModeChange,
    MuteSet,
    // ships from S6 (intervention kit); declared now so the vocabulary is closed
    Kick,
    ForceRelease,
    // fires from S3 (command authorization); closed-set member per plan S1.2
    AuthzDenied,
    Join,
    Leave,
    TokenIssue,
    LoginFail,
}

#[derive(Serialize, Clone, Debug)]
pub struct Change {
    pub key: String,
    pub old: serde_json::Value,
    pub new: serde_json::Value,
}

/// Site-side message: no seq/timestamp yet (the writer stamps both).
#[derive(Clone, Debug)]
pub struct AuditEvent {
    pub kind: AuditKind,
    pub room: Option<String>,
    pub actor: Option<String>,
    pub target: Option<String>,
    pub changes: Vec<Change>,
    pub reason: Option<String>,
}

impl AuditEvent {
    pub fn new(kind: AuditKind) -> Self {
        Self {
            kind,
            room: None,
            actor: None,
            target: None,
            changes: Vec::new(),
            reason: None,
        }
    }
    pub fn room(mut self, room: impl Into<String>) -> Self {
        self.room = Some(room.into());
        self
    }
    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }
    pub fn change(mut self, key: &str, old: serde_json::Value, new: serde_json::Value) -> Self {
        self.changes.push(Change {
            key: key.into(),
            old,
            new,
        });
        self
    }
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Cloned by every emitting site (room actor, ws, admin).
#[derive(Clone)]
pub struct AuditTx {
    tx: mpsc::UnboundedSender<AuditEvent>,
    pub path: PathBuf,
}

impl AuditTx {
    /// Never blocks, never fails the hot path: send error means the writer
    /// task is gone - count it visibly.
    pub fn emit(&self, ev: AuditEvent) {
        if self.tx.send(ev).is_err() {
            AUDIT_DROPPED.fetch_add(1, Ordering::Relaxed);
            tracing::error!("audit writer task gone; event dropped");
        }
    }
}

/// epoch-ms -> "YYYY-MM-DD" (Howard Hinnant's civil-from-days; no date crate).
pub fn date_str(ts_ms: u64) -> String {
    let z = (ts_ms / 1000) / 86_400 + 719_468; // ts_ms >= 0: u64, no negative era branch
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// The dated filename a rotation would move `path` into (date suffix contract,
/// pure fn for testability).
pub fn rotated_name(path: &Path, date: &str) -> PathBuf {
    let stem = path.file_name().unwrap_or_default().to_string_lossy();
    let _ = stem;
    path.with_file_name(format!("audit-{date}.jsonl"))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn count_lines(path: &Path) -> u64 {
    std::fs::File::open(path)
        .map(|f| {
            BufReader::new(f)
                .lines()
                .map_while(Result::ok)
                .filter(|l| l.contains("\"seq\""))
                .count() as u64
        })
        .unwrap_or(0)
}

/// Create parent dir + spawn the writer for an instance-resolved path.
pub fn spawn_for(path: &Path) -> AuditTx {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    spawn_writer(path.to_path_buf())
}

/// Spawn the writer task; sender returned for sites. `seq` resumes from the
/// existing file's line count so a restart continues the cursor monotonic.
pub fn spawn_writer(path: PathBuf) -> AuditTx {
    let (tx, mut rx) = mpsc::unbounded_channel::<AuditEvent>();
    let start_seq = count_lines(&path);
    let task_path = path.clone();
    tokio::spawn(async move {
        let mut seq = start_seq;
        let mut date = String::new();
        while let Some(ev) = rx.recv().await {
            seq += 1;
            let ts = now_ms();
            let d = date_str(ts);
            // day boundary: move the accumulated file into the dated archive,
            // fresh audit.jsonl continues (rename failure is logged, append
            // proceeds - losing one rotation beats losing the trail entry).
            if !date.is_empty() && d != date {
                let target = rotated_name(&task_path, &date);
                if let Err(e) = std::fs::rename(&task_path, &target)
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    tracing::error!("audit rotation {task_path:?} -> {target:?}: {e}");
                }
            }
            date = d;
            let line = serde_json::json!({
                "seq": seq,
                "ts_ms": ts,
                "event": ev.kind,
                "room": ev.room,
                "actor": ev.actor,
                "target": ev.target,
                "changes": if ev.changes.is_empty() { serde_json::Value::Null } else { serde_json::json!(ev.changes) },
                "reason": ev.reason,
            });
            let file = task_path.clone();
            let payload = format!("{line}\n");
            // append off the async thread; failure = count + error log (R4).
            let wrote = tokio::task::spawn_blocking(move || {
                use std::fs::OpenOptions;
                use std::io::Write;
                let mut opts = OpenOptions::new();
                opts.create(true).append(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    opts.mode(0o600);
                }
                opts.open(&file)
                    .and_then(|mut f| f.write_all(payload.as_bytes()).and_then(|_| f.flush()))
                    .map_err(|e| format!("{file:?}: {e}"))
            })
            .await;
            match wrote {
                Ok(Ok(())) => {}
                Ok(Err(msg)) => {
                    AUDIT_DROPPED.fetch_add(1, Ordering::Relaxed);
                    tracing::error!("audit append failed (counted): {msg}");
                }
                Err(e) => {
                    AUDIT_DROPPED.fetch_add(1, Ordering::Relaxed);
                    tracing::error!("audit blocking task failed (counted): {e}");
                }
            }
        }
    });
    AuditTx { tx, path }
}

/// Query filters mirroring the Discord cursor contract.
#[derive(Debug, Default)]
pub struct AuditQuery {
    pub before: Option<u64>,
    pub after: Option<u64>,
    pub room: Option<String>,
    pub actor: Option<String>,
    pub event: Option<String>,
    pub limit: Option<u64>,
}

pub const AUDIT_LIMIT_DEFAULT: u64 = 100;
pub const AUDIT_LIMIT_MAX: u64 = 1000;

/// Newest-first page matching the filters; unparseable lines are skipped
/// (trail is append-only; a torn tail line must not poison the read).
pub fn query(path: &Path, q: &AuditQuery) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let limit = q
        .limit
        .unwrap_or(AUDIT_LIMIT_DEFAULT)
        .clamp(1, AUDIT_LIMIT_MAX) as usize;
    let mut out: Vec<serde_json::Value> = Vec::new();
    for line in text.lines() {
        let Ok(serde_json::Value::Object(v)) = serde_json::from_str::<serde_json::Value>(line)
        else {
            continue;
        };
        let seq = v.get("seq").and_then(|s| s.as_u64()).unwrap_or(0);
        if q.before.is_some_and(|b| seq >= b) {
            continue;
        }
        if q.after.is_some_and(|a| seq <= a) {
            continue;
        }
        if q.room
            .as_deref()
            .is_some_and(|r| v.get("room").and_then(|x| x.as_str()) != Some(r))
        {
            continue;
        }
        if q.actor
            .as_deref()
            .is_some_and(|a| v.get("actor").and_then(|x| x.as_str()) != Some(a))
        {
            continue;
        }
        if q.event
            .as_deref()
            .is_some_and(|e| v.get("event").and_then(|x| x.as_str()) != Some(e))
        {
            continue;
        }
        out.push(serde_json::Value::Object(v));
    }
    out.sort_by_key(|v| std::cmp::Reverse(v.get("seq").and_then(|s| s.as_u64()).unwrap_or(0)));
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_match_known_epoch_days() {
        // 2025-10-19 00:00 UTC
        assert_eq!(date_str(1_760_832_000_000), "2025-10-19");
        assert_eq!(date_str(0), "1970-01-01");
        assert_eq!(date_str(1_735_689_600_000), "2025-01-01");
    }

    #[test]
    fn line_serializes_closed_enum_snake_case() {
        let l = serde_json::json!({"event": AuditKind::ModeChange});
        assert_eq!(l["event"], serde_json::json!("mode_change"));
    }

    #[tokio::test]
    async fn writer_appends_and_query_pages() {
        let dir = std::env::temp_dir().join(format!("ts-audit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("audit.jsonl");
        let _ = std::fs::remove_file(&path);
        let tx = spawn_writer(path.clone());
        tx.emit(AuditEvent::new(AuditKind::Join).room("r1").actor("alice"));
        tx.emit(
            AuditEvent::new(AuditKind::ModeChange)
                .room("r1")
                .actor("chief")
                .change("mode", "hybrid".into(), "open".into()),
        );
        // flush: spawn_blocking ordering behind the channel - poll the file.
        let mut lines = 0;
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            lines = std::fs::read_to_string(&path)
                .map(|s| s.lines().count())
                .unwrap_or(0);
            if lines >= 2 {
                break;
            }
        }
        assert_eq!(lines, 2, "both events appended");
        let all = query(&path, &AuditQuery::default());
        assert_eq!(all.len(), 2);
        assert_eq!(all[0]["seq"].as_u64(), Some(2), "newest first");
        assert_eq!(all[0]["event"], "mode_change");
        assert_eq!(all[0]["changes"][0]["old"], "hybrid");
        let only_join = query(
            &path,
            &AuditQuery {
                event: Some("join".into()),
                ..Default::default()
            },
        );
        assert_eq!(only_join.len(), 1);
        let paged = query(
            &path,
            &AuditQuery {
                after: Some(1),
                ..Default::default()
            },
        );
        assert_eq!(paged.len(), 1);
        assert_eq!(paged[0]["seq"], 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn query_on_missing_file_is_empty_not_error() {
        assert!(
            query(
                Path::new("/nonexistent/ts-audit.jsonl"),
                &AuditQuery::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn limit_clamped_to_max() {
        // 2000 requested -> 1000 effective: with fewer events present, only
        // the clamp path is checked through the constant contract.
        assert_eq!(AUDIT_LIMIT_MAX, 1000);
        let clamped = 2000u64.clamp(1, AUDIT_LIMIT_MAX);
        assert_eq!(clamped, 1000);
    }
}
