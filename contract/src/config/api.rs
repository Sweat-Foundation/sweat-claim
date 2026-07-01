use claim_model::{api::ConfigApi, Duration};
use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{near_bindgen, require};

use crate::{auth::Roles, Contract, ContractExt};

#[near_bindgen]
impl ConfigApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn set_claim_period(&mut self, period: Duration) {
        require!(
            period < self.burn_period,
            "Claim period should be less than burn period"
        );

        self.claim_period = period;
    }

    #[access_control_any(roles(Roles::Maintainer))]
    fn set_burn_period(&mut self, period: Duration) {
        require!(period > 0, "Burn period should be greater than 0");
        require!(
            period > self.claim_period,
            "Burn period should be greater than claim period"
        );

        self.burn_period = period;
    }
}
