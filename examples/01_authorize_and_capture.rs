// Standard two-step payment flow: authorize → capture
//
// The payer creates a payment intent, signs the EIP-712 payload, and
// submits the signature. The payee then calls authorize (funds move to
// escrow), prepares a capture transaction, signs it offline, and submits it.
//
// On-chain flow:
//
//   payer signs EIP-712    → authorize()   funds move payer → escrow
//   payee signs capture tx → capture()     funds move escrow → payee (minus fee)
//   payee signs void tx    → void()        alternative: funds move escrow → payer
//   anyone                 → release()     fallback after authorization_expiry
//
// Run:
//
//   cargo run --example 01_authorize_and_capture

use std::time::{SystemTime, UNIX_EPOCH};

use rail0::{
    CapturePaymentRequest, ClientOptions, CreatePaymentRequest, PayerSignatureRequest,
    PaymentConfig, Rail0Client, SubmitTransactionRequest,
};

#[tokio::main]
async fn main() {
    let client = Rail0Client::new(ClientOptions {
        base_url: "https://api.rail0.xyz".into(),
        ..Default::default()
    });

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let payment = PaymentConfig {
        payer: "0xBuyerAddress000000000000000000000000000000".into(),
        payee: "0xMerchantAddress0000000000000000000000000000".into(),
        token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // USDC on Base
        max_amount: "100000000".into(),                             // 100 USDC (6 decimals)
        authorization_expiry: now + 60 * 60 * 24,                  // merchant has 24 h to capture
        refund_expiry: now + 60 * 60 * 24 * 7,                     // refund window: 7 days
        fee_bps: 50,                                               // 0.5% protocol fee
        fee_receiver: "0xFeeReceiverAddress000000000000000000000000".into(),
    };

    // ----------------------------------------------------------------
    // Step 1 — Payer creates a payment intent and signs the EIP-712 payload
    // ----------------------------------------------------------------

    let create_resp = client
        .payments
        .create_payment(&CreatePaymentRequest {
            payment: payment.clone(),
            amount: "50000000".into(),
            chain_id: 8453, // Base
            mode: "authorize".into(),
        })
        .await
        .unwrap_or_else(|e| panic!("create_payment: {e}"));

    println!("Payment ID: {}", create_resp.payment_id);
    println!("Config hash: {}", create_resp.config_hash);

    // The payer signs create_resp.signing_payload using eth_signTypedData_v4 or sign_authorize:
    //
    //   let key = rail0::hex_to_private_key("0xYourPrivateKey").unwrap();
    //   let sig = rail0::sign_authorize(&rail0::SignPaymentParams {
    //       private_key: key,
    //       payment: payment.clone(),
    //       amount: 50_000_000,
    //       nonce: create_resp.signing_payload.message.nonce.clone(),
    //       contract_address: create_resp.rail0_contract.clone(),
    //       token_domain: rail0::TokenDomain {
    //           name: create_resp.signing_payload.domain.name.clone(),
    //           version: create_resp.signing_payload.domain.version.clone(),
    //           chain_id: create_resp.signing_payload.domain.chain_id as u64,
    //           verifying_contract: create_resp.signing_payload.domain.verifying_contract.clone(),
    //       },
    //       valid_after: None,
    //       valid_before: None,
    //   }).unwrap();

    // Step 2 — Payer submits the signature
    let sig_resp = client
        .payments
        .sign(
            &create_resp.payment_id,
            &PayerSignatureRequest {
                v: 27, // from signature
                r: "0x1111111111111111111111111111111111111111111111111111111111111111".into(),
                s: "0x2222222222222222222222222222222222222222222222222222222222222222".into(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("sign: {e}"));

    println!("Signature status: {}", sig_resp.status);

    // ----------------------------------------------------------------
    // Step 3 — Payee authorizes (relays the stored signature on-chain)
    // ----------------------------------------------------------------

    let auth_resp = client
        .payments
        .authorize(&create_resp.payment_id)
        .await
        .unwrap_or_else(|e| panic!("authorize: {e}"));

    println!("Authorized: tx={} capturable={}", auth_resp.transaction_hash, auth_resp.capturable_amount);

    // ----------------------------------------------------------------
    // Step 4a — Payee prepares and submits a capture transaction
    // ----------------------------------------------------------------

    let prep_capture = client
        .payments
        .prepare_capture(
            &create_resp.payment_id,
            &CapturePaymentRequest { amount: "50000000".into() },
        )
        .await
        .unwrap_or_else(|e| panic!("prepare_capture: {e}"));

    // Payee signs prep_capture.unsigned_transaction offline, then submits:
    //   let signed_tx = payee_wallet.sign_transaction(&prep_capture.unsigned_transaction);
    let signed_tx = "0x02f8..."; // placeholder

    let capture_resp = client
        .payments
        .submit_capture(
            &create_resp.payment_id,
            &SubmitTransactionRequest { signed_transaction: signed_tx.into() },
        )
        .await
        .unwrap_or_else(|e| panic!("submit_capture: {e}"));

    println!("Captured: tx={} captured={}", capture_resp.transaction_hash, capture_resp.captured_amount);
    let _ = prep_capture;

    // ----------------------------------------------------------------
    // Step 4b — Alternatively: payee voids (order cancelled)
    // ----------------------------------------------------------------

    // let prep_void = client.payments.prepare_void(&create_resp.payment_id).await?;
    // let signed_void = payee_wallet.sign_transaction(&prep_void.unsigned_transaction);
    // client.payments.submit_void(&create_resp.payment_id, &SubmitTransactionRequest { signed_transaction: signed_void }).await?;

    // ----------------------------------------------------------------
    // Step 4c — Release (fallback after authorization_expiry, permissionless)
    // ----------------------------------------------------------------

    // let release_resp = client.payments.release(&create_resp.payment_id).await?;
}
