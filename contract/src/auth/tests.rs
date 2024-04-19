#![cfg(test)]

use claim_model::api::{AuthApi, RecordApi};
use near_sdk::json_types::U128;

use crate::common::{tests::Context, AccountAccessor};

#[test]
fn add_oracle_by_contract_owner() {
    let (mut context, mut contract, accounts) = Context::init();
    context.switch_account(&accounts.owner);
    contract.add_oracle(accounts.oracle.clone());

    let oracles = contract.get_oracles();
    assert_eq!(oracles, vec![accounts.oracle.clone()]);
}

#[test]
#[should_panic(expected = "Method is private")]
fn add_oracle_not_by_contract_owner() {
    let (mut context, mut contract, accounts) = Context::init();

    context.switch_account(&accounts.alice);
    contract.add_oracle(accounts.oracle.clone());
}

#[test]
#[should_panic(expected = "Already exists")]
fn add_oracle_twice() {
    let (mut context, mut contract, accounts) = Context::init();

    context.switch_account(&accounts.owner);
    contract.add_oracle(accounts.oracle.clone());
    contract.add_oracle(accounts.oracle.clone());
}

#[test]
fn remove_oracle_by_contract_owner() {
    let (mut context, mut contract, accounts) = Context::init();

    context.switch_account(&accounts.owner);
    contract.add_oracle(accounts.oracle.clone());

    let oracles = contract.get_oracles();
    assert_eq!(oracles, vec![accounts.oracle.clone()]);

    contract.remove_oracle(accounts.oracle.clone());

    let oracles = contract.get_oracles();
    assert!(oracles.is_empty());
}

#[test]
#[should_panic(expected = "Method is private")]
fn remove_oracle_not_by_contract_owner() {
    let (mut context, mut contract, accounts) = Context::init();

    contract.oracles.insert(accounts.oracle.clone());

    context.switch_account(&accounts.alice);
    contract.remove_oracle(accounts.oracle.clone());
}

#[test]
#[should_panic(expected = "No such oracle")]
fn remove_not_existing_oracle() {
    let (mut context, mut contract, accounts) = Context::init();

    context.switch_account(&accounts.owner);
    contract.remove_oracle(accounts.oracle.clone());
}

#[test]
fn unlock_account_by_oracle_legacy() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold_legacy(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts_legacy.get_mut(&alice_id).unwrap().is_locked = true;

    contract.unlock_account(alice_id.clone());

    assert!(!contract.accounts_legacy.get(&alice_id).unwrap().is_locked);
}

#[test]
#[should_panic(expected = "Only oracle can do this")]
fn unlock_account_not_by_oracle_legacy() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&alice_id);
    contract.record_batch_for_hold_legacy(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts_legacy.get_mut(&alice_id).unwrap().is_locked = true;

    contract.unlock_account(alice_id.clone());
}

#[test]
fn unlock_account_by_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts.get_account_mut(&alice_id).is_locked = true;

    contract.unlock_account(alice_id.clone());

    assert!(!contract.accounts.get_account(&alice_id).is_locked);
}

#[test]
#[should_panic(expected = "Only oracle can do this")]
fn unlock_account_not_by_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&alice_id);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts.get_account_mut(&alice_id).is_locked = true;

    contract.unlock_account(alice_id.clone());
}
