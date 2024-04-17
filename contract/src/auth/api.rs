use claim_model::api::AuthApi;
use near_sdk::{env::log_str, near_bindgen, require, AccountId};

use crate::{common::AccountAccessor, Contract, ContractExt};

#[near_bindgen]
impl AuthApi for Contract {
    fn add_oracle(&mut self, account_id: AccountId) {
        Self::assert_private();

        require!(self.oracles.insert(account_id.clone()), "Already exists");
        log_str(&format!("Oracle {account_id} was added"));
    }

    fn remove_oracle(&mut self, account_id: AccountId) {
        Self::assert_private();

        require!(self.oracles.remove(&account_id), "No such oracle");
        log_str(&format!("Oracle {account_id} was removed"));
    }

    fn get_oracles(&self) -> Vec<AccountId> {
        self.oracles.iter().cloned().collect()
    }

    fn unlock_account(&mut self, account_id: AccountId) {
        self.assert_oracle();

        if let Some(account) = self.accounts_legacy.get_mut(&account_id) {
            account.is_locked = false;
        }

        let account = self.accounts.get_account_mut(&account_id);
        account.is_locked = false;
    }
}
