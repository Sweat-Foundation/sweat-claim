#![cfg(test)]

use std::env;

use claim_model::{
    api::{ClaimApi, ConfigApi, RecordApi},
    ClaimAvailabilityView, TokensAmount, UnixTimestamp,
};
use near_sdk::{json_types::U128, PromiseOrValue};
use plotters::prelude::*;

use crate::{
    claim::api::test::EXT_TRANSFER_FUTURE,
    common::tests::{data::set_test_future_success, Context},
};

#[test]
fn test_check_claim_availability_when_user_is_not_registered() {
    let (_, contract, accounts) = Context::init_with_oracle();

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(0, alice_new_balance);

    let alice_can_claim = contract.is_claim_available(accounts.alice);
    assert_eq!(ClaimAvailabilityView::Unregistered, alice_can_claim);
}

#[test]
fn test_check_claim_availability_when_user_has_tokens_and_claim_period_after_claim_is_not_passed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance = 400_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(alice_balance, alice_new_balance);

    let claim_timestamp = contract.claim_period as u64 + 100;
    context.set_block_timestamp_in_seconds(claim_timestamp);
    context.switch_account(&accounts.alice);
    contract.claim();

    let check_timestamp = claim_timestamp + 10;
    context.set_block_timestamp_in_seconds(check_timestamp);

    let alice_can_claim = contract.is_claim_available(accounts.alice.clone());
    assert_eq!(
        alice_can_claim,
        ClaimAvailabilityView::Unavailable((claim_timestamp as UnixTimestamp, contract.claim_period))
    );
}

#[test]
fn test_check_claim_availability_when_user_has_tokens_and_claim_period_after_claim_is_passed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance = 300_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(alice_balance, alice_new_balance);

    let claim_timestamp = contract.claim_period as u64 + 100;
    context.set_block_timestamp_in_seconds(claim_timestamp);
    context.switch_account(&accounts.alice);
    contract.claim();

    let check_timestamp = claim_timestamp + contract.claim_period as u64 + 100;
    context.set_block_timestamp_in_seconds(check_timestamp);

    let alice_can_claim = contract.is_claim_available(accounts.alice.clone());
    assert_eq!(alice_can_claim, ClaimAvailabilityView::Available(0));
}

#[test]
fn test_check_claim_availability_when_user_has_tokens_and_claim_period_after_record_creation_is_not_passed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance = 400_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(alice_balance, alice_new_balance);

    let alice_can_claim = contract.is_claim_available(accounts.alice.clone());
    assert_eq!(
        alice_can_claim,
        ClaimAvailabilityView::Unavailable((0, contract.claim_period))
    );
}

#[test]
fn test_check_claim_availability_when_user_has_tokens_and_claim_period_after_record_creation_is_passed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance = 300_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    context.set_block_timestamp_in_seconds(contract.claim_period as u64 + 100);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(alice_balance, alice_new_balance);

    let alice_can_claim = contract.is_claim_available(accounts.alice.clone());
    assert_eq!(alice_can_claim, ClaimAvailabilityView::Available(0));
}

#[test]
fn test_check_claim_availability_when_user_has_multiple_claim_records_and_claim_period_after_record_creation_is_passed()
{
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let record_count: u16 = 5;
    let alice_balance = 300_000;
    context.switch_account(&accounts.oracle);

    for i in 0..record_count {
        contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);
        context.set_block_timestamp_in_seconds(i as _);
    }

    context.set_block_timestamp_in_seconds(contract.claim_period as u64 + 100);

    let alice_can_claim = contract.is_claim_available(accounts.alice.clone());
    assert_eq!(alice_can_claim, ClaimAvailabilityView::Available(0));
}

#[test]
#[should_panic(expected = "Claim is not available at the moment")]
fn test_claim_when_user_is_not_registered() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_TRANSFER_FUTURE, true);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(0, alice_new_balance);

    context.switch_account(&accounts.alice);
    contract.claim();
}

#[test]
#[should_panic(expected = "Claim is not available at the moment")]
fn test_claim_when_user_has_tokens_and_claim_period_is_not_passed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_TRANSFER_FUTURE, true);

    let alice_balance = 200_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    context.switch_account(&accounts.alice);
    contract.claim();
}

#[test]
fn test_claim_when_user_has_tokens_and_current_time_matches_claim_period() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance = 500_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    context.set_block_timestamp_in_seconds(2 * contract.burn_period as u64);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(0, alice_new_balance);

    let alice_can_claim = contract.is_claim_available(accounts.alice.clone());
    assert_eq!(alice_can_claim, ClaimAvailabilityView::Available(0));
}

#[test]
fn test_claim_when_user_has_tokens_and_claim_period_is_passed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_TRANSFER_FUTURE, true);

    let alice_balance = 700_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    context.set_block_timestamp_in_seconds(contract.claim_period as u64 + 100);

    context.switch_account(&accounts.alice);
    let claimed_amount = match contract.claim() {
        PromiseOrValue::Promise(_) => panic!("Expected value"),
        PromiseOrValue::Value(value) => value,
    };
    assert_eq!(alice_balance, claimed_amount.total.0);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(0, alice_new_balance);
}

#[test]
fn test_claim_when_user_has_tokens_and_burn_period_is_passed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_TRANSFER_FUTURE, true);

    let alice_balance = 12_000_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    context.set_block_timestamp_in_seconds(2 * contract.burn_period as u64 + 100);

    context.switch_account(&accounts.alice);
    let claimed_amount = match contract.claim() {
        PromiseOrValue::Promise(_) => panic!("Expected value"),
        PromiseOrValue::Value(value) => value,
    };
    assert_eq!(0, claimed_amount.total.0);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(0, alice_new_balance);
}

#[test]
fn test_claim_when_user_has_tokens_and_claim_period_is_passed_and_transfer_failed() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    set_test_future_success(EXT_TRANSFER_FUTURE, false);

    let alice_balance = 123_100_000;
    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    context.set_block_timestamp_in_seconds(contract.claim_period as u64 + 100);

    context.switch_account(&accounts.alice);
    let claimed_amount = match contract.claim() {
        PromiseOrValue::Promise(_) => panic!("Expected value"),
        PromiseOrValue::Value(value) => value,
    };
    assert_eq!(0, claimed_amount.total.0);

    let alice_new_balance = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
    assert_eq!(alice_balance, alice_new_balance);
}

#[test]
fn demo_burn() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance = 1_000_000;
    context.switch_account(&accounts.oracle);
    contract.set_burn_period(400_000);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    let mut data: Vec<(UnixTimestamp, TokensAmount)> = vec![];
    let mut current_time: u64 = 0;

    context.switch_account(&accounts.alice);
    while current_time < (1.8 * contract.burn_period as f64) as u64 {
        context.set_block_timestamp_in_seconds(current_time);

        let available_for_claim = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        data.push((current_time as _, available_for_claim as _));

        current_time += 3600;
    }

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(2_000_000))]);

    context.switch_account(&accounts.alice);
    while current_time < (2 * contract.burn_period) as u64 {
        context.set_block_timestamp_in_seconds(current_time);

        let available_for_claim = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        data.push((current_time as _, available_for_claim as _));

        current_time += 3600;
    }

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(1_500_000))]);

    context.switch_account(&accounts.alice);
    while current_time < (4 * contract.burn_period) as u64 {
        context.set_block_timestamp_in_seconds(current_time);

        let available_for_claim = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        data.push((current_time as _, available_for_claim as _));

        current_time += 3600;
    }

    render_chart("SWEAT evaporating", data, "evaporating.png").unwrap()
}

#[test]
fn demo_bur_with_claim() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let alice_balance = 1_000_000;
    context.switch_account(&accounts.oracle);
    contract.set_burn_period(400_000);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(alice_balance))]);

    let mut data: Vec<(UnixTimestamp, TokensAmount)> = vec![];
    let mut current_time: u64 = 0;

    context.switch_account(&accounts.alice);
    while current_time < (1.8 * contract.burn_period as f64) as u64 {
        context.set_block_timestamp_in_seconds(current_time);

        let available_for_claim = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        data.push((current_time as _, available_for_claim as _));

        current_time += 3600;
    }

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(300_000))]);

    context.switch_account(&accounts.alice);
    while current_time < (2.2 * contract.burn_period as f64) as u64 {
        context.set_block_timestamp_in_seconds(current_time);

        let available_for_claim = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        data.push((current_time as _, available_for_claim as _));

        current_time += 3600;
    }

    contract.claim();

    while current_time < (2.5 * contract.burn_period as f64) as u64 {
        context.set_block_timestamp_in_seconds(current_time);

        let available_for_claim = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        data.push((current_time as _, available_for_claim as _));

        current_time += 3600;
    }

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(accounts.alice.clone(), U128(500_000))]);

    while current_time < (5 * contract.burn_period) as u64 {
        context.set_block_timestamp_in_seconds(current_time);

        let available_for_claim = contract.get_claimable_balance_for_account(accounts.alice.clone()).0;
        data.push((current_time as _, available_for_claim as _));

        current_time += 3600;
    }

    render_chart("SWEAT evaporating", data, "evaporating_with_claim.png").unwrap()
}

fn render_chart(
    name: &str,
    data: Vec<(UnixTimestamp, TokensAmount)>,
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_file_path = format!("{}/{file_name}", env::current_dir().unwrap().display());

    let root = BitMapBackend::new(output_file_path.as_str(), (1024, 768)).into_drawing_area();

    root.fill(&WHITE)?;

    let min_x: UnixTimestamp = data.iter().map(|(x, _)| *x).min().unwrap();
    let max_x: UnixTimestamp = data.iter().map(|(x, _)| *x).max().unwrap();

    let min_y: TokensAmount = data.iter().map(|(_, y)| *y).min().unwrap();
    let max_y: TokensAmount = data.iter().map(|(_, y)| *y).max().unwrap();

    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 60)
        .caption(name, ("sans-serif", 40))
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart
        .configure_mesh()
        .x_desc("Time (in seconds)")
        .y_desc("$SWEAT")
        .draw()?;

    let series_data = data.iter().map(|(x, y)| (*x as u32, *y as u128));
    let series = AreaSeries::new(series_data, 0, BLUE.mix(0.3));
    chart.draw_series(series)?;

    root.present().expect("Unable to write result to file");
    Ok(())
}
