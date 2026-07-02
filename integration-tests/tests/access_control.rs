use near_workspaces::types::NearToken;
use serde_json::json;

mod common;
use common::{
    helpers::{fast_forward_minutes, grant_role},
    panic::PanicFinder,
    prepare::prepare_contract,
};

const INSUFFICIENT_PERMISSIONS: &str = "Insufficient permissions";

#[tokio::test]
#[tracing::instrument]
async fn record_batch_for_hold_by_oracle_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn record_batch_for_hold_by_non_oracle_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn burn_by_burn_manager_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn burn_by_non_burn_manager_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_claim_period_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "set_claim_period")
        .args_json(json!({ "period": 100u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_claim_period_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "set_claim_period")
        .args_json(json!({ "period": 100u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_claim_period_above_one_year_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    // Without an upper bound, this single call would freeze claim() contract-wide
    // for every existing account by making claim_period_refreshed_at look
    // "recently refreshed" indefinitely.
    let result = context
        .manager
        .call(context.claim.id(), "set_claim_period")
        .args_json(json!({ "period": u32::MAX / 2 }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Claim period exceeds the maximum allowed"));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_burn_period_above_one_year_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "set_burn_period")
        .args_json(json!({ "period": u32::MAX / 2 }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Burn period exceeds the maximum allowed"));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_burn_period_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "set_burn_period")
        .args_json(json!({ "period": 10_000u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_burn_period_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "set_burn_period")
        .args_json(json!({ "period": 10_000u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn clean_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .manager
        .call(context.claim.id(), "clean")
        .args_json(json!({ "account_ids": [context.alice.id()] }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn clean_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "clean")
        .args_json(json!({ "account_ids": [context.alice.id()] }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_account_enabled_by_maintainer_succeeds_and_blocks_claim() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .manager
        .call(context.claim.id(), "set_account_enabled")
        .args_json(json!({ "account_id": context.alice.id(), "enabled": false }))
        .transact()
        .await?
        .into_result();
    assert!(result.is_ok());

    // CLAIM_PERIOD (prepare.rs default) must elapse before claim() gets past the
    // availability check and reaches the is_enabled check this test targets.
    fast_forward_minutes(&context.worker, 31).await?;

    let claim_result = context
        .alice
        .call(context.claim.id(), "claim")
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(claim_result.has_panic("Account is disabled"));

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_account_enabled_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .alice
        .call(context.claim.id(), "set_account_enabled")
        .args_json(json!({ "account_id": context.alice.id(), "enabled": false }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn unlock_account_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .manager
        .call(context.claim.id(), "unlock_account")
        .args_json(json!({ "account_id": context.alice.id() }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn unlock_account_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .alice
        .call(context.claim.id(), "unlock_account")
        .args_json(json!({ "account_id": context.alice.id() }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn reset_service_call_flag_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "reset_service_call_flag")
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn reset_service_call_flag_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "reset_service_call_flag")
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn role_holder_without_the_right_role_is_still_rejected() -> anyhow::Result<()> {
    // `manager` holds Oracle/BurnManager/Maintainer (see prepare.rs), so use a
    // freshly-granted single-role account to prove roles don't cross-authorize
    // each other's methods.
    let context = prepare_contract(None, None).await?;
    let root = context.worker.root_account()?;
    let oracle_only = root
        .create_subaccount("oracleonly")
        .initial_balance(NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;
    grant_role(&context.claim, "Oracle", oracle_only.id()).await?;

    // Oracle role holder can record...
    let record_result = oracle_only
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();
    assert!(record_result.is_ok());

    // ...but not burn, which requires BurnManager.
    let burn_result = oracle_only
        .call(context.claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(burn_result.has_panic(INSUFFICIENT_PERMISSIONS));

    Ok(())
}
