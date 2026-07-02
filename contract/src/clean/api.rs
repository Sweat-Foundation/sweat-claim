use claim_model::event::{emit, CleanData, EventKind};
use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{near_bindgen, AccountId};

use crate::{auth::Roles, Contract, ContractExt};

pub trait CleanApi {
    // Invoked via the near_bindgen-generated wasm export; appears unused on the host build.
    #[allow(dead_code)]
    fn clean(&mut self, account_ids: Vec<AccountId>);
}

#[near_bindgen]
impl CleanApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn clean(&mut self, account_ids: Vec<AccountId>) {
        for account_id in account_ids.clone() {
            let balance = self.accounts.get(&account_id).map(|account| account.into_latest().balance);
            if let Some(balance) = balance {
                self.credit_balance_to_burn(balance);
            }
            self.accounts.set(account_id, None);
        }

        emit(EventKind::Clean(CleanData { account_ids }));
    }
}
