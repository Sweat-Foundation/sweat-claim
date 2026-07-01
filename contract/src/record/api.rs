use claim_model::{
    api::RecordApi,
    event::{emit, EventKind::Record, RecordAmountDetailed, RecordData},
};
use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{json_types::U128, near_bindgen, AccountId};

use crate::{
    auth::Roles,
    common::{now_seconds, AccountAccessor},
    Contract, ContractExt,
};

#[near_bindgen]
impl RecordApi for Contract {
    #[access_control_any(roles(Roles::Oracle))]
    fn record_batch_for_hold(&mut self, amounts: Vec<(AccountId, U128)>) {
        // Default value can be 0 only in tests.
        let claimable_window_start = self.get_claimable_window_start();
        let mut event_data = RecordData::new(now_seconds());

        for (account_id, amount) in amounts {
            let account = self.accounts.get_or_insert_account_mut(&account_id);
            let balance_to_burn = account.get_balance_to_burn(self.burn_period, claimable_window_start);

            if balance_to_burn > 0 {
                self.balance_to_burn += balance_to_burn;

                account.balance -= balance_to_burn;
                account.burn_since = claimable_window_start;
            }

            account.balance += amount.0;

            event_data.amounts.push((
                account_id.clone(),
                RecordAmountDetailed {
                    credited: amount,
                    burnt: U128(balance_to_burn),
                },
            ));
        }

        emit(Record(event_data));
    }
}
