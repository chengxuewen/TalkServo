//! PBKDF2-HMAC-SHA1 password hashing for the admin login surface (D22).
//! Zero new deps: `hmac`+`sha1` are workspace deps already (turn.rs).
//! Stored format: `pbkdf2$<iters>$<hex-salt>$<hex-dk>` — 16-byte salt,
//! 600_000 iterations by default (OWASP-era KDF floor; parse accepts any
//! count so the knob can move without a migration).
//!
//! Derived keys are compared in constant time. Passwords NEVER hit argv,
//! logs, or the process table (sister PIT-171 lesson).

use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::io::Read as _;

/// Default iteration count for freshly minted hashes.
pub const DEFAULT_ITERS: u32 = 600_000;
const DK_LEN: usize = 20; // sha1 digest size — single PBKDF2 block

/// PBKDF2-F with dkLen=20 (block index fixed at 1 — the whole output for us).
fn pbkdf2_block(pw: &[u8], salt: &[u8], iters: u32) -> [u8; DK_LEN] {
    let mut mac = Hmac::<Sha1>::new_from_slice(pw)
        // HMAC keys are unbounded-length by construction; only an
        // initialization-pool failure could error here.
        .expect("hmac init");
    mac.update(salt);
    mac.update(&1u32.to_be_bytes());
    let mut u: [u8; DK_LEN] = mac.finalize().into_bytes().into();
    let mut t = u;
    for _ in 1..iters {
        let mut mac = Hmac::<Sha1>::new_from_slice(pw).expect("hmac init");
        mac.update(&u);
        u = mac.finalize().into_bytes().into();
        for (a, b) in t.iter_mut().zip(&u) {
            *a ^= *b;
        }
    }
    t
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Constant-time byte comparison (length pre-checked by caller path).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Mint a stored hash for `pw` at DEFAULT_ITERS (production shape).
pub fn hash_password(pw: &str) -> String {
    hash_password_with_iters(pw, DEFAULT_ITERS)
}

/// Knob form — integration tests mint low-iteration hashes (verify reads the
/// count back from the string, so the full parse path stays exercised).
pub fn hash_password_with_iters(pw: &str, iters: u32) -> String {
    let mut salt = [0u8; 16];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut salt)).is_err() {
        // loud fallback per gen_secret precedent — time/pid mixed salt
        eprintln!("WARN: urandom unavailable — weak salt fallback, RE-hash before production");
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
            ^ (u128::from(std::process::id()) << 32) as u64;
        salt[..8].copy_from_slice(&seed.to_le_bytes());
    }
    let dk = pbkdf2_block(pw.as_bytes(), &salt, iters);
    format!("pbkdf2${iters}${}${}", hex(&salt), hex(&dk))
}

/// Parse + re-derive + constant-time compare. Malformed stored values are
/// a hard false (fail-closed), never a panic.
pub fn verify_password(pw: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    let (Some(tag), Some(iters), Some(salt), Some(hash)) =
        (parts.first(), parts.get(1), parts.get(2), parts.get(3))
    else {
        return false;
    };
    if *tag != "pbkdf2" || parts.len() != 4 {
        return false;
    }
    let Ok(iters) = iters.parse::<u32>() else { return false };
    if iters == 0 {
        return false;
    }
    let (Some(salt), Some(expect)) = (unhex(salt), unhex(hash)) else { return false };
    if expect.len() != DK_LEN || salt.is_empty() {
        return false;
    }
    let dk = pbkdf2_block(pw.as_bytes(), &salt, iters);
    ct_eq(&dk, &expect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbkdf2_matches_rfc6070_vectors() {
        // ground truth: python hashlib.pbkdf2_hmac('sha1', P, S, c)
        for (iters, want) in [
            (1u32, "0c60c80f961f0e71f3a9b524af6012062fe037a6"),
            (2, "ea6c014dc72d6f8ccd1ed92ace1d41f0d8de8957"),
            (4096, "4b007901b765489abead49d926f721d065a429c1"),
        ] {
            assert_eq!(hex(&pbkdf2_block(b"password", b"salt", iters)), want, "c={iters}");
        }
    }

    #[test]
    fn verify_accepts_own_hash_and_rejects_wrong_password() {
        // low-iteration stored value keeps the test fast; verify parses the
        // count from the string, so the full parse+compare path is exercised
        let salt = [7u8; 16];
        let dk = pbkdf2_block(b"correct horse", &salt, 4096);
        let stored = format!("pbkdf2$4096${}${}", hex(&salt), hex(&dk));
        assert!(verify_password("correct horse", &stored));
        assert!(!verify_password("battery sting", &stored));
        // malformed / tampered stored values fail closed
        assert!(!verify_password("x", "pbkdf2$0${}{}", ));
        assert!(!verify_password("x", "garbage"));
        assert!(!verify_password("x", &format!("pbkdf2$4096${}${}", hex(&salt), "abcd")));
    }

    #[test]
    fn hash_format_shape() {
        // 600k iters runs ~seconds in debug — assert shape only, KAT covers math
        let h = hash_password("p@ss");
        let parts: Vec<&str> = h.split('$').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "pbkdf2");
        assert_eq!(parts[1], DEFAULT_ITERS.to_string());
        assert_eq!(parts[2].len(), 32);
        assert_eq!(parts[3].len(), 40);
    }
}
