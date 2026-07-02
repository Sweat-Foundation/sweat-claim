#![cfg(test)]

use claim_model::api::RecordApi;
use near_sdk::json_types::U128;

use crate::{
    clean::api::CleanApi,
    common::{tests::Context, MAX_CLEAN_BATCH_SIZE},
};

#[test]
fn test_clean_single_account_by_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    context.switch_account(&accounts.oracle);

    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(100_000_000))]);

    let record = contract.accounts.get(&accounts.alice).unwrap().into_latest();
    assert_ne!(0, record.balance);

    contract.clean(vec![accounts.alice.clone()]);

    let record = contract.accounts.get(&accounts.alice);
    assert!(record.is_none());
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn test_clean_single_account_by_not_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    context.switch_account(&accounts.oracle);

    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(100_000_000))]);

    let record = contract.accounts.get(&accounts.alice).unwrap().into_latest();
    assert_ne!(0, record.balance);

    context.switch_account(&accounts.alice);
    contract.clean(vec![accounts.alice.clone()]);
}

#[test]
fn test_clean_multiple_accounts_by_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    context.switch_account(&accounts.oracle);

    contract.record_batch_for_hold(vec![
        (accounts.alice.clone(), U128(100_000_000)),
        (accounts.bob.clone(), U128(1_000_000_000)),
    ]);

    let alice_record = contract.accounts.get(&accounts.alice).unwrap().into_latest();
    assert_ne!(0, alice_record.balance);

    let bob_record = contract.accounts.get(&accounts.bob).unwrap().into_latest();
    assert_ne!(0, bob_record.balance);

    contract.clean(vec![accounts.alice.clone(), accounts.bob.clone()]);

    let alice_record = contract.accounts.get(&accounts.alice);
    assert!(alice_record.is_none());

    let bob_record = contract.accounts.get(&accounts.bob);
    assert!(bob_record.is_none());
}

#[test]
#[should_panic(expected = "Batch size exceeds the maximum allowed")]
fn clean_rejects_a_batch_over_the_max_size() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    context.switch_account(&accounts.oracle);

    let account_ids: Vec<_> = (0..=MAX_CLEAN_BATCH_SIZE).map(|i| format!("account{i}").parse().unwrap()).collect();
    contract.clean(account_ids);
}
