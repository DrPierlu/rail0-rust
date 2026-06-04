pub mod accounts;
pub mod auth;
pub mod chains;
pub mod tokens;
pub mod client;
pub mod error;
pub mod http;
pub mod payments;
pub mod signing;
pub mod stablecoins;
pub mod types;
pub mod types_gen;

// ================================================================
//  Flat public re-exports
// ================================================================

pub use accounts::AccountsClient;
pub use auth::{personal_sign, private_key_to_address, AuthClient, AuthResponse, NonceResponse};
pub use chains::{Blockchain, ChainsClient};
pub use tokens::{Token, TokensClient};
pub use client::Rail0Client;
pub use error::Rail0Error;
pub use http::{debug_logger, ClientOptions, LogEntry, Logger};
pub use payments::PaymentsClient;
pub use signing::{
    hex_to_private_key, sign_authorize, sign_charge, sign_transfer_with_authorization,
    Eip3009Signature, SignPaymentParams, SignTransferParams, TokenDomain,
};
pub use stablecoins::{
    chain_info, eip2612_tokens, eip3009_tokens, ChainStablecoins, StablecoinInfo, StablecoinToken,
};
pub use types::{
    Address, ApiError, AuthorizePaymentResponse, Bytes32, CapturePaymentRequest,
    CapturePaymentResponse, ChargePaymentResponse, CreatePaymentInput, CreatePaymentRequest,
    CreatePaymentResponse, EIP3009Message, EIP712Domain, EIP712TypeEntry, EIP712Types, OnChainState,
    PayerSignatureRequest, PayerSignatureResponse, PaymentConfig, PaymentMethod, PaymentResponse,
    PrepareTransactionResponse, RefundPayloadRequest, RefundPaymentResponse, ReleasePaymentResponse,
    RefundPrepareResponse, ReleaseRequest, SigningPayload, SubmitTransactionRequest,
    Uint256String, VoidPaymentResponse,
};
