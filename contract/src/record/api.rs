use claim_model::{
    api::RecordApi,
    event::{emit, EventKind::Record, RecordData},
};
use near_sdk::{json_types::U128, near_bindgen, AccountId};

use crate::{
    common::{now_seconds, AccountAccessor},
    Contract, ContractExt,
};

#[near_bindgen]
impl RecordApi for Contract {
    fn record_batch_for_hold(&mut self, amounts: Vec<(AccountId, U128)>) {
        self.assert_oracle();

        let now_seconds = now_seconds();
        let mut event_data = RecordData::new(now_seconds);

        for (account_id, amount) in amounts {
            self.migrate_account_if_outdated(&account_id);

            event_data.amounts.push((account_id.clone(), amount));

            let account = self.accounts.get_account_mut(&account_id);
            account.balance += amount.0;
            account.last_top_up_at = now_seconds;
        }

        emit(Record(event_data));
    }
}
