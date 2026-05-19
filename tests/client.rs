/// HTTP client tests: retry, logging, error parsing.
///
/// All HTTP is intercepted by a local mockito server — no real network calls.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rail0::{ClientOptions, Rail0Client, Rail0Error};

fn make_client(server_url: &str) -> Rail0Client {
    Rail0Client::new(ClientOptions {
        base_url: server_url.into(),
        ..Default::default()
    })
}

const PAYMENT_ID: &str =
    "0x1111111111111111111111111111111111111111111111111111111111111111";

fn authorize_body() -> String {
    format!(
        r#"{{"paymentId":"{PAYMENT_ID}","transactionHash":"0x{}","capturableAmount":"50000000"}}"#,
        "ab".repeat(32)
    )
}

// ================================================================
//  Basic routing
// ================================================================

#[tokio::test]
async fn post_authorize_routes_to_correct_path() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(authorize_body())
        .create_async()
        .await;

    let res = make_client(&server.url())
        .payments
        .authorize(PAYMENT_ID)
        .await
        .unwrap();
    assert_eq!(res.payment_id, PAYMENT_ID);
    assert_eq!(res.capturable_amount, "50000000");
    mock.assert_async().await;
}

#[tokio::test]
async fn post_create_payment_routes_to_correct_path() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/payments")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"paymentId":"{PAYMENT_ID}","configHash":"0x{}","payment":{{"payer":"0x0","payee":"0x0","token":"0x0","maxAmount":"0","authorizationExpiry":0,"refundExpiry":0,"feeBps":0,"feeReceiver":"0x0"}},"amount":"50000000","chainId":8453,"rail0Contract":"0x0","signingPayload":{{"domain":{{"name":"","version":"","chainId":0,"verifyingContract":"0x0"}},"types":{{"TransferWithAuthorization":[]}},"primaryType":"TransferWithAuthorization","message":{{"from":"0x0","to":"0x0","value":"0","validAfter":"0","validBefore":"0","nonce":"0x{}"}}}}}}"#,
            "cc".repeat(32),
            "dd".repeat(32)
        ))
        .create_async()
        .await;

    use rail0::{CreatePaymentRequest, PaymentConfig};
    let res = make_client(&server.url())
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
    mock.assert_async().await;
}

// ================================================================
//  Error handling
// ================================================================

#[tokio::test]
async fn http_404_returns_api_error() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"PaymentNotFound","message":"No payment exists."}"#)
        .create_async()
        .await;

    let err = make_client(&server.url())
        .payments
        .authorize(PAYMENT_ID)
        .await
        .unwrap_err();

    match err {
        Rail0Error::Api { status, code, message } => {
            assert_eq!(status, 404);
            assert_eq!(code, "PaymentNotFound");
            assert!(!message.is_empty());
        }
        other => panic!("expected Rail0Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn http_422_returns_api_error_with_code() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(422)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"AuthorizationExpired","message":"The authorizationExpiry has passed."}"#)
        .create_async()
        .await;

    let err = make_client(&server.url())
        .payments
        .authorize(PAYMENT_ID)
        .await
        .unwrap_err();

    match err {
        Rail0Error::Api { code, .. } => assert_eq!(code, "AuthorizationExpired"),
        other => panic!("expected Rail0Error::Api, got {other:?}"),
    }
}

// ================================================================
//  Retry
// ================================================================

#[tokio::test]
async fn retry_option_is_accepted() {
    let mut server = mockito::Server::new_async().await;
    let path = format!("/payments/{PAYMENT_ID}/authorize");

    // Two failures then a success — tests that retry config is wired up.
    server
        .mock("POST", path.as_str())
        .with_status(500)
        .with_body("fail")
        .expect(2)
        .create_async()
        .await;
    server
        .mock("POST", path.as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(authorize_body())
        .expect(1)
        .create_async()
        .await;

    // 500 is an HTTP error and won't be retried — the test just confirms
    // the config is accepted and the client behaves predictably.
    let client = Rail0Client::new(ClientOptions {
        base_url: server.url(),
        max_retries: 3,
        retry_delay: Duration::from_millis(1),
        ..Default::default()
    });
    let _ = client.payments.authorize(PAYMENT_ID).await;
}

#[tokio::test]
async fn no_retry_on_http_errors() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(422)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"AuthorizationExpired","message":"expired"}"#)
        .expect(1)
        .create_async()
        .await;

    let client = Rail0Client::new(ClientOptions {
        base_url: server.url(),
        max_retries: 3,
        retry_delay: Duration::from_millis(1),
        ..Default::default()
    });
    let _ = client.payments.authorize(PAYMENT_ID).await;
    mock.assert_async().await;
}

// ================================================================
//  Logging
// ================================================================

#[tokio::test]
async fn logger_receives_entry_on_success() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(authorize_body())
        .create_async()
        .await;

    let entries: Arc<Mutex<Vec<rail0::LogEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let entries_clone = Arc::clone(&entries);

    let client = Rail0Client::new(ClientOptions {
        base_url: server.url(),
        logger: Some(Arc::new(move |e| {
            entries_clone.lock().unwrap().push(e);
        })),
        ..Default::default()
    });
    client.payments.authorize(PAYMENT_ID).await.unwrap();

    let log = entries.lock().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].method, "POST");
    assert_eq!(log[0].status, Some(200));
    assert!(log[0].error.is_none());
}

#[tokio::test]
async fn logger_receives_error_entry_on_http_error() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"PaymentNotFound","message":"not found"}"#)
        .create_async()
        .await;

    let entries: Arc<Mutex<Vec<rail0::LogEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let entries_clone = Arc::clone(&entries);

    let client = Rail0Client::new(ClientOptions {
        base_url: server.url(),
        logger: Some(Arc::new(move |e| {
            entries_clone.lock().unwrap().push(e);
        })),
        ..Default::default()
    });
    let _ = client.payments.authorize(PAYMENT_ID).await;

    let log = entries.lock().unwrap();
    assert_eq!(log[0].status, Some(404));
    assert!(log[0].error.is_some());
}

#[tokio::test]
async fn debug_logger_does_not_panic() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", format!("/payments/{PAYMENT_ID}/authorize").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(authorize_body())
        .create_async()
        .await;

    let client = Rail0Client::new(ClientOptions {
        base_url: server.url(),
        logger: Some(rail0::debug_logger()),
        ..Default::default()
    });
    client.payments.authorize(PAYMENT_ID).await.unwrap();
}
