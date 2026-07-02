#![cfg(test)]

use claim_model::{
    api::{ClaimApi, ConfigApi, RecordApi},
    ClaimableBalanceView,
};
use near_sdk::json_types::U128;

use crate::common::{tests::Context, AccountAccessor, MAX_BATCH_SIZE};

#[test]
fn record_by_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance_1 = 1_000_000;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance_1))]);

    let alice_actual_balance = contract.get_claimable_balance_for_account(accounts.alice.clone(), None);
    assert_eq!(alice_balance_1, alice_actual_balance.available_balance());

    context.set_block_timestamp_in_seconds(1_000);

    let alice_balance_2 = 500_000;
    let bob_balance = 200_000;

    contract.record_batch_for_hold(vec![
        (accounts.alice.clone(), U128(alice_balance_2)),
        (accounts.bob.clone(), U128(bob_balance)),
    ]);

    let alice_actual_balance = contract.get_claimable_balance_for_account(accounts.alice.clone(), None);
    assert_eq!(
        alice_balance_1 + alice_balance_2,
        alice_actual_balance.available_balance()
    );

    let bob_actual_balance = contract.get_claimable_balance_for_account(accounts.bob.clone(), None);
    assert_eq!(bob_balance, bob_actual_balance.available_balance());
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn record_by_not_oracle() {
    let (_context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance_1 = 1_000_000;
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance_1))]);
}

#[test]
fn record_batch_for_hold_advances_burn_since_even_when_nothing_crystallizes() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    context.switch_account(&accounts.oracle);
    contract.set_claim_period(10);
    contract.set_burn_period(1_000);

    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(1_000))]);

    context.set_block_timestamp_in_seconds(20);
    context.switch_account(&accounts.alice);
    let _ = contract.claim();

    // Advance far enough that claimable_window_start moves past the account's
    // now-stale burn_since (set by the claim above), while its balance is still 0 —
    // so record_batch_for_hold's crystallization branch has nothing to do.
    let later = 20 + 2_000;
    context.set_block_timestamp_in_seconds(later);

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(500))]);

    let expected_burn_since = later as u32 - 1_000;
    let actual_burn_since = contract.accounts.get_account(&accounts.alice).burn_since;
    assert_eq!(
        expected_burn_since, actual_burn_since,
        "burn_since must advance to claimable_window_start even when balance_to_burn is 0"
    );
}

#[test]
#[should_panic(expected = "Batch size exceeds the maximum allowed")]
fn record_batch_for_hold_rejects_a_batch_over_the_max_size() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    context.switch_account(&accounts.oracle);

    let amounts: Vec<_> = (0..=MAX_BATCH_SIZE)
        .map(|i| (format!("account{i}").parse().unwrap(), U128(1)))
        .collect();
    contract.record_batch_for_hold(amounts);
}
