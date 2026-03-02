use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub enum ApiErrorCode {
    InvalidVpa,
    InvalidIfsc,
    InvalidAmount,
    SamePayerPayee,
    TimestampOutOfRange,
    Timeout,
    Processing,
    Unknown,
}

impl std::fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::InvalidVpa => "INVALID_VPA",
            Self::InvalidIfsc => "INVALID_IFSC",
            Self::InvalidAmount => "INVALID_AMOUNT",
            Self::SamePayerPayee => "SAME_PAYER_PAYEE",
            Self::TimestampOutOfRange => "TIMESTAMP_OUT_OF_RANGE",
            Self::Timeout => "TIMEOUT",
            Self::Processing => "PROCESSING",
            Self::Unknown => "UNKNOWN",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("invalid vpa")]
    InvalidVpa,
    #[error("invalid ifsc")]
    InvalidIfsc,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("payer and payee must differ")]
    SamePayerPayee,
    #[error("timestamp out of range")]
    TimestampOutOfRange,
}

impl ValidationError {
    pub fn code(&self) -> ApiErrorCode {
        match self {
            Self::InvalidVpa => ApiErrorCode::InvalidVpa,
            Self::InvalidIfsc => ApiErrorCode::InvalidIfsc,
            Self::InvalidAmount => ApiErrorCode::InvalidAmount,
            Self::SamePayerPayee => ApiErrorCode::SamePayerPayee,
            Self::TimestampOutOfRange => ApiErrorCode::TimestampOutOfRange,
        }
    }
}
