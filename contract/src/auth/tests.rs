#![cfg(test)]

use claim_model::api::{AuthApi, RecordApi};
use near_plugins::AccessControllable;
use near_sdk::json_types::U128;

use crate::common::{tests::Context, AccountAccessor};

#[test]
fn grant_oracle_role_by_super_admin() {
    let (mut context, mut contract, accounts) = Context::init();
    context.switch_account(&accounts.owner);

    let granted = contract.acl_grant_role("Oracle".to_string(), accounts.oracle.clone());
    assert_eq!(Some(true), granted);

    let grantees = contract.acl_get_grantees("Oracle".to_string(), 0, 10);
    assert_eq!(grantees, vec![accounts.oracle.clone()]);
}

#[test]
fn grant_oracle_role_by_non_admin_is_noop() {
    let (mut context, mut contract, accounts) = Context::init();
    context.switch_account(&accounts.alice);

    let granted = contract.acl_grant_role("Oracle".to_string(), accounts.oracle.clone());
    assert_eq!(None, granted);

    let grantees = contract.acl_get_grantees("Oracle".to_string(), 0, 10);
    assert!(grantees.is_empty());
}

#[test]
fn revoke_oracle_role_by_super_admin() {
    let (mut context, mut contract, accounts) = Context::init();
    context.switch_account(&accounts.owner);

    contract.acl_grant_role("Oracle".to_string(), accounts.oracle.clone());
    let revoked = contract.acl_revoke_role("Oracle".to_string(), accounts.oracle.clone());
    assert_eq!(Some(true), revoked);

    let grantees = contract.acl_get_grantees("Oracle".to_string(), 0, 10);
    assert!(grantees.is_empty());
}

#[test]
fn unlock_account_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts.get_or_insert_account_mut(&alice_id).is_locked = true;

    contract.unlock_account(alice_id.clone());

    assert!(!contract.accounts.get_account(&alice_id).is_locked);
}

#[test]
#[should_panic(expected = "Account not found")]
fn unlock_not_existing_account_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.unlock_account(alice_id.clone());
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn unlock_account_not_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts.get_or_insert_account_mut(&alice_id).is_locked = true;

    context.switch_account(&alice_id);
    contract.unlock_account(alice_id.clone());
}

#[test]
fn reset_service_call_flag_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    contract.is_service_call_running = true;

    context.switch_account(&accounts.oracle);
    contract.reset_service_call_flag();

    assert!(!contract.is_service_call_running);
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn reset_service_call_flag_not_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    contract.is_service_call_running = true;

    context.switch_account(&accounts.alice);
    contract.reset_service_call_flag();
}

#[test]
fn set_account_enabled_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);

    contract.set_account_enabled(alice_id.clone(), false);
    assert!(!contract.accounts.get_account(&alice_id).is_enabled);

    contract.set_account_enabled(alice_id.clone(), true);
    assert!(contract.accounts.get_account(&alice_id).is_enabled);
}

#[test]
#[should_panic(expected = "Account not found")]
fn set_account_enabled_for_not_existing_account_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.set_account_enabled(alice_id, false);
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn set_account_enabled_not_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);

    context.switch_account(&alice_id);
    contract.set_account_enabled(alice_id.clone(), false);
}
