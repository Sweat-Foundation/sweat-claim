#![allow(deprecated)]

use claim_model::{
    account_record::{AccountRecordLegacy, AccountRecordVersioned},
    Duration, TokensAmount, UnixTimestamp,
};
use near_plugins::AccessControllable;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    store::{LookupMap, UnorderedMap, UnorderedSet, Vector},
    AccountId,
};

use crate::{auth::Roles, Contract, ContractExt};

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
struct OldState {
    token_account_id: AccountId,
    oracles: UnorderedSet<AccountId>,
    claim_period: Duration,
    burn_period: Duration,
    accruals: UnorderedMap<UnixTimestamp, (Vector<TokensAmount>, TokensAmount)>,
    accounts_legacy: LookupMap<AccountId, AccountRecordLegacy>,
    accounts: LookupMap<AccountId, AccountRecordVersioned>,
    is_service_call_running: bool,
    balance_to_burn: TokensAmount,
}

#[near_bindgen]
impl Contract {
    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let old_state: OldState = env::state_read().expect("Failed to read old state");

        let mut contract = Self {
            token_account_id: old_state.token_account_id,
            claim_period: old_state.claim_period,
            burn_period: old_state.burn_period,
            accruals: old_state.accruals,
            accounts_legacy: old_state.accounts_legacy,
            accounts: old_state.accounts,
            is_service_call_running: old_state.is_service_call_running,
            balance_to_burn: old_state.balance_to_burn,
        };

        contract.acl_init_super_admin(env::current_account_id());

        for oracle in old_state.oracles.iter() {
            contract.acl_get_or_init().grant_role_unchecked(Roles::Oracle, oracle);
            contract
                .acl_get_or_init()
                .grant_role_unchecked(Roles::BurnManager, oracle);
            contract
                .acl_get_or_init()
                .grant_role_unchecked(Roles::Maintainer, oracle);
        }

        contract
    }
}
