use claim_model::{api::ConfigApi, Duration};
use near_sdk::{near_bindgen, require};

use crate::{Contract, ContractExt};

#[near_bindgen]
impl ConfigApi for Contract {
    fn set_claim_period(&mut self, period: Duration) {
        self.assert_called_by_oracle();
        require!(
            period < self.burn_period,
            "Claim period should be less than burn period"
        );

        self.claim_period = period;
    }

    fn set_burn_period(&mut self, period: Duration) {
        self.assert_called_by_oracle();
        require!(period > 0, "Burn period should be greater than 0");
        require!(
            period > self.claim_period,
            "Burn period should be greater than claim period"
        );

        self.burn_period = period;
    }
}
