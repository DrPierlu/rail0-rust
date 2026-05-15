use std::sync::Arc;

use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::types::TokenStatusResponse;

/// Token allowlist queries.
pub struct TokensClient {
    http: Arc<HttpClient>,
}

impl TokensClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Returns whether the given ERC-20 token address is in this deployment's allowlist.
    pub async fn is_accepted(&self, address: &str) -> Result<TokenStatusResponse, Rail0Error> {
        self.http.get(&format!("/tokens/{address}")).await
    }
}
