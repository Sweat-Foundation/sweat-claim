#![cfg(test)]

use claim_model::{
    api::{BurnApi, ClaimApi, RecordApi},
    UnixTimestamp,
};
use near_sdk::{json_types::U128, PromiseOrValue};

use crate::{
    burn::api::test::EXT_BURN_FUTURE,
    common::tests::{data::set_test_future_success, Context},
};

#[test]
fn test_burn_when_outdated_tokens_exist() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_BURN_FUTURE, true);

    let alice_balance = 100_000;
    let bob_balance = 200_000;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![
        (accounts.alice.clone(), U128(alice_balance)),
        (accounts.bob.clone(), U128(bob_balance)),
    ]);

    context.set_block_timestamp_in_seconds(2 * contract.burn_period as u64 + 100);

    context.switch_account(&accounts.alice);
    contract.claim();

    context.switch_account(&accounts.bob);
    contract.claim();

    context.switch_account(&accounts.oracle);
    let burn_result = contract.burn();
    let burnt_amount = match burn_result {
        PromiseOrValue::Promise(_) => panic!("Expected value"),
        PromiseOrValue::Value(value) => value.0,
    };

    assert_eq!(alice_balance + bob_balance, burnt_amount);
    assert_eq!(0, contract.balance_to_burn);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice).0;
    assert_eq!(0, alice_new_balance);

    let bob_new_balance = contract.get_claimable_balance_for_account(accounts.bob).0;
    assert_eq!(0, bob_new_balance);

    assert!(!contract.is_service_call_running);
}

#[test]
fn test_ext_error_on_burn_when_outdated_tokens_exist() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_BURN_FUTURE, false);

    let alice_balance = 100_000;
    let bob_balance = 200_000;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![
        (accounts.alice.clone(), U128(alice_balance)),
        (accounts.bob.clone(), U128(bob_balance)),
    ]);

    context.set_block_timestamp_in_seconds(2 * contract.burn_period as u64 + 100);

    context.switch_account(&accounts.alice);
    contract.claim();

    context.switch_account(&accounts.bob);
    contract.claim();

    context.switch_account(&accounts.oracle);
    let burn_result = contract.burn();
    let burnt_amount = match burn_result {
        PromiseOrValue::Promise(_) => panic!("Expected value"),
        PromiseOrValue::Value(value) => value.0,
    };

    assert_eq!(0, burnt_amount);

    assert_eq!(alice_balance + bob_balance, contract.balance_to_burn);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice).0;
    assert_eq!(0, alice_new_balance);

    let bob_new_balance = contract.get_claimable_balance_for_account(accounts.bob).0;
    assert_eq!(0, bob_new_balance);

    assert!(!contract.is_service_call_running);
}

#[test]
fn test_burn_when_outdated_tokens_don_not_exist() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_BURN_FUTURE, true);

    context.switch_account(&accounts.oracle);
    let burn_result = contract.burn();
    let burnt_amount = match burn_result {
        PromiseOrValue::Promise(_) => panic!("Expected value"),
        PromiseOrValue::Value(value) => value.0,
    };

    assert_eq!(0, burnt_amount);

    assert!(!contract.is_service_call_running);
}

#[test]
fn test_burn_status_legacy() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);

    let mut current_timestamp = 360;

    let _top_up_timestamp_1 = current_timestamp;
    context.set_block_timestamp_in_seconds(current_timestamp);
    contract.record_batch_for_hold_legacy(vec![(alice_id.clone(), U128(1_000_000_000))]);

    current_timestamp += (contract.claim_period + 10) as u64;
    context.set_block_timestamp_in_seconds(current_timestamp);

    context.switch_account(&alice_id);
    let claim_timestamp = current_timestamp;
    // contract.claim();
    // mime legacy claim
    let account = contract.accounts_legacy.get_mut(&alice_id).unwrap();
    account.claim_period_refreshed_at = claim_timestamp as _;
    account.accruals.clear();
    // ---

    context.switch_account(&accounts.oracle);

    current_timestamp += 3600;
    let top_up_timestamp_2 = current_timestamp;
    context.set_block_timestamp_in_seconds(top_up_timestamp_2);
    contract.record_batch_for_hold_legacy(vec![(alice_id.clone(), U128(500_000_000))]);

    let burn_status = contract.get_burn_status(alice_id.clone());
    assert_eq!(Some(top_up_timestamp_2 as UnixTimestamp), burn_status.min_claimable_ts);
    assert_eq!(claim_timestamp as UnixTimestamp, burn_status.claim_period_refreshed_at);
    assert_eq!(contract.burn_period, burn_status.burn_period);
}

#[test]
fn test_burn_status() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);

    let mut current_timestamp = 360;

    let _top_up_timestamp_1 = current_timestamp;
    context.set_block_timestamp_in_seconds(current_timestamp);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);

    current_timestamp += (contract.claim_period + 10) as u64;
    context.set_block_timestamp_in_seconds(current_timestamp);

    context.switch_account(&alice_id);
    let claim_timestamp = current_timestamp;
    contract.claim();

    context.switch_account(&accounts.oracle);

    current_timestamp += 3600;
    let top_up_timestamp_2 = current_timestamp;
    context.set_block_timestamp_in_seconds(top_up_timestamp_2);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(500_000_000))]);

    let burn_status = contract.get_burn_status(alice_id.clone());
    assert_eq!(None, burn_status.min_claimable_ts);
    assert_eq!(claim_timestamp as UnixTimestamp, burn_status.claim_period_refreshed_at);
    assert_eq!(contract.burn_period, burn_status.burn_period);
}
