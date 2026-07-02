#![allow(deprecated)]

use claim_model::{
    account_record::{AccountRecordLegacy, AccountRecordVersioned},
    Duration, TokensAmount, UnixTimestamp,
};
use near_plugins::AccessControllable;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, near_bindgen, require,
    store::{LookupMap, UnorderedMap, UnorderedSet, Vector},
    AccountId,
};

use crate::{auth::Roles, Contract, ContractExt};

mod tests;

pub(crate) const WITHDRAWN_SWEAT: TokensAmount = 50_000_000 * 10u128.pow(18);

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
    pub fn migrate(
        burn_managers: Vec<AccountId>,
        maintainers: Vec<AccountId>,
        staging_managers: Vec<AccountId>,
        upgrade_managers: Vec<AccountId>,
    ) -> Self {
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

        // One ACL storage handle, reused for every grant below, instead of a fresh
        // read+deserialize of the ACL root per account.
        let mut acl = contract.acl_get_or_init();

        // Oracle is the only role carried over unconditionally, mirroring the pre-ACL
        // `oracles` set. All other roles must be explicitly assigned by the caller.
        for oracle in old_state.oracles.iter() {
            acl.grant_role_unchecked(Roles::Oracle, oracle);
        }
        for account in &burn_managers {
            acl.grant_role_unchecked(Roles::BurnManager, account);
        }
        for account in &maintainers {
            acl.grant_role_unchecked(Roles::Maintainer, account);
        }
        for account in &staging_managers {
            acl.grant_role_unchecked(Roles::StagingManager, account);
        }
        for account in &upgrade_managers {
            acl.grant_role_unchecked(Roles::UpgradeManager, account);
        }

        require!(
            contract.balance_to_burn >= WITHDRAWN_SWEAT,
            "balance_to_burn is less than the 50M SWEAT already withdrawn"
        );
        contract.debit_balance_to_burn(WITHDRAWN_SWEAT);

        contract
    }
}
