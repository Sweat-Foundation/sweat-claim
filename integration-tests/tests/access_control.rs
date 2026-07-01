use near_workspaces::types::NearToken;
use serde_json::json;

mod common;
use common::{panic::PanicFinder, prepare::prepare_contract};

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
    context
        .claim
        .call("acl_grant_role")
        .args_json(json!({ "role": "Oracle", "account_id": oracle_only.id() }))
        .transact()
        .await?
        .into_result()?;

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
