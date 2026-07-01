#![cfg(test)]

use claim_model::api::ConfigApi;

use crate::common::tests::Context;

#[test]
fn set_claim_period_by_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    contract.burn_period = 10_000_000;

    let claim_period = 1_000_000;
    context.switch_account(&accounts.oracle);
    contract.set_claim_period(claim_period);

    assert_eq!(claim_period, contract.claim_period);

    let claim_period = 3_000_000;
    context.switch_account(&accounts.oracle);
    contract.set_claim_period(claim_period);

    assert_eq!(claim_period, contract.claim_period);
}

#[test]
#[should_panic(expected = "Claim period should be less than burn period")]
fn set_invalid_claim_period() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    contract.burn_period = 10_000_000;

    context.switch_account(&accounts.oracle);
    contract.set_claim_period(20_000_000);
}

#[test]
#[should_panic(expected = "Burn period should be greater than claim period")]
fn set_invalid_burn_period() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    contract.claim_period = 5_000_000;

    context.switch_account(&accounts.oracle);
    contract.set_burn_period(1_000_000);
}

#[test]
#[should_panic(expected = "Burn period should be greater than 0")]
fn set_zero_burn_period() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    context.switch_account(&accounts.oracle);
    contract.set_burn_period(0);
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn set_claim_period_by_not_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let claim_period = 1_000_000;
    context.switch_account(&accounts.alice);
    contract.set_claim_period(claim_period);
}

#[test]
fn set_burn_period_by_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let burn_period = 1_000_000;
    context.switch_account(&accounts.oracle);
    contract.set_burn_period(burn_period);

    assert_eq!(burn_period, contract.burn_period);

    let burn_period = 3_000_000;
    context.switch_account(&accounts.oracle);
    contract.set_burn_period(burn_period);

    assert_eq!(burn_period, contract.burn_period);
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn set_burn_period_by_not_oracle() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();

    let burn_period = 1_000_000;
    context.switch_account(&accounts.alice);
    contract.set_burn_period(burn_period);
}
