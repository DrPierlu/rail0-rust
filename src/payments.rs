use std::sync::Arc;

use crate::error::Rail0Error;
use crate::http::HttpClient;
use crate::types::{
    AuthorizeParams, CaptureParams, ChargeParams, HashResponse, NonceResponse,
    Payment, PaymentResponse, RefundParams, ReleaseParams, TransactionResponse, VoidParams,
};

/// Payment lifecycle operations: authorize, charge, capture, void, release, refund.
pub struct PaymentsClient {
    http: Arc<HttpClient>,
}

impl PaymentsClient {
    pub(crate) fn new(http: Arc<HttpClient>) -> Self {
        Self { http }
    }

    /// Returns the on-chain state and config hash for a payment.
    pub async fn get(&self, payment_id: &str) -> Result<PaymentResponse, Rail0Error> {
        self.http.get(&format!("/payments/{payment_id}")).await
    }

    /// Locks `amount` from the payer into escrow using an EIP-3009 `transferWithAuthorization`
    /// signature. Build the signature with [`sign_authorize`](crate::sign_authorize).
    pub async fn authorize(
        &self,
        payment_id: &str,
        params: AuthorizeParams,
    ) -> Result<TransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/authorize"), &params)
            .await
    }

    /// Authorize and immediately capture in a single transaction.
    /// Build the signature with [`sign_charge`](crate::sign_charge).
    pub async fn charge(
        &self,
        payment_id: &str,
        params: ChargeParams,
    ) -> Result<TransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/charge"), &params)
            .await
    }

    /// Moves escrowed funds to the payee. Caller must be the payee.
    pub async fn capture(
        &self,
        payment_id: &str,
        params: CaptureParams,
    ) -> Result<TransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/capture"), &params)
            .await
    }

    /// Cancels an authorization, returning escrowed funds to the payer. Caller must be the payee.
    pub async fn void(
        &self,
        payment_id: &str,
        params: VoidParams,
    ) -> Result<TransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/void"), &params)
            .await
    }

    /// Returns escrowed funds to the payer after `authorization_expiry`. Permissionless.
    pub async fn release(
        &self,
        payment_id: &str,
        params: ReleaseParams,
    ) -> Result<TransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/release"), &params)
            .await
    }

    /// Returns a previously captured amount to the payer. Must be called before `refund_expiry`.
    pub async fn refund(
        &self,
        payment_id: &str,
        params: RefundParams,
    ) -> Result<TransactionResponse, Rail0Error> {
        self.http
            .post(&format!("/payments/{payment_id}/refund"), &params)
            .await
    }

    /// Returns the EIP-3009 nonce the payer must include in the `authorize` signature.
    pub async fn authorize_nonce(
        &self,
        payment_id: &str,
        payer: &str,
    ) -> Result<NonceResponse, Rail0Error> {
        self.http
            .get(&format!("/payments/{payment_id}/authorize-nonce?payer={payer}"))
            .await
    }

    /// Returns the EIP-3009 nonce the payer must include in the `charge` signature.
    pub async fn charge_nonce(
        &self,
        payment_id: &str,
        payer: &str,
    ) -> Result<NonceResponse, Rail0Error> {
        self.http
            .get(&format!("/payments/{payment_id}/charge-nonce?payer={payer}"))
            .await
    }

    /// Computes the canonical EIP-712 digest of a [`Payment`] configuration.
    pub async fn hash(&self, payment: &Payment) -> Result<HashResponse, Rail0Error> {
        self.http.post("/payments/hash", payment).await
    }
}
