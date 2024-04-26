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

#[tokio::test]
async fn migrate_accounts_without_evaporating() -> Result<()> {
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
    context
        .sweat_claim()
        .migrate_accounts(vec![alice.to_near()])
        .with_user(&oracle)
        .await?;

    // Check that the contract cannot restore it's state
    let new_balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;

    assert_eq!(balance.0, new_balance.0);

    Ok(())
}

#[tokio::test]
async fn migrate_accounts_with_evaporating() -> Result<()> {
    let burn_period_minutes = 2;
    let mut context =
        prepare_custom_contract(Some(0), Some(burn_period_minutes * 60), "sweat_claim_legacy".into()).await?;

    let alice = context.alice().await?;
    let oracle = context.manager().await?;

    // Add some data before migration

    // First Alice reports 1000 steps
    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), 1000)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&oracle)
        .await?;

    let alice_balance_1 = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;
    assert!(alice_balance_1.0 > 0);

    // Half burn period later Alice reports 5000 more steps
    context.fast_forward_minutes((burn_period_minutes / 2) as u64).await?;

    context
        .ft_contract()
        .defer_batch(
            vec![(alice.to_near(), 5000)],
            context.sweat_claim().contract.as_account().to_near(),
        )
        .with_user(&oracle)
        .await?;

    let alice_balance_2 = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;
    assert!(alice_balance_2.0 > alice_balance_1.0);

    let alice_target_balance = alice_balance_2.0 - alice_balance_1.0;

    // Half burn period later migration happens
    context.fast_forward_minutes((burn_period_minutes / 2) as u64).await?;

    // Redeploy claim contract
    context.redeploy_claim_contract("../res/sweat_claim.wasm").await?;
    context
        .sweat_claim()
        .migrate()
        .with_user(context.sweat_claim().contract.as_account())
        .await?;
    context
        .sweat_claim()
        .migrate_accounts(vec![alice.to_near()])
        .with_user(&oracle)
        .await?;

    // Check balance one minute later
    context.fast_forward_minutes(1).await?;

    let new_balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;

    // Expect that the balance starts evaporating from alice_target_balance, which is
    // the balance after submission of the second batch of steps.
    // The balance should be less than the target balance and more than 0
    assert!(new_balance.0 > 0);
    assert!(alice_target_balance > new_balance.0);

    Ok(())
}
