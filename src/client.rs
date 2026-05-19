use std::sync::Arc;

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
    /// Payment lifecycle operations: create_payment, sign, authorize, charge,
    /// prepare_capture, submit_capture, prepare_void, submit_void, release,
    /// prepare_approve, submit_approve, prepare_refund, submit_refund.
    pub payments: PaymentsClient,
}

impl Rail0Client {
    /// Creates a new client from the provided options.
    pub fn new(opts: ClientOptions) -> Self {
        let http = Arc::new(HttpClient::new(opts));
        Self {
            payments: PaymentsClient::new(Arc::clone(&http)),
        }
    }
}
