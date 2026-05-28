# rail0-rust

Rust SDK for the [RAIL0](https://github.com/commercelayer/rail0) stablecoin payment API.

RAIL0 is an immutable smart contract that brings the authorize → capture → refund lifecycle of card networks to stablecoin payments — no intermediaries, no protocol fees, no permission required. This SDK wraps the REST API that sits in front of the contract, giving you fully-typed access to every operation.

## Requirements

- Rust ≥ 1.75
- Tokio async runtime

## Installation

```toml
[dependencies]
rail0 = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick start

```rust
use rail0::{Rail0Client, ClientOptions, CreatePaymentRequest, PaymentInput,
            PayerSignatureRequest, SubmitTransactionRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Rail0Client::new(ClientOptions {
        base_url: "https://api.rail0.xyz".into(),
        ..Default::default()
    });

    // Step 1 — discover payment methods
    let methods = client.merchants.payment_methods(1).await?;
    let usdc = &methods[0]; // pick USDC on the target chain

    // Step 2 — create payment intent
    let resp = client.payments.create_payment(&CreatePaymentRequest {
        payment: PaymentInput {
            payer: "0xBuyer...".into(),
            payee: usdc.wallet_address.clone(),
            token: usdc.token_address.clone(),
            amount: "50000000".into(), // 50 USDC (6 decimals)
            ..Default::default()
        },
        chain_id: usdc.chain_id as i64,
        mode: "authorize".into(),
    }).await?;

    // Step 3 — payer signs the EIP-3009 payload off-chain
    let key = rail0::hex_to_private_key("0x...")?;
    let sig = rail0::sign_authorize(&rail0::SignPaymentParams {
        private_key:      key,
        payment:          resp.payment.clone(),
        amount:           resp.payment.amount.parse()?,
        nonce:            resp.signing_payload.message.nonce.clone(),
        contract_address: resp.rail0_contract.clone(),
        token_domain: rail0::TokenDomain {
            name: "USD Coin".into(), version: "2".into(),
            chain_id: usdc.chain_id as i64,
            verifying_contract: usdc.token_address.clone(),
        },
        valid_after: None, valid_before: None,
    })?;

    // Step 4 — submit payer signature (single 65-byte hex string)
    client.payments.sign(&resp.rail0_id, &PayerSignatureRequest {
        signature: sig.to_hex(), // "0x" + 130 hex chars
    }).await?;

    // Step 5 — payee prepares the unsigned authorize tx
    let tx = client.payments.authorize(&resp.rail0_id).await?;
    // sign tx.unsigned_transaction with payee's EIP-1559 key

    // Step 6 — broadcast signed authorize tx
    client.payments.submit(&resp.rail0_id, &SubmitTransactionRequest {
        signed_transaction: signed_bytes,
    }).await?;

    println!("authorized: {}", resp.rail0_id);
    Ok(())
}
```

## Payment lifecycle

```text
                            authorizationExpiry       refundExpiry
                                   │                       │
  ─────────────────────────────────┼───────────────────────┼──────▶ time
   create → sign → authorize       │   capture / void       │   approve+refund
                                    │   release              │
```

| Operation | Caller | What it does |
|-----------|--------|--------------|
| `authorize` + `submit` | payee | Prepare + broadcast the authorize tx; funds move to escrow |
| `charge` | payee | Server-side one-shot: authorize + capture with no escrow window |
| `prepare_capture` + `submit` | payee | Moves escrowed funds to the merchant |
| `prepare_void` + `submit` | payee | Cancels the hold, returns funds to the payer |
| `prepare_release` + `submit` | anyone | Reclaims escrow after `authorization_expiry` |
| `prepare_approve` + `submit` | payee | ERC-20 `approve()` required before a refund |
| `prepare_refund` + `submit` | payee | Returns captured funds to the payer |

## API reference

### `Rail0Client::new(opts)`

```rust
let client = Rail0Client::new(ClientOptions {
    base_url:    "https://api.rail0.xyz".into(),
    headers:     [("Authorization".into(), "Bearer ...".into())].into(),
    timeout:     Duration::from_secs(30),    // default 30s
    max_retries: 3,                          // default 0 (no retry)
    retry_delay: Duration::from_millis(200), // base delay, doubles each attempt
    logger:      Some(rail0::debug_logger()),
    ..Default::default()
});
```

---

### Logging

Pass any `Arc<dyn Fn(LogEntry) + Send + Sync>` as `logger` to receive structured log entries.

```rust
let client = Rail0Client::new(ClientOptions {
    logger: Some(rail0::debug_logger()), // writes one line per request to stderr
    ..Default::default()
});
```

Output:
```text
[rail0] GET 200 https://.../payments/0x... 87ms
[rail0] ERROR POST https://.../payments/0x.../capture ! connection refused
```

To integrate with `tracing`:

```rust
let client = Rail0Client::new(ClientOptions {
    logger: Some(Arc::new(|e: rail0::LogEntry| {
        if e.error.is_some() {
            tracing::error!(method = %e.method, url = %e.url, "rail0 request failed");
        } else {
            tracing::debug!(method = %e.method, status = ?e.status, ms = e.duration_ms);
        }
    })),
    ..Default::default()
});
```

---

### `client.merchants`

#### `.payment_methods(merchant_id)` → `Result<Vec<PaymentMethod>, Rail0Error>`

Returns the active payment methods (chain + token + wallet) for a merchant.

```rust
let methods = client.merchants.payment_methods(1).await?;
// methods[0].chain_id, .token_address, .wallet_address, .token_symbol, .chain_slug
```

---

### `client.payments`

All methods are `async` and return `Result<T, Rail0Error>`.

#### `.get(payment_id)` → `PaymentResponse`

Fetches the current payment state (DB status + live on-chain escrow balances).

```rust
let res = client.payments.get(&payment_id).await?;
// res.status, res.on_chain.capturable_amount, res.on_chain.refundable_amount
```

#### `.create_payment(params)` → `CreatePaymentResponse`

Creates a payment intent. Returns `signing_payload` for the payer to sign, plus `rail0_contract`.

#### `.sign(payment_id, params)` → `PayerSignatureResponse`

Submits the payer's EIP-712 signature as a single 65-byte hex string.

#### `.authorize(payment_id)` → `PrepareTransactionResponse`

Prepares the unsigned `authorize()` transaction. Called by the payee. Sign `unsigned_transaction` with the payee's key and pass to `submit`.

#### `.submit(payment_id, params)` → `AuthorizePaymentResponse`

Broadcasts the signed authorize transaction. Funds are moved to escrow.

```rust
let tx = client.payments.authorize(&payment_id).await?;
let res = client.payments.submit(&payment_id, &SubmitTransactionRequest {
    signed_transaction: signed_bytes,
}).await?;
// res.transaction_hash, res.capturable_amount
```

#### `.charge(payment_id)` → `ChargePaymentResponse`

Server-side one-shot: authorize + capture in a single transaction. No `submit` step. Called by the payee.

#### `.prepare_capture(payment_id, params)` / `.submit(payment_id, params)`

Build and broadcast the capture transaction. Partial captures are supported.

```rust
let tx = client.payments.prepare_capture(&payment_id, &CapturePaymentRequest {
    amount: "50000000".into(),
}).await?;
let res = client.payments.submit(&payment_id, &SubmitTransactionRequest {
    signed_transaction: signed,
}).await?;
// res.captured_amount, res.capturable_amount, res.refundable_amount
```

#### `.prepare_void(payment_id)` / `.submit(payment_id, params)`

Void the authorization — releases all escrowed funds to the payer.

#### `.prepare_release(payment_id, params)` / `.submit(payment_id, params)`

Release escrowed funds after `authorization_expiry`. Set `caller_address` in `ReleaseRequest` for buyer-initiated release.

```rust
let tx = client.payments.prepare_release(&payment_id, &ReleaseRequest {
    caller_address: Some(buyer_address),
}).await?;
client.payments.submit(&payment_id, &SubmitTransactionRequest {
    signed_transaction: buyer_signed,
}).await?;
```

#### `.prepare_approve(payment_id, params)` / `.submit(payment_id, params)`

ERC-20 `approve()` before a refund. Include `amount` in `SubmitApproveRequest` so the API records it.

```rust
let tx = client.payments.prepare_approve(&payment_id, &ApproveRequest {
    amount: "50000000".into(),
}).await?;
client.payments.submit(&payment_id, &SubmitApproveRequest {
    signed_transaction: signed,
    amount: Some("50000000".into()),
}).await?;
```

#### `.prepare_refund(payment_id, params)` / `.submit(payment_id, params)`

Build and broadcast the refund transaction. Partial refunds are supported.

---

## Off-chain signing

RAIL0 uses EIP-3009 `transferWithAuthorization` — the payer signs off-chain and the API submits on their behalf (gasless for the payer).

```rust
let key = rail0::hex_to_private_key("0xYourPrivateKey...")?;
let sig = rail0::sign_authorize(&rail0::SignPaymentParams {
    private_key:      key,
    payment:          resp.payment.clone(),
    amount:           resp.payment.amount.parse()?,
    nonce:            resp.signing_payload.message.nonce.clone(),
    contract_address: resp.rail0_contract.clone(),
    token_domain: rail0::TokenDomain {
        name: "USD Coin".into(), version: "2".into(),
        chain_id: 84532, verifying_contract: token.clone(),
    },
    valid_after: None, valid_before: None,
})?;
// sig.to_hex() → "0x..." (65-byte hex) — pass to sign()
```

Use `sign_charge` instead of `sign_authorize` when `mode: "charge"`.

---

### Stablecoin registry

```rust
// All EIP-3009 tokens on Base (compatible with RAIL0)
let tokens = rail0::eip3009_tokens("base");
// tokens[0]: StablecoinToken { symbol: "USDC", address: "0x833...", decimals: 6 }

// Chain metadata
let info = rail0::chain_info("base").unwrap();
println!("chain_id: {}", info.chain_id); // 8453
```

---

## Error handling

Every non-2xx response is returned as `Rail0Error::Api`:

```rust
use rail0::Rail0Error;

match client.payments.submit(&payment_id, &params).await {
    Err(Rail0Error::Api { status, code, message }) => {
        eprintln!("HTTP {status}: [{code}] {message}");
    }
    Err(Rail0Error::Http(e)) => eprintln!("network error: {e}"),
    Err(e) => eprintln!("other error: {e}"),
    Ok(tx) => println!("{}", tx.transaction_hash),
}
```

Common error codes:

| Code | Cause |
|------|-------|
| `PaymentAlreadyExists` | `authorize`/`charge` called twice with the same `payment_id` |
| `PaymentNotFound` | `payment_id` does not exist |
| `AuthorizationExpired` | `authorization_expiry` is in the past (capture) |
| `AuthorizationNotExpired` | `authorization_expiry` has not passed yet (release) |
| `RefundExpired` | `refund_expiry` is in the past |
| `InvalidAmount` | `amount` is 0 |
| `NotPayee` | caller is not `payment.payee` |

---

## Development

```bash
cargo test

# Regenerate src/types_gen.rs after an API change:
# 1. Update the schema in rail0-api (sibling repo),
#    or set RAIL0_SCHEMA_PATH to point to a local file.
# 2. Regenerate:
cargo run --bin generate
cargo build
```

## Project structure

```text
src/
  lib.rs          public re-exports
  client.rs       Rail0Client
  merchants.rs    MerchantsClient
  payments.rs     PaymentsClient
  types.rs        hand-documented types
  types_gen.rs    generated types (never hand-edited)
  signing.rs      EIP-712/EIP-3009 off-chain signing
  stablecoins.rs  stablecoin address registry
  http.rs         internal HTTP client (retry, logging)
  error.rs        Rail0Error

gen/
  generate.rs     generates types_gen.rs from the schema

Cargo.toml
```

---

## License

[MIT](LICENSE)
