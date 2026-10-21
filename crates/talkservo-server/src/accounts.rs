//! Admin account registry (D22 user/pwd, sister `AccountRegistry` shape).
//!
//! `etc/accounts.yaml` is a PROGRAM-maintained list — `[{user, hash, role}]` —
//! NOT the hand-edited operator file (server.yaml keeps that role, and its
//! flat-scalar discipline cannot express nested rows anyway). Read fresh per
//! login (tiny file; no cache layer until proven a bottleneck), written
//! atomically (tmp + rename, sister discipline).
//!
//! While the file is absent the implicit bootstrap account `admin` backed by
//! server.yaml `admin_password_hash:` authenticates (prior shape keeps
//! working); the first upsert materializes the file and carries that hash
//! over as row `admin` so the bootstrap identity is never lost.

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::kdf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub user: String,
    /// PBKDF2 store line (see kdf) — API-minted rows use this. Never
    /// exposed by the list endpoint.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub hash: String,
    /// Plaintext password (sister ops shape: operators hand-edit this file
    /// and write the password directly). Non-empty wins over `hash`.
    /// The file MUST be 0600 and must never leave the host (gitignored).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub password: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "admin".into()
}

/// ids land in paths and yaml rows: same charset as admin token subs (plan-5 C3).
pub fn valid_user(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

pub fn load(path: &std::path::Path) -> Vec<Account> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_saphyr::from_str(&text).unwrap_or_else(|e| {
            tracing::warn!(event = "accounts_parse", path = %path.display(), "treating as empty: {e}");
            Vec::new()
        }),
        Err(_) => Vec::new(),
    }
}

/// Atomic write (temp sibling + rename, 0600 — rows may carry plaintext
/// passwords). A failed write leaves the previous file untouched: a torn
/// account store is a lockout.
pub fn save(path: &std::path::Path, accounts: &[Account]) -> Result<(), String> {
    // `&accounts`, not `accounts`: serde-saphyr's to_string takes a Sized `&T`, so the
    // slice reference itself is the value.
    let body = serde_saphyr::to_string(&accounts).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("yaml.tmp");
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts
        .open(&tmp)
        .map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::io::Write::write_all(&mut f, body.as_bytes())
        .and_then(|()| f.sync_all())
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| format!("write {}: {e}", path.display()))
}

/// Constant-time string compare (plaintext rows).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn accounts_path(cfg: &Config) -> Result<std::path::PathBuf, String> {
    cfg.accounts_file
        .clone()
        .ok_or_else(|| "accounts registry needs an instance layout (--config path); use the script mint on bare-env runs".to_string())
}

/// Verify (user, password). Constant-time inside kdf; unknown user on an
/// existing file still runs a dummy derivation? No — PoC scope: timing
/// equality across users is an Alpha concern (single admin realm today).
pub fn authenticate(cfg: &Config, user: &str, password: &str) -> bool {
    if let Some(p) = &cfg.accounts_file {
        let list = load(p);
        if let Some(a) = list.iter().find(|a| a.user == user) {
            if !a.password.is_empty() {
                return ct_eq(password.as_bytes(), a.password.as_bytes());
            }
            return !a.hash.is_empty() && kdf::verify_password(password, &a.hash);
        }
        if !list.is_empty() {
            return false;
        }
    }
    // implicit bootstrap while accounts.yaml is not materialized yet
    user == "admin"
        && cfg
            .admin_password_hash
            .as_deref()
            .is_some_and(|h| kdf::verify_password(password, h))
}

/// Create or reset the password for `user` (upsert = the "change password"
/// path; PoC has no separate PUT).
pub fn upsert(cfg: &Config, user: &str, password: &str) -> Result<(), String> {
    upsert_role(cfg, user, password, None)
}

/// S5: create honors `role` (validated by the caller); RESET keeps the stored
/// tier - a password reset must never silently change privilege.
pub fn upsert_role(
    cfg: &Config,
    user: &str,
    password: &str,
    role: Option<&str>,
) -> Result<(), String> {
    let path = accounts_path(cfg)?;
    let mut list = load(&path);
    if list.is_empty() && !path.exists() {
        // materialize: carry the implicit bootstrap row over (never strand it)
        if let Some(h) = &cfg.admin_password_hash {
            list.push(Account {
                user: "admin".into(),
                hash: h.clone(),
                password: String::new(),
                role: "admin".into(),
            });
        }
    }
    let hash = kdf::hash_password_configured(password);
    match list.iter_mut().find(|a| a.user == user) {
        // reset must clear any hand-written plaintext so the new hash rules
        Some(a) => {
            a.hash = hash;
            a.password.clear();
        }
        None => list.push(Account {
            user: user.to_string(),
            hash,
            password: String::new(),
            role: role.map(str::to_string).unwrap_or_else(default_role),
        }),
    }
    save(&path, &list)
}

/// Remove a user; returns how many rows went (0 => caller answers 404).
pub fn remove(cfg: &Config, user: &str) -> Result<usize, String> {
    let path = accounts_path(cfg)?;
    let mut list = load(&path);
    let before = list.len();
    list.retain(|a| a.user != user);
    if before == list.len() {
        return Ok(0);
    }
    save(&path, &list)?;
    Ok(before - list.len())
}

/// List view WITHOUT hashes (sister parity: the registry never prints secrets).
pub fn list_view(cfg: &Config) -> Vec<serde_json::Value> {
    let mut out: Vec<serde_json::Value> = cfg
        .accounts_file
        .as_ref()
        .map(|p| load(p))
        .unwrap_or_default()
        .into_iter()
        .map(|a| serde_json::json!({ "user": a.user, "role": a.role }))
        .collect();
    if out.is_empty()
        && cfg.admin_password_hash.is_some()
        && !cfg.accounts_file.as_ref().is_some_and(|p| p.exists())
    {
        out.push(serde_json::json!({ "user": "admin", "role": "admin" }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with(dir: &std::path::Path, yaml_hash: Option<&str>) -> Config {
        let mut c = Config::for_test("s");
        c.accounts_file = Some(dir.join("etc/accounts.yaml"));
        c.admin_password_hash = yaml_hash.map(str::to_string);
        c
    }

    fn fast_hash(pw: &str) -> String {
        kdf::hash_password_with_iters(pw, 4096)
    }

    #[test]
    fn upsert_authenticate_remove_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ts-acc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("etc")).unwrap();
        let cfg = cfg_with(&dir, None);
        upsert(&cfg, "ops-1", "secretpw").unwrap(); // no bootstrap to carry
        assert!(authenticate(&cfg, "ops-1", "secretpw"));
        assert!(!authenticate(&cfg, "ops-1", "wrong-pw"));
        assert!(
            !authenticate(&cfg, "ghost", "secretpw"),
            "unknown user on non-empty file"
        );
        let v = list_view(&cfg);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0]["user"], "ops-1");
        assert!(v[0].get("hash").is_none(), "list never exposes hashes");
        assert_eq!(remove(&cfg, "ops-1").unwrap(), 1);
        assert_eq!(
            remove(&cfg, "ops-1").unwrap(),
            0,
            "double remove => 404 path"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn implicit_bootstrap_authenticates_then_materializes_on_upsert() {
        let dir = std::env::temp_dir().join(format!("ts-acc-imp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("etc")).unwrap();
        let boot = fast_hash("bootpw");
        let cfg = cfg_with(&dir, Some(&boot));
        // prior D22 shape keeps working (yaml hash, user "admin", no file)
        assert!(authenticate(&cfg, "admin", "bootpw"));
        assert!(!authenticate(&cfg, "nope", "bootpw"));
        // first upsert materializes the file AND carries the bootstrap row
        std::fs::create_dir_all(dir.join("etc")).unwrap();
        upsert(&cfg, "ops-2", "otherpw").unwrap();
        let raw = std::fs::read_to_string(cfg.accounts_file.as_ref().unwrap()).unwrap();
        assert!(raw.contains("admin"), "bootstrap row carried:\n{raw}");
        assert!(
            authenticate(&cfg, "admin", "bootpw"),
            "bootstrap still valid via file"
        );
        assert!(authenticate(&cfg, "ops-2", "otherpw"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_is_atomic_and_load_tolerates_corruption() {
        let dir = std::env::temp_dir().join(format!("ts-acc-atom-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("etc")).unwrap();
        let path = dir.join("etc/accounts.yaml");
        std::fs::write(&path, "NOT-YAML: [").unwrap();
        assert!(
            load(&path).is_empty(),
            "corrupt store reads empty, never panics"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn plaintext_password_row_authenticates() {
        // sister ops shape: hand-edit accounts.yaml with a plaintext password
        let dir = std::env::temp_dir().join(format!("ts-acc-plain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("etc")).unwrap();
        let cfg = cfg_with(&dir, None);
        std::fs::write(
            cfg.accounts_file.as_ref().unwrap(),
            "- user: admin\n  password: operator-typed\n  role: admin\n",
        )
        .unwrap();
        assert!(authenticate(&cfg, "admin", "operator-typed"));
        assert!(!authenticate(&cfg, "admin", "operator-typedX"));
        assert!(!authenticate(&cfg, "other", "operator-typed"));
        // API upsert on the same user wins with a hash and clears plaintext
        upsert(&cfg, "admin", "api-reset-pw-123").unwrap();
        let raw = std::fs::read_to_string(cfg.accounts_file.as_ref().unwrap()).unwrap();
        assert!(
            raw.contains("pbkdf2$") && !raw.contains("operator-typed"),
            "{raw}"
        );
        assert!(authenticate(&cfg, "admin", "api-reset-pw-123"));
        // list view never leaks either field
        assert_eq!(list_view(&cfg)[0].get("password"), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_sets_0600_mode() {
        let dir = std::env::temp_dir().join(format!("ts-acc-mode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("etc")).unwrap();
        let path = dir.join("etc/accounts.yaml");
        save(
            &path,
            &[Account {
                user: "a".into(),
                hash: String::new(),
                password: "p".into(),
                role: "admin".into(),
            }],
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn user_charset_gate() {
        assert!(valid_user("ops-1"));
        assert!(valid_user("a.b_c-d"));
        assert!(!valid_user(""));
        assert!(!valid_user("ops 1"));
        assert!(!valid_user("ops/../x"));
    }
}
