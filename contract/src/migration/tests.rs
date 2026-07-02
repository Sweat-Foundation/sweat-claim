#![cfg(test)]

use claim_model::TokensAmount;
use near_plugins::AccessControllable;
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
    write_old_state_with_oracles(balance_to_burn, vec![]);
}

fn write_old_state_with_oracles(balance_to_burn: TokensAmount, oracles: Vec<AccountId>) {
    let mut oracles_set = UnorderedSet::new(StorageKey::_OraclesLegacy);
    for oracle in oracles {
        oracles_set.insert(oracle);
    }

    let old_state = OldState {
        token_account_id: "token".parse().unwrap(),
        oracles: oracles_set,
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

    let contract = Contract::migrate(vec![], vec![], vec![], vec![]);

    assert_eq!(500, contract.balance_to_burn);
}

#[test]
#[should_panic(expected = "balance_to_burn is less than the 50M SWEAT already withdrawn")]
fn migrate_panics_if_balance_to_burn_is_insufficient() {
    init_context();
    write_old_state(WITHDRAWN_SWEAT - 1);

    Contract::migrate(vec![], vec![], vec![], vec![]);
}

#[test]
fn migrate_grants_oracle_role_unconditionally_and_other_roles_only_when_specified() {
    init_context();

    let oracle: AccountId = "oracle".parse().unwrap();
    let carol: AccountId = "carol".parse().unwrap();

    write_old_state_with_oracles(WITHDRAWN_SWEAT, vec![oracle.clone()]);

    let contract = Contract::migrate(vec![], vec![carol.clone()], vec![], vec![]);

    assert_eq!(
        vec![oracle],
        contract.acl_get_grantees("Oracle".to_string(), 0, 10),
        "Oracle role must be granted unconditionally to every former oracle"
    );
    assert!(
        contract.acl_get_grantees("BurnManager".to_string(), 0, 10).is_empty(),
        "BurnManager must not be granted automatically; it wasn't passed as an argument"
    );
    assert_eq!(
        vec![carol],
        contract.acl_get_grantees("Maintainer".to_string(), 0, 10),
        "Maintainer must be granted to accounts passed via the maintainers argument"
    );
    assert!(
        contract.acl_get_grantees("StagingManager".to_string(), 0, 10).is_empty()
    );
    assert!(
        contract.acl_get_grantees("UpgradeManager".to_string(), 0, 10).is_empty()
    );
}
