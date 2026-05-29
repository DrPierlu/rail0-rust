use std::sync::Arc;

use crate::accounts::AccountsClient;
use crate::chains::ChainsClient;
use crate::tokens::TokensClient;
use crate::http::{ClientOptions, HttpClient};
use crate::payments::PaymentsClient;

/// Entry point for the RAIL0 SDK.
///
/// ```no_run
/// use rail0::{Rail0Client, ClientOptions};
///
/// let client = Rail0Client::new(ClientOptions {
///     base_url: "https://api.rail0.xyz".into(),
///     ..Default::default()
/// });
/// ```
pub struct Rail0Client {
    /// Account configuration operations: `payment_methods`.
    pub accounts: AccountsClient,
    /// Blockchain and token catalog operations: `blockchains`, `tokens`.
    pub chains: ChainsClient,
    pub tokens: TokensClient,
    /// Payment lifecycle operations: `get`, `create_payment`, `sign`, `authorize`,
    /// `submit_authorize`, `charge`, `prepare_capture`, `submit_capture`, `prepare_void`,
    /// `submit_void`, `prepare_release`, `submit_release`, `prepare_approve`,
    /// `submit_approve`, `prepare_refund`, `submit_refund`.
    pub payments: PaymentsClient,
}

impl Rail0Client {
    /// Creates a new client from the provided options.
    pub fn new(opts: ClientOptions) -> Self {
        let http = Arc::new(HttpClient::new(opts));
        Self {
            accounts: AccountsClient::new(Arc::clone(&http)),
            chains: ChainsClient::new(Arc::clone(&http)),
            tokens: TokensClient::new(Arc::clone(&http)),
            payments: PaymentsClient::new(Arc::clone(&http)),
        }
    }
}
