use crate::bank_client::BankClient;
use crate::types::TxnState;
use std::sync::Arc;
use tokio::time::{Duration, timeout};

pub struct PaymentOrchestrator {
    pub payer_bank: Arc<dyn BankClient>,
    pub payee_bank: Arc<dyn BankClient>,
    hard_timeout_ms: u64,
}

impl PaymentOrchestrator {
    pub fn new(payer_bank: Arc<dyn BankClient>, payee_bank: Arc<dyn BankClient>) -> Self {
        Self {
            payer_bank,
            payee_bank,
            hard_timeout_ms: 30_000,
        }
    }

    pub fn with_timeout(
        payer_bank: Arc<dyn BankClient>,
        payee_bank: Arc<dyn BankClient>,
        hard_timeout_ms: u64,
    ) -> Self {
        Self {
            payer_bank,
            payee_bank,
            hard_timeout_ms,
        }
    }

    pub async fn execute(
        &self,
        payer_vpa: &str,
        payee_vpa: &str,
        amount_paise: u64,
        txn_id: &str,
    ) -> TxnState {
        let result = timeout(
            Duration::from_millis(self.hard_timeout_ms),
            self.run_payment(payer_vpa, payee_vpa, amount_paise, txn_id),
        )
        .await;

        match result {
            Ok(state) => state,
            Err(_) => TxnState::TimedOut,
        }
    }

    async fn run_payment(
        &self,
        payer_vpa: &str,
        payee_vpa: &str,
        amount_paise: u64,
        txn_id: &str,
    ) -> TxnState {
        let debit_result = self.payer_bank.debit(payer_vpa, amount_paise, txn_id).await;

        if let Err(err) = debit_result {
            return TxnState::Failed {
                reason: format!("DEBIT_FAILED_{err}"),
            };
        }

        let credit_result = self
            .payee_bank
            .credit(payee_vpa, amount_paise, txn_id)
            .await;

        match credit_result {
            Ok(rrn) => TxnState::Settled { rrn },
            Err(err) => {
                let _ = self
                    .payer_bank
                    .reverse_debit(payer_vpa, amount_paise, txn_id)
                    .await;
                TxnState::Failed {
                    reason: format!("CREDIT_FAILED_{err}"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PaymentOrchestrator;
    use crate::bank_client::{BankClient, BankError};
    use crate::types::TxnState;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::time::{Duration, sleep};

    struct StubBank {
        debit_result: Result<String, BankError>,
        credit_result: Result<String, BankError>,
        credit_delay_ms: u64,
    }

    #[async_trait]
    impl BankClient for StubBank {
        async fn debit(
            &self,
            _vpa: &str,
            _amount_paise: u64,
            _txn_id: &str,
        ) -> Result<String, BankError> {
            self.debit_result.clone()
        }

        async fn credit(
            &self,
            _vpa: &str,
            _amount_paise: u64,
            _txn_id: &str,
        ) -> Result<String, BankError> {
            if self.credit_delay_ms > 0 {
                sleep(Duration::from_millis(self.credit_delay_ms)).await;
            }
            self.credit_result.clone()
        }

        async fn reverse_debit(
            &self,
            _vpa: &str,
            _amount_paise: u64,
            _txn_id: &str,
        ) -> Result<(), BankError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn settles_on_debit_credit_success() {
        let payer = Arc::new(StubBank {
            debit_result: Ok("DRRN".to_string()),
            credit_result: Ok("IGNORED".to_string()),
            credit_delay_ms: 0,
        });
        let payee = Arc::new(StubBank {
            debit_result: Ok("IGNORED".to_string()),
            credit_result: Ok("CRRN".to_string()),
            credit_delay_ms: 0,
        });
        let orchestrator = PaymentOrchestrator::new(payer, payee);
        let state = orchestrator.execute("a", "b", 1000, "txn").await;
        assert_eq!(
            state,
            TxnState::Settled {
                rrn: "CRRN".to_string()
            }
        );
    }

    #[tokio::test]
    async fn fails_on_debit_failure() {
        let payer = Arc::new(StubBank {
            debit_result: Err(BankError::InsufficientFunds),
            credit_result: Ok("IGNORED".to_string()),
            credit_delay_ms: 0,
        });
        let payee = Arc::new(StubBank {
            debit_result: Ok("IGNORED".to_string()),
            credit_result: Ok("IGNORED".to_string()),
            credit_delay_ms: 0,
        });
        let orchestrator = PaymentOrchestrator::new(payer, payee);
        let state = orchestrator.execute("a", "b", 1000, "txn").await;
        match state {
            TxnState::Failed { reason } => assert!(reason.contains("DEBIT_FAILED")),
            _ => panic!("expected failed state"),
        }
    }

    #[tokio::test]
    async fn fails_on_credit_failure() {
        let payer = Arc::new(StubBank {
            debit_result: Ok("DRRN".to_string()),
            credit_result: Ok("IGNORED".to_string()),
            credit_delay_ms: 0,
        });
        let payee = Arc::new(StubBank {
            debit_result: Ok("IGNORED".to_string()),
            credit_result: Err(BankError::Unavailable),
            credit_delay_ms: 0,
        });
        let orchestrator = PaymentOrchestrator::new(payer, payee);
        let state = orchestrator.execute("a", "b", 1000, "txn").await;
        match state {
            TxnState::Failed { reason } => assert!(reason.contains("CREDIT_FAILED")),
            _ => panic!("expected failed state"),
        }
    }

    #[tokio::test]
    async fn times_out_on_slow_credit() {
        let payer = Arc::new(StubBank {
            debit_result: Ok("DRRN".to_string()),
            credit_result: Ok("IGNORED".to_string()),
            credit_delay_ms: 0,
        });
        let payee = Arc::new(StubBank {
            debit_result: Ok("IGNORED".to_string()),
            credit_result: Ok("CRRN".to_string()),
            credit_delay_ms: 100,
        });
        let orchestrator = PaymentOrchestrator::with_timeout(payer, payee, 10);
        let state = orchestrator.execute("a", "b", 1000, "txn").await;
        assert_eq!(state, TxnState::TimedOut);
    }
}
