use claim_model::{
    account_record::AccountRecord,
    api::RecordApi,
    event::{emit, EventKind::Record, RecordData},
};
use near_sdk::{json_types::U128, near_bindgen, store::Vector, AccountId};

use crate::{common::now_seconds, Contract, ContractExt, StorageKey::AccrualsEntry};

#[near_bindgen]
impl RecordApi for Contract {
    fn record_batch_for_hold(&mut self, amounts: Vec<(AccountId, U128)>) {
        self.assert_oracle();

        let now_seconds = now_seconds();
        let mut event_data = RecordData::new(now_seconds);

        for (account_id, amount) in amounts {
            event_data.amounts.push((account_id.clone(), amount));

            let account = self.get_account_mut(&account_id);
            account.balance.checked_add(amount.0).expect("Balance overflow");
        }

        emit(Record(event_data));
    }
}
