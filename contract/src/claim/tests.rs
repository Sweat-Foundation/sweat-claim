#![cfg(test)]

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

mod demo {
    use std::env;

    use claim_model::{
        api::{ClaimApi, ConfigApi, RecordApi},
        Duration, TokensAmount, UnixTimestamp,
    };
    use near_sdk::{json_types::U128, AccountId};
    use plotters::{
        backend::BitMapBackend,
        chart::{ChartBuilder, LabelAreaPosition},
        prelude::{AreaSeries, BLUE, WHITE, *},
    };
    use rand::{rngs::ThreadRng, Rng};

    use crate::{common::tests::Context, Contract};

    #[test]
    fn demo_evaporating() {
        let (mut context, mut contract, accounts) = Context::init_with_oracle();

        let burn_period = 400_000;
        context.switch_account(&accounts.oracle);
        contract.set_burn_period(burn_period);

        let data = generate_data(
            &mut context,
            &mut contract,
            &accounts.alice,
            &accounts.oracle,
            vec![(0, Action::Record(10u128.pow(18)))],
            3600,
        );

        render_chart("$SWEAT evaporating", data, "evaporating.png").unwrap()
    }

    #[test]
    fn demo_continuous_evaporating() {
        let mut rng = rand::thread_rng();
        let (mut context, mut contract, accounts) = Context::init_with_oracle();

        let burn_period_days = 5;
        let burn_period = burn_period_days * 24 * 60 * 60;
        context.switch_account(&accounts.oracle);
        contract.set_burn_period(burn_period);

        let actions = (0..2 * burn_period)
            .step_by(8 * 60 * 60)
            .map(|timestamp| (timestamp, Action::Record(rng.gen_range(1u128..10u128) * 10u128.pow(17))))
            .collect();

        let data = generate_data(
            &mut context,
            &mut contract,
            &accounts.alice,
            &accounts.oracle,
            actions,
            3600,
        );

        render_chart("$SWEAT continuous evaporating", data, "evaporating_continuous.png").unwrap()
    }

    #[test]
    fn demo_evaporating_with_claim() {
        let rng = &mut rand::thread_rng();
        let (mut context, mut contract, accounts) = Context::init_with_oracle();

        let burn_period_days = 5;
        let burn_period = burn_period_days * 24 * 60 * 60;
        context.switch_account(&accounts.oracle);
        contract.set_burn_period(burn_period);

        let mut actions = generate_top_up_sequence(rng, 0, 2 * burn_period);
        actions.push(((2.5 * burn_period as f64) as UnixTimestamp, Action::Claim));
        actions.extend(generate_top_up_sequence(rng, 3 * burn_period, 4 * burn_period));

        let data = generate_data(
            &mut context,
            &mut contract,
            &accounts.alice,
            &accounts.oracle,
            actions,
            3600,
        );

        render_chart("$SWEAT evaporating with claim", data, "evaporating_with_claim.png").unwrap()
    }

    fn generate_top_up_sequence(
        rng: &mut ThreadRng,
        start_timestamp: UnixTimestamp,
        end_timestamp: UnixTimestamp,
    ) -> Vec<(UnixTimestamp, Action)> {
        (start_timestamp..end_timestamp)
            .step_by(8 * 60 * 60)
            .map(|timestamp| (timestamp, Action::Record(generate_token_amount(rng))))
            .collect()
    }

    fn generate_token_amount(rng: &mut ThreadRng) -> TokensAmount {
        rng.gen_range(1u128..10u128) * 10u128.pow(17)
    }

    fn generate_data(
        context: &mut Context,
        contract: &mut Contract,
        alice: &AccountId,
        oracle: &AccountId,
        actions: Vec<(UnixTimestamp, Action)>,
        step: Duration,
    ) -> Vec<(UnixTimestamp, TokensAmount)> {
        let mut actions = actions.to_vec();
        actions.sort_by(|(left, _), (right, _)| left.cmp(right));

        let mut result: Vec<(UnixTimestamp, TokensAmount)> = vec![];
        let mut current_time: UnixTimestamp = 0;

        context.switch_account(&alice);
        for (timestamp, action) in actions {
            result.extend(generate_balance_sequence(
                context,
                contract,
                alice,
                current_time,
                timestamp,
                step,
            ));

            match action {
                Action::Record(amount) => {
                    context.switch_account(&oracle);
                    contract.record_batch_for_hold(vec![(alice.clone(), U128(amount))]);
                    context.switch_account(&alice);
                }
                Action::Claim => {
                    contract.claim();
                }
            }

            current_time = timestamp;
        }

        let sequence_end_timestamp = current_time + 2 * contract.burn_period;
        result.extend(generate_balance_sequence(
            context,
            contract,
            alice,
            current_time,
            sequence_end_timestamp,
            step,
        ));

        result
    }

    fn generate_balance_sequence(
        context: &mut Context,
        contract: &mut Contract,
        alice: &AccountId,
        start_timestamp: UnixTimestamp,
        end_timestamp: UnixTimestamp,
        step: Duration,
    ) -> Vec<(UnixTimestamp, TokensAmount)> {
        let mut result = vec![];

        let mut current_time = start_timestamp;
        while current_time <= end_timestamp {
            context.set_block_timestamp_in_seconds(current_time as _);
            let available_for_claim = contract.get_claimable_balance_for_account(alice.clone()).0;

            result.push((current_time, available_for_claim as _));
            current_time += step;
        }

        result
    }

    fn render_chart(
        name: &str,
        data: Vec<(UnixTimestamp, TokensAmount)>,
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_dir = env::current_dir().unwrap();
        let root_dir = current_dir.parent().unwrap();
        let output_file_path = format!("{}/doc/{file_name}", root_dir.display());

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

    #[derive(Copy, Clone, Debug, PartialEq)]
    enum Action {
        Record(TokensAmount),
        Claim,
    }
}
