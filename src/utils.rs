use std::sync::Arc;

use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::types::{DomainSeparatorResponse, VersionResponse};

/// Contract introspection: EIP-712 domain separator and version.
pub struct UtilsClient {
    http: Arc<HttpClient>,
}

impl UtilsClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Returns the EIP-712 domain separator for the RAIL0 contract.
    pub async fn domain_separator(&self) -> Result<DomainSeparatorResponse, Rail0Error> {
        self.http.get("/domain-separator").await
    }

    /// Returns the contract version number.
    pub async fn version(&self) -> Result<VersionResponse, Rail0Error> {
        self.http.get("/version").await
    }
}
