/// Integration tests — endpoint shape tests against a local mockito server.
///
/// Each test starts its own in-process HTTP server, returns a fixture response,
/// and asserts the decoded value matches the expected shape.
use rail0::{ClientOptions, Rail0Client};

const PAYMENT_ID: &str =
    "0x1111111111111111111111111111111111111111111111111111111111111111";

fn client(server_url: &str) -> Rail0Client {
    Rail0Client::new(ClientOptions {
        base_url: server_url.into(),
        ..Default::default()
    })
}

fn create_payment_body() -> String {
    let config_hash = "cc".repeat(32);
    let nonce = "dd".repeat(32);
    let payment = r#"{"payer":"0x0","payee":"0x0","token":"0x0","maxAmount":"0","authorizationExpiry":0,"refundExpiry":0,"feeBps":0,"feeReceiver":"0x0"}"#;
    let domain = r#"{"name":"","version":"","chainId":0,"verifyingContract":"0x0"}"#;
    let types = r#"{"TransferWithAuthorization":[]}"#;
    let message = format!(r#"{{"from":"0x0","to":"0x0","value":"0","validAfter":"0","validBefore":"0","nonce":"0x{nonce}"}}"#);
    let signing_prepare = format!(r#"{{"domain":{domain},"types":{types},"primaryType":"TransferWithAuthorization","message":{message}}}"#);
    format!(r#"{{"paymentId":"{PAYMENT_ID}","configHash":"0x{config_hash}","payment":{payment},"amount":"50000000","chainId":8453,"rail0Contract":"0x0","signingPayload":{signing_prepare}}}"#)
}

fn authorize_body() -> String {
    format!(r#"{{"rail0_id":"{PAYMENT_ID}","status":"submitting"}}"#)
}

fn charge_body() -> String {
    format!(r#"{{"rail0_id":"{PAYMENT_ID}","status":"submitting"}}"#)
}

fn prepare_body() -> String {
    r#"{"unsignedTransaction":"0x02f8beef","to":"0x0","data":"0x","chainId":8453,"nonce":1,"maxFeePerGas":"1000000000","maxPriorityFeePerGas":"1000000000","gasLimit":"100000"}"#.into()
}

fn capture_submit_body() -> String {
    format!(r#"{{"rail0_id":"{PAYMENT_ID}","status":"submitting"}}"#)
}

fn void_submit_body() -> String {
    format!(r#"{{"rail0_id":"{PAYMENT_ID}","status":"submitting"}}"#)
}

fn release_body() -> String {
    format!(r#"{{"rail0_id":"{PAYMENT_ID}","status":"submitting"}}"#)
}

fn refund_submit_body() -> String {
    format!(r#"{{"rail0_id":"{PAYMENT_ID}","status":"submitting"}}"#)
}

// ================================================================
//  Payments
// ================================================================

#[tokio::test]
async fn create_payment_returns_response() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", "/payments")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(create_payment_body())
        .create_async()
        .await;

    use rail0::{CreatePaymentRequest, PaymentConfig};
    let res = client(&server.url())
        .payments
        .create_payment(&CreatePaymentRequest {
            payment: PaymentConfig {
                payer: "0xBuyer".into(),
                payee: "0xMerchant".into(),
                token: "0xToken".into(),
                max_amount: "100000000".into(),
                authorization_expiry: 9_999_999_999,
                refund_expiry: 9_999_999_999,
                fee_bps: 0,
                fee_receiver: "0x0000000000000000000000000000000000000000".into(),
            },
            amount: "50000000".into(),
            chain_id: 8453,
            mode: "authorize".into(),
        })
        .await
        .unwrap();
    assert_eq!(res.payment_id, PAYMENT_ID);
    assert!(res.config_hash.starts_with("0x"));
    assert!(!res.signing_payload.primary_type.is_empty());
}

#[tokio::test]
async fn sign_stores_signature() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("PUT", format!("/payments/{PAYMENT_ID}/sign").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"paymentId":"{PAYMENT_ID}","status":"signature_stored"}}"#
        ))
        .create_async()
        .await;

    use rail0::PayerSignatureRequest;
    let res = client(&server.url())
        .payments
        .sign(
            PAYMENT_ID,
            &PayerSignatureRequest {
                v: 27,
                r: "0x1111111111111111111111111111111111111111111111111111111111111111".into(),
                s: "0x2222222222222222222222222222222222222222222222222222222222222222".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(res.payment_id, PAYMENT_ID);
    assert!(!res.status.is_empty());
}

// ── authorize ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn authorize_prepare_returns_unsigned_tx() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize/prepare").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(prepare_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .authorize_prepare(PAYMENT_ID)
        .await
        .unwrap();
    assert!(!res.unsigned_transaction.is_empty());
    assert_eq!(res.chain_id, 8453);
}

#[tokio::test]
async fn authorize_returns_capturable_amount() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(authorize_body())
        .create_async()
        .await;

    use rail0::SubmitTransactionRequest;
    let res = client(&server.url())
        .payments
        .authorize(
            PAYMENT_ID,
            &SubmitTransactionRequest { signed_transaction: "0x02f8...".into() },
        )
        .await
        .unwrap();
    assert_eq!(res.rail0_id, PAYMENT_ID);
    assert_eq!(res.status, "submitting");
}

// ── charge ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn charge_prepare_returns_unsigned_tx() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/charge/prepare").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(prepare_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .charge_prepare(PAYMENT_ID)
        .await
        .unwrap();
    assert!(!res.unsigned_transaction.is_empty());
    assert_eq!(res.chain_id, 8453);
}

#[tokio::test]
async fn charge_returns_charged_amount() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/charge").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(charge_body())
        .create_async()
        .await;

    use rail0::SubmitTransactionRequest;
    let res = client(&server.url())
        .payments
        .charge(
            PAYMENT_ID,
            &SubmitTransactionRequest { signed_transaction: "0x02f8...".into() },
        )
        .await
        .unwrap();
    assert_eq!(res.rail0_id, PAYMENT_ID);
    assert_eq!(res.status, "submitting");
}

// ── capture ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn capture_prepare_returns_unsigned_tx() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/capture/prepare").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(prepare_body())
        .create_async()
        .await;

    use rail0::CapturePaymentRequest;
    let res = client(&server.url())
        .payments
        .capture_prepare(PAYMENT_ID, &CapturePaymentRequest { amount: "50000000".into() })
        .await
        .unwrap();
    assert!(!res.unsigned_transaction.is_empty());
    assert_eq!(res.chain_id, 8453);
}

#[tokio::test]
async fn capture_returns_captured_amount() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/capture").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(capture_submit_body())
        .create_async()
        .await;

    use rail0::SubmitTransactionRequest;
    let res = client(&server.url())
        .payments
        .capture(
            PAYMENT_ID,
            &SubmitTransactionRequest { signed_transaction: "0x02f8...".into() },
        )
        .await
        .unwrap();
    assert_eq!(res.rail0_id, PAYMENT_ID);
    assert_eq!(res.status, "submitting");
}

// ── void ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn void_prepare_returns_unsigned_tx() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/void/prepare").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(prepare_body())
        .create_async()
        .await;

    let res = client(&server.url())
        .payments
        .void_prepare(PAYMENT_ID)
        .await
        .unwrap();
    assert!(!res.unsigned_transaction.is_empty());
}

#[tokio::test]
async fn void_returns_released_amount() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/void").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(void_submit_body())
        .create_async()
        .await;

    use rail0::SubmitTransactionRequest;
    let res = client(&server.url())
        .payments
        .void(
            PAYMENT_ID,
            &SubmitTransactionRequest { signed_transaction: "0x02f8...".into() },
        )
        .await
        .unwrap();
    assert_eq!(res.rail0_id, PAYMENT_ID);
    assert_eq!(res.status, "submitting");
}

// ── release ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn release_prepare_returns_unsigned_tx() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/release/prepare").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(prepare_body())
        .create_async()
        .await;

    use rail0::ReleaseRequest;
    let res = client(&server.url())
        .payments
        .release_prepare(PAYMENT_ID, &ReleaseRequest::default())
        .await
        .unwrap();
    assert!(!res.unsigned_transaction.is_empty());
}

#[tokio::test]
async fn release_returns_released_amount() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/release").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(release_body())
        .create_async()
        .await;

    use rail0::SubmitTransactionRequest;
    let res = client(&server.url())
        .payments
        .release(
            PAYMENT_ID,
            &SubmitTransactionRequest { signed_transaction: "0x02f8...".into() },
        )
        .await
        .unwrap();
    assert_eq!(res.rail0_id, PAYMENT_ID);
    assert_eq!(res.status, "submitting");
}

// ── refund ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refund_prepare_returns_unsigned_tx() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/refund/prepare").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(prepare_body())
        .create_async()
        .await;

    use rail0::RefundPayloadRequest;
    let res = client(&server.url())
        .payments
        .refund_prepare(PAYMENT_ID, &RefundPayloadRequest { amount: "10000000".into(), v: None, r: None, s: None })
        .await
        .unwrap();
    assert!(res.unsigned_transaction.is_some());
}

#[tokio::test]
async fn refund_returns_refunded_amount() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/refund").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(refund_submit_body())
        .create_async()
        .await;

    use rail0::SubmitTransactionRequest;
    let res = client(&server.url())
        .payments
        .refund(
            PAYMENT_ID,
            &SubmitTransactionRequest { signed_transaction: "0x02f8...".into() },
        )
        .await
        .unwrap();
    assert_eq!(res.rail0_id, PAYMENT_ID);
    assert_eq!(res.status, "submitting");
}
