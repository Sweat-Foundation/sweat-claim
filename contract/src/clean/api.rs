use claim_model::event::{emit, CleanData, EventKind};
use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{near_bindgen, require, AccountId};

use crate::{auth::Roles, common::MAX_BATCH_SIZE, Contract, ContractExt};

pub trait CleanApi {
    fn clean(&mut self, account_ids: Vec<AccountId>);
}

#[near_bindgen]
impl CleanApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn clean(&mut self, account_ids: Vec<AccountId>) {
        require!(account_ids.len() <= MAX_BATCH_SIZE, "Batch size exceeds the maximum allowed");

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
