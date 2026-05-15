/// Integration tests — endpoint shape tests against a local mockito server.
///
/// Each test starts its own in-process HTTP server, returns a fixture response,
/// and asserts the decoded value matches the expected shape.
use rail0::{ClientOptions, Rail0Client, TransactionStatus};

const PAYMENT_ID: &str =
    "0x1111111111111111111111111111111111111111111111111111111111111111";
const PAYER: &str = "0xBuyerAddress000000000000000000000000000000";

fn tx_body() -> String {
    format!(
        r#"{{"transactionHash":"0x{}","status":"pending"}}"#,
        "ab".repeat(32)
    )
}

fn nonce_body() -> String {
    format!(r#"{{"nonce":"0x{}"}}"#, "cc".repeat(32))
}

fn client(server_url: &str) -> Rail0Client {
    Rail0Client::new(ClientOptions {
        base_url: server_url.into(),
        ..Default::default()
    })
}

fn payment() -> rail0::Payment {
    rail0::Payment {
        payer: PAYER.into(),
        payee: "0xMerchantAddress0000000000000000000000000000".into(),
        token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
        max_amount: "100000000".into(),
        authorization_expiry: 9_999_999_999,
        refund_expiry: 9_999_999_999,
        fee_bps: 0,
        fee_receiver: "0x0000000000000000000000000000000000000000".into(),
    }
}

fn sig() -> (u8, String, String) {
    (
        27,
        "0x1111111111111111111111111111111111111111111111111111111111111111".into(),
        "0x2222222222222222222222222222222222222222222222222222222222222222".into(),
    )
}

// ================================================================
//  Payments
// ================================================================

#[tokio::test]
async fn get_payment_returns_state_and_config_hash() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("GET", format!("/payments/{PAYMENT_ID}").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"paymentId":"{PAYMENT_ID}","state":{{"exists":true,"capturableAmount":"50000000","refundableAmount":"0"}},"configHash":"0x{}"}}"#,
            "ff".repeat(32)
        ))
        .create_async()
        .await;

    let res = client(&server.url()).payments.get(PAYMENT_ID).await.unwrap();
    assert!(res.state.exists);
    assert_eq!(res.state.capturable_amount, "50000000");
    assert!(res.config_hash.starts_with("0x"));
}

#[tokio::test]
async fn authorize_returns_pending_transaction() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(202)
        .with_header("content-type", "application/json")
        .with_body(tx_body())
        .create_async()
        .await;

    let (v, r, s) = sig();
    let res = client(&server.url())
        .payments
        .authorize(PAYMENT_ID, rail0::AuthorizeParams { payment: payment(), amount: "50000000".into(), v, r, s })
        .await
        .unwrap();
    assert!(res.transaction_hash.starts_with("0x"));
    assert_eq!(res.status, TransactionStatus::Pending);
}

#[tokio::test]
async fn charge_returns_pending_transaction() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/charge").as_str())
        .with_status(202)
        .with_header("content-type", "application/json")
        .with_body(tx_body())
        .create_async()
        .await;

    let (v, r, s) = sig();
    let res = client(&server.url())
        .payments
        .charge(PAYMENT_ID, rail0::ChargeParams { payment: payment(), amount: "25000000".into(), v, r, s })
        .await
        .unwrap();
    assert!(res.transaction_hash.starts_with("0x"));
}

#[tokio::test]
async fn capture_returns_pending_transaction() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/capture").as_str())
        .with_status(202)
        .with_header("content-type", "application/json")
        .with_body(tx_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .capture(PAYMENT_ID, rail0::CaptureParams { payment: payment(), amount: "50000000".into() })
        .await
        .unwrap();
    assert!(res.transaction_hash.starts_with("0x"));
}

#[tokio::test]
async fn void_returns_pending_transaction() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/void").as_str())
        .with_status(202)
        .with_header("content-type", "application/json")
        .with_body(tx_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .void(PAYMENT_ID, rail0::VoidParams { payment: payment() })
        .await
        .unwrap();
    assert!(res.transaction_hash.starts_with("0x"));
}

#[tokio::test]
async fn release_returns_pending_transaction() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/release").as_str())
        .with_status(202)
        .with_header("content-type", "application/json")
        .with_body(tx_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .release(PAYMENT_ID, rail0::ReleaseParams { payment: payment() })
        .await
        .unwrap();
    assert!(res.transaction_hash.starts_with("0x"));
}

#[tokio::test]
async fn refund_returns_pending_transaction() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/refund").as_str())
        .with_status(202)
        .with_header("content-type", "application/json")
        .with_body(tx_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .refund(PAYMENT_ID, rail0::RefundParams { payment: payment(), amount: "50000000".into() })
        .await
        .unwrap();
    assert!(res.transaction_hash.starts_with("0x"));
}

#[tokio::test]
async fn authorize_nonce_returns_bytes32() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock(
            "GET",
            format!("/payments/{PAYMENT_ID}/authorize-nonce?payer={PAYER}").as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(nonce_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .authorize_nonce(PAYMENT_ID, PAYER)
        .await
        .unwrap();
    assert!(res.nonce.starts_with("0x"));
    assert_eq!(res.nonce.len(), 66);
}

#[tokio::test]
async fn charge_nonce_returns_bytes32() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock(
            "GET",
            format!("/payments/{PAYMENT_ID}/charge-nonce?payer={PAYER}").as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(nonce_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .charge_nonce(PAYMENT_ID, PAYER)
        .await
        .unwrap();
    assert!(res.nonce.starts_with("0x"));
}

#[tokio::test]
async fn hash_returns_bytes32_digest() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", "/payments/hash")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(r#"{{"hash":"0x{}"}}"#, "dd".repeat(32)))
        .create_async()
        .await;

    let res = client(&server.url()).payments.hash(&payment()).await.unwrap();
    assert!(res.hash.starts_with("0x"));
    assert_eq!(res.hash.len(), 66);
}

// ================================================================
//  Tokens
// ================================================================

#[tokio::test]
async fn is_accepted_returns_status() {
    let token = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
    let mut server = mockito::Server::new_async().await;
    server
        .mock("GET", format!("/tokens/{token}").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(r#"{{"address":"{token}","accepted":true}}"#))
        .create_async()
        .await;

    let res = client(&server.url()).tokens.is_accepted(token).await.unwrap();
    assert_eq!(res.address, token);
    assert!(res.accepted);
}

// ================================================================
//  Utils
// ================================================================

#[tokio::test]
async fn domain_separator_returns_bytes32() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("GET", "/domain-separator")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(r#"{{"domainSeparator":"0x{}"}}"#, "ee".repeat(32)))
        .create_async()
        .await;

    let res = client(&server.url()).utils.domain_separator().await.unwrap();
    assert!(res.domain_separator.starts_with("0x"));
    assert_eq!(res.domain_separator.len(), 66);
}

#[tokio::test]
async fn version_returns_positive_integer() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("GET", "/version")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"version":6}"#)
        .create_async()
        .await;

    let res = client(&server.url()).utils.version().await.unwrap();
    assert!(res.version > 0);
}
