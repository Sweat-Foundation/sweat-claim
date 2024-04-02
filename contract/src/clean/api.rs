use claim_model::event::{emit, CleanData, EventKind};
use near_sdk::{near_bindgen, AccountId};

use crate::{Contract, ContractExt};

pub trait CleanApi {
    fn clean(&mut self, account_ids: Vec<AccountId>);
}

#[near_bindgen]
impl CleanApi for Contract {
    fn clean(&mut self, account_ids: Vec<AccountId>) {
        self.assert_oracle();

        for account_id in account_ids.clone() {
            self.accounts_legacy.set(account_id, None);
        }

        emit(EventKind::Clean(CleanData { account_ids }));
    }
}
