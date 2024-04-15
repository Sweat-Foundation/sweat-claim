use claim_model::api::AuthApi;
use near_sdk::{env::log_str, near_bindgen, require, AccountId};

use crate::{Contract, ContractExt};

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

        let account = self.accounts.get_mut(&account_id).expect("Account is not found");
        account.is_locked = false;
    }
}
