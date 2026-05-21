// Refund flow
//
// After a capture (or charge) the merchant can refund up to the full
// captured amount back to the payer, as long as refund_expiry has not
// passed. The payee must first approve the RAIL0 contract as a spender
// on the token (so the contract can pull funds back from the payee).
//
// On-chain flow:
//
//   payee signs approve tx → approve()  RAIL0 contract approved as spender
//   payee signs refund tx  → refund()   funds move payee → payer
//
// Run:
//
//   cargo run --example 03_refund

use rail0::{
    ApproveRequest, ClientOptions, Rail0Client, Rail0Error, RefundPaymentRequest,
    SubmitApproveRequest, SubmitTransactionRequest,
};

#[tokio::main]
async fn main() {
    let client = Rail0Client::new(ClientOptions {
        base_url: "https://api.rail0.xyz".into(),
        ..Default::default()
    });

    // Assume the payment was previously created and captured.
    let payment_id = "0xdeadbeef00000000000000000000000000000000000000000000000000000002";

    // ----------------------------------------------------------------
    // Step 1 — Payee approves the RAIL0 contract as token spender
    // ----------------------------------------------------------------

    let prep_approve = client
        .payments
        .prepare_approve(
            payment_id,
            &ApproveRequest {
                // unlimited approval
                amount: "115792089237316195423570985008687907853269984665640564039457584007913129639935".into(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("prepare_approve: {e}"));

    // Payee signs prep_approve.unsigned_transaction offline, then submits:
    //   let signed_approve = payee_wallet.sign_transaction(&prep_approve.unsigned_transaction);
    let signed_approve = "0x02f8..."; // placeholder

    let approve_resp = client
        .payments
        .submit_approve(
            payment_id,
            &SubmitApproveRequest { signed_transaction: signed_approve.into(), amount: None },
        )
        .await
        .unwrap_or_else(|e| panic!("submit_approve: {e}"));

    println!("Approved: tx={} spender={}", approve_resp.transaction_hash, approve_resp.spender);
    let _ = prep_approve;

    // ----------------------------------------------------------------
    // Step 2 — Payee prepares and submits a refund transaction
    // ----------------------------------------------------------------

    let prep_refund = client
        .payments
        .prepare_refund(
            payment_id,
            &RefundPaymentRequest { amount: "50000000".into() },
        )
        .await
        .unwrap_or_else(|e| panic!("prepare_refund: {e}"));

    // Payee signs prep_refund.unsigned_transaction offline, then submits:
    //   let signed_refund = payee_wallet.sign_transaction(&prep_refund.unsigned_transaction);
    let signed_refund = "0x02f8..."; // placeholder

    let refund_resp = client
        .payments
        .submit_refund(
            payment_id,
            &SubmitTransactionRequest { signed_transaction: signed_refund.into() },
        )
        .await
        .unwrap_or_else(|e| {
            if let Rail0Error::Api { code, message, .. } = &e {
                panic!("SubmitRefund failed [{code}]: {message}");
            }
            panic!("submit_refund: {e}");
        });

    println!(
        "Refunded: tx={} refunded={} remaining={}",
        refund_resp.transaction_hash, refund_resp.refunded_amount, refund_resp.refundable_amount
    );
    let _ = prep_refund;
}
