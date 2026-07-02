use near_workspaces::types::Gas;
use serde_json::json;
use tracing::info;

mod common;
use common::{
    helpers::{claim_availability, defer_steps, fast_forward_minutes},
    panic::PanicFinder,
    prepare::prepare_contract,
};

const ALICE_STEPS: u32 = 10_000;

#[tokio::test]
#[tracing::instrument]
async fn insufficient_gas_on_claim() -> anyhow::Result<()> {
    let context = prepare_contract(Some(0), Some(60 * 60)).await?;

    defer_steps(&context, context.alice.id(), ALICE_STEPS).await?;

    let availability = claim_availability(&context.claim, context.alice.id()).await?;
    assert_eq!(availability["type"], "available");

    info!("claim() with only 9 Tgas — must fail the gas guard");
    let result = context
        .alice
        .call(context.claim.id(), "claim")
        .gas(Gas::from_tgas(9))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Not enough gas for further operations"));

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn insufficient_gas_on_burn() -> anyhow::Result<()> {
    let context = prepare_contract(Some(0), Some(1)).await?;

    defer_steps(&context, context.alice.id(), ALICE_STEPS).await?;

    fast_forward_minutes(&context.worker, 1).await?;

    // All tokens have evaporated by now.
    info!("claim() [signer=alice]");
    context
        .alice
        .call(context.claim.id(), "claim")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    info!("burn() with only 1 Tgas — must fail the gas guard");
    let result = context
        .manager
        .call(context.claim.id(), "burn")
        .args_json(json!({}))
        .gas(Gas::from_tgas(1))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Not enough gas for further operations"));

    Ok(())
}
