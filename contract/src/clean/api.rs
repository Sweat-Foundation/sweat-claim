use claim_model::event::{emit, CleanData, EventKind};
use near_sdk::{near_bindgen, AccountId};

use crate::{common::AccountAccessor, Contract, ContractExt};

pub trait CleanApi {
    fn clean(&mut self, account_ids: Vec<AccountId>);
}

#[near_bindgen]
impl CleanApi for Contract {
    fn clean(&mut self, account_ids: Vec<AccountId>) {
        self.assert_called_by_oracle();

        for account_id in account_ids.clone() {
            if let Some(account) = self.accounts.try_get_account(&account_id) {
                self.balance_to_burn += account.balance;
            }
            self.accounts.set(account_id, None);
        }

        emit(EventKind::Clean(CleanData { account_ids }));
    }
}
