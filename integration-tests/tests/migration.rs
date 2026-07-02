use std::path::PathBuf;

use anyhow::Result;
use serde_json::json;

mod common;
use common::{
    helpers::{balance_to_burn, fast_forward_minutes},
    prepare::create_user,
};

/// 50,000,000 SWEAT already withdrawn outside the contract's own burn flow —
/// `migrate()` must find at least this much in the old `balance_to_burn` and
/// subtract it. Mirrors `contract::migration::WITHDRAWN_SWEAT`, which can't be
/// imported directly since integration-tests is a separate workspace/crate.
const WITHDRAWN_SWEAT: u128 = 50_000_000 * 10u128.pow(18);

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

    let sweat_placeholder = create_user(&root, "token").await?;

    claim
        .call("init")
        .args_json(json!({ "token_account_id": sweat_placeholder.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let oracle = create_user(&root, "oracle").await?;

    claim
        .call("add_oracle")
        .args_json(json!({ "account_id": oracle.id() }))
        .transact()
        .await?
        .into_result()?;

    // Seed balance_to_burn with exactly WITHDRAWN_SWEAT via the pre-ACL contract's own
    // claim/evaporation mechanics, so migrate()'s new correction has something real to
    // subtract (it panics if balance_to_burn is below this amount). A short burn_period
    // lets the recorded balance fully evaporate well within the test's fast-forward.
    oracle
        .call(claim.id(), "set_claim_period")
        .args_json(json!({ "period": 0u32 }))
        .transact()
        .await?
        .into_result()?;
    oracle
        .call(claim.id(), "set_burn_period")
        .args_json(json!({ "period": 1u32 }))
        .transact()
        .await?
        .into_result()?;
    oracle
        .call(claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[oracle.id(), WITHDRAWN_SWEAT.to_string()]] }))
        .transact()
        .await?
        .into_result()?;

    fast_forward_minutes(&worker, 2).await?;

    // Fully evaporated (amount_to_claim == 0), so this resolves synchronously and
    // folds the recorded amount into balance_to_burn without any cross-contract
    // transfer — safe even though `token_account_id` isn't a real token contract here.
    oracle.call(claim.id(), "claim").max_gas().transact().await?.into_result()?;
    assert_eq!(WITHDRAWN_SWEAT, balance_to_burn(&claim).await?, "balance_to_burn should be seeded");

    // Upgrade to the ACL-based contract and migrate state.
    let new_bytes = std::fs::read(new_wasm_path())?;
    claim.as_account().deploy(&new_bytes).await?.into_result()?;
    claim
        .call("migrate")
        .args_json(json!({
            "burn_managers": [oracle.id()],
            "maintainers": [oracle.id()],
            "staging_managers": [],
            "upgrade_managers": [],
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    assert_eq!(
        0,
        balance_to_burn(&claim).await?,
        "balance_to_burn should be corrected by the withdrawn amount"
    );

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

    // period must stay below the still-active burn_period (1, set while seeding
    // balance_to_burn above) — 0 satisfies that regardless.
    let config_result = oracle
        .call(claim.id(), "set_claim_period")
        .args_json(json!({ "period": 0u32 }))
        .transact()
        .await?
        .into_result();
    assert!(config_result.is_ok(), "Maintainer role not migrated: {config_result:?}");

    Ok(())
}
