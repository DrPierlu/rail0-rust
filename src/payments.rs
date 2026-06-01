use std::sync::Arc;

use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::types::{
    AuthorizePaymentResponse, CapturePaymentRequest, CapturePaymentResponse,
    ChargePaymentResponse, CreatePaymentRequest, CreatePaymentResponse, PayerSignatureRequest,
    PayerSignatureResponse, PaymentResponse, PrepareTransactionResponse, RefundPayloadRequest,
    RefundPaymentResponse, ReleasePaymentResponse, ReleaseRequest, SubmitTransactionRequest,
    VoidPaymentResponse,
};

/// Payment lifecycle operations.
pub struct PaymentsClient {
    http: Arc<HttpClient>,
}

impl PaymentsClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// List payments for the authenticated account.
    pub async fn list(&self) -> Result<Vec<PaymentResponse>, Rail0Error> {
        self.http.get("/payments").await
    }

    /// Fetch current payment state (DB status + live on-chain escrow balances).
    pub async fn get(&self, payment_id: &str) -> Result<PaymentResponse, Rail0Error> {
        self.http.get(&format!("/payments/{payment_id}")).await
    }

    /// Create a payment intent. Returns the EIP-712 `signing_prepare` for the payer to sign.
    pub async fn create_payment(
        &self,
        params: &CreatePaymentRequest,
    ) -> Result<CreatePaymentResponse, Rail0Error> {
        self.http.post("/payments", params).await
    }

    /// Submit the payer's EIP-712 signature (v, r, s).
    pub async fn sign(
        &self,
        payment_id: &str,
        params: &PayerSignatureRequest,
    ) -> Result<PayerSignatureResponse, Rail0Error> {
        self.http
            .put(&format!("/payments/{payment_id}/sign"), params)
            .await
    }

    /// Prepare the unsigned `authorize()` transaction. Called by the payee.
    /// Sign `unsigned_transaction` with the payee's key and pass to [`authorize`](Self::authorize).
    pub async fn authorize_prepare(
        &self,
        payment_id: &str,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/authorize/prepare"), &())
            .await
    }

    /// Broadcast a signed authorize transaction (HTTP 202, async). Called by the payee.
    /// Poll [`get`](Self::get) until status leaves `"submitting"`.
    pub async fn authorize(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<AuthorizePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/authorize"), params)
            .await
    }

    /// Prepare the unsigned `charge()` transaction (one-shot, no escrow). Called by the payee.
    /// The payer signature must have been submitted first via [`sign`](Self::sign).
    pub async fn charge_prepare(
        &self,
        payment_id: &str,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/charge/prepare"), &())
            .await
    }

    /// Broadcast a signed charge transaction (HTTP 202, async). Called by the payee.
    pub async fn charge(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<ChargePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/charge"), params)
            .await
    }

    /// Build the unsigned `capture()` transaction. Called by the payee.
    pub async fn capture_prepare(
        &self,
        payment_id: &str,
        params: &CapturePaymentRequest,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/capture/prepare"), params)
            .await
    }

    /// Broadcast a signed capture transaction (HTTP 202, async). Called by the payee.
    pub async fn capture(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<CapturePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/capture"), params)
            .await
    }

    /// Build the unsigned `void()` transaction. Called by the payee.
    pub async fn void_prepare(
        &self,
        payment_id: &str,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/void/prepare"), &())
            .await
    }

    /// Broadcast a signed void transaction (HTTP 202, async). Called by the payee.
    pub async fn void(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<VoidPaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/void"), params)
            .await
    }

    /// Build the unsigned `release()` transaction.
    /// Set `caller_address` in [`ReleaseRequest`] to build the tx for the buyer (payer).
    /// `release()` can only succeed after `authorization_expiry` has passed on-chain.
    pub async fn release_prepare(
        &self,
        payment_id: &str,
        params: &ReleaseRequest,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/release/prepare"), params)
            .await
    }

    /// Broadcast a signed release transaction (HTTP 202, async).
    pub async fn release(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<ReleasePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/release"), params)
            .await
    }

    /// Refund payload — two-phase EIP-3009 `receiveWithAuthorization` flow. Called by the payee.
    ///
    /// **Phase 1** — set only `amount` in [`RefundPayloadRequest`]:
    /// Returns the EIP-3009 signing payload. Sign it off-chain to obtain `v`, `r`, `s`.
    ///
    /// **Phase 2** — set `amount` plus `v`, `r`, `s`:
    /// Returns the unsigned on-chain refund transaction ready to sign and submit.
    ///
    /// No ERC-20 `approve()` step is required — uses EIP-3009.
    pub async fn refund_prepare(
        &self,
        payment_id: &str,
        params: &RefundPayloadRequest,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/refund/prepare"), params)
            .await
    }

    /// Broadcast a signed refund transaction (HTTP 202, async). Called by the payee.
    pub async fn refund(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<RefundPaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/refund"), params)
            .await
    }
}
