use hex;
use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};

use crate::error::Rail0Error;
use crate::types::{Address, Bytes32, PaymentConfig};

// ================================================================
//  EIP-712 type strings and pre-computed type hashes
// ================================================================

const DOMAIN_TYPE_STR: &str =
    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";
const TRANSFER_TYPE_STR: &str =
    "TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)";

fn domain_typehash() -> [u8; 32] {
    keccak256(DOMAIN_TYPE_STR.as_bytes())
}

fn transfer_typehash() -> [u8; 32] {
    keccak256(TRANSFER_TYPE_STR.as_bytes())
}

// ================================================================
//  Public types
// ================================================================

/// EIP-712 domain of the ERC-20 token (NOT the RAIL0 contract).
///
/// For USDC on Base: `name = "USD Coin"`, `version = "2"`, `chain_id = 8453`,
/// `verifying_contract = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"`.
#[derive(Debug, Clone)]
pub struct TokenDomain {
    /// Token's EIP-712 name, e.g. `"USD Coin"` for USDC.
    pub name: String,
    /// Token's EIP-712 version string, e.g. `"2"` for USDC.
    pub version: String,
    pub chain_id: u64,
    /// Token contract address used as `verifyingContract` in the domain.
    pub verifying_contract: Address,
}

/// EIP-3009 `transferWithAuthorization` signature, ready to spread into [`AuthorizeParams`](crate::AuthorizeParams).
#[derive(Debug, Clone)]
pub struct Eip3009Signature {
    /// Recovery identifier (27 or 28).
    pub v: u8,
    pub r: Bytes32,
    pub s: Bytes32,
}

/// Parameters for a raw `transferWithAuthorization` signature.
///
/// Prefer [`sign_authorize`] or [`sign_charge`] for typical RAIL0 flows.
#[derive(Debug, Clone)]
pub struct SignTransferParams {
    pub from: Address,
    /// Recipient of the transfer — the RAIL0 contract address.
    pub to: Address,
    /// Amount in token base units (e.g. 6 decimals for USDC). Fits in `u128` for all RAIL0 ops.
    pub value: u128,
    /// Earliest timestamp the signature is valid. `None` means 0 (immediate).
    pub valid_after: Option<u128>,
    /// Latest timestamp the signature is valid.
    pub valid_before: u128,
    /// Unique bytes32 that must not have been used before for this `(from, to)` pair.
    pub nonce: Bytes32,
}

/// Parameters for [`sign_authorize`] and [`sign_charge`].
///
/// Obtain `nonce` from [`payments.authorize_nonce`](crate::PaymentsClient::authorize_nonce)
/// or [`payments.charge_nonce`](crate::PaymentsClient::charge_nonce).
///
/// The contract hardcodes `validAfter=0` and `validBefore=payment.authorization_expiry`;
/// these are not configurable by the caller.
#[derive(Debug, Clone)]
pub struct SignPaymentParams {
    /// 32-byte secp256k1 private key of the payer.
    /// Use [`hex_to_private_key`] to convert from a `0x`-prefixed hex string.
    pub private_key: Vec<u8>,
    pub payment: PaymentConfig,
    /// Amount to pull from the payer, in token base units.
    pub amount: u128,
    /// Nonce from `authorize_nonce` or `charge_nonce`.
    pub nonce: Bytes32,
    pub contract_address: Address,
    pub token_domain: TokenDomain,
}

// ================================================================
//  Helper: private key conversion
// ================================================================

/// Decodes a `0x`-prefixed or raw 64-char hex string into 32 raw bytes.
///
/// ```no_run
/// let key = rail0::hex_to_private_key("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").unwrap();
/// ```
pub fn hex_to_private_key(hex_key: &str) -> Result<Vec<u8>, Rail0Error> {
    let stripped = hex_key.strip_prefix("0x").unwrap_or(hex_key);
    let bytes = hex::decode(stripped)
        .map_err(|e| Rail0Error::InvalidInput(format!("invalid private key hex: {e}")))?;
    if bytes.len() != 32 {
        return Err(Rail0Error::InvalidInput(
            "private key must be exactly 32 bytes".into(),
        ));
    }
    Ok(bytes)
}

// ================================================================
//  Internal: keccak256 and ABI encoding
// ================================================================

pub(crate) fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut h = Keccak256::new();
    h.update(data);
    h.finalize().into()
}

fn encode_address(addr: &str) -> [u8; 32] {
    let hex_str = addr.strip_prefix("0x").unwrap_or(addr);
    let bytes = hex::decode(hex_str).unwrap_or_default();
    let mut out = [0u8; 32];
    if bytes.len() <= 20 {
        out[32 - bytes.len()..].copy_from_slice(&bytes);
    }
    out
}

fn encode_uint256(v: u128) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[16..].copy_from_slice(&v.to_be_bytes());
    out
}

fn encode_bytes32(h: &str) -> [u8; 32] {
    let hex_str = h.strip_prefix("0x").unwrap_or(h);
    let bytes = hex::decode(hex_str).unwrap_or_default();
    let mut out = [0u8; 32];
    let len = bytes.len().min(32);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

// ================================================================
//  EIP-712 digest construction
// ================================================================

fn hash_domain_separator(domain: &TokenDomain) -> [u8; 32] {
    let name_hash = keccak256(domain.name.as_bytes());
    let version_hash = keccak256(domain.version.as_bytes());
    let mut encoded = Vec::with_capacity(5 * 32);
    encoded.extend_from_slice(&domain_typehash());
    encoded.extend_from_slice(&name_hash);
    encoded.extend_from_slice(&version_hash);
    encoded.extend_from_slice(&encode_uint256(domain.chain_id as u128));
    encoded.extend_from_slice(&encode_address(&domain.verifying_contract));
    keccak256(&encoded)
}

fn hash_struct_transfer(
    from: &str,
    to: &str,
    value: u128,
    valid_after: u128,
    valid_before: u128,
    nonce: &str,
) -> [u8; 32] {
    let mut encoded = Vec::with_capacity(7 * 32);
    encoded.extend_from_slice(&transfer_typehash());
    encoded.extend_from_slice(&encode_address(from));
    encoded.extend_from_slice(&encode_address(to));
    encoded.extend_from_slice(&encode_uint256(value));
    encoded.extend_from_slice(&encode_uint256(valid_after));
    encoded.extend_from_slice(&encode_uint256(valid_before));
    encoded.extend_from_slice(&encode_bytes32(nonce));
    keccak256(&encoded)
}

fn build_digest(
    domain: &TokenDomain,
    from: &str,
    to: &str,
    value: u128,
    valid_after: u128,
    valid_before: u128,
    nonce: &str,
) -> [u8; 32] {
    let ds = hash_domain_separator(domain);
    let sh = hash_struct_transfer(from, to, value, valid_after, valid_before, nonce);
    let mut msg = [0u8; 66];
    msg[0] = 0x19;
    msg[1] = 0x01;
    msg[2..34].copy_from_slice(&ds);
    msg[34..66].copy_from_slice(&sh);
    keccak256(&msg)
}

// ================================================================
//  Core signing
// ================================================================

fn do_sign(private_key: &[u8], digest: &[u8; 32]) -> Result<Eip3009Signature, Rail0Error> {
    let signing_key = SigningKey::from_slice(private_key)
        .map_err(|e| Rail0Error::Sign(e.to_string()))?;

    let (sig, recovery_id) = signing_key
        .sign_prehash_recoverable(digest)
        .map_err(|e| Rail0Error::Sign(e.to_string()))?;

    let v = recovery_id.to_byte() + 27;
    let sig_bytes = sig.to_bytes();
    let r = format!("0x{}", hex::encode(&sig_bytes[..32]));
    let s = format!("0x{}", hex::encode(&sig_bytes[32..]));
    Ok(Eip3009Signature { v, r, s })
}

// ================================================================
//  Public signing API
// ================================================================

/// Signs a raw EIP-3009 `transferWithAuthorization` message.
///
/// For RAIL0 payment flows prefer [`sign_authorize`] or [`sign_charge`], which derive
/// `from`, `to`, and `valid_before` automatically from the [`PaymentConfig`] struct.
pub fn sign_transfer_with_authorization(
    private_key: &[u8],
    domain: &TokenDomain,
    params: SignTransferParams,
) -> Result<Eip3009Signature, Rail0Error> {
    let valid_after = params.valid_after.unwrap_or(0);
    let digest = build_digest(
        domain,
        &params.from,
        &params.to,
        params.value,
        valid_after,
        params.valid_before,
        &params.nonce,
    );
    do_sign(private_key, &digest)
}

/// Signs the EIP-3009 payload required by an `authorize` call.
///
/// The nonce comes from `create_payment` response: `resp.signing_payload.message.nonce`.
///
/// ```no_run
/// # use rail0::*;
/// # async fn example(client: &Rail0Client) {
/// # let payment: PaymentConfig = todo!();
/// # let create_resp: CreatePaymentResponse = todo!();
/// let key = hex_to_private_key("0xYourPrivateKey").unwrap();
/// let sig = sign_authorize(&SignPaymentParams {
///     private_key: key,
///     payment,
///     amount: 50_000_000,
///     nonce: create_resp.signing_payload.message.nonce,
///     contract_address: create_resp.rail0_contract,
///     token_domain: TokenDomain {
///         name: create_resp.signing_payload.domain.name,
///         version: create_resp.signing_payload.domain.version,
///         chain_id: create_resp.signing_payload.domain.chain_id as u64,
///         verifying_contract: create_resp.signing_payload.domain.verifying_contract,
///     },
/// }).unwrap();
/// # }
/// ```
pub fn sign_authorize(params: &SignPaymentParams) -> Result<Eip3009Signature, Rail0Error> {
    let digest = build_digest(
        &params.token_domain,
        &params.payment.payer,
        &params.contract_address,
        params.amount,
        0,
        params.payment.authorization_expiry as u128,
        &params.nonce,
    );
    do_sign(&params.private_key, &digest)
}

/// Signs the EIP-3009 payload required by a `charge` call.
///
/// Use the nonce from `create_payment` response (`signing_payload.message.nonce`),
/// obtained with `mode: "charge"` to get a charge-specific nonce.
pub fn sign_charge(params: &SignPaymentParams) -> Result<Eip3009Signature, Rail0Error> {
    sign_authorize(params)
}
