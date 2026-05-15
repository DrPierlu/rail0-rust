pub mod client;
pub mod error;
pub mod http;
pub mod payments;
pub mod signing;
pub mod stablecoins;
pub mod tokens;
pub mod types;
pub mod utils;

// ================================================================
//  Flat public re-exports
// ================================================================

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
pub use tokens::TokensClient;
pub use types::{
    Address, AuthorizeParams, Bytes32, CaptureParams, ChargeParams, DomainSeparatorResponse,
    HashResponse, NonceResponse, Payment, PaymentResponse, PaymentState, RefundParams,
    ReleaseParams, TokenStatusResponse, TransactionResponse, TransactionStatus, Uint256String,
    VersionResponse, VoidParams,
};
pub use utils::UtilsClient;
