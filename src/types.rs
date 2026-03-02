use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
pub struct PaymentRequest {
    pub transaction_id: Uuid,
    pub payer_vpa: String,
    pub payee_vpa: String,
    pub amount_paise: u64,
    pub payer_bank_ifsc: String,
    pub payee_bank_ifsc: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentResponse {
    pub transaction_id: Uuid,
    pub status: TxnStatus,
    pub rrn: Option<String>,
    pub error_code: Option<String>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxnState {
    Processing,
    Settled { rrn: String },
    Failed { reason: String },
    TimedOut,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TxnStatus {
    Success,
    Failed,
    Timeout,
    Duplicate,
}
