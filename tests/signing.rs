/// EIP-712 / EIP-3009 signing tests.
///
/// Fixtures mirror rail0-go/test/signing_test.go and rail0-ts/test/signing.test.ts
/// so all three SDKs produce identical signatures for the same inputs.
use rail0::{
    hex_to_private_key, sign_authorize, sign_charge, sign_transfer_with_authorization,
    SignPaymentParams, SignTransferParams, TokenDomain,
    PaymentConfig,
};

// Anvil account #0 — widely used test key, never use in production.
const PRIVATE_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const PAYER: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
const CONTRACT: &str = "0x5FbDB2315678afecb367f032d93F642f64180aa3";
const TOKEN: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const NONCE: &str = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

fn domain() -> TokenDomain {
    TokenDomain {
        name: "USD Coin".into(),
        version: "2".into(),
        chain_id: 8453,
        verifying_contract: TOKEN.into(),
    }
}

fn payment() -> PaymentConfig {
    PaymentConfig {
        payer: PAYER.into(),
        payee: "0x70997970C51812dc3A010C7d01b50e0d17dc79C8".into(),
        token: TOKEN.into(),
        max_amount: "100000000".into(),
        authorization_expiry: 9_999_999_999,
        refund_expiry: 9_999_999_999,
        fee_bps: 0,
        fee_receiver: "0x0000000000000000000000000000000000000000".into(),
    }
}

// ================================================================
//  hex_to_private_key
// ================================================================

#[test]
fn hex_to_private_key_roundtrip() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    assert_eq!(key.len(), 32);
}

#[test]
fn hex_to_private_key_without_prefix() {
    let without = &PRIVATE_KEY[2..];
    let key = hex_to_private_key(without).unwrap();
    assert_eq!(key.len(), 32);
    assert_eq!(key, hex_to_private_key(PRIVATE_KEY).unwrap());
}

#[test]
fn hex_to_private_key_bad_hex_errors() {
    assert!(hex_to_private_key("0xnothex").is_err());
}

#[test]
fn hex_to_private_key_wrong_length_errors() {
    assert!(hex_to_private_key("0xdeadbeef").is_err());
}

// ================================================================
//  sign_transfer_with_authorization
// ================================================================

#[test]
fn sign_transfer_produces_v_27_or_28() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let sig = sign_transfer_with_authorization(
        &key,
        &domain(),
        SignTransferParams {
            from: PAYER.into(),
            to: CONTRACT.into(),
            value: 50_000_000,
            valid_after: None,
            valid_before: 9_999_999_999,
            nonce: NONCE.into(),
        },
    )
    .unwrap();
    assert!(sig.v == 27 || sig.v == 28, "v must be 27 or 28, got {}", sig.v);
}

#[test]
fn sign_transfer_r_and_s_are_hex_bytes32() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let sig = sign_transfer_with_authorization(
        &key,
        &domain(),
        SignTransferParams {
            from: PAYER.into(),
            to: CONTRACT.into(),
            value: 50_000_000,
            valid_after: None,
            valid_before: 9_999_999_999,
            nonce: NONCE.into(),
        },
    )
    .unwrap();
    assert!(sig.r.starts_with("0x"), "r must start with 0x");
    assert_eq!(sig.r.len(), 66, "r must be 66 chars (0x + 32 bytes)");
    assert!(sig.s.starts_with("0x"), "s must start with 0x");
    assert_eq!(sig.s.len(), 66, "s must be 66 chars");
}

#[test]
fn sign_transfer_is_deterministic() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let params = || SignTransferParams {
        from: PAYER.into(),
        to: CONTRACT.into(),
        value: 50_000_000,
        valid_after: None,
        valid_before: 9_999_999_999,
        nonce: NONCE.into(),
    };
    let sig1 = sign_transfer_with_authorization(&key, &domain(), params()).unwrap();
    let sig2 = sign_transfer_with_authorization(&key, &domain(), params()).unwrap();
    assert_eq!(sig1.v, sig2.v);
    assert_eq!(sig1.r, sig2.r);
    assert_eq!(sig1.s, sig2.s);
}

#[test]
fn sign_transfer_changes_with_different_amount() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let make = |value: u128| {
        sign_transfer_with_authorization(
            &key,
            &domain(),
            SignTransferParams {
                from: PAYER.into(),
                to: CONTRACT.into(),
                value,
                valid_after: None,
                valid_before: 9_999_999_999,
                nonce: NONCE.into(),
            },
        )
        .unwrap()
    };
    let sig1 = make(50_000_000);
    let sig2 = make(25_000_000);
    assert_ne!(sig1.r, sig2.r);
}

// ================================================================
//  sign_authorize
// ================================================================

#[test]
fn sign_authorize_valid_signature() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let sig = sign_authorize(&SignPaymentParams {
        private_key: key,
        payment: payment(),
        amount: 50_000_000,
        nonce: NONCE.into(),
        contract_address: CONTRACT.into(),
        token_domain: domain(),
    })
    .unwrap();
    assert!(sig.v == 27 || sig.v == 28);
    assert_eq!(sig.r.len(), 66);
    assert_eq!(sig.s.len(), 66);
}

#[test]
fn sign_authorize_is_deterministic() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let params = || SignPaymentParams {
        private_key: key.clone(),
        payment: payment(),
        amount: 50_000_000,
        nonce: NONCE.into(),
        contract_address: CONTRACT.into(),
        token_domain: domain(),
    };
    assert_eq!(sign_authorize(&params()).unwrap().r, sign_authorize(&params()).unwrap().r);
}

// ================================================================
//  sign_charge
// ================================================================

#[test]
fn sign_charge_valid_signature() {
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let sig = sign_charge(&SignPaymentParams {
        private_key: key,
        payment: payment(),
        amount: 25_000_000,
        nonce: NONCE.into(),
        contract_address: CONTRACT.into(),
        token_domain: domain(),
    })
    .unwrap();
    assert!(sig.v == 27 || sig.v == 28);
    assert_eq!(sig.r.len(), 66);
}

#[test]
fn sign_charge_differs_from_authorize_with_same_nonce() {
    // Same nonce but different amounts should produce different signatures.
    let key = hex_to_private_key(PRIVATE_KEY).unwrap();
    let base = SignPaymentParams {
        private_key: key,
        payment: payment(),
        amount: 50_000_000,
        nonce: NONCE.into(),
        contract_address: CONTRACT.into(),
        token_domain: domain(),
    };
    let auth = sign_authorize(&base).unwrap();
    let mut charge_params = base.clone();
    charge_params.amount = 25_000_000;
    let charge = sign_charge(&charge_params).unwrap();
    assert_ne!(auth.r, charge.r);
}

// ================================================================
//  stablecoins
// ================================================================

#[test]
fn eip3009_tokens_base_contains_usdc() {
    let tokens = rail0::eip3009_tokens("base");
    assert!(tokens.iter().any(|t| t.symbol == "USDC"));
}

#[test]
fn eip3009_tokens_unknown_chain_returns_empty() {
    let tokens = rail0::eip3009_tokens("unknown-chain");
    assert!(tokens.is_empty());
}

#[test]
fn chain_info_returns_correct_chain_id() {
    assert_eq!(rail0::chain_info("base").unwrap().chain_id, 8453);
    assert_eq!(rail0::chain_info("ethereum").unwrap().chain_id, 1);
}
