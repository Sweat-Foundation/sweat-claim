use anyhow::Result;
use claim_model::api::{ClaimApiIntegration, MigrationApiIntegration};
use nitka::misc::ToNear;
use sweat_model::SweatDeferIntegration;

use crate::{
    common::PanicFinder,
    prepare::{prepare_custom_contract, IntegrationContext},
};

#[tokio::test]
async fn failed_migration() -> Result<()> {
    let mut context = prepare_custom_contract(None, None, "sweat_claim_legacy".into()).await?;

    let alice = context.alice().await?;

    // Add some data before migration
    let balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;

    assert_eq!(balance.0, 0);

    // Redeploy claim contract
    context.redeploy_claim_contract("../res/sweat_claim.wasm").await?;

    // Check that the contract cannot restore it's state
    let result = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .result()
        .await;

    assert!(result.has_panic("Cannot deserialize the contract state."));

    Ok(())
}

#[tokio::test]
async fn successful_migration() -> Result<()> {
    let mut context = prepare_custom_contract(None, None, "sweat_claim_legacy".into()).await?;

    let alice = context.alice().await?;
    let oracle = context.manager().await?;

    // Add some data before migration
    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), 1000)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&oracle)
        .await?;

    let balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;

    assert!(balance.0 > 0);

    // Redeploy claim contract
    context.redeploy_claim_contract("../res/sweat_claim.wasm").await?;
    context
        .sweat_claim()
        .migrate()
        .with_user(context.sweat_claim().contract.as_account())
        .await?;

    // Check that the contract cannot restore it's state
    let new_balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;

    assert_eq!(balance.0, new_balance.0);

    Ok(())
}
