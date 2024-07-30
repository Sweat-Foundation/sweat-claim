#![cfg(test)]

use anyhow::Result;
use claim_model::{
    api::{BurnApiIntegration, ClaimApiIntegration},
    ClaimAvailabilityView,
};
use near_gas::NearGas;
use near_sdk::{
    json_types::{U128, U64},
    serde_json::json,
};
use nitka::misc::ToNear;
use sweat_model::{FungibleTokenCoreIntegration, Payout, SweatApiIntegration, SweatContract, SweatDeferIntegration};

use crate::{
    common::PanicFinder,
    prepare::{prepare_contract, IntegrationContext},
};

mod common;
mod measure;
mod migration;
mod prepare;

#[tokio::test]
async fn happy_flow() -> Result<()> {
    let claim_period = 5 * 60;
    let burn_period = 20 * 60;
    let mut context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    let alice_steps = 10_000;
    let alice_initial_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?;

    let target_token_amount = context.ft_contract().formula(U64(0), alice_steps).await?.0;
    let target_payout = Payout::from(target_token_amount);

    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), alice_steps)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&manager)
        .await?;

    let claim_contract_balance = context
        .ft_contract()
        .ft_balance_of(context.sweat_claim().contract.as_account().to_near())
        .await?;

    assert_eq!(claim_contract_balance.0, target_payout.amount_for_user);

    let alice_deferred_balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;
    assert_eq!(alice_deferred_balance.0, target_payout.amount_for_user);

    let is_claim_available = context.sweat_claim().is_claim_available(alice.to_near()).await?;
    assert!(matches!(is_claim_available, ClaimAvailabilityView::Unavailable(_)));

    context.fast_forward_minutes((claim_period / 60) as u64).await?;

    let is_claim_available = context.sweat_claim().is_claim_available(alice.to_near()).await?;
    assert_eq!(is_claim_available, ClaimAvailabilityView::Available(0));

    context.sweat_claim().claim().with_user(&alice).await?;

    let alice_balance = context.ft_contract().ft_balance_of(alice.to_near()).await?;
    let alice_balance_change = alice_balance.0 - alice_initial_balance.0;
    assert_eq!(alice_balance_change, target_payout.amount_for_user);

    Ok(())
}

#[tokio::test]
async fn burn_total() -> Result<()> {
    let claim_period = 60;
    let burn_period = 2 * 60;
    let mut context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;

    let alice_steps = 10_000;

    let target_token_amount = context.ft_contract().formula(U64(0), alice_steps).await?.0;
    let target_payout = Payout::from(target_token_amount);

    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), alice_steps)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&manager)
        .await?;

    let claim_contract_balance = context
        .ft_contract()
        .ft_balance_of(context.sweat_claim().contract.as_account().to_near())
        .await?;

    assert_eq!(claim_contract_balance.0, target_payout.amount_for_user);

    let burn_result = context.sweat_claim().burn(None).with_user(&manager).await?;
    assert_eq!(0, burn_result.0);

    context.fast_forward_minutes((2 * burn_period) as u64).await?;

    context.sweat_claim().claim().with_user(&alice).await?;

    let burn_result = context.sweat_claim().burn(None).with_user(&manager).await?;
    assert_eq!(target_payout.amount_for_user, burn_result.0);

    let alice_deferred_balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;
    assert_eq!(0, alice_deferred_balance.0);

    let balance_to_burn = context.sweat_claim().get_balance_to_burn().await?;
    assert_eq!(0, balance_to_burn.0);

    Ok(())
}

#[tokio::test]
async fn burn_part() -> Result<()> {
    let claim_period = 0;
    let burn_period = 1;
    let mut context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let manager = context.manager().await?;
    let alice = context.alice().await?;

    let alice_steps = 100_000;

    let target_token_amount = context.ft_contract().formula(U64(0), alice_steps).await?.0;
    let target_payout = Payout::from(target_token_amount);

    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), alice_steps)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&manager)
        .await?;

    let claim_contract_balance = context
        .ft_contract()
        .ft_balance_of(context.sweat_claim().contract.as_account().to_near())
        .await?;

    assert_eq!(claim_contract_balance.0, target_payout.amount_for_user);

    let burn_result = context.sweat_claim().burn(None).with_user(&manager).await?;
    assert_eq!(0, burn_result.0);

    context.fast_forward_minutes((2 * burn_period) as u64).await?;

    context.sweat_claim().claim().with_user(&alice).await?;

    let target_amount_to_burn = 100;
    let burn_result = context
        .sweat_claim()
        .burn(Some(U128(target_amount_to_burn)))
        .with_user(&manager)
        .await?;
    assert_eq!(target_amount_to_burn, burn_result.0);

    let alice_deferred_balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;
    assert_eq!(0, alice_deferred_balance.0);

    let balance_to_burn = context.sweat_claim().get_balance_to_burn().await?;
    assert_eq!(target_payout.amount_for_user - target_amount_to_burn, balance_to_burn.0);

    Ok(())
}

#[tokio::test]
async fn on_burn_direct_call() -> Result<()> {
    let mut context = prepare_contract(None, None).await?;

    let alice = context.alice().await?;

    let result = alice
        .call(context.sweat_claim().contract.as_account().id(), "on_burn")
        .args_json(json!({
            "total_to_burn": "100000",
            "keys_to_remove": vec![1702303000, 1702304333],
        }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Method on_burn is private"));

    Ok(())
}

#[tokio::test]
async fn on_transfer_direct_call() -> Result<()> {
    let mut context = prepare_contract(None, None).await?;

    let alice = context.alice().await?;

    let result = alice
        .call(context.sweat_claim().contract.as_account().id(), "on_transfer")
        .args_json(json!({
            "now": 1702304333,
            "account_id": alice.id().to_string(),
            "total_accrual": "100000",
            "details": vec![(1702303000, "100000")],
        }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Method on_transfer is private"));

    Ok(())
}

#[tokio::test]
async fn insufficient_gas_on_claim() -> Result<()> {
    let claim_period = 0;
    let burn_period = 60 * 60;
    let mut context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    let alice_steps = 10_000;
    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), alice_steps)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&manager)
        .await?;

    let is_claim_available = context.sweat_claim().is_claim_available(alice.to_near()).await?;
    assert_eq!(is_claim_available, ClaimAvailabilityView::Available(0));

    let result = alice
        .call(context.sweat_claim().contract.as_account().id(), "claim")
        .gas(NearGas::from_tgas(9))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Not enough gas for further operations"));

    Ok(())
}

#[tokio::test]
async fn insufficient_gas_on_burn() -> Result<()> {
    let claim_period = 0;
    let burn_period = 1;
    let mut context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let alice = context.alice().await?;
    let manager = context.manager().await?;

    let alice_steps = 10_000;
    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), alice_steps)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&manager)
        .await?;

    context.fast_forward_minutes(1).await?;

    // All the tokens must evaporate at the moment
    context.sweat_claim().claim().with_user(&alice).await?;

    let result = manager
        .call(context.sweat_claim().contract.as_account().id(), "burn")
        .gas(NearGas::from_tgas(9))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Not enough gas for further operations"));

    Ok(())
}

trait FTExt {
    async fn formula_detailed(&self, steps_since_tge: U64, steps: u32) -> Result<(U128, U128, U128)>;
}

impl FTExt for SweatContract<'_> {
    async fn formula_detailed(&self, steps_since_tge: U64, steps: u32) -> Result<(U128, U128, U128)> {
        let token_amount = self.formula(steps_since_tge, steps).await?.0;
        let payout = Payout::from(token_amount);

        Ok((U128(payout.fee), U128(payout.amount_for_user), U128(token_amount)))
    }
}
