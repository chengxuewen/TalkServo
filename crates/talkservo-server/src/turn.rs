//! TURN short-term credential issuance (modules/06, auth ladder rung 4).
//!
//! coturn `use-auth-secret` protocol (draft-uberti-rtcweb-turn-rest):
//!   username   = `{expiry_unix_ts}` (plain decimal string)
//!   credential = base64(HMAC-SHA1(TURN_SECRET, username))
//! coturn recomputes the HMAC on request — stateless, no DB; rotating
//! TURN_SECRET invalidates every outstanding credential at once. Physically
//! separate from TS_JWT_SECRET (independent rotation, modules/06 §43).

use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha1::Sha1;

/// Sign one credential set. `now_s` = unix seconds; expiry = now + ttl.
pub fn issue(secret: &str, ttl_s: u64, now_s: u64, uri: &str) -> Option<serde_json::Value> {
    if secret.is_empty() || uri.is_empty() {
        return None; // TURN disabled — Welcome carries empty uris
    }
    let expiry = now_s.saturating_add(ttl_s);
    let username = expiry.to_string();
    let mut mac = Hmac::<Sha1>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(username.as_bytes());
    let digest = mac.finalize().into_bytes();
    let credential = base64::engine::general_purpose::STANDARD.encode(digest);
    Some(serde_json::json!({
        "uris": [uri],
        "username": username,
        "credential": credential,
        "ttl": ttl_s,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "turn-secret-0123456789";
    const URI: &str = "turn:turn.example.com:3478?transport=udp";

    /// Reference vector cross-checked with coturn's turn-rest protocol
    /// (generated independently via openssl):
    ///   printf '1790740000' | openssl dgst -sha1 -hmac 'turn-secret-0123456789' -binary | base64
    #[test]
    fn hmac_vector_matches_openssl() {
        let creds = issue(SECRET, 0, 1_790_740_000, URI).expect("enabled");
        // ttl 0 → expiry == now
        assert_eq!(creds["username"], "1790740000");
        assert_eq!(
            creds["credential"],
            "4r5xxcZjEWd0Cs3HRaz1ikUYxEE=",
            "HMAC-SHA1 base64 must match the openssl reference"
        );
        assert_eq!(creds["uris"], serde_json::json!([URI]));
    }

    #[test]
    fn expiry_advances_by_ttl() {
        let creds = issue(SECRET, 3600, 1_000_000, URI).expect("enabled");
        assert_eq!(creds["username"], "1003600");
    }

    #[test]
    fn disabled_without_secret_or_uri() {
        assert!(issue("", 3600, 0, URI).is_none());
        assert!(issue(SECRET, 3600, 0, "").is_none());
    }

    #[test]
    fn distinct_secrets_distinct_credentials() {
        let a = issue("alpha", 60, 1_000, URI).unwrap();
        let b = issue("beta", 60, 1_000, URI).unwrap();
        assert_ne!(a["credential"], b["credential"]);
    }
}
