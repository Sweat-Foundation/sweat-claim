#![cfg(test)]

use claim_model::{
    account_record::AccountRecordLegacy,
    api::InitApi,
    event::{emit, EventKind::Record, RecordAmountDetailed, RecordData},
    Duration, TokensAmount,
};
use near_sdk::{json_types::U128, store::Vector, test_utils::VMContextBuilder, testing_env, AccountId};

use crate::{common::now_seconds, Contract, StorageKey::_AccrualsEntryLegacy};

pub(crate) struct Context {
    builder: VMContextBuilder,
}

pub(crate) fn days_to_seconds(days: u64) -> Duration {
    (days * 24 * 60 * 60) as Duration
}

pub(crate) fn sweat_to_atto(sweat: u128) -> TokensAmount {
    sweat * 10u128.pow(18)
}

impl Context {
    pub(crate) fn init_with_oracle() -> (Context, Contract, TestAccounts) {
        let (context, mut contract, accounts) = Self::init();
        contract.oracles.insert(accounts.oracle.clone());

        (context, contract, accounts)
    }

    pub(crate) fn init() -> (Context, Contract, TestAccounts) {
        let accounts = TestAccounts::default();
        let token_account = accounts.token.clone();

        let mut builder = VMContextBuilder::new();

        builder
            .current_account_id(accounts.owner.clone())
            .signer_account_id(accounts.owner.clone())
            .predecessor_account_id(accounts.owner.clone())
            .block_timestamp(0);

        testing_env!(builder.build());

        let contract = Contract::init(token_account);
        let context = Context { builder };

        (context, contract, accounts)
    }

    pub(crate) fn switch_account(&mut self, account_id: &AccountId) {
        self.builder
            .predecessor_account_id(account_id.clone())
            .signer_account_id(account_id.clone());
        testing_env!(self.builder.build());
    }

    pub(crate) fn set_block_timestamp_in_seconds(&mut self, seconds: u64) {
        self.set_block_timestamp(std::time::Duration::from_secs(seconds));
    }

    fn set_block_timestamp(&mut self, duration: std::time::Duration) {
        self.builder.block_timestamp(duration.as_nanos() as u64);
        testing_env!(self.builder.build());
    }
}

#[derive(Debug)]
pub(crate) struct TestAccounts {
    pub alice: AccountId,
    pub bob: AccountId,
    pub oracle: AccountId,
    pub token: AccountId,
    pub owner: AccountId,
}

impl Default for TestAccounts {
    fn default() -> Self {
        Self {
            alice: AccountId::new_unchecked("alice".to_string()),
            bob: AccountId::new_unchecked("bob".to_string()),
            oracle: AccountId::new_unchecked("oracle".to_string()),
            token: AccountId::new_unchecked("token".to_string()),
            owner: AccountId::new_unchecked("owner".to_string()),
        }
    }
}

pub(crate) mod data {
    use std::{
        collections::BTreeMap,
        sync::{Mutex, MutexGuard},
    };

    type ThreadId = String;
    type ValueKey = String;
    type Value = String;

    type Map = BTreeMap<ThreadId, BTreeMap<ValueKey, Value>>;

    struct TestDataStorage {
        data: Mutex<Map>,
    }

    static DATA: TestDataStorage = TestDataStorage {
        data: Mutex::new(BTreeMap::new()),
    };

    fn data() -> MutexGuard<'static, Map> {
        DATA.data.lock().unwrap()
    }

    pub(crate) fn set_test_future_success(name: &str, success: bool) {
        let mut data = data();
        let map = data.entry(thread_name()).or_default();
        map.insert(name.to_owned(), success.to_string());
    }

    pub(crate) fn get_test_future_success(name: &str) -> bool {
        let data = data();

        let Some(map) = data.get(&thread_name()) else {
            return true;
        };

        let Some(value) = map.get(name) else {
            return true;
        };

        value.parse().unwrap()
    }

    fn thread_name() -> String {
        std::thread::current().name().unwrap().to_owned()
    }

    #[test]
    fn thread_name_test() {
        assert_eq!(thread_name(), "common::tests::data::thread_name_test");
    }

    #[test]
    fn test_data_storage() {
        let name = "test_future";
        assert!(get_test_future_success(name));
        set_test_future_success(name, false);
        assert!(!get_test_future_success(name));
        set_test_future_success(name, true);
        assert!(get_test_future_success(name));
    }
}

#[cfg(test)]
pub(crate) mod balance_tests {
    use claim_model::{
        api::{ClaimApi, ConfigApi, RecordApi},
        get_burn_rate,
    };
    use near_sdk::json_types::U128;

    use crate::common::tests::Context;

    #[test]
    fn test_effective_balance() {
        let (mut context, mut contract, accounts) = Context::init_with_oracle();
        let burn_period = 100_000;

        context.switch_account(&accounts.oracle);
        contract.set_claim_period(0);
        contract.set_burn_period(burn_period);

        context.set_block_timestamp_in_seconds(0);

        let alice_balance = 100_000_000;
        let alice_burn_rate = get_burn_rate(alice_balance, burn_period);

        contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

        for i in 1..=5 {
            let seconds_after_burn_start: u64 = burn_period as u64 / i as u64;
            context.set_block_timestamp_in_seconds(burn_period as u64 + seconds_after_burn_start);

            let alice_current_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
            assert_eq!(
                alice_balance - alice_burn_rate * seconds_after_burn_start as u128,
                alice_current_balance
            );
        }
    }
}

#[cfg(test)]
mod account_record_tests {
    use claim_model::account_record::AccountRecord;

    #[test]
    fn test_burn_rate_for_multiple_balance() {
        let burn_period = 100_000_000;

        let mut account = AccountRecord::new(0);
        account.balance = 10u128.pow(18);

        assert_eq!(10_000_000_000, account.get_burn_rate(burn_period));
    }

    #[test]
    fn test_burn_rate_for_minimal_balance_with_long_burn_period() {
        let burn_period = 30 * 24 * 60 * 60; // 30 days

        let mut account = AccountRecord::new(0);
        account.balance = 1;

        assert_eq!(1, account.get_burn_rate(burn_period));
    }

    #[test]
    fn test_burn_rate_rounding() {
        let burn_period = 21 * 24 * 60 * 60; // 21 days

        let mut account = AccountRecord::new(0);
        account.balance = 2 * 10u128.pow(18);

        // Precise value is 1_102_292_768_959,4356261023
        assert_eq!(1_102_292_768_960, account.get_burn_rate(burn_period));
    }

    #[test]
    fn test_balance_to_burn_when_balance_doesnt_evaporate() {
        let burn_period = 30 * 24 * 60 * 60; // 30 days

        let mut account = AccountRecord::new(0);
        account.balance = 2 * 10u128.pow(18);
        account.claim_period_refreshed_at = burn_period / 2;

        let balance_to_burn = account.get_balance_to_burn(burn_period, 0);
        assert_eq!(0, balance_to_burn);
    }

    #[test]
    fn test_balance_to_burn_when_balance_evaporates() {
        let burn_period = 1_000;

        let mut account = AccountRecord::new(0);
        // User claimed their funds
        account.claim_period_refreshed_at = 1_713_880_390;
        account.burn_since = account.claim_period_refreshed_at;
        // And then earned some $SWEAT
        account.balance = 1_000;

        let claimable_window_start = 1_713_880_400; // 10 seconds after last claim
        let balance_to_burn = account.get_balance_to_burn(burn_period, claimable_window_start);
        assert_eq!(10, balance_to_burn);
    }

    #[test]
    fn test_balance_to_burn_when_balance_evaporated_to_zer() {
        let burn_period = 5 * 24 * 60 * 60; // 5 days

        let mut account = AccountRecord::new(0);
        account.balance = 5_000;

        let claimable_window_start = 3 * burn_period; // 3 burn periods later
        let balance_to_burn = account.get_balance_to_burn(burn_period, claimable_window_start);
        assert_eq!(account.balance, balance_to_burn);
    }
}

#[cfg(test)]
mod contract_common_tests {
    use crate::Contract;

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn should_test_on_default() {
        Contract::default();
    }
}
