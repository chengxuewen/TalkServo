//! JWT issuance/validation (modules/06 auth ladder rung 1: HS256).

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use talkservo_core::wire::Role;

/// Room-scoped claims — `aud` = room id, `sub` = peer identity (modules/06).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub room: String,
    pub role: Role,
    /// Room-scoped audience (modules/06) — mirrors `room`.
    pub aud: String,
    /// Rooms this peer may govern (plan-7 S4, R3 ruling). serde-default so
    /// pre-S4 tokens deserialize as an EMPTY list (additive compat, R4
    /// legacy-compat gate); mod authority = Dispatch role OR membership here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mod_rooms: Vec<String>,
    pub exp: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("token expired")]
    Expired,
    #[error("wrong audience (room)")]
    WrongRoom,
    #[error("invalid token: {0}")]
    Invalid(String),
}

pub fn issue(
    secret: &str,
    sub: &str,
    room: &str,
    role: Role,
    ttl_s: u64,
) -> Result<String, AuthError> {
    issue_with_mods(secret, sub, room, role, ttl_s, &[])
}

/// S4: issue carrying moderator room claims (validated upstream: charset +
/// <=64 per name; a mod claim on the token's own room is legal and redundant
/// with dispatch role - the check ORs, not dedupes).
pub fn issue_with_mods(
    secret: &str,
    sub: &str,
    room: &str,
    role: Role,
    ttl_s: u64,
    mod_rooms: &[String],
) -> Result<String, AuthError> {
    // exp from the real clock — jsonwebtoken validates against it; tests use
    // short TTLs instead of fake clocks (verification honesty).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let claims = Claims {
        sub: sub.to_string(),
        room: room.to_string(),
        role,
        aud: room.to_string(),
        mod_rooms: mod_rooms.to_vec(),
        exp: now + ttl_s,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::Invalid(e.to_string()))
}

pub fn validate(secret: &str, token: &str, room: &str) -> Result<Claims, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_aud = true;
    validation.set_audience(&[room]);
    validation.validate_exp = true;
    // server-side checks are exact — no leeway (deviation from the lib default 60s)
    validation.leeway = 0;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::Expired,
        jsonwebtoken::errors::ErrorKind::InvalidAudience => AuthError::WrongRoom,
        _ => AuthError::Invalid(e.to_string()),
    })?;
    Ok(data.claims)
}

/// Admin-console claims (plan-5 S1). DELIBERATELY has no `room` field:
/// /ws decodes `Claims` (room mandatory) in its two-pass flow, so an admin
/// token structurally cannot pass — audience isolation survives even under
/// the misconfiguration of both secrets being equal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: String,
    /// fixed "admin" — the only audience the admin validator accepts
    pub aud: String,
    pub exp: u64,
}

/// Mint an admin token (tests / future rotation tooling; bootstrap issuance
/// is scripts/issue-admin-token.sh on the server host).
pub fn issue_admin(secret: &str, sub: &str, ttl_s: u64) -> Result<String, AuthError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let claims = AdminClaims {
        sub: sub.to_string(),
        aud: "admin".to_string(),
        exp: now + ttl_s,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::Invalid(e.to_string()))
}

pub fn validate_admin(secret: &str, token: &str) -> Result<AdminClaims, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_aud = true;
    validation.set_audience(&["admin"]);
    validation.validate_exp = true;
    validation.leeway = 0;
    let data = decode::<AdminClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::Expired,
        jsonwebtoken::errors::ErrorKind::InvalidAudience => AuthError::WrongRoom,
        _ => AuthError::Invalid(e.to_string()),
    })?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-0123456789";

    #[test]
    fn issue_validate_roundtrip() {
        let tok = issue(SECRET, "peer-1", "room-7", Role::Field, 60).unwrap();
        let claims = validate(SECRET, &tok, "room-7").unwrap();
        assert_eq!(claims.sub, "peer-1");
        assert_eq!(claims.role, Role::Field);
    }

    #[test]
    fn wrong_room_rejected() {
        let tok = issue(SECRET, "p", "room-7", Role::Dispatch, 60).unwrap();
        assert!(matches!(
            validate(SECRET, &tok, "other-room"),
            Err(AuthError::WrongRoom)
        ));
    }

    #[test]
    fn expired_rejected() {
        // TTL 0 → exp = issue-second; any later second rejects strictly
        // (jsonwebtoken condition is exp < now, equality passes — hence the sleep)
        let tok = issue(SECRET, "p", "r", Role::Field, 0).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1_200));
        match validate(SECRET, &tok, "r") {
            Err(AuthError::Expired) => {}
            other => panic!("expected Expired, got {other:?}"),
        }
    }

    #[test]
    fn tampered_rejected() {
        let tok = issue(SECRET, "p", "r", Role::Field, 60).unwrap();
        let bad = format!("{tok}x");
        assert!(validate(SECRET, &bad, "r").is_err());
    }

    #[test]
    fn dispatch_and_field_distinct() {
        let d = issue(SECRET, "p", "r", Role::Dispatch, 60).unwrap();
        let f = issue(SECRET, "p", "r", Role::Field, 60).unwrap();
        assert_ne!(d, f);
        assert_eq!(validate(SECRET, &d, "r").unwrap().role, Role::Dispatch);
    }

    #[test]
    fn admin_token_roundtrip_and_audience_isolation() {
        let tok = issue_admin("admin-secret-a", "root", 60).unwrap();
        let claims = validate_admin("admin-secret-a", &tok).unwrap();
        assert_eq!(claims.sub, "root");
        // wrong admin secret
        assert!(validate_admin("admin-secret-b", &tok).is_err());
        // structural isolation: an admin token must NEVER satisfy /ws's
        // Claims decode (no room field) even under an EQUAL secret (the
        // misconfiguration the startup guard is supposed to prevent anyway)
        assert!(validate("admin-secret-a", &tok, "room-x").is_err());
    }

    #[test]
    fn user_token_rejected_by_admin_validator() {
        let user = issue("shared", "peer-1", "room-x", Role::Field, 60).unwrap();
        // aud = room-x, not "admin" AND the claims shape differs — reject
        assert!(validate_admin("shared", &user).is_err());
    }
}
