use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;
use upi_switch::types::{PaymentRequest, PaymentResponse, TxnStatus};
use upi_switch::{app, AppState};
use uuid::Uuid;

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn req_with(txn_id: Uuid, payee_vpa: &str, amount_paise: u64) -> PaymentRequest {
    PaymentRequest {
        transaction_id: txn_id,
        payer_vpa: "payer@okaxis".to_string(),
        payee_vpa: payee_vpa.to_string(),
        amount_paise,
        payer_bank_ifsc: "UTIB0000001".to_string(),
        payee_bank_ifsc: "YESB0000001".to_string(),
        timestamp_ms: now_ms(),
    }
}

async fn post_pay(state: AppState, req: &PaymentRequest) -> (StatusCode, PaymentResponse) {
    let app = app(state);
    let request = Request::builder()
        .method("POST")
        .uri("/pay")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(req).expect("serialize request")))
        .expect("build request");

    let response = app.oneshot(request).await.expect("router response");
    let status = response.status();
    let bytes = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("read body")
        .to_bytes();
    let payload: PaymentResponse = serde_json::from_slice(&bytes).expect("parse response");

    (status, payload)
}

#[tokio::test]
async fn first_payment_then_duplicate() {
    let state = AppState::prototype();
    let txn_id = Uuid::new_v4();
    let req = req_with(txn_id, "merchant@ybl", 1000);

    let (first_status, first) = post_pay(state.clone(), &req).await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(first.status, TxnStatus::Success);

    let (second_status, second) = post_pay(state, &req).await;
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(second.status, TxnStatus::Duplicate);
    assert_eq!(second.rrn, first.rrn);
}

#[tokio::test]
async fn deterministic_credit_failure() {
    let state = AppState::prototype();
    let req = req_with(Uuid::new_v4(), "merchant-creditfail@ybl", 1000);

    let (status, payload) = post_pay(state, &req).await;
    assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(payload.status, TxnStatus::Failed);
    assert!(payload
        .error_code
        .expect("error code")
        .contains("CREDIT_FAILED"));
}

#[tokio::test]
async fn timeout_path_returns_gateway_timeout() {
    let state = AppState::prototype();
    let req = req_with(Uuid::new_v4(), "merchant-slow@ybl", 1000);

    let (status, payload) = post_pay(state, &req).await;
    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
    assert_eq!(payload.status, TxnStatus::Timeout);
    assert_eq!(payload.error_code.as_deref(), Some("TIMEOUT"));
}

#[tokio::test]
async fn same_transaction_concurrency_keeps_single_success() {
    let state = AppState::prototype();
    let txn_id = Uuid::new_v4();
    let req = req_with(txn_id, "merchant@ybl", 1000);

    let mut tasks = Vec::new();
    for _ in 0..5 {
        let state_clone = state.clone();
        let req_clone = req.clone();
        tasks.push(tokio::spawn(async move { post_pay(state_clone, &req_clone).await }));
    }

    let mut success = 0_u8;
    let mut duplicate = 0_u8;

    for task in tasks {
        let (status, payload) = task.await.expect("task join");
        if status == StatusCode::OK && payload.status == TxnStatus::Success {
            success += 1;
        }
        if status == StatusCode::OK && payload.status == TxnStatus::Duplicate {
            duplicate += 1;
        }
    }

    assert_eq!(success, 1);
    assert_eq!(duplicate, 4);
}
