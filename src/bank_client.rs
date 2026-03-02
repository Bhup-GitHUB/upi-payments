use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BankError {
    #[error("INSUFFICIENT_FUNDS")]
    InsufficientFunds,
    #[error("ACCOUNT_NOT_FOUND")]
    AccountNotFound,
    #[error("BANK_TIMEOUT")]
    Timeout,
    #[error("BANK_UNAVAILABLE")]
    Unavailable,
}

#[async_trait]
pub trait BankClient: Send + Sync {
    async fn debit(&self, vpa: &str, amount_paise: u64, txn_id: &str) -> Result<String, BankError>;
    async fn credit(&self, vpa: &str, amount_paise: u64, txn_id: &str) -> Result<String, BankError>;
    async fn reverse_debit(&self, vpa: &str, amount_paise: u64, txn_id: &str) -> Result<(), BankError>;
}

pub struct MockBank {
    pub name: String,
}

impl MockBank {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    fn rrn(&self, txn_id: &str, phase: &str) -> String {
        let prefix: String = txn_id.chars().take(8).collect();
        format!("RRN-{prefix}-{phase}-{}", self.name.replace(' ', ""))
    }
}

#[async_trait]
impl BankClient for MockBank {
    async fn debit(&self, vpa: &str, amount_paise: u64, txn_id: &str) -> Result<String, BankError> {
        sleep(Duration::from_millis(200)).await;

        if vpa.contains("debitfail") || amount_paise == 11_111 {
            return Err(BankError::InsufficientFunds);
        }
        if vpa.contains("missing") {
            return Err(BankError::AccountNotFound);
        }

        Ok(self.rrn(txn_id, "DEBIT"))
    }

    async fn credit(&self, vpa: &str, amount_paise: u64, txn_id: &str) -> Result<String, BankError> {
        if vpa.contains("slow") || amount_paise == 33_333 {
            sleep(Duration::from_secs(35)).await;
            return Err(BankError::Timeout);
        }

        sleep(Duration::from_millis(200)).await;

        if vpa.contains("creditfail") || amount_paise == 22_222 {
            return Err(BankError::Unavailable);
        }
        if vpa.contains("missing") {
            return Err(BankError::AccountNotFound);
        }

        Ok(self.rrn(txn_id, "CREDIT"))
    }

    async fn reverse_debit(
        &self,
        _vpa: &str,
        _amount_paise: u64,
        _txn_id: &str,
    ) -> Result<(), BankError> {
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }
}
