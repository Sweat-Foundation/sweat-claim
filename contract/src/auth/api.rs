use claim_model::api::AuthApi;
use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{near_bindgen, AccountId};

use crate::{auth::Roles, common::AccountAccessor, Contract, ContractExt};

#[near_bindgen]
impl AuthApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn unlock_account(&mut self, account_id: AccountId) {
        let account = self.accounts.get_account_mut(&account_id);
        account.is_locked = false;
    }
}
