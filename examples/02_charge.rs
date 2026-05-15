// One-shot payment: charge
//
// Combines authorize and capture in a single transaction — funds go
// directly from the buyer to the merchant with no escrow window.
// Use this when there is no need for a hold period (e.g. digital goods,
// instant fulfilment).
//
// On-chain flow:
//
//   buyer signs EIP-3009 → charge()  funds move buyer → merchant (minus fee), atomically
//
// Run:
//
//   cargo run --example 02_charge

use std::time::{SystemTime, UNIX_EPOCH};

use rail0::{ChargeParams, ClientOptions, Payment, Rail0Client, Rail0Error};

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
        token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // USDC on Base
        max_amount: "25000000".into(),                              // 25 USDC
        authorization_expiry: now + 60 * 5,  // short window — charge captures immediately
        refund_expiry: now + 60 * 60 * 24 * 30, // 30-day refund window
        fee_bps: 0,
        fee_receiver: "0x0000000000000000000000000000000000000000".into(),
    };

    let payment_id =
        "0xdeadbeef00000000000000000000000000000000000000000000000000000001";

    // Fetch the charge nonce (different from the authorize nonce).
    let nonce_resp = client
        .payments
        .charge_nonce(payment_id, &payment.payer)
        .await
        .unwrap_or_else(|e| panic!("charge_nonce: {e}"));

    // The buyer signs transferWithAuthorization off-chain using sign_charge:
    //
    //   let key = rail0::hex_to_private_key("0xYourPrivateKey").unwrap();
    //   let sig = rail0::sign_charge(&rail0::SignPaymentParams {
    //       private_key: key,
    //       payment: payment.clone(),
    //       amount: 25_000_000,
    //       nonce: nonce_resp.nonce.clone(),
    //       contract_address: "0xRAIL0ContractAddress".into(),
    //       token_domain: rail0::TokenDomain {
    //           name: "USD Coin".into(), version: "2".into(), chain_id: 8453,
    //           verifying_contract: payment.token.clone(),
    //       },
    //       valid_after: None,
    //       valid_before: None,
    //   }).unwrap();

    let tx = client
        .payments
        .charge(
            payment_id,
            ChargeParams {
                payment: payment.clone(),
                amount: "25000000".into(), // 25 USDC — exact amount, no hold
                v: 27,
                r: "0x1111111111111111111111111111111111111111111111111111111111111111".into(),
                s: "0x2222222222222222222222222222222222222222222222222222222222222222".into(),
            },
        )
        .await
        .unwrap_or_else(|e| {
            if let Rail0Error::Api { code, message, .. } = &e {
                panic!("Charge failed [{code}]: {message}");
            }
            panic!("charge: {e}");
        });

    println!("Charged: {} — status: {:?}", tx.transaction_hash, tx.status);
    println!("Nonce used: {}", nonce_resp.nonce);
}
