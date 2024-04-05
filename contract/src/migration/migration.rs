#![allow(deprecated)]

use claim_model::{
    account_record::{AccountRecordLegacy, AccountRecordVersioned},
    Duration, TokensAmount, UnixTimestamp,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    store::{LookupMap, UnorderedMap, UnorderedSet, Vector},
    AccountId, PanicOnDefault,
};

use crate::{Contract, ContractExt, StorageKey};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
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
impl Contract {
    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
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

    pub fn migrate_accounts(&mut self, accounts: Vec<AccountId>) {
        self.assert_oracle();

        for account_id in accounts {
            self.migrate_account_if_outdated(&account_id);
        }
    }
}

impl Contract {
    pub(crate) fn migrate_account_if_outdated(&mut self, account_id: &AccountId) {
        let Some(account) = self.accounts_legacy.remove(account_id) else {
            return;
        };

        let last_top_up_at = account
            .accruals
            .iter()
            .map(|(datetime, _)| datetime)
            .max()
            .copied()
            .unwrap_or_default();

        let balance = account
            .accruals
            .iter()
            .copied()
            .map(|(datetime, index)| {
                self.accruals
                    .get(&datetime)
                    .map(|(accruals, _)| accruals.get(index).copied().unwrap_or(0))
                    .unwrap_or_default()
            })
            .sum();

        let account = AccountRecordVersioned::from_legacy(&account, balance, last_top_up_at);
        self.accounts.insert(account_id.clone(), account);
    }
}
