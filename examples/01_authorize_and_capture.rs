// Standard two-step payment flow: authorize → capture
//
// The buyer locks funds in escrow using an EIP-3009 signature (authorize).
// The merchant releases them once the order is fulfilled (capture).
// If something goes wrong before capture the merchant can void,
// or anyone can call release after authorization_expiry.
//
// On-chain flow:
//
//   buyer signs EIP-3009 → authorize()  funds move buyer → escrow
//   merchant             → capture()    funds move escrow → merchant (minus fee)
//   merchant             → void()       alternative: funds move escrow → buyer
//   anyone               → release()    fallback after authorization_expiry
//
// Run:
//
//   cargo run --example 01_authorize_and_capture

use std::time::{SystemTime, UNIX_EPOCH};

use rail0::{AuthorizeParams, CaptureParams, ClientOptions, Payment, Rail0Client};

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

    // A unique ID for this payment — in practice derive it from your order ID,
    // e.g. keccak256(abi.encode("order", order_id)).
    let payment_id =
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    let payment = Payment {
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
    // Step 1 — Buyer fetches the authorize nonce, signs EIP-3009, calls authorize
    // ----------------------------------------------------------------

    let nonce_resp = client
        .payments
        .authorize_nonce(payment_id, &payment.payer)
        .await
        .unwrap_or_else(|e| panic!("authorize_nonce: {e}"));

    // The buyer builds and signs transferWithAuthorization off-chain.
    // In production use sign_authorize:
    //
    //   let key = rail0::hex_to_private_key("0xYourPrivateKey").unwrap();
    //   let sig = rail0::sign_authorize(&rail0::SignPaymentParams {
    //       private_key: key,
    //       payment: payment.clone(),
    //       amount: 50_000_000,
    //       nonce: nonce_resp.nonce.clone(),
    //       contract_address: "0xRAIL0ContractAddress".into(),
    //       token_domain: rail0::TokenDomain {
    //           name: "USD Coin".into(), version: "2".into(), chain_id: 8453,
    //           verifying_contract: payment.token.clone(),
    //       },
    //       valid_after: None,
    //       valid_before: None,
    //   }).unwrap();

    let auth_tx = client
        .payments
        .authorize(
            payment_id,
            AuthorizeParams {
                payment: payment.clone(),
                amount: "50000000".into(), // 50 USDC
                v: 27,                     // from signature
                r: "0x1111111111111111111111111111111111111111111111111111111111111111".into(),
                s: "0x2222222222222222222222222222222222222222222222222222222222222222".into(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("authorize: {e}"));

    println!("Authorized: {} — status: {:?}", auth_tx.transaction_hash, auth_tx.status);
    println!("Nonce used: {}", nonce_resp.nonce);

    // ----------------------------------------------------------------
    // Step 2a — Merchant captures 50 USDC (happy path)
    // ----------------------------------------------------------------

    let capture_tx = client
        .payments
        .capture(
            payment_id,
            CaptureParams {
                payment: payment.clone(),
                amount: "50000000".into(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("capture: {e}"));

    println!("Captured: {}", capture_tx.transaction_hash);

    // ----------------------------------------------------------------
    // Step 2b — Merchant voids (alternative: order cancelled)
    // ----------------------------------------------------------------

    // let void_tx = client.payments.void(payment_id, rail0::VoidParams { payment: payment.clone() }).await?;

    // ----------------------------------------------------------------
    // Step 2c — Release (fallback: merchant never captured)
    // Only callable after authorization_expiry. Anyone can call this.
    // ----------------------------------------------------------------

    // let release_tx = client.payments.release(payment_id, rail0::ReleaseParams { payment: payment.clone() }).await?;

    // ----------------------------------------------------------------
    // Inspect on-chain state at any point
    // ----------------------------------------------------------------

    let state = client
        .payments
        .get(payment_id)
        .await
        .unwrap_or_else(|e| panic!("get: {e}"));

    println!(
        "Payment state: exists={} capturable={} refundable={}",
        state.state.exists, state.state.capturable_amount, state.state.refundable_amount
    );
}
