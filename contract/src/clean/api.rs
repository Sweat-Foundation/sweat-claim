use claim_model::event::{emit, CleanData, EventKind};
use near_sdk::{near_bindgen, AccountId};

use crate::{Contract, ContractExt};

pub trait CleanApi {
    // Invoked via the near_bindgen-generated wasm export; appears unused on the host build.
    #[allow(dead_code)]
    fn clean(&mut self, account_ids: Vec<AccountId>);
}

#[near_bindgen]
impl CleanApi for Contract {
    fn clean(&mut self, account_ids: Vec<AccountId>) {
        self.assert_oracle();

        for account_id in account_ids.clone() {
            if let Some(account) = self.accounts.get(&account_id) {
                self.balance_to_burn += account.into_latest().balance;
            }
            self.accounts.set(account_id, None);
        }

        emit(EventKind::Clean(CleanData { account_ids }));
    }
}
