#![cfg(test)]

use claim_model::api::{BurnApi, ClaimApi, RecordApi};
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

    context.set_block_timestamp_in_seconds(contract.burn_period as u64 + 100);

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

    context.set_block_timestamp_in_seconds(contract.burn_period as u64 + 100);

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
