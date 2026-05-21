use std::sync::Arc;

use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::types::{
    ApproveRequest, ApproveResponse, AuthorizePaymentResponse, Bytes32, CapturePaymentRequest,
    CapturePaymentResponse, ChargePaymentResponse, CreatePaymentRequest, CreatePaymentResponse,
    PayerSignatureRequest, PayerSignatureResponse, PaymentResponse, PrepareTransactionResponse,
    RefundPaymentRequest, RefundPaymentResponse, ReleasePaymentResponse, ReleaseRequest,
    SubmitApproveRequest, SubmitTransactionRequest, VoidPaymentResponse,
};

/// Payment lifecycle operations.
pub struct PaymentsClient {
    http: Arc<HttpClient>,
}

impl PaymentsClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Fetch current payment state (DB status + live on-chain escrow balances).
    pub async fn get(&self, payment_id: &str) -> Result<PaymentResponse, Rail0Error> {
        self.http.get(&format!("/payments/{payment_id}")).await
    }

    /// Create a payment intent. Returns the EIP-712 `signingPayload` for the payer to sign.
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
    pub async fn authorize(
        &self,
        payment_id: &str,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/authorize"), &())
            .await
    }

    /// Broadcast a signed authorize transaction. Called by the payee.
    pub async fn submit_authorize(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<AuthorizePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/authorize/submit"), params)
            .await
    }

    /// Relay the stored EIP-3009 signature to the RAIL0 `charge()` function (one-shot). Called by the payee.
    pub async fn charge(
        &self,
        payment_id: &str,
    ) -> Result<ChargePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/charge"), &())
            .await
    }

    /// Build the unsigned `capture()` transaction. Called by the payee.
    pub async fn prepare_capture(
        &self,
        payment_id: &str,
        params: &CapturePaymentRequest,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/capture"), params)
            .await
    }

    /// Broadcast a signed capture transaction. Called by the payee.
    pub async fn submit_capture(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<CapturePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/capture/submit"), params)
            .await
    }

    /// Build the unsigned `void()` transaction. Called by the payee.
    pub async fn prepare_void(
        &self,
        payment_id: &str,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/void"), &())
            .await
    }

    /// Broadcast a signed void transaction. Called by the payee.
    pub async fn submit_void(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<VoidPaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/void/submit"), params)
            .await
    }

    /// Build the unsigned `release()` transaction.
    /// Pass `ReleaseRequest { caller_address: Some(buyer_addr) }` to build the tx for the buyer.
    /// `release()` can only succeed after `authorization_expiry` has passed on-chain.
    pub async fn prepare_release(
        &self,
        payment_id: &str,
        params: &ReleaseRequest,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/release"), params)
            .await
    }

    /// Broadcast a signed release transaction.
    pub async fn submit_release(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<ReleasePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/release/submit"), params)
            .await
    }

    /// Build the unsigned ERC-20 `approve()` transaction needed before a refund. Called by the payee.
    pub async fn prepare_approve(
        &self,
        payment_id: &str,
        params: &ApproveRequest,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/approve"), params)
            .await
    }

    /// Broadcast a signed ERC-20 approve transaction. Called by the payee.
    /// Set `amount` in `params` so the API records the approved amount in the transaction log.
    pub async fn submit_approve(
        &self,
        payment_id: &str,
        params: &SubmitApproveRequest,
    ) -> Result<ApproveResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/approve/submit"), params)
            .await
    }

    /// Build the unsigned `refund()` transaction. Called by the payee.
    pub async fn prepare_refund(
        &self,
        payment_id: &str,
        params: &RefundPaymentRequest,
    ) -> Result<PrepareTransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/refund"), params)
            .await
    }

    /// Broadcast a signed refund transaction. Called by the payee.
    pub async fn submit_refund(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
    ) -> Result<RefundPaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/refund/submit"), params)
            .await
    }
}
