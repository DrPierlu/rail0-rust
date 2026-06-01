/// Authentication tests: nonce, verify, full SIWE login flow, and signing utilities.
///
/// All HTTP is intercepted by a local mockito server — no real network calls.
use rail0::{ClientOptions, Rail0Client};

/// Hardhat key #0 private key bytes.
const HARDHAT_KEY: &[u8] =
    &hex_literal(b"ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
const HARDHAT_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

/// Compile-time hex decode for a 32-byte key literal (64 hex chars, no 0x prefix).
const fn hex_literal(s: &[u8]) -> [u8; 32] {
    assert!(s.len() == 64, "expected 64 hex chars");
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        let hi = nibble(s[i * 2]);
        let lo = nibble(s[i * 2 + 1]);
        out[i] = (hi << 4) | lo;
        i += 1;
    }
    out
}

const fn nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}

fn make_client(server_url: &str) -> Rail0Client {
    Rail0Client::new(ClientOptions {
        base_url: server_url.into(),
        ..Default::default()
    })
}

// ================================================================
//  Routing
// ================================================================

#[tokio::test]
async fn get_nonce_routes_to_correct_path() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/auth/nonce")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"nonce":"test-nonce","expires_at":"2099-01-01T00:00:00Z"}"#)
        .create_async()
        .await;

    let res = make_client(&server.url()).auth.get_nonce().await.unwrap();
    assert_eq!(res.nonce, "test-nonce");
    mock.assert_async().await;
}

#[tokio::test]
async fn verify_posts_to_auth() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/auth")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"token":"jwt-token","address":"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266","account_id":"uuid-123","expires_at":"2099-01-01T00:00:00Z"}"#,
        )
        .create_async()
        .await;

    let res = make_client(&server.url())
        .auth
        .verify("some message", "0xdeadbeef")
        .await
        .unwrap();
    assert_eq!(res.token, "jwt-token");
    assert_eq!(res.account_id, "uuid-123");
    mock.assert_async().await;
}

// ================================================================
//  Full SIWE login flow
// ================================================================

#[tokio::test]
async fn login_performs_full_siwe_flow() {
    let mut server = mockito::Server::new_async().await;

    // Step 1: GET /auth/nonce
    let nonce_mock = server
        .mock("GET", "/auth/nonce")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"nonce":"flow-nonce","expires_at":"2099-01-01T00:00:00Z"}"#)
        .expect(1)
        .create_async()
        .await;

    // Step 2: POST /auth — capture the body for assertion
    let captured_body = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let captured_clone = std::sync::Arc::clone(&captured_body);

    let auth_mock = server
        .mock("POST", "/auth")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"token":"flow-jwt","address":"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266","account_id":"uuid-flow","expires_at":"2099-01-01T00:00:00Z"}"#,
        )
        .with_body_from_request(move |req| {
            *captured_clone.lock().unwrap() = String::from_utf8_lossy(req.body().expect("body")).to_string();
            r#"{"token":"flow-jwt","address":"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266","account_id":"uuid-flow","expires_at":"2099-01-01T00:00:00Z"}"#
                .into()
        })
        .expect(1)
        .create_async()
        .await;

    let res = make_client(&server.url())
        .auth
        .login(HARDHAT_KEY, "localhost")
        .await
        .unwrap();

    nonce_mock.assert_async().await;
    auth_mock.assert_async().await;

    // Parse captured POST body
    let body_str = captured_body.lock().unwrap().clone();
    let body: serde_json::Value = serde_json::from_str(&body_str).expect("POST body must be JSON");

    let message = body["message"].as_str().expect("message field must be present");
    let signature = body["signature"].as_str().expect("signature field must be present");

    // Signature: 0x + 130 hex chars = 132 total
    assert!(signature.starts_with("0x"), "signature must start with 0x");
    assert_eq!(signature.len(), 132, "signature must be 132 chars (0x + 130 hex)");

    // Message must contain the address and "Nonce:"
    assert!(
        message.contains(HARDHAT_ADDRESS),
        "message must contain the signer address"
    );
    assert!(message.contains("Nonce:"), "message must contain 'Nonce:'");

    assert_eq!(res.token, "flow-jwt");
}

// ================================================================
//  Signing utilities (unit tests — no HTTP)
// ================================================================

#[test]
fn personal_sign_produces_correct_length() {
    let sig = rail0::personal_sign(HARDHAT_KEY, "hello world").unwrap();
    assert!(sig.starts_with("0x"), "signature must start with 0x");
    assert_eq!(sig.len(), 132, "signature must be 132 chars (0x + 130 hex)");
}

#[test]
fn private_key_to_address_returns_checksummed() {
    let addr = rail0::private_key_to_address(HARDHAT_KEY).unwrap();
    assert_eq!(addr, HARDHAT_ADDRESS);
}
