use std::path::PathBuf;

use anyhow::Result;
use serde_json::json;

mod common;

fn pre_acl_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("res").join("sweat_claim_pre_acl.wasm")
}

fn new_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("res")
        .join("sweat_claim.wasm")
}

#[tokio::test]
#[tracing::instrument]
async fn oracle_survives_migration_to_acl() -> Result<()> {
    common::helpers::init_tracing();

    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;

    // Deploy the pre-ACL contract and register an oracle the old way.
    let pre_acl_bytes = std::fs::read(pre_acl_wasm_path())?;
    let claim = worker.dev_deploy(&pre_acl_bytes).await?;

    let sweat_placeholder = root
        .create_subaccount("token")
        .initial_balance(near_workspaces::types::NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;

    claim
        .call("init")
        .args_json(json!({ "token_account_id": sweat_placeholder.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let oracle = root
        .create_subaccount("oracle")
        .initial_balance(near_workspaces::types::NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;

    claim
        .call("add_oracle")
        .args_json(json!({ "account_id": oracle.id() }))
        .transact()
        .await?
        .into_result()?;

    // Upgrade to the ACL-based contract and migrate state.
    let new_bytes = std::fs::read(new_wasm_path())?;
    claim.as_account().deploy(&new_bytes).await?.into_result()?;
    claim
        .call("migrate")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // The oracle should still be able to call all three role-gated actions.
    let record_result = oracle
        .call(claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[oracle.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();
    assert!(record_result.is_ok(), "Oracle role not migrated: {record_result:?}");

    let burn_result = oracle
        .call(claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(burn_result.is_ok(), "BurnManager role not migrated: {burn_result:?}");

    let config_result = oracle
        .call(claim.id(), "set_claim_period")
        .args_json(json!({ "period": 100u32 }))
        .transact()
        .await?
        .into_result();
    assert!(config_result.is_ok(), "Maintainer role not migrated: {config_result:?}");

    Ok(())
}
