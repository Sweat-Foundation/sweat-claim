#![cfg(test)]

use std::time::Duration;

use claim_model::{
    account_record::AccountRecordLegacy,
    api::InitApi,
    event::{emit, EventKind::Record, RecordData},
};
use near_sdk::{json_types::U128, store::Vector, test_utils::VMContextBuilder, testing_env, AccountId};

use crate::{common::now_seconds, Contract, StorageKey::_AccrualsEntryLegacy};

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

impl Contract {
    pub(crate) fn record_batch_for_hold_legacy(&mut self, amounts: Vec<(AccountId, U128)>) {
        self.assert_oracle();

        let now_seconds = now_seconds();
        let mut event_data = RecordData::new(now_seconds);

        let balances = self
            .accruals
            .entry(now_seconds)
            .or_insert_with(|| (Vector::new(_AccrualsEntryLegacy(now_seconds)), 0));

        for (account_id, amount) in amounts {
            event_data.amounts.push((account_id.clone(), amount));

            let amount = amount.0;
            let index = balances.0.len();

            balances.1 += amount;
            balances.0.push(amount);

            if let Some(record) = self.accounts_legacy.get_mut(&account_id) {
                record.accruals.push((now_seconds, index));
            } else {
                let record = AccountRecordLegacy {
                    accruals: vec![(now_seconds, index)],
                    ..AccountRecordLegacy::new(now_seconds)
                };

                self.accounts_legacy.insert(account_id, record);
            }
        }

        emit(Record(event_data));
    }
}
