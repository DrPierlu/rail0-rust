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
pub struct PaymentConfig {
    pub payer: Address,
    pub payee: Address,
    pub token: Address,
    pub max_amount: Uint256String,
    pub authorization_expiry: i64,
    pub refund_expiry: i64,
    pub fee_bps: u32,
    pub fee_receiver: Address,
}

/// EIP-712 domain for the token contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EIP712Domain {
    pub name: String,
    pub version: String,
    pub chain_id: i64,
    pub verifying_contract: Address,
}

/// A single field entry in EIP-712 type definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EIP712TypeEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
}

/// Type definitions for the SigningPayload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EIP712Types {
    pub transfer_with_authorization: Vec<EIP712TypeEntry>,
}

/// Message fields for the EIP-3009 TransferWithAuthorization typed-data signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EIP3009Message {
    pub from: Address,
    pub to: Address,
    pub value: Uint256String,
    pub valid_after: Uint256String,
    pub valid_before: Uint256String,
    pub nonce: Bytes32,
}

/// EIP-712 typed-data structure returned by `POST /payments`.
/// Pass verbatim to `eth_signTypedData_v4`, or compute the digest manually
/// with any EIP-712 library and sign with secp256k1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningPayload {
    pub domain: EIP712Domain,
    pub types: EIP712Types,
    pub primary_type: String,
    pub message: EIP3009Message,
}

// ================================================================
//  Request bodies
// ================================================================

/// Request body for [`payments.create_payment`](crate::PaymentsClient::create_payment).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentRequest {
    pub payment: PaymentConfig,
    pub amount: Uint256String,
    pub chain_id: i64,
    /// `"authorize"` or `"charge"`.
    pub mode: String,
}

/// Request body for [`payments.sign`](crate::PaymentsClient::sign).
#[derive(Debug, Clone, Serialize)]
pub struct PayerSignatureRequest {
    pub v: u8,
    pub r: Bytes32,
    pub s: Bytes32,
}

/// Request body for [`payments.prepare_capture`](crate::PaymentsClient::prepare_capture).
#[derive(Debug, Clone, Serialize)]
pub struct CapturePaymentRequest {
    pub amount: Uint256String,
}

/// Request body for submit operations (capture, void, approve, refund).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTransactionRequest {
    pub signed_transaction: String,
}

/// Request body for [`payments.prepare_approve`](crate::PaymentsClient::prepare_approve).
#[derive(Debug, Clone, Serialize)]
pub struct ApproveRequest {
    pub amount: Uint256String,
}

/// Request body for [`payments.prepare_refund`](crate::PaymentsClient::prepare_refund).
#[derive(Debug, Clone, Serialize)]
pub struct RefundPaymentRequest {
    pub amount: Uint256String,
}

// ================================================================
//  Response shapes
// ================================================================

/// Returned by [`payments.create_payment`](crate::PaymentsClient::create_payment).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentResponse {
    pub payment_id: Bytes32,
    pub config_hash: Bytes32,
    pub payment: PaymentConfig,
    pub amount: Uint256String,
    pub chain_id: i64,
    pub rail0_contract: Address,
    pub signing_payload: SigningPayload,
}

/// Returned by [`payments.sign`](crate::PaymentsClient::sign).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayerSignatureResponse {
    pub payment_id: Bytes32,
    pub status: String,
    pub recovered_payer: Option<Address>,
}

/// Returned by [`payments.authorize`](crate::PaymentsClient::authorize).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizePaymentResponse {
    pub payment_id: Bytes32,
    pub transaction_hash: Bytes32,
    pub capturable_amount: Uint256String,
    pub authorization_expiry: Option<i64>,
}

/// Returned by [`payments.charge`](crate::PaymentsClient::charge).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargePaymentResponse {
    pub payment_id: Bytes32,
    pub transaction_hash: Bytes32,
    pub charged_amount: Uint256String,
    pub fee_amount: Uint256String,
    pub refundable_amount: Uint256String,
}

/// Returned by prepare operations. An unsigned EIP-1559 transaction ready for signing.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareTransactionResponse {
    pub unsigned_transaction: String,
    pub to: Address,
    pub data: String,
    pub chain_id: i64,
    pub nonce: i64,
    pub max_fee_per_gas: Uint256String,
    pub max_priority_fee_per_gas: Uint256String,
    pub gas_limit: Uint256String,
}

/// Returned by [`payments.submit_capture`](crate::PaymentsClient::submit_capture).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturePaymentResponse {
    pub payment_id: Bytes32,
    pub transaction_hash: Bytes32,
    pub captured_amount: Uint256String,
    pub fee_amount: Option<Uint256String>,
    pub capturable_amount: Uint256String,
    pub refundable_amount: Uint256String,
    pub authorization_expiry: Option<i64>,
}

/// Returned by [`payments.submit_void`](crate::PaymentsClient::submit_void).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidPaymentResponse {
    pub payment_id: Bytes32,
    pub transaction_hash: Bytes32,
    pub released_amount: Uint256String,
}

/// Returned by [`payments.release`](crate::PaymentsClient::release).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePaymentResponse {
    pub payment_id: Bytes32,
    pub transaction_hash: Bytes32,
    pub released_amount: Uint256String,
}

/// Returned by [`payments.submit_approve`](crate::PaymentsClient::submit_approve).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveResponse {
    pub transaction_hash: Bytes32,
    pub token: Address,
    pub spender: Address,
    pub amount: Uint256String,
}

/// Returned by [`payments.submit_refund`](crate::PaymentsClient::submit_refund).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundPaymentResponse {
    pub payment_id: Bytes32,
    pub transaction_hash: Bytes32,
    pub refunded_amount: Uint256String,
    pub refundable_amount: Uint256String,
}

/// Shape of error responses from the RAIL0 API.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}
