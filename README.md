# rail0-rust

Rust SDK for the [RAIL0](https://github.com/rail0/rail0) stablecoin payment API.

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
use rail0::{Rail0Client, ClientOptions, Payment, AuthorizeParams, CaptureParams};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    let client = Rail0Client::new(ClientOptions {
        base_url: "https://api.rail0.xyz".into(),
        ..Default::default()
    });

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    let payment = Payment {
        payer:               "0xBuyer...".into(),
        payee:               "0xMerchant...".into(),
        token:               "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // USDC on Base
        max_amount:          "100000000".into(),        // 100 USDC (6 decimals)
        authorization_expiry: now + 3600 * 24,          // 24 h to capture
        refund_expiry:        now + 3600 * 24 * 7,      // 7-day refund window
        fee_bps:              0,
        fee_receiver:         "0x0000000000000000000000000000000000000000".into(),
    };

    let payment_id = "0xabc..."; // your unique identifier for this payment

    // Step 1 — get the nonce for the payer's EIP-3009 signature
    let nonce = client.payments.authorize_nonce(payment_id, &payment.payer).await.unwrap();

    // Step 2 — sign off-chain (payer's private key never leaves the client)
    let key = rail0::hex_to_private_key("0x...").unwrap();
    let sig = rail0::sign_authorize(&rail0::SignPaymentParams {
        private_key:      key,
        payment:          payment.clone(),
        amount:           50_000_000,
        nonce:            nonce.nonce,
        contract_address: "0xRAIL0ContractAddress...".into(),
        token_domain:     rail0::TokenDomain {
            name:               "USD Coin".into(),
            version:            "2".into(),
            chain_id:           8453,
            verifying_contract: payment.token.clone(),
        },
        valid_after:  None,
        valid_before: None,
    }).unwrap();

    // Step 3 — payer locks funds in escrow
    client.payments.authorize(payment_id, AuthorizeParams {
        payment: payment.clone(),
        amount: "50000000".into(),
        v: sig.v,
        r: sig.r,
        s: sig.s,
    }).await.unwrap();

    // Step 4 — merchant releases them
    let tx = client.payments.capture(payment_id, CaptureParams {
        payment: payment.clone(),
        amount: "50000000".into(),
    }).await.unwrap();

    println!("{} {:?}", tx.transaction_hash, tx.status);
}
```

## Payment lifecycle

```text
                    authorization_expiry         refund_expiry
                           │                         │
  ─────────────────────────┼─────────────────────────┼──────▶ time
   authorize / charge       │   capture / void         │   refund
                            │   release (permissionless)
```

| Operation | Caller | What it does |
|-----------|--------|--------------|
| `authorize` | payer | Locks `amount` in escrow via EIP-3009 signature |
| `charge` | payer | Authorize + capture in one transaction |
| `capture` | payee | Moves escrowed funds to the merchant |
| `void` | payee | Cancels the hold, returns funds to the payer |
| `release` | anyone | Reclaims escrow after `authorization_expiry` with no capture |
| `refund` | payee | Returns previously captured funds to the payer |

## API reference

### `Rail0Client::new(opts)`

```rust
let client = Rail0Client::new(ClientOptions {
    base_url:    "https://api.rail0.xyz".into(),
    headers:     [("Authorization".into(), "Bearer ...".into())].into(),
    timeout:     Duration::from_secs(30),   // default 30s
    max_retries: 3,                         // default 0 (no retry)
    retry_delay: Duration::from_millis(200), // base delay, doubles each attempt
    logger:      Some(debug_logger()),       // optional
    client:      None,                       // optional — custom reqwest::Client
});
```

---

### Logging

Pass any `Arc<dyn Fn(LogEntry) + Send + Sync>` as `logger` to receive structured log entries.

```rust
// Built-in logger — writes one line per request to stderr
let client = Rail0Client::new(ClientOptions {
    base_url: "https://api.rail0.xyz".into(),
    logger: Some(rail0::debug_logger()),
    ..Default::default()
});
```

Output:
```text
[rail0] GET 200 https://.../payments/0x... 87ms
[rail0] ERROR POST https://.../payments/0x.../authorize ! connection refused
```

To integrate with `tracing` or any other logging crate:

```rust
let client = Rail0Client::new(ClientOptions {
    logger: Some(Arc::new(|e: rail0::LogEntry| {
        if e.error.is_some() {
            tracing::error!(method = %e.method, url = %e.url, "rail0 request failed");
        } else {
            tracing::debug!(method = %e.method, status = ?e.status, ms = e.duration_ms, "rail0 request");
        }
    })),
    ..Default::default()
});
```

`LogEntry` fields:

| Field | Type | Present |
|-------|------|---------|
| `method` | `String` | always |
| `url` | `String` | always |
| `duration_ms` | `u64` | always |
| `request_body` | `Option<Value>` | POST requests |
| `status` | `Option<u16>` | when a response was received |
| `response_body` | `Option<Value>` | when a response was received |
| `error` | `Option<String>` | on HTTP errors and network failures |
| `attempt` | `Option<u32>` | when `max_retries > 0` |
| `will_retry` | `bool` | when `max_retries > 0` and a retry is scheduled |

---

### `client.payments`

All methods are `async` and return `Result<T, Rail0Error>`.

#### `.get(payment_id)`

Returns the on-chain state and config hash for a payment.

```rust
let res = client.payments.get(payment_id).await?;
// res.state: PaymentState { exists, capturable_amount, refundable_amount }
// res.config_hash: EIP-712 digest committed on creation
```

#### `.authorize(payment_id, params)`

Locks `amount` from the payer into escrow. Build the EIP-3009 signature with `sign_authorize`.

#### `.charge(payment_id, params)`

Authorize and capture in one transaction. Build the EIP-3009 signature with `sign_charge`.

#### `.capture(payment_id, params)` / `.void(payment_id, params)`

Capture escrowed funds or void (return them to payer). Caller must be the payee.

#### `.release(payment_id, params)`

Return escrowed funds to the payer after `authorization_expiry`. Permissionless.

#### `.refund(payment_id, params)`

Return a previously captured amount to the payer. Must be called before `refund_expiry`.

#### `.authorize_nonce(payment_id, payer)` / `.charge_nonce(payment_id, payer)`

Returns the EIP-3009 nonce to include in the payer's signature.

#### `.hash(payment)`

Computes the EIP-712 digest of a `Payment` configuration.

---

### `client.tokens`

#### `.is_accepted(address)`

Returns whether the given ERC-20 token is in this deployment's allowlist.

---

### `client.utils`

#### `.domain_separator()`

Returns the EIP-712 domain separator for the RAIL0 contract.

#### `.version()`

Returns the contract version number.

---

### Off-chain signing

RAIL0 uses EIP-3009 `transferWithAuthorization` — the payer signs a payload off-chain and the API
submits the transaction on their behalf (gasless for the payer).

```rust
// 1. Get the nonce for this (payment_id, payer) pair
let nonce = client.payments.authorize_nonce(payment_id, &payment.payer).await?;

// 2. Sign
let key = rail0::hex_to_private_key("0xYourPrivateKey...")?;
let sig = rail0::sign_authorize(&rail0::SignPaymentParams {
    private_key:      key,
    payment:          payment.clone(),
    amount:           50_000_000,
    nonce:            nonce.nonce,
    contract_address: "0xRAIL0...".into(),
    token_domain:     rail0::TokenDomain {
        name:               "USD Coin".into(),
        version:            "2".into(),
        chain_id:           8453,
        verifying_contract: payment.token.clone(),
    },
    valid_after:  None,
    valid_before: None,
})?;

// 3. Submit
client.payments.authorize(payment_id, AuthorizeParams {
    payment, amount: "50000000".into(), v: sig.v, r: sig.r, s: sig.s,
}).await?;
```

`sign_charge` works the same way — use `.charge_nonce` to obtain the nonce.

For raw control use `sign_transfer_with_authorization`:

```rust
let sig = rail0::sign_transfer_with_authorization(
    &key,
    &domain,
    rail0::SignTransferParams {
        from:         payment.payer.clone(),
        to:           contract_address.into(),
        value:        50_000_000,
        valid_after:  None,                              // 0 = immediate
        valid_before: payment.authorization_expiry as u128,
        nonce:        nonce.nonce,
    },
)?;
```

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

### Error handling

Every non-2xx response is returned as `Rail0Error::Api`:

```rust
use rail0::Rail0Error;

match client.payments.capture(payment_id, params).await {
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
| `PaymentMismatch` | `payment` config does not match the stored hash |
| `AuthorizationExpired` | `authorization_expiry` is in the past (capture) |
| `AuthorizationNotExpired` | `authorization_expiry` has not passed yet (release) |
| `RefundExpired` | `refund_expiry` is in the past |
| `InvalidAmount` | `amount` is 0 or exceeds `max_amount` |
| `TokenNotAccepted` | token is not in this deployment's allowlist |
| `NotPayee` | caller is not `payment.payee` |

---

## Development

### Run tests

```bash
cargo test
```

### Run examples

```bash
cargo run --example 01_authorize_and_capture
cargo run --example 02_charge
cargo run --example 03_refund
```

### Regenerate types after an API change

```bash
# 1. Drop the updated spec into gen/
cp path/to/new-openapi.json gen/openapi.json

# 2. Regenerate src/types_gen.rs
cargo run --bin generate

# 3. Check for breakage
cargo build
```

---

## Project structure

```text
gen/              OpenAPI spec + generation pipeline
  openapi.json    source of truth for the API surface
  generate.rs     generates src/types_gen.rs from the spec
  README.md

tests/            test suite
  signing.rs      signing utility tests (EIP-712 cross-check)
  client.rs       HTTP client tests (retry, logging, error handling)
  integration.rs  endpoint shape tests (mockito mock server)

src/              package rail0 — SDK source
  lib.rs          public re-exports
  client.rs       Rail0Client struct
  types.rs        public types (hand-documented)
  error.rs        Rail0Error
  http.rs         internal HTTP client (retry, logging)
  payments.rs     PaymentsClient
  tokens.rs       TokensClient
  utils.rs        UtilsClient
  signing.rs      EIP-712/EIP-3009 off-chain signing
  stablecoins.rs  stablecoin address registry

Cargo.toml
```

---

## License

[MIT](LICENSE)
