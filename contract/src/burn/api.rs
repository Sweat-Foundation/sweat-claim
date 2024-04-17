use claim_model::{
    api::BurnApi,
    event::{emit, BurnData, EventKind},
    BurnStatus, TokensAmount, UnixTimestamp,
};
use near_sdk::{env::panic_str, json_types::U128, near_bindgen, require, AccountId, PromiseOrValue};

use crate::{
    common::{now_seconds, UnixTimestampExtension},
    Contract, ContractExt,
};

#[near_bindgen]
impl BurnApi for Contract {
    fn burn(&mut self) -> PromiseOrValue<U128> {
        self.assert_oracle();

        require!(!self.is_service_call_running, "Another service call is running");

        self.is_service_call_running = true;

        let mut total_to_burn = 0;
        let mut keys_to_remove = vec![];
        let now = now_seconds();

        for (datetime, (_, total)) in self.accruals.iter() {
            if !datetime.is_within_period(now, self.burn_period) {
                keys_to_remove.push(*datetime);
                total_to_burn += total;
            }
        }

        if total_to_burn > 0 {
            self.burn_external(total_to_burn, keys_to_remove)
        } else {
            self.is_service_call_running = false;

            PromiseOrValue::Value(U128(0))
        }
    }

    fn get_burn_status(&self, account_id: AccountId) -> BurnStatus {
        let now = now_seconds();
        let account = self
            .accounts
            .get(&account_id)
            .unwrap_or_else(|| panic_str(format!("Account {account_id} not found").as_str()));
        let min_claimable_ts: Option<UnixTimestamp> = account
            .accruals
            .iter()
            .map(|(timestamp, _)| timestamp)
            .filter(|timestamp| timestamp.is_within_period(now, self.burn_period))
            .min()
            .cloned();

        BurnStatus {
            min_claimable_ts,
            claim_period_refreshed_at: account.claim_period_refreshed_at,
            burn_period: self.burn_period,
        }
    }
}

impl Contract {
    fn on_burn_internal(
        &mut self,
        total_to_burn: TokensAmount,
        keys_to_remove: Vec<UnixTimestamp>,
        is_success: bool,
    ) -> U128 {
        self.is_service_call_running = false;

        if !is_success {
            return U128(0);
        }

        for datetime in keys_to_remove {
            self.accruals.remove(&datetime);
        }

        emit(EventKind::Burn(BurnData {
            burnt_amount: U128(total_to_burn),
        }));

        U128(total_to_burn)
    }
}

#[cfg(not(test))]
pub(crate) mod prod {
    use claim_model::{TokensAmount, UnixTimestamp};
    use near_sdk::{
        env, ext_contract, is_promise_success, json_types::U128, near_bindgen, serde_json::json, Gas, Promise,
        PromiseOrValue,
    };

    use crate::{Contract, ContractExt};

    #[ext_contract(ext_self)]
    pub trait SelfCallback {
        fn on_burn(&mut self, total_to_burn: TokensAmount, keys_to_remove: Vec<UnixTimestamp>) -> U128;
    }

    #[near_bindgen]
    impl SelfCallback for Contract {
        #[private]
        fn on_burn(&mut self, total_to_burn: TokensAmount, keys_to_remove: Vec<UnixTimestamp>) -> U128 {
            self.on_burn_internal(total_to_burn, keys_to_remove, is_promise_success())
        }
    }

    impl Contract {
        pub(crate) fn burn_external(
            &mut self,
            total_to_burn: TokensAmount,
            keys_to_remove: Vec<UnixTimestamp>,
        ) -> PromiseOrValue<U128> {
            let args = json!({
                "amount": U128(total_to_burn),
            })
            .to_string()
            .as_bytes()
            .to_vec();

            Promise::new(self.token_account_id.clone())
                .function_call("burn".to_string(), args, 0, Gas(5 * Gas::ONE_TERA.0))
                .then(
                    ext_self::ext(env::current_account_id())
                        .with_static_gas(Gas(5 * Gas::ONE_TERA.0))
                        .on_burn(total_to_burn, keys_to_remove),
                )
                .into()
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use claim_model::{TokensAmount, UnixTimestamp};
    use near_sdk::{json_types::U128, PromiseOrValue};

    use crate::{common::tests::data::get_test_future_success, Contract};

    pub(crate) const EXT_BURN_FUTURE: &str = "ext_burn";

    impl Contract {
        pub(crate) fn burn_external(
            &mut self,
            total_to_burn: TokensAmount,
            keys_to_remove: Vec<UnixTimestamp>,
        ) -> PromiseOrValue<U128> {
            PromiseOrValue::Value(self.on_burn_internal(
                total_to_burn,
                keys_to_remove,
                get_test_future_success(EXT_BURN_FUTURE),
            ))
        }
    }
}
