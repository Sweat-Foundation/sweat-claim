use claim_model::{
    api::BurnApi,
    event::{emit, BurnData, EventKind},
    BurnStatus, TokensAmount, UnixTimestamp, UnixTimestampExtension,
};
use near_sdk::{json_types::U128, near_bindgen, require, AccountId, PromiseOrValue};

use crate::{
    common::{now_seconds, AccountAccessor},
    Contract, ContractExt,
};

#[near_bindgen]
impl BurnApi for Contract {
    fn burn(&mut self) -> PromiseOrValue<U128> {
        self.assert_oracle();

        require!(!self.is_service_call_running, "Another service call is running");

        let amount_to_burn = self.balance_to_burn;

        if amount_to_burn > 0 {
            self.is_service_call_running = true;
            self.balance_to_burn = 0;

            self.burn_external(amount_to_burn)
        } else {
            PromiseOrValue::Value(U128(0))
        }
    }

    fn get_burn_status(&self, account_id: AccountId) -> BurnStatus {
        let now = now_seconds();

        if let Some(account) = self.accounts_legacy.get(&account_id) {
            let min_claimable_ts: Option<UnixTimestamp> = account
                .accruals
                .iter()
                .map(|(timestamp, _)| timestamp)
                .filter(|timestamp| timestamp.is_within_period(now, self.burn_period))
                .min()
                .copied();

            return BurnStatus {
                min_claimable_ts,
                claim_period_refreshed_at: account.claim_period_refreshed_at,
                burn_period: self.burn_period,
            };
        }

        let account = self.accounts.get_account(&account_id);

        BurnStatus {
            min_claimable_ts: None,
            claim_period_refreshed_at: account.claim_period_refreshed_at,
            burn_period: self.burn_period,
        }
    }
}

impl Contract {
    fn on_burn_internal(&mut self, amount_to_burn: TokensAmount, is_success: bool) -> U128 {
        self.is_service_call_running = false;

        if is_success {
            emit(EventKind::Burn(BurnData {
                burnt_amount: U128(amount_to_burn),
            }));

            U128(amount_to_burn)
        } else {
            // If burning failed, return the amount back to the balance.
            // Another `claim` call can increase `balance_to_burn`, so it can be non-zero at this point.
            self.balance_to_burn += amount_to_burn;

            U128(0)
        }
    }
}

#[cfg(not(test))]
pub(crate) mod prod {
    use claim_model::TokensAmount;
    use near_sdk::{
        env, ext_contract, is_promise_success, json_types::U128, near_bindgen, require, serde_json::json, Gas, Promise,
        PromiseOrValue,
    };

    use crate::{common::asserts::assert_enough_gas, Contract, ContractExt};

    const GAS_FOR_BURN: Gas = Gas(5 * Gas::ONE_TERA.0);
    const GAS_FOR_BURN_CALLBACK: Gas = Gas(5 * Gas::ONE_TERA.0);

    #[ext_contract(ext_self)]
    pub trait SelfCallback {
        fn on_burn(&mut self, amount_to_burn: TokensAmount) -> U128;
    }

    #[near_bindgen]
    impl SelfCallback for Contract {
        #[private]
        fn on_burn(&mut self, amount_to_burn: TokensAmount) -> U128 {
            self.on_burn_internal(amount_to_burn, is_promise_success())
        }
    }

    impl Contract {
        pub(crate) fn burn_external(&mut self, amount_to_burn: TokensAmount) -> PromiseOrValue<U128> {
            require!(amount_to_burn > 0, "Nothing to burn");
            assert_enough_gas(GAS_FOR_BURN + GAS_FOR_BURN_CALLBACK);

            let args = json!({
                "amount": U128(amount_to_burn),
            })
            .to_string()
            .as_bytes()
            .to_vec();

            Promise::new(self.token_account_id.clone())
                .function_call("burn".to_string(), args, 0, GAS_FOR_BURN)
                .then(
                    ext_self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_BURN_CALLBACK)
                        .on_burn(amount_to_burn),
                )
                .into()
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use claim_model::TokensAmount;
    use near_sdk::{json_types::U128, PromiseOrValue};

    use crate::{common::tests::data::get_test_future_success, Contract};

    pub(crate) const EXT_BURN_FUTURE: &str = "ext_burn";

    impl Contract {
        pub(crate) fn burn_external(&mut self, amount_to_burn: TokensAmount) -> PromiseOrValue<U128> {
            PromiseOrValue::Value(self.on_burn_internal(amount_to_burn, get_test_future_success(EXT_BURN_FUTURE)))
        }
    }
}
