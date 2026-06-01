use std::sync::Arc;

use k256::ecdsa::SigningKey;
use serde::{Deserialize, Serialize};
use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::signing::keccak256;

// ================================================================
//  Response types
// ================================================================

/// Response from GET /auth/nonce.
#[derive(Debug, Deserialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub expires_at: String,
}

/// Response from POST /auth.
#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub address: String,
    pub account_id: String,
    pub expires_at: String,
}

#[derive(Serialize)]
struct AuthRequest<'a> {
    message: &'a str,
    signature: &'a str,
}

// ================================================================
//  AuthClient
// ================================================================

/// SIWE authentication operations.
pub struct AuthClient {
    http: Arc<HttpClient>,
}

impl AuthClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Fetches a one-time SIWE nonce from the API.
    pub async fn get_nonce(&self) -> Result<NonceResponse, Rail0Error> {
        self.http.get("/auth/nonce").await
    }

    /// Submits a signed EIP-4361 message and returns a JWT on success.
    pub async fn verify(&self, message: &str, signature: &str) -> Result<AuthResponse, Rail0Error> {
        self.http
            .post("/auth", &AuthRequest { message, signature })
            .await
    }

    /// Performs the full SIWE authentication flow:
    /// 1. GET /auth/nonce
    /// 2. Build EIP-4361 message
    /// 3. Sign with EIP-191 personal_sign
    /// 4. POST /auth { message, signature }
    ///
    /// `private_key_bytes` must be 32 raw bytes.
    /// `domain` is the API host, e.g. `"api.rail0.xyz"`.
    pub async fn login(
        &self,
        private_key_bytes: &[u8],
        domain: &str,
    ) -> Result<AuthResponse, Rail0Error> {
        let nonce_resp = self.get_nonce().await?;
        let address = private_key_to_address(private_key_bytes)?;
        let message = build_siwe_message(domain, &address, &nonce_resp.nonce);
        let signature = personal_sign(private_key_bytes, &message)?;
        self.verify(&message, &signature).await
    }
}

// ================================================================
//  EIP-4361 message builder (minimal, no external crate needed)
// ================================================================

/// Builds a minimal EIP-4361 SIWE message string.
fn build_siwe_message(domain: &str, address: &str, nonce: &str) -> String {
    format!(
        "{domain} wants you to sign in with your Ethereum account:\n\
         {address}\n\
         \n\
         Sign in to RAIL0\n\
         \n\
         URI: https://{domain}\n\
         Version: 1\n\
         Chain ID: 1\n\
         Nonce: {nonce}\n\
         Issued At: {}",
        chrono_like_now()
    )
}

/// Returns an ISO-8601 timestamp for the current moment.
/// We use a fixed compile-time placeholder if std time is unavailable,
/// but in practice std::time always works on supported platforms.
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as RFC3339 / ISO-8601 (seconds precision, UTC).
    let s = secs;
    let (year, month, day, hour, minute, second) = unix_to_utc(s);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

/// Minimal UTC breakdown — avoids pulling in the `chrono` crate.
fn unix_to_utc(mut secs: u64) -> (u64, u8, u8, u8, u8, u8) {
    let second = (secs % 60) as u8;
    secs /= 60;
    let minute = (secs % 60) as u8;
    secs /= 60;
    let hour = (secs % 24) as u8;
    secs /= 24;
    // Days since 1970-01-01
    let mut days = secs;
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let months: [u8; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u8;
    for m in months.iter() {
        if days < *m as u64 {
            break;
        }
        days -= *m as u64;
        month += 1;
    }
    let day = (days + 1) as u8;
    (year, month, day, hour, minute, second)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

// ================================================================
//  EIP-191 personal_sign
// ================================================================

/// Signs `message` with the EIP-191 personal_sign prefix using the given private key.
/// Returns a 0x-prefixed 65-byte hex string: R(32) || S(32) || V(1), V in {27, 28}.
pub fn personal_sign(private_key_bytes: &[u8], message: &str) -> Result<String, Rail0Error> {
    let prefixed = format!("\x19Ethereum Signed Message:\n{}{}", message.len(), message);
    let digest = keccak256(prefixed.as_bytes());

    let signing_key = SigningKey::from_slice(private_key_bytes)
        .map_err(|e| Rail0Error::Sign(e.to_string()))?;

    let (sig, recovery_id) = signing_key
        .sign_prehash_recoverable(&digest)
        .map_err(|e| Rail0Error::Sign(e.to_string()))?;

    let sig_bytes = sig.to_bytes();
    let v = recovery_id.to_byte() + 27u8;
    let mut out = [0u8; 65];
    out[..64].copy_from_slice(&sig_bytes);
    out[64] = v;
    Ok(format!("0x{}", hex::encode(out)))
}

// ================================================================
//  Ethereum address derivation (EIP-55 checksum)
// ================================================================

/// Derives the EIP-55 checksummed Ethereum address from raw private key bytes.
pub fn private_key_to_address(private_key_bytes: &[u8]) -> Result<String, Rail0Error> {
    if private_key_bytes.len() != 32 {
        return Err(Rail0Error::InvalidInput(
            "private key must be exactly 32 bytes".into(),
        ));
    }
    let signing_key = SigningKey::from_slice(private_key_bytes)
        .map_err(|e| Rail0Error::InvalidInput(e.to_string()))?;
    let verifying_key = signing_key.verifying_key();
    // Uncompressed public key: 0x04 || X(32) || Y(32)
    let uncompressed = verifying_key
        .to_encoded_point(false);
    let pubkey_bytes = uncompressed.as_bytes();
    // Skip the 0x04 prefix and hash the remaining 64 bytes
    let hash = keccak256(&pubkey_bytes[1..]);
    // Last 20 bytes of the hash
    let addr_bytes = &hash[12..];
    Ok(eip55_checksum(addr_bytes))
}

/// Applies EIP-55 checksum encoding to 20 raw address bytes.
fn eip55_checksum(addr_bytes: &[u8]) -> String {
    let lower_hex = hex::encode(addr_bytes);
    let hash = keccak256(lower_hex.as_bytes());
    let mut out = String::with_capacity(42);
    out.push_str("0x");
    for (i, c) in lower_hex.chars().enumerate() {
        let nibble = (hash[i / 2] >> (4 - (i % 2) * 4)) & 0xf;
        if c.is_ascii_alphabetic() && nibble >= 8 {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push(c);
        }
    }
    out
}
