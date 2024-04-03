#![cfg(test)]

use std::time::Duration;

use claim_model::api::InitApi;
use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId};

use crate::Contract;

pub(crate) struct Context {
    builder: VMContextBuilder,
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
        self.set_block_timestamp(Duration::from_secs(seconds));
    }

    fn set_block_timestamp(&mut self, duration: Duration) {
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
    use claim_model::api::{ClaimApi, ConfigApi, RecordApi};
    use near_sdk::json_types::U128;

    use crate::common::tests::Context;

    #[test]
    fn test_effective_balance() {
        let (mut context, mut contract, accounts) = Context::init_with_oracle();
        context.switch_account(&accounts.oracle);
        contract.set_claim_period(0);
        contract.set_burn_period(10_000);

        context.set_block_timestamp_in_seconds(0);

        let mut alice_balance = 0;

        let mut alice_top_up = 400_000;
        contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_top_up))]);
        alice_balance += alice_top_up;

        context.set_block_timestamp_in_seconds(3_000);

        alice_top_up = 100_000;
        contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_top_up))]);
        alice_balance += alice_top_up;

        context.set_block_timestamp_in_seconds(5_000);

        alice_top_up = 500_000;
        contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_top_up))]);
        alice_balance += alice_top_up;

        context.set_block_timestamp_in_seconds(6_000);

        let alice_current_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        assert_eq!(alice_balance, alice_current_balance);

        context.set_block_timestamp_in_seconds(10_000);

        let alice_current_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        assert_eq!(alice_balance, alice_current_balance);

        context.set_block_timestamp_in_seconds(12_000);

        let alice_current_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        assert_eq!((alice_balance / 100) * 60, alice_current_balance);

        context.set_block_timestamp_in_seconds(14_000);

        let alice_current_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        assert_eq!((alice_balance / 100) * 20, alice_current_balance);

        context.set_block_timestamp_in_seconds(20_000);

        let alice_current_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        assert_eq!(0, alice_current_balance);
    }
}
