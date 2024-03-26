use near_sdk::{
    env::{current_account_id, predecessor_account_id},
    require,
};

use crate::Contract;

impl Contract {
    pub(crate) fn assert_oracle(&self) {
        require!(
            self.oracles.contains(&predecessor_account_id()),
            "Unauthorized access! Only oracle can do this!"
        );
    }

    pub(crate) fn assert_private() {
        require!(current_account_id() == predecessor_account_id(), "Method is private",);
    }

    pub(crate) fn assert_update(&self) {
        self.assert_oracle();
    }
}
