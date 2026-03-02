use crate::bank_client::BankClient;
use crate::types::TxnState;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

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

        let credit_result = self.payee_bank.credit(payee_vpa, amount_paise, txn_id).await;

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
