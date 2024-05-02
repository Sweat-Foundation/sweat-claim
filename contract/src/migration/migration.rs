#![allow(deprecated)]

use std::cmp::max;

use claim_model::{
    account_record::{AccountRecordLegacy, AccountRecordVersioned},
    api::MigrationApi,
    Duration, TokensAmount, UnixTimestamp, UnixTimestampExtension,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    store::{LookupMap, UnorderedMap, UnorderedSet, Vector},
    AccountId,
};

use crate::{common::now_seconds, Contract, ContractExt, StorageKey};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct OldState {
    token_account_id: AccountId,
    oracles: UnorderedSet<AccountId>,
    claim_period: Duration,
    burn_period: Duration,
    accruals: UnorderedMap<UnixTimestamp, (Vector<TokensAmount>, TokensAmount)>,
    accounts: LookupMap<AccountId, AccountRecordLegacy>,
    is_service_call_running: bool,
}

#[near_bindgen]
impl MigrationApi for Contract {
    #[private]
    #[init(ignore_state)]
    fn migrate() -> Self {
        let old_state: OldState = env::state_read().expect("Failed to read old state");

        Self {
            token_account_id: old_state.token_account_id,
            oracles: old_state.oracles,
            claim_period: old_state.claim_period,
            burn_period: old_state.burn_period,
            accruals: old_state.accruals,
            accounts_legacy: old_state.accounts,
            accounts: LookupMap::new(StorageKey::Accounts),
            is_service_call_running: old_state.is_service_call_running,
            balance_to_burn: 0,
        }
    }

    fn migrate_accounts(&mut self, accounts: Vec<AccountId>) {
        self.assert_oracle();

        for account_id in accounts {
            self.migrate_account_if_outdated(&account_id);
        }
    }

    fn cleanup(&mut self, keys: Vec<UnixTimestamp>) {
        self.assert_oracle();

        for key in keys {
            if let Some(accruals_entry) = self.accruals.get_mut(&key) {
                accruals_entry.0.clear();
                self.accruals.remove(&key);
            }
        }
    }
}

impl Contract {
    pub(crate) fn migrate_account_if_outdated(&mut self, account_id: &AccountId) {
        let Some(account) = self.accounts_legacy.remove(account_id) else {
            return;
        };

        let now = now_seconds();
        let mut account_balance: TokensAmount = 0;
        let mut balance_to_burn: TokensAmount = 0;

        for (timestamp, accrual_index) in &account.accruals {
            let amount = self
                .accruals
                .get(timestamp)
                .map(|(accruals, _)| accruals.get(*accrual_index).copied().unwrap_or(0))
                .unwrap_or_default();

            if timestamp.is_within_period(now, self.burn_period) {
                account_balance += amount;
            } else {
                balance_to_burn += amount;
            }
        }

        let burn_since = max(account.claim_period_refreshed_at, self.get_claimable_window_start());
        let account = AccountRecordVersioned::from_legacy(&account, account_balance, burn_since);

        self.accounts.insert(account_id.clone(), account);
        self.balance_to_burn += balance_to_burn;
    }
}
