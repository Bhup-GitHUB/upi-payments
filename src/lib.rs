use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;

pub mod bank_client;
pub mod error;
pub mod idempotency;
pub mod orchestrator;
pub mod types;
pub mod validator;

use bank_client::MockBank;
use error::ApiErrorCode;
use idempotency::{finalize, get_state, new_store, try_begin, IdempotencyStore};
use orchestrator::PaymentOrchestrator;
use types::{PaymentRequest, PaymentResponse, TxnState, TxnStatus};
use validator::validate_request;

#[derive(Clone)]
pub struct AppState {
    pub store: IdempotencyStore,
    pub orchestrator: Arc<PaymentOrchestrator>,
}

impl AppState {
    pub fn prototype() -> Self {
        Self {
            store: new_store(),
            orchestrator: Arc::new(PaymentOrchestrator::new(
                Arc::new(MockBank::new("Axis Bank".to_string())),
                Arc::new(MockBank::new("Yes Bank".to_string())),
            )),
        }
    }
}

pub fn app(state: AppState) -> Router {
    Router::new().route("/pay", post(handle_payment)).with_state(state)
}

pub async fn handle_payment(
    State(app): State<AppState>,
    Json(req): Json<PaymentRequest>,
) -> (StatusCode, Json<PaymentResponse>) {
    let start = Instant::now();
    let txn_id = req.transaction_id;

    if let Err(err) = validate_request(&req) {
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(PaymentResponse {
                transaction_id: txn_id,
                status: TxnStatus::Failed,
                rrn: None,
                error_code: Some(err.code().to_string()),
                processing_time_ms: start.elapsed().as_millis() as u64,
            }),
        );
    }

    if !try_begin(&app.store, txn_id) {
        let cached = get_state(&app.store, &txn_id);
        let (rrn, error_code) = match cached {
            Some(TxnState::Settled { rrn }) => (Some(rrn), None),
            Some(TxnState::Failed { reason }) => (None, Some(reason)),
            Some(TxnState::TimedOut) => (None, Some(ApiErrorCode::Timeout.to_string())),
            Some(TxnState::Processing) => (None, Some(ApiErrorCode::Processing.to_string())),
            None => (None, Some(ApiErrorCode::Unknown.to_string())),
        };

        return (
            StatusCode::OK,
            Json(PaymentResponse {
                transaction_id: txn_id,
                status: TxnStatus::Duplicate,
                rrn,
                error_code,
                processing_time_ms: start.elapsed().as_millis() as u64,
            }),
        );
    }

    let final_state = app
        .orchestrator
        .execute(
            &req.payer_vpa,
            &req.payee_vpa,
            req.amount_paise,
            &txn_id.to_string(),
        )
        .await;

    finalize(&app.store, txn_id, final_state.clone());

    let (http_status, status, rrn, error_code) = match final_state {
        TxnState::Settled { rrn } => (StatusCode::OK, TxnStatus::Success, Some(rrn), None),
        TxnState::Failed { reason } => (StatusCode::PAYMENT_REQUIRED, TxnStatus::Failed, None, Some(reason)),
        TxnState::TimedOut => (
            StatusCode::GATEWAY_TIMEOUT,
            TxnStatus::Timeout,
            None,
            Some(ApiErrorCode::Timeout.to_string()),
        ),
        TxnState::Processing => (
            StatusCode::INTERNAL_SERVER_ERROR,
            TxnStatus::Failed,
            None,
            Some(ApiErrorCode::Unknown.to_string()),
        ),
    };

    (
        http_status,
        Json(PaymentResponse {
            transaction_id: txn_id,
            status,
            rrn,
            error_code,
            processing_time_ms: start.elapsed().as_millis() as u64,
        }),
    )
}
