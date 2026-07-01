use serde_json::json;
use tracing::info;

mod common;
use common::{
    helpers::{balance_to_burn, claimable_balance, fast_forward_minutes, formula, ft_balance_of, payout},
    prepare::prepare_contract,
};

const ALICE_STEPS: u32 = 10_000;

/// Calls `burn` on the claim contract as the manager and returns the burnt amount.
async fn burn(context: &common::prepare::Context, amount: Option<u128>) -> anyhow::Result<u128> {
    let burnt: String = context
        .manager
        .call(context.claim.id(), "burn")
        .args_json(json!({ "amount": amount.map(|a| a.to_string()) }))
        .max_gas()
        .transact()
        .await?
        .json()?;
    Ok(burnt.parse()?)
}

async fn defer_alice(context: &common::prepare::Context, steps: u32) -> anyhow::Result<()> {
    context
        .manager
        .call(context.sweat.id(), "defer_batch")
        .args_json(json!({
            "steps_batch": [[context.alice.id(), steps]],
            "holding_account_id": context.claim.id(),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn burn_total() -> anyhow::Result<()> {
    let claim_period = 60;
    let burn_period = 2 * 60;
    let context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let target_token_amount = formula(&context.sweat, 0, ALICE_STEPS).await?;
    let (amount_for_user, _fee) = payout(target_token_amount);

    defer_alice(&context, ALICE_STEPS).await?;
    assert_eq!(ft_balance_of(&context.sweat, context.claim.id()).await?, amount_for_user);

    assert_eq!(burn(&context, None).await?, 0, "nothing to burn yet");

    // Full evaporation needs the clock past 2 * burn_period (4 min); 10 leaves margin.
    info!("fast forward past the burn window");
    fast_forward_minutes(&context.worker, 10).await?;

    info!("claim() [signer=alice] — everything has evaporated");
    context
        .alice
        .call(context.claim.id(), "claim")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    assert_eq!(burn(&context, None).await?, amount_for_user, "all evaporated tokens burnt");
    assert_eq!(claimable_balance(&context.claim, context.alice.id()).await?, 0);
    assert_eq!(balance_to_burn(&context.claim).await?, 0);

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn burn_part() -> anyhow::Result<()> {
    let claim_period = 0;
    let burn_period = 1;
    let context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let alice_steps = 100_000;
    let target_token_amount = formula(&context.sweat, 0, alice_steps).await?;
    let (amount_for_user, _fee) = payout(target_token_amount);

    defer_alice(&context, alice_steps).await?;
    assert_eq!(ft_balance_of(&context.sweat, context.claim.id()).await?, amount_for_user);

    assert_eq!(burn(&context, None).await?, 0, "nothing to burn yet");

    fast_forward_minutes(&context.worker, 2).await?;

    info!("claim() [signer=alice]");
    context
        .alice
        .call(context.claim.id(), "claim")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let target_amount_to_burn = 100;
    assert_eq!(burn(&context, Some(target_amount_to_burn)).await?, target_amount_to_burn);

    assert_eq!(claimable_balance(&context.claim, context.alice.id()).await?, 0);
    assert_eq!(
        balance_to_burn(&context.claim).await?,
        amount_for_user - target_amount_to_burn
    );

    Ok(())
}
