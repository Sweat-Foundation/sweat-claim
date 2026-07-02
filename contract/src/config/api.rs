use claim_model::{api::ConfigApi, Duration};
use near_plugins::{access_control_any, AccessControllable};
use near_sdk::{near_bindgen, require};

use crate::{auth::Roles, Contract, ContractExt};

/// Upper bound on both `claim_period` and `burn_period`. Without this, a single
/// `set_claim_period`/`set_burn_period` call raising a period far enough could
/// make every existing account's `claim_period_refreshed_at` look "recently
/// refreshed" indefinitely, freezing `claim()` contract-wide.
pub(crate) const MAX_PERIOD_SECS: Duration = 365 * 24 * 60 * 60; // 1 year

#[near_bindgen]
impl ConfigApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn set_claim_period(&mut self, period: Duration) {
        require!(period <= MAX_PERIOD_SECS, "Claim period exceeds the maximum allowed");
        require!(
            period < self.burn_period,
            "Claim period should be less than burn period"
        );

        self.claim_period = period;
    }

    #[access_control_any(roles(Roles::Maintainer))]
    fn set_burn_period(&mut self, period: Duration) {
        require!(period > 0, "Burn period should be greater than 0");
        require!(period <= MAX_PERIOD_SECS, "Burn period exceeds the maximum allowed");
        require!(
            period > self.claim_period,
            "Burn period should be greater than claim period"
        );

        self.burn_period = period;
    }
}
