use claim_model::{
    account_record::{AccountRecord, AccountRecordVersioned},
    TokensAmount, UnixTimestamp,
};
use near_sdk::{
    env,
    env::{block_timestamp_ms, panic_str},
    store::LookupMap,
    AccountId, Gas,
};

use crate::Contract;

pub(crate) mod asserts;
pub(crate) mod tests;

pub(crate) fn remaining_gas() -> Gas {
    env::prepaid_gas().saturating_sub(env::used_gas())
}

fn ms_timestamp_to_seconds(ms: u64) -> UnixTimestamp {
    u32::try_from(ms / 1000)
        .unwrap_or_else(|err| panic_str(&format!("Failed to get convert milliseconds to Unix timestamp: {err}")))
}

pub(crate) fn now_seconds() -> UnixTimestamp {
    ms_timestamp_to_seconds(block_timestamp_ms())
}

#[test]
fn convert_milliseconds_to_unix_timestamp_successfully() {
    let millis: u64 = 1_699_038_575_819;
    let timestamp = ms_timestamp_to_seconds(millis);

    assert_eq!(1_699_038_575, timestamp);
}

#[test]
#[should_panic(expected = "Failed to get convert milliseconds to Unix timestamp")]
fn convert_milliseconds_to_unix_timestamp_with_unsuccessfully() {
    let millis: u64 = u64::MAX;
    let _timestamp = ms_timestamp_to_seconds(millis);
}

pub type AccountMap = LookupMap<AccountId, AccountRecordVersioned>;

pub(crate) trait AccountAccessor {
    fn get_account(&self, account_id: &AccountId) -> &AccountRecord;

    fn get_account_mut(&mut self, account_id: &AccountId) -> &mut AccountRecord;

    fn get_or_insert_account_mut(&mut self, account_id: &AccountId) -> &mut AccountRecord;
}

impl AccountAccessor for AccountMap {
    fn get_account(&self, account_id: &AccountId) -> &AccountRecord {
        let AccountRecordVersioned::V1(account) = self.get(account_id).expect("Account not found");
        account
    }

    fn get_account_mut(&mut self, account_id: &AccountId) -> &mut AccountRecord {
        let AccountRecordVersioned::V1(account) = self.get_mut(account_id).expect("Account not found");
        account
    }

    fn get_or_insert_account_mut(&mut self, account_id: &AccountId) -> &mut AccountRecord {
        let versioned = self
            .entry(account_id.clone())
            .or_insert_with(|| AccountRecordVersioned::new(now_seconds()));
        let AccountRecordVersioned::V1(account) = versioned;
        account
    }
}

impl Contract {
    pub(crate) fn get_claimable_window_start(&self) -> UnixTimestamp {
        // Can be 0 only in tests.
        now_seconds().saturating_sub(self.burn_period)
    }

    /// Adds `amount` to `balance_to_burn`. Callers are responsible for any
    /// amount validation specific to their call site.
    pub(crate) fn credit_balance_to_burn(&mut self, amount: TokensAmount) {
        self.balance_to_burn += amount;
    }

    /// Subtracts `amount` from `balance_to_burn`, clamping to 0 rather than
    /// underflowing. Callers should still validate `amount <= balance_to_burn`
    /// beforehand with a message specific to their call site; this is a
    /// defensive floor for the shared invariant, not a substitute for that.
    pub(crate) fn debit_balance_to_burn(&mut self, amount: TokensAmount) {
        self.balance_to_burn = self.balance_to_burn.saturating_sub(amount);
    }
}
