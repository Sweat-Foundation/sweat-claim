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
        let mut old_state: OldState = env::state_read().expect("Failed to read old state");

        let mut contract = Self {
            token_account_id: old_state.token_account_id,
            claim_period: old_state.claim_period,
            burn_period: old_state.burn_period,
            accounts: old_state.accounts,
            is_service_call_running: old_state.is_service_call_running,
            balance_to_burn: old_state.balance_to_burn,
        };

        // The new Contract has no `accruals`/`accounts_legacy` fields — both are
        // dead weight since the linear-burn rewrite, long before this ACL
        // migration. Unlike the (small, bounded) oracles admin set, `accruals`
        // accumulated one entry per record_batch_for_hold timestamp bucket over
        // the contract's entire pre-linear-burn operational history — it could
        // hold far more entries than a single transaction's gas budget can
        // iterate. Deliberately NOT calling old_state.accruals.clear() here to
        // avoid an OOG mid-migration; a safe reclaim needs a separate, paginated
        // cleanup path callable across multiple transactions. `accounts_legacy`
        // has no clear()/iteration capability at all (LookupMap can't enumerate
        // its own keys), so it's in the same "left alone" boat regardless.
        // Both maps are simply dropped here, leaving any existing entries exactly
        // as unreachable as they already were before this migration.

        contract.acl_init_super_admin(env::current_account_id());

        // One ACL storage handle, reused for every grant below, instead of a fresh
        // read+deserialize of the ACL root per account.
        let mut acl = contract.acl_get_or_init();

        // Oracle is the only role carried over unconditionally, mirroring the pre-ACL
        // `oracles` set. All other roles must be explicitly assigned by the caller.
        for oracle in old_state.oracles.iter() {
            acl.grant_role_unchecked(Roles::Oracle, oracle);
        }
        // The new Contract has no `oracles` field, so this set's storage would
        // otherwise be permanently orphaned once this state blob is overwritten.
        old_state.oracles.clear();

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
