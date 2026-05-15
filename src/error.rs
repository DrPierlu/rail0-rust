use thiserror::Error;

/// Errors returned by the RAIL0 SDK.
#[derive(Debug, Error)]
pub enum Rail0Error {
    /// The API returned a non-2xx response with a structured error body.
    #[error("rail0 {code} (HTTP {status}): {message}")]
    Api {
        /// HTTP status code (e.g. 422).
        status: u16,
        /// Machine-readable error code from the contract (e.g. `"AuthorizationExpired"`).
        code: String,
        /// Human-readable description.
        message: String,
    },

    /// A network-level error from `reqwest` (timeout, DNS failure, etc.).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialisation or deserialisation failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// EIP-712 / EIP-3009 signing failed.
    #[error("signing error: {0}")]
    Sign(String),

    /// Invalid input supplied by the caller (bad hex, wrong length, etc.).
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// Shape of error responses from the RAIL0 API (internal, for deserialisation).
#[derive(Debug, serde::Deserialize)]
pub(crate) struct ApiErrorBody {
    pub error: String,
    pub message: String,
}
