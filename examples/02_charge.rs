// One-shot payment: charge
//
// Combines authorize and capture in a single transaction — funds go
// directly from the payer to the payee with no escrow window.
// Use this when there is no need for a hold period (e.g. digital goods,
// instant fulfilment).
//
// On-chain flow:
//
//   payer signs EIP-712 → charge()  funds move payer → payee (minus fee), atomically
//
// Run:
//
//   cargo run --example 02_charge

use std::time::{SystemTime, UNIX_EPOCH};

use rail0::{
    ClientOptions, CreatePaymentRequest, PayerSignatureRequest, PaymentConfig, Rail0Client,
    Rail0Error,
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
        max_amount: "25000000".into(),
        authorization_expiry: now + 60 * 5, // short window — charge captures immediately
        refund_expiry: now + 60 * 60 * 24 * 30, // 30-day refund window
        fee_bps: 0,
        fee_receiver: "0x0000000000000000000000000000000000000000".into(),
    };

    // ----------------------------------------------------------------
    // Step 1 — Payer creates a payment intent (mode = "charge")
    // ----------------------------------------------------------------

    let create_resp = client
        .payments
        .create_payment(&CreatePaymentRequest {
            payment: payment.clone(),
            amount: "25000000".into(),
            chain_id: 8453,
            mode: "charge".into(),
        })
        .await
        .unwrap_or_else(|e| panic!("create_payment: {e}"));

    println!("Payment ID: {}", create_resp.payment_id);

    // The payer signs create_resp.signing_payload using eth_signTypedData_v4 or sign_charge:
    //
    //   let key = rail0::hex_to_private_key("0xYourPrivateKey").unwrap();
    //   let sig = rail0::sign_charge(&rail0::SignPaymentParams {
    //       private_key: key,
    //       payment: payment.clone(),
    //       amount: 25_000_000,
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
    client
        .payments
        .sign(
            &create_resp.payment_id,
            &PayerSignatureRequest {
                v: 27,
                r: "0x1111111111111111111111111111111111111111111111111111111111111111".into(),
                s: "0x2222222222222222222222222222222222222222222222222222222222222222".into(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("sign: {e}"));

    // ----------------------------------------------------------------
    // Step 3 — Payee triggers the one-shot charge
    // ----------------------------------------------------------------

    let tx = client
        .payments
        .charge(&create_resp.payment_id)
        .await
        .unwrap_or_else(|e| {
            if let Rail0Error::Api { code, message, .. } = &e {
                panic!("Charge failed [{code}]: {message}");
            }
            panic!("charge: {e}");
        });

    println!(
        "Charged: tx={} charged={} fee={}",
        tx.transaction_hash, tx.charged_amount, tx.fee_amount
    );
}
