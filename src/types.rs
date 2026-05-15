use serde::{Deserialize, Serialize};

/// Checksummed or lowercase Ethereum address (42 chars, `0x`-prefixed).
pub type Address = String;

/// 32-byte value, hex-encoded with `0x` prefix (66 chars total).
/// Used for payment IDs, hashes, and signature components.
pub type Bytes32 = String;

/// Unsigned 256-bit integer serialised as a decimal string.
/// Avoids precision loss for amounts that exceed `u64::MAX`.
pub type Uint256String = String;

/// Immutable payment configuration committed on the first `authorize` or `charge` call.
///
/// Every subsequent operation on the same `payment_id` must supply the exact same struct —
/// a mismatch causes the contract to revert with `PaymentMismatch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    /// Buyer address. Signs the EIP-3009 authorization for `authorize` / `charge`.
    pub payer: Address,
    /// Merchant address. Must call `capture`, `void`, or `refund`.
    pub payee: Address,
    /// ERC-20 token address. Must be in the allowlist and support EIP-3009.
    pub token: Address,
    /// Upper bound on what can be authorized (fits in `uint120` on-chain).
    pub max_amount: Uint256String,
    /// Unix timestamp after which `capture` is rejected and `release` opens.
    pub authorization_expiry: i64,
    /// Unix timestamp after which `refund` is rejected.
    pub refund_expiry: i64,
    /// Protocol fee in basis points (`0` = no fee).
    pub fee_bps: u32,
    /// Address that receives the protocol fee. Must be the zero address when `fee_bps` is `0`.
    pub fee_receiver: Address,
}

/// On-chain mutable state for a payment, packed in one storage slot.
///
/// - `capturable_amount` holds escrowed funds (`authorize` → `capture` / `void` / `release` path).
/// - `refundable_amount` holds already-disbursed funds (`capture` → `refund` path).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentState {
    /// `true` once a payment has been created via `authorize` or `charge`.
    pub exists: bool,
    /// Escrowed balance available for `capture` or `release`.
    pub capturable_amount: Uint256String,
    /// Balance already sent to the payee but still eligible for `refund`.
    pub refundable_amount: Uint256String,
}

/// Request body for `authorize` and `charge`.
///
/// `v`, `r`, `s` are the EIP-3009 `transferWithAuthorization` signature produced by the payer's
/// private key. Use [`sign_authorize`](crate::sign_authorize) or [`sign_charge`](crate::sign_charge)
/// to build the signature off-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeParams {
    pub payment: Payment,
    pub amount: Uint256String,
    /// Recovery identifier from the EIP-3009 signature (27 or 28).
    pub v: u8,
    pub r: Bytes32,
    pub s: Bytes32,
}

/// Request body for `charge` (one-shot `authorize` + `capture`). Same shape as [`AuthorizeParams`].
pub type ChargeParams = AuthorizeParams;

/// Request body for `capture`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureParams {
    pub payment: Payment,
    pub amount: Uint256String,
}

/// Request body for `void`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidParams {
    pub payment: Payment,
}

/// Request body for `release` (permissionless after `authorization_expiry`).
pub type ReleaseParams = VoidParams;

/// Request body for `refund`.
pub type RefundParams = CaptureParams;

/// Full on-chain state returned by [`payments.get`](crate::PaymentsClient::get).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    pub payment_id: Bytes32,
    pub state: PaymentState,
    /// EIP-712 digest of the `Payment` configuration committed on creation.
    pub config_hash: Bytes32,
}

/// Confirmation status of a submitted transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

/// Returned by every write operation. The transaction may still be pending confirmation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    pub transaction_hash: Bytes32,
    pub status: TransactionStatus,
}

/// Returned by [`tokens.is_accepted`](crate::TokensClient::is_accepted).
#[derive(Debug, Clone, Deserialize)]
pub struct TokenStatusResponse {
    pub address: Address,
    pub accepted: bool,
}

/// Returned by [`payments.authorize_nonce`](crate::PaymentsClient::authorize_nonce) and
/// [`payments.charge_nonce`](crate::PaymentsClient::charge_nonce).
///
/// Pass `nonce` into [`sign_authorize`](crate::sign_authorize) or
/// [`sign_charge`](crate::sign_charge) when building the EIP-3009 signature.
#[derive(Debug, Clone, Deserialize)]
pub struct NonceResponse {
    pub nonce: Bytes32,
}

/// EIP-712 digest of a `Payment` struct, returned by [`payments.hash`](crate::PaymentsClient::hash).
#[derive(Debug, Clone, Deserialize)]
pub struct HashResponse {
    pub hash: Bytes32,
}

/// EIP-712 domain separator of the RAIL0 contract.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainSeparatorResponse {
    pub domain_separator: Bytes32,
}

/// Contract version number.
#[derive(Debug, Clone, Deserialize)]
pub struct VersionResponse {
    pub version: u32,
}
