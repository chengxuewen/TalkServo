//! JWT issuance/validation (modules/06 auth ladder rung 1: HS256).

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
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

pub fn issue(secret: &str, sub: &str, room: &str, role: Role, ttl_s: u64) -> Result<String, AuthError> {
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
}
