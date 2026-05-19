use std::sync::Arc;

use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::types::{
    ApproveRequest, ApproveResponse, AuthorizePaymentResponse, Bytes32, CapturePaymentRequest,
    CapturePaymentResponse, ChargePaymentResponse, CreatePaymentRequest, CreatePaymentResponse,
    PayerSignatureRequest, PayerSignatureResponse, PrepareTransactionResponse,
    RefundPaymentRequest, RefundPaymentResponse, ReleasePaymentResponse,
    SubmitTransactionRequest, VoidPaymentResponse,
};

/// Payment lifecycle operations.
pub struct PaymentsClient {
    http: Arc<HttpClient>,
}

impl PaymentsClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
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

    /// Relay the stored EIP-3009 signature to the RAIL0 `authorize()` function. Called by the payee.
    pub async fn authorize(
        &self,
        payment_id: &str,
    ) -> Result<AuthorizePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/authorize"), &())
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

    /// Release escrowed funds back to the payer after `authorization_expiry`. Permissionless.
    pub async fn release(
        &self,
        payment_id: &str,
    ) -> Result<ReleasePaymentResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/release"), &())
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
    pub async fn submit_approve(
        &self,
        payment_id: &str,
        params: &SubmitTransactionRequest,
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
