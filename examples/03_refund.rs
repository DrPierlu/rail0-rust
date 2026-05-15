// Refund flow
//
// After a capture (or charge) the merchant can refund up to the full
// captured amount back to the buyer, as long as refund_expiry has not
// passed and the refundable_amount is sufficient.
//
// The refund is initiated by the payee (merchant). The API submits the
// transaction on behalf of the payee.
//
// On-chain flow:
//
//   merchant → refund()  funds move merchant → buyer
//
// Run:
//
//   cargo run --example 03_refund

use std::time::{SystemTime, UNIX_EPOCH};

use rail0::{ClientOptions, Payment, Rail0Client, Rail0Error, RefundParams};

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

    let payment = Payment {
        payer: "0xBuyerAddress000000000000000000000000000000".into(),
        payee: "0xMerchantAddress0000000000000000000000000000".into(),
        token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
        max_amount: "100000000".into(),
        authorization_expiry: now - 30 * 60,      // already captured
        refund_expiry: now + 60 * 60 * 24 * 6,    // still within refund window
        fee_bps: 50,
        fee_receiver: "0xFeeReceiverAddress000000000000000000000000".into(),
    };

    let payment_id =
        "0xdeadbeef00000000000000000000000000000000000000000000000000000002";

    // ----------------------------------------------------------------
    // Check current refundable balance before acting
    // ----------------------------------------------------------------

    let state = client
        .payments
        .get(payment_id)
        .await
        .unwrap_or_else(|e| panic!("get: {e}"));

    println!("Refundable balance: {}", state.state.refundable_amount); // e.g. "50000000"

    // ----------------------------------------------------------------
    // Refund — partial or full
    // ----------------------------------------------------------------

    let tx = client
        .payments
        .refund(
            payment_id,
            RefundParams {
                payment: payment.clone(),
                amount: "50000000".into(), // partial refund — 50 USDC out of 50 captured
            },
        )
        .await
        .unwrap_or_else(|e| {
            if let Rail0Error::Api { code, message, .. } = &e {
                // Common: RefundExpired, InvalidRefundAmount, NotPayee
                panic!("Refund failed [{code}]: {message}");
            }
            panic!("refund: {e}");
        });

    println!("Refunded: {}", tx.transaction_hash);
}
