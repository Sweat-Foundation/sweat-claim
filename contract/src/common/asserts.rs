use near_sdk::{
    env::{current_account_id, predecessor_account_id},
    require, AccountId, Gas,
};

use crate::{common::remaining_gas, Contract};

impl Contract {
    pub(crate) fn assert_oracle(&self, account_id: &AccountId) {
        require!(
            self.oracles.contains(account_id),
            "Unauthorized access! Only oracle can do this!"
        );
    }

    pub(crate) fn assert_called_by_oracle(&self) {
        self.assert_oracle(&predecessor_account_id());
    }

    pub(crate) fn assert_private() {
        require!(current_account_id() == predecessor_account_id(), "Method is private",);
    }
}

pub(crate) fn assert_enough_gas(required: Gas) {
    require!(remaining_gas() >= required, "Not enough gas for further operations");
}
