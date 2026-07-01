#![cfg(test)]

use claim_model::TokensAmount;
use near_sdk::{
    env,
    store::{LookupMap, UnorderedMap, UnorderedSet},
    test_utils::VMContextBuilder,
    testing_env, AccountId,
};

use crate::{
    migration::{OldState, WITHDRAWN_SWEAT},
    Contract, StorageKey,
};

fn owner() -> AccountId {
    "owner".parse().unwrap()
}

fn init_context() {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(owner())
        .signer_account_id(owner())
        .predecessor_account_id(owner())
        .block_timestamp(0);
    testing_env!(builder.build());
}

fn write_old_state(balance_to_burn: TokensAmount) {
    let old_state = OldState {
        token_account_id: "token".parse().unwrap(),
        oracles: UnorderedSet::new(StorageKey::_OraclesLegacy),
        claim_period: 1,
        burn_period: 2,
        accruals: UnorderedMap::new(StorageKey::Accruals),
        accounts_legacy: LookupMap::new(StorageKey::AccountsLegacy),
        accounts: LookupMap::new(StorageKey::Accounts),
        is_service_call_running: false,
        balance_to_burn,
    };
    env::state_write(&old_state);
}

#[test]
fn migrate_decrements_balance_to_burn_by_withdrawn_sweat() {
    init_context();
    write_old_state(WITHDRAWN_SWEAT + 500);

    let contract = Contract::migrate();

    assert_eq!(500, contract.balance_to_burn);
}

#[test]
#[should_panic(expected = "balance_to_burn is less than the 50M SWEAT already withdrawn")]
fn migrate_panics_if_balance_to_burn_is_insufficient() {
    init_context();
    write_old_state(WITHDRAWN_SWEAT - 1);

    Contract::migrate();
}
