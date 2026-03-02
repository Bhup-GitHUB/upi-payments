use crate::error::ValidationError;
use crate::types::PaymentRequest;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_PAST_MS: u64 = 24 * 60 * 60 * 1000;
const MAX_FUTURE_MS: u64 = 5 * 60 * 1000;

pub fn validate_request(req: &PaymentRequest) -> Result<(), ValidationError> {
    if req.amount_paise == 0 {
        return Err(ValidationError::InvalidAmount);
    }

    if req.payer_vpa.trim().is_empty() || req.payee_vpa.trim().is_empty() {
        return Err(ValidationError::InvalidVpa);
    }

    if req.payer_vpa == req.payee_vpa {
        return Err(ValidationError::SamePayerPayee);
    }

    if req.payer_bank_ifsc.trim().is_empty() || req.payee_bank_ifsc.trim().is_empty() {
        return Err(ValidationError::InvalidIfsc);
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    if req.timestamp_ms + MAX_PAST_MS < now_ms || req.timestamp_ms > now_ms + MAX_FUTURE_MS {
        return Err(ValidationError::TimestampOutOfRange);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_request;
    use crate::error::ValidationError;
    use crate::types::PaymentRequest;
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    fn base_request() -> PaymentRequest {
        PaymentRequest {
            transaction_id: Uuid::new_v4(),
            payer_vpa: "alice@okaxis".to_string(),
            payee_vpa: "bob@ybl".to_string(),
            amount_paise: 1000,
            payer_bank_ifsc: "UTIB0000001".to_string(),
            payee_bank_ifsc: "YESB0000001".to_string(),
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    #[test]
    fn accepts_valid_request() {
        let req = base_request();
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn rejects_zero_amount() {
        let mut req = base_request();
        req.amount_paise = 0;
        assert!(matches!(
            validate_request(&req),
            Err(ValidationError::InvalidAmount)
        ));
    }
}
