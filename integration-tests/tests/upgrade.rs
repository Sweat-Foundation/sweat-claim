use serde_json::json;

mod common;
use common::{
    helpers::{claimable_balance, grant_role},
    panic::PanicFinder,
    prepare::{claim_wasm_bytes, prepare_contract},
};

const INSUFFICIENT_PERMISSIONS: &str = "Insufficient permissions";

/// `up_stage_code` is restricted to `StagingManager` and `up_deploy_code` to
/// `UpgradeManager`. The two roles are distinct: holding the stager role does
/// not grant the deployer one.
#[tokio::test]
#[tracing::instrument]
async fn test_upgrade_access_control() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    let code = claim_wasm_bytes()?;

    let result = context
        .alice
        .call(context.claim.id(), "up_stage_code")
        .args(code.clone())
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));

    let staged: Option<String> = context.claim.view("up_staged_code_hash").await?.json()?;
    assert_eq!(staged, None, "unauthorized staging must not store any code");

    let result = context
        .alice
        .call(context.claim.id(), "up_deploy_code")
        .args_json(json!({ "hash": "ignored", "function_call_args": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));

    grant_role(&context.claim, "StagingManager", context.alice.id()).await?;

    let result = context
        .alice
        .call(context.claim.id(), "up_stage_code")
        .args(code.clone())
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    assert!(result.outcome().is_success());

    let staged: Option<String> = context.claim.view("up_staged_code_hash").await?.json()?;
    assert!(staged.is_some(), "staged code hash should be set after staging");

    let result = context
        .alice
        .call(context.claim.id(), "up_deploy_code")
        .args_json(json!({ "hash": staged.unwrap(), "function_call_args": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(
        result.has_panic(INSUFFICIENT_PERMISSIONS),
        "the stager role must not grant deploy permission"
    );

    Ok(())
}

/// Full stage → deploy flow: an `UpgradeManager` re-deploys the contract over
/// itself and existing state survives the upgrade.
#[tokio::test]
#[tracing::instrument]
async fn test_upgrade_deploy() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    let code = claim_wasm_bytes()?;

    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;
    let balance_before = claimable_balance(&context.claim, context.alice.id()).await?;
    assert_ne!(0, balance_before, "there should be recorded balance before the upgrade");

    for role in ["StagingManager", "UpgradeManager"] {
        grant_role(&context.claim, role, context.manager.id()).await?;
    }

    context
        .manager
        .call(context.claim.id(), "up_stage_code")
        .args(code)
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let staged_hash: Option<String> = context.claim.view("up_staged_code_hash").await?.json()?;
    let staged_hash = staged_hash.expect("code must be staged before deploy");

    let result = context
        .manager
        .call(context.claim.id(), "up_deploy_code")
        .args_json(json!({ "hash": staged_hash, "function_call_args": null }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    assert!(result.outcome().is_success(), "deploy should succeed");

    let balance_after = claimable_balance(&context.claim, context.alice.id()).await?;
    assert_eq!(balance_before, balance_after, "claimable balance must survive the upgrade");

    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "500"]] }))
        .transact()
        .await?
        .into_result()?;
    let balance_final = claimable_balance(&context.claim, context.alice.id()).await?;
    assert_eq!(
        balance_before + 500,
        balance_final,
        "the upgraded contract should keep recording balances"
    );

    Ok(())
}
