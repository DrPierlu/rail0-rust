use std::sync::Arc;

use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::types::PaymentMethod;

/// Merchant configuration operations.
pub struct AccountsClient {
    http: Arc<HttpClient>,
}

impl AccountsClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Return the active payment methods (chain + token + wallet) for the given merchant.
    pub async fn payment_methods(&self, account_id: u32) -> Result<Vec<PaymentMethod>, Rail0Error> {
        self.http
            .get(&format!("/accounts/{account_id}/payment-methods"))
            .await
    }
}
