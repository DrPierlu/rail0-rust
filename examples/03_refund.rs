// Refund flow — EIP-3009 receiveWithAuthorization
//
// After a capture (or charge) the merchant can refund up to the full
// captured amount back to the payer, as long as refund_expiry has not
// passed.
//
// Uses EIP-3009 receiveWithAuthorization — no ERC-20 approve() step needed.
// The refund_payload endpoint is a two-phase flow:
//
//   Phase 1 — send only `amount` → returns a signing_payload (EIP-3009).
//             Sign it off-chain to get v, r, s.
//   Phase 2 — send `amount` + v, r, s → returns the unsigned refund tx.
//             Sign and submit via refund().
//
// On-chain flow:
//
//   payee signs EIP-3009   → refund()   funds move payee → payer
//
// Run:
//
//   cargo run --example 03_refund

use rail0::{ClientOptions, Rail0Client, Rail0Error, RefundPayloadRequest, SubmitTransactionRequest};

#[tokio::main]
async fn main() {
    let client = Rail0Client::new(ClientOptions {
        base_url: "https://api.rail0.xyz".into(),
        ..Default::default()
    });

    // Assume the payment was previously created and captured.
    let payment_id = "0xdeadbeef00000000000000000000000000000000000000000000000000000002";

    // ----------------------------------------------------------------
    // Phase 1 — Get the EIP-3009 signing payload
    // ----------------------------------------------------------------

    let phase1 = client
        .payments
        .refund_payload(
            payment_id,
            &RefundPayloadRequest {
                amount: "50000000".into(),
                v: None,
                r: None,
                s: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("refund_payload phase 1: {e}"));

    println!("Phase 1 — sign this payload off-chain:");
    println!("  unsigned_transaction: {}", phase1.unsigned_transaction);

    // Payee signs the EIP-3009 payload off-chain to obtain v, r, s:
    //
    //   let key = rail0::hex_to_private_key("0xYourPrivateKey").unwrap();
    //   let sig = rail0::sign_transfer_with_authorization(&rail0::SignTransferParams {
    //       private_key: key,
    //       from: "0xPayeeAddress".into(),
    //       to:   contract_address,
    //       value: 50_000_000,
    //       valid_after:  0,
    //       valid_before: 9_999_999_999,
    //       nonce: ...,
    //       token_domain: ...,
    //   }).unwrap();

    let (v, r, s) = (27u8,
        "0x1111111111111111111111111111111111111111111111111111111111111111",
        "0x2222222222222222222222222222222222222222222222222222222222222222");

    // ----------------------------------------------------------------
    // Phase 2 — Get the unsigned refund transaction
    // ----------------------------------------------------------------

    let phase2 = client
        .payments
        .refund_payload(
            payment_id,
            &RefundPayloadRequest {
                amount: "50000000".into(),
                v: Some(v),
                r: Some(r.into()),
                s: Some(s.into()),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("refund_payload phase 2: {e}"));

    println!("Phase 2 — unsigned refund tx ready for signing");

    // Payee signs phase2.unsigned_transaction offline, then submits:
    //   let signed_refund = payee_wallet.sign_transaction(&phase2.unsigned_transaction);
    let signed_refund = "0x02f8..."; // placeholder

    let refund_resp = client
        .payments
        .refund(
            payment_id,
            &SubmitTransactionRequest { signed_transaction: signed_refund.into() },
        )
        .await
        .unwrap_or_else(|e| {
            if let Rail0Error::Api { code, message, .. } = &e {
                panic!("Refund failed [{code}]: {message}");
            }
            panic!("refund: {e}");
        });

    println!(
        "Refunded: tx={} refunded={} remaining={}",
        refund_resp.transaction_hash,
        refund_resp.refunded_amount,
        refund_resp.refundable_amount
    );
    let _ = phase2;
}
