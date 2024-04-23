#![cfg(test)]

use claim_model::{
    api::{ClaimApi, ConfigApi},
    UnixTimestamp,
};
use near_sdk::json_types::U128;

use crate::common::tests::Context;

#[test]
fn test_migration_with_burnt_balance_and_evaporating_sweat() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice = &accounts.alice.clone();

    context.switch_account(&accounts.oracle);

    let burn_period = 864_000; // 10 days in seconds
    contract.set_burn_period(burn_period);

    let mut current_timestamp = 0;

    // Record first top-up – 1 $SWEAT
    let alice_balance_1 = U128(10u128.pow(18)); // 1 $SWEAT
    contract.record_batch_for_hold_legacy(vec![(alice.clone(), alice_balance_1)]);

    // Time travel to some point in the future where alice_balance_1 is still not evaporated.
    current_timestamp += 345_600; // + 4 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 4

    // Record second top-up – 2 $SWEAT
    let alice_balance_2 = U128(2 * 10u128.pow(18)); // 2 $SWEAT
    contract.record_batch_for_hold_legacy(vec![(alice.clone(), alice_balance_2)]);

    // Time travel to some point in the future where alice_balance_1 evaporates
    // Now is 4 days from the start, so add not lees than 6 days.
    current_timestamp += 691_200; // + 8 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 12

    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    assert_eq!(alice_balance_2, alice_current_balance);

    // Migrate the Alice account
    let alice_account_outdated = contract.accounts_legacy.get(&alice).cloned().unwrap();
    contract.migrate_accounts(vec![alice.clone()]);

    // Check that the account record moved from legacy collection to the new one
    assert!(contract.accounts_legacy.get(alice).is_none());
    assert!(contract.accounts.get(alice).is_some());

    // Check that the balance is still the same
    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    assert_eq!(alice_balance_2, alice_current_balance);

    let alice_account = contract.accounts.get(alice).unwrap().into_latest();
    assert_eq!(
        (current_timestamp - burn_period as u64) as UnixTimestamp,
        alice_account.burn_since
    );
    assert_eq!(
        alice_account_outdated.claim_period_refreshed_at,
        alice_account.claim_period_refreshed_at
    );
    assert_eq!(alice_balance_2.0, alice_account.balance);
    assert!(alice_account.is_enabled);
    assert!(!alice_account.is_locked);

    // Time travel to some point in the future where alice_balance_2 evaporates.
    // As Alice hasn't claimed anything yet, it should start evaporating right after migration.
    current_timestamp += 172_800; // + 2 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 14

    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    // Expect balance minus fee for 2 days of evaporating
    let expected_balance = 1_599_999_999_999_968_000;
    assert_eq!(expected_balance, alice_current_balance.0);
}

#[test]
fn test_migration_with_burnt_sweat_and_not_evaporating_sweat() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice = &accounts.alice.clone();

    context.switch_account(&accounts.oracle);

    let burn_period = 864_000; // 10 days in seconds
    contract.set_burn_period(burn_period);

    let mut current_timestamp = 0;

    // Record first top-up – 1 $SWEAT
    let alice_balance_1 = U128(10u128.pow(18)); // 1 $SWEAT
    contract.record_batch_for_hold_legacy(vec![(alice.clone(), alice_balance_1)]);

    // Time travel to some point in the future where alice_balance_1 is still not evaporated.
    current_timestamp += 172_800; // + 2 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 2

    // Record second top-up – 4 $SWEAT
    let alice_balance_2 = U128(4 * 10u128.pow(18)); // 4 $SWEAT
    contract.record_batch_for_hold_legacy(vec![(alice.clone(), alice_balance_2)]);

    // Time travel to some point in the future where alice_balance_1 evaporates
    // Now is 2 days from the start, so add not lees than 8 days.
    current_timestamp += 691_200; // + 8 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 10

    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    assert_eq!(alice_balance_2, alice_current_balance);

    // One day later Alice claims their tokens
    current_timestamp += 86_400; // + 1 day
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 11

    // mime legacy claim
    let account = contract.accounts_legacy.get_mut(&accounts.alice.clone()).unwrap();
    account.claim_period_refreshed_at = current_timestamp as UnixTimestamp;
    account.accruals.clear();
    // ---

    // Time travel to the near future
    current_timestamp += 86_400; // + 1 day
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 12

    // Record third top-up – 1 $SWEAT
    let alice_balance_3 = U128(10u128.pow(18)); // 1 $SWEAT
    contract.record_batch_for_hold_legacy(vec![(alice.clone(), alice_balance_3)]);

    // Two more day later a migration happens
    current_timestamp += 172_800; // + 2 day
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 14

    // Migrate the Alice account
    let alice_account_outdated = contract.accounts_legacy.get(&alice).cloned().unwrap();
    contract.migrate_accounts(vec![alice.clone()]);

    // Check that the account record moved from legacy collection to the new one
    assert!(contract.accounts_legacy.get(alice).is_none());
    assert!(contract.accounts.get(alice).is_some());

    // Check that the balance is still the same
    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    assert_eq!(alice_balance_3, alice_current_balance);

    let alice_account = contract.accounts.get(alice).unwrap().into_latest();
    assert_eq!(
        alice_account_outdated.claim_period_refreshed_at,
        alice_account.burn_since
    );
    assert_eq!(
        alice_account_outdated.claim_period_refreshed_at,
        alice_account.claim_period_refreshed_at
    );
    assert_eq!(alice_balance_3.0, alice_account.balance);
    assert!(alice_account.is_enabled);
    assert!(!alice_account.is_locked);

    // Time travel to some point in the future withing burn period.
    // As Alice has claimed their balance withing burn period, it shouldn't evaporate.
    current_timestamp += 86_400; // + 1 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 15

    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    // Expect balance without any fees, as Alice claimed right before migration
    let expected_balance = alice_balance_3.0;
    assert_eq!(expected_balance, alice_current_balance.0);
}

#[test]
fn test_migration_with_no_burnt_sweat() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice = &accounts.alice.clone();

    context.switch_account(&accounts.oracle);

    let burn_period = 864_000; // 10 days in seconds
    contract.set_burn_period(burn_period);

    let mut current_timestamp = 0;

    // Record first top-up – 3 $SWEAT
    let alice_balance_1 = U128(3 * 10u128.pow(18)); // 3 $SWEAT
    contract.record_batch_for_hold_legacy(vec![(alice.clone(), alice_balance_1)]);

    // Time travel to some point in the future where alice_balance_1 is still not evaporated.
    current_timestamp += 172_800; // + 2 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 2

    // Record second top-up – 2 $SWEAT
    let alice_balance_2 = U128(2 * 10u128.pow(18)); // 2 $SWEAT
    contract.record_batch_for_hold_legacy(vec![(alice.clone(), alice_balance_2)]);

    // Time travel to some point in the future where both top-ups still don't evaporate
    // Now is 2 days from the start, so add not more than 8 days.
    current_timestamp += 172_800; // + 2 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 4

    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    assert_eq!(alice_balance_1.0 + alice_balance_2.0, alice_current_balance.0);

    // Migrate the Alice account
    let alice_account_outdated = contract.accounts_legacy.get(&alice).cloned().unwrap();
    contract.migrate_accounts(vec![alice.clone()]);

    // Check that the account record moved from legacy collection to the new one
    assert!(contract.accounts_legacy.get(alice).is_none());
    assert!(contract.accounts.get(alice).is_some());

    // Check that the balance is still the same
    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    assert_eq!(alice_balance_1.0 + alice_balance_2.0, alice_current_balance.0);

    let alice_account = contract.accounts.get(alice).unwrap().into_latest();
    assert_eq!(
        alice_account_outdated.claim_period_refreshed_at,
        alice_account.burn_since
    );
    assert_eq!(
        alice_account_outdated.claim_period_refreshed_at,
        alice_account.claim_period_refreshed_at
    );
    assert_eq!(alice_balance_1.0 + alice_balance_2.0, alice_account.balance);
    assert!(alice_account.is_enabled);
    assert!(!alice_account.is_locked);

    // Time travel to some point in the future withing burn period.
    // As burn period hasn't passed, it shouldn't evaporate.
    current_timestamp += 86_400; // + 1 days
    context.set_block_timestamp_in_seconds(current_timestamp); // Now it's day 5

    let alice_current_balance = contract.get_claimable_balance_for_account(alice.clone());
    // Expect balance without any fees, as Alice claimed right before migration
    let expected_balance = alice_balance_1.0 + alice_balance_2.0;
    assert_eq!(expected_balance, alice_current_balance.0);
}
