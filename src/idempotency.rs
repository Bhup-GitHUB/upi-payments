use crate::types::TxnState;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub type IdempotencyStore = Arc<DashMap<Uuid, TxnState>>;

pub fn new_store() -> IdempotencyStore {
    Arc::new(DashMap::new())
}

pub fn try_begin(store: &IdempotencyStore, txn_id: Uuid) -> bool {
    use dashmap::mapref::entry::Entry;

    match store.entry(txn_id) {
        Entry::Vacant(entry) => {
            entry.insert(TxnState::Processing);
            true
        }
        Entry::Occupied(_) => false,
    }
}

pub fn finalize(store: &IdempotencyStore, txn_id: Uuid, state: TxnState) {
    store.insert(txn_id, state);
}

pub fn get_state(store: &IdempotencyStore, txn_id: &Uuid) -> Option<TxnState> {
    store.get(txn_id).map(|state| state.clone())
}

#[cfg(test)]
mod tests {
    use super::{finalize, get_state, new_store, try_begin};
    use crate::types::TxnState;
    use uuid::Uuid;

    #[test]
    fn allows_single_begin() {
        let store = new_store();
        let txn_id = Uuid::new_v4();

        assert!(try_begin(&store, txn_id));
        assert!(!try_begin(&store, txn_id));
    }

    #[test]
    fn finalizes_and_reads_state() {
        let store = new_store();
        let txn_id = Uuid::new_v4();

        finalize(
            &store,
            txn_id,
            TxnState::Settled {
                rrn: "RRN-test".to_string(),
            },
        );

        assert_eq!(
            get_state(&store, &txn_id),
            Some(TxnState::Settled {
                rrn: "RRN-test".to_string()
            })
        );
    }
}
