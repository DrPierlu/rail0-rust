use std::sync::Arc;

use crate::http::{ClientOptions, HttpClient};
use crate::payments::PaymentsClient;
use crate::tokens::TokensClient;
use crate::utils::UtilsClient;

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
    /// Payment lifecycle operations: authorize, charge, capture, void, release, refund.
    pub payments: PaymentsClient,
    /// Token allowlist queries.
    pub tokens: TokensClient,
    /// Contract introspection: domain separator, version.
    pub utils: UtilsClient,
}

impl Rail0Client {
    /// Creates a new client from the provided options.
    pub fn new(opts: ClientOptions) -> Self {
        let http = Arc::new(HttpClient::new(opts));
        Self {
            payments: PaymentsClient::new(Arc::clone(&http)),
            tokens: TokensClient::new(Arc::clone(&http)),
            utils: UtilsClient::new(Arc::clone(&http)),
        }
    }
}
