use tracing::info;

mod common;
use common::{
    helpers::{claim_availability, claimable_balance, defer_steps, fast_forward_minutes, formula, ft_balance_of, payout},
    prepare::prepare_contract,
};

const ALICE_STEPS: u32 = 10_000;

#[tokio::test]
#[tracing::instrument]
async fn happy_flow() -> anyhow::Result<()> {
    let claim_period = 5 * 60;
    let burn_period = 20 * 60;
    let context = prepare_contract(Some(claim_period), Some(burn_period)).await?;

    let alice_initial_balance = ft_balance_of(&context.sweat, context.alice.id()).await?;

    let target_token_amount = formula(&context.sweat, 0, ALICE_STEPS).await?;
    let (amount_for_user, _fee) = payout(target_token_amount);

    info!("defer_batch([(alice, {ALICE_STEPS})]) [signer=manager]");
    defer_steps(&context, context.alice.id(), ALICE_STEPS).await?;

    let claim_contract_balance = ft_balance_of(&context.sweat, context.claim.id()).await?;
    assert_eq!(claim_contract_balance, amount_for_user);

    let alice_deferred_balance = claimable_balance(&context.claim, context.alice.id()).await?;
    assert_eq!(alice_deferred_balance, amount_for_user);

    let availability = claim_availability(&context.claim, context.alice.id()).await?;
    assert_eq!(availability["type"], "unavailable");

    fast_forward_minutes(&context.worker, u64::from(claim_period) / 60).await?;

    let availability = claim_availability(&context.claim, context.alice.id()).await?;
    assert_eq!(availability["type"], "available");
    assert_eq!(availability["data"], 0);

    info!("claim() [signer=alice]");
    context
        .alice
        .call(context.claim.id(), "claim")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let alice_balance = ft_balance_of(&context.sweat, context.alice.id()).await?;
    assert_eq!(alice_balance - alice_initial_balance, amount_for_user);

    Ok(())
}
