use claim_model::{
    api::ClaimApi,
    event::{emit, ClaimData, EventKind},
    ClaimAvailabilityView, ClaimResultView, ClaimableBalanceView, TokensAmount, UnixTimestamp, UnixTimestampExtension,
};
use near_sdk::{env, json_types::U128, near_bindgen, require, AccountId, PromiseOrValue};

use crate::{
    common::{now_seconds, AccountAccessor},
    Contract, ContractExt,
};

#[near_bindgen]
impl ClaimApi for Contract {
    fn get_claimable_balance_for_account(&self, account_id: AccountId, detailed: Option<bool>) -> ClaimableBalanceView {
        let detailed = detailed.unwrap_or(false);

        if let Some(account) = self.accounts.get(&account_id) {
            let account = account.into_latest();

            let amount_to_burn = account.get_balance_to_burn(self.burn_period, self.get_claimable_window_start());
            let amount_to_claim = account.balance - amount_to_burn;

            return ClaimableBalanceView::new(account.balance, amount_to_claim, detailed);
        }

        ClaimableBalanceView::new(0, 0, detailed)
    }

    fn is_claim_available(&self, account_id: AccountId) -> ClaimAvailabilityView {
        if let Some(account) = self.accounts.get(&account_id) {
            let account = account.into_latest();
            let claim_period_refreshed_at = account.claim_period_refreshed_at;

            return if claim_period_refreshed_at.is_within_period(now_seconds(), self.claim_period) {
                ClaimAvailabilityView::Unavailable((claim_period_refreshed_at, self.claim_period))
            } else {
                ClaimAvailabilityView::Available(0)
            };
        }

        ClaimAvailabilityView::Unregistered
    }

    fn claim(&mut self) -> PromiseOrValue<ClaimResultView> {
        let account_id = env::predecessor_account_id();

        require!(
            matches!(
                self.is_claim_available(account_id.clone()),
                ClaimAvailabilityView::Available(_)
            ),
            "Claim is not available at the moment"
        );

        let account = self.accounts.get_account(&account_id);
        require!(!account.is_locked, "Another operation is running");

        if account.balance == 0 {
            return PromiseOrValue::Value(ClaimResultView::new(0));
        }

        let amount_to_burn = account.get_balance_to_burn(self.burn_period, self.get_claimable_window_start());
        let amount_to_claim = account.balance - amount_to_burn;

        let account = self.accounts.get_or_insert_account_mut(&account_id);
        account.balance = 0;

        if amount_to_claim == 0 {
            return PromiseOrValue::Value(self.on_claim_result(now_seconds(), account_id, 0, amount_to_burn, true));
        }

        account.is_locked = true;
        self.transfer_external(now_seconds(), account_id, amount_to_claim, amount_to_burn)
    }
}

impl Contract {
    fn on_claim_result(
        &mut self,
        now: UnixTimestamp,
        account_id: AccountId,
        amount_to_claim: TokensAmount,
        amount_to_burn: TokensAmount,
        is_success: bool,
    ) -> ClaimResultView {
        let account = self.accounts.get_or_insert_account_mut(&account_id);
        account.is_locked = false;

        if !is_success {
            account.balance += amount_to_claim + amount_to_burn;
            return ClaimResultView::new(0);
        }

        // `balance_to_burn` is updated here because parallel `burn` call can modify this value.
        // In this case rolling back a user state to a previous state can lead to inconsistency.
        self.balance_to_burn += amount_to_burn;

        account.claim_period_refreshed_at = now;
        account.burn_since = now;

        let event_data = ClaimData {
            account_id,
            claimed: U128(amount_to_claim),
            burnt: U128(amount_to_burn),
        };
        emit(EventKind::Claim(event_data));

        ClaimResultView::new(amount_to_claim)
    }
}

#[cfg(not(test))]
mod prod {
    use claim_model::{ClaimResultView, TokensAmount, UnixTimestamp};
    use near_sdk::{
        env, ext_contract, is_promise_success, near_bindgen, require, serde_json::json, AccountId, Gas, NearToken,
        Promise, PromiseOrValue,
    };

    use crate::{common::asserts::assert_enough_gas, Contract, ContractExt};

    const GAS_FOR_TRANSFER: Gas = Gas::from_tgas(5);
    const GAS_FOR_TRANSFER_CALLBACK: Gas = Gas::from_tgas(5);

    #[ext_contract(ext_self)]
    pub trait SelfCallback {
        // Invoked via the near_bindgen-generated wasm export; appears unused on the host build.
        #[allow(dead_code)]
        fn on_transfer(
            &mut self,
            now: UnixTimestamp,
            account_id: AccountId,
            amount_to_claim: TokensAmount,
            amount_to_burn: TokensAmount,
        ) -> ClaimResultView;
    }

    #[near_bindgen]
    impl SelfCallback for Contract {
        #[private]
        fn on_transfer(
            &mut self,
            now: UnixTimestamp,
            account_id: AccountId,
            amount_to_claim: TokensAmount,
            amount_to_burn: TokensAmount,
        ) -> ClaimResultView {
            self.on_claim_result(now, account_id, amount_to_claim, amount_to_burn, is_promise_success())
        }
    }

    impl Contract {
        pub(crate) fn transfer_external(
            &mut self,
            now: UnixTimestamp,
            account_id: AccountId,
            amount_to_claim: TokensAmount,
            amount_to_burn: TokensAmount,
        ) -> PromiseOrValue<ClaimResultView> {
            require!(amount_to_claim > 0, "Cannot transfer zero tokens");
            assert_enough_gas(GAS_FOR_TRANSFER.saturating_add(GAS_FOR_TRANSFER_CALLBACK));

            let callback = ext_self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_TRANSFER_CALLBACK)
                .on_transfer(now, account_id.clone(), amount_to_claim, amount_to_burn);

            let args = json!({
                "receiver_id": account_id.clone(),
                "amount": amount_to_claim.to_string(),
                "memo": "",
            })
            .to_string()
            .as_bytes()
            .to_vec();

            Promise::new(self.token_account_id.clone())
                .function_call("ft_transfer".to_string(), args, NearToken::from_yoctonear(1), GAS_FOR_TRANSFER)
                .then(callback)
                .into()
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use claim_model::{api::RecordApi, ClaimResultView, TokensAmount, UnixTimestamp};
    use near_sdk::{json_types::U128, AccountId, PromiseOrValue};

    use crate::{
        common::{
            tests::{data::get_test_future_success, Context},
            AccountAccessor,
        },
        Contract,
    };

    pub(crate) const EXT_TRANSFER_FUTURE: &str = "ext_transfer";

    impl Contract {
        pub(crate) fn transfer_external(
            &mut self,
            now: UnixTimestamp,
            account_id: AccountId,
            amount_to_claim: TokensAmount,
            amount_to_burn: TokensAmount,
        ) -> PromiseOrValue<ClaimResultView> {
            PromiseOrValue::Value(self.on_claim_result(
                now,
                account_id,
                amount_to_claim,
                amount_to_burn,
                get_test_future_success(EXT_TRANSFER_FUTURE),
            ))
        }
    }

    #[test]
    fn on_claim_result_failure_does_not_discard_balance_recorded_during_flight() {
        let (mut context, mut contract, accounts) = Context::init_with_oracle();

        context.switch_account(&accounts.oracle);
        contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(1_000))]);

        // Simulate `claim()` having zeroed the balance and locked the account
        // while its transfer promise is in flight.
        let account = contract.accounts.get_or_insert_account_mut(&accounts.alice);
        account.balance = 0;
        account.is_locked = true;

        // Oracle credits the account mid-flight; record_batch_for_hold doesn't check is_locked.
        contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(500))]);

        // The in-flight transfer then fails.
        let result = contract.on_claim_result(0, accounts.alice.clone(), 1_000, 0, false);
        assert_eq!(0, result.total.0);

        let balance = contract.accounts.get_account(&accounts.alice).balance;
        assert_eq!(
            1_500, balance,
            "balance recorded while claim was in flight must not be discarded"
        );
    }
}
