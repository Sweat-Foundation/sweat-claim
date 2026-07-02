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
    let oracle = create_user(&root, "oracle").await?;
    let claim = deploy_pre_acl_and_seed_balance_to_burn(&worker, &root, &oracle).await?;

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

/// Deploys the pre-ACL contract, registers `oracle` as a legacy oracle, and
/// records+claims exactly `WITHDRAWN_SWEAT` through it so `migrate()`'s balance
/// correction has something real to subtract. Returns the deployed contract.
async fn deploy_pre_acl_and_seed_balance_to_burn(
    worker: &near_workspaces::Worker<near_workspaces::network::Sandbox>,
    root: &near_workspaces::Account,
    oracle: &near_workspaces::Account,
) -> Result<near_workspaces::Contract> {
    let pre_acl_bytes = std::fs::read(pre_acl_wasm_path())?;
    let claim = worker.dev_deploy(&pre_acl_bytes).await?;

    let sweat_placeholder = create_user(root, "token").await?;
    claim
        .call("init")
        .args_json(json!({ "token_account_id": sweat_placeholder.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    claim
        .call("add_oracle")
        .args_json(json!({ "account_id": oracle.id() }))
        .transact()
        .await?
        .into_result()?;

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

    fast_forward_minutes(worker, 2).await?;

    oracle.call(claim.id(), "claim").max_gas().transact().await?.into_result()?;
    assert_eq!(WITHDRAWN_SWEAT, balance_to_burn(&claim).await?, "balance_to_burn should be seeded");

    Ok(claim)
}

/// Real on-chain regression test for the storage-leak fix: `migrate()` must not
/// leave the pre-ACL `oracles` set's storage entries orphaned. Compares final
/// account storage_usage between a contract that had many legacy oracles (whose
/// old-set entries should be reclaimed) and one that grants the same number of
/// roles entirely through migrate()'s argument path (never had that old-set
/// storage in the first place) — if the fix works, the two end up within a small
/// constant of each other; if it leaks, the many-oracles case stays measurably
/// larger by roughly one old-oracle-set entry per extra account.
#[tokio::test]
#[tracing::instrument]
async fn migrate_reclaims_old_oracles_storage() -> Result<()> {
    common::helpers::init_tracing();
    const EXTRA_ACCOUNTS: usize = 19;

    // Scenario A: `setup_oracle` plus EXTRA_ACCOUNTS more legacy oracles — all
    // granted the Oracle role automatically by migrate()'s unconditional path.
    let worker_a = near_workspaces::sandbox().await?;
    let root_a = worker_a.root_account()?;
    let setup_oracle_a = create_user(&root_a, "oracle").await?;
    let claim_a = deploy_pre_acl_and_seed_balance_to_burn(&worker_a, &root_a, &setup_oracle_a).await?;

    let mut extra_a = Vec::with_capacity(EXTRA_ACCOUNTS);
    for i in 0..EXTRA_ACCOUNTS {
        let account = create_user(&root_a, &format!("legacyoracle{i}")).await?;
        claim_a
            .call("add_oracle")
            .args_json(json!({ "account_id": account.id() }))
            .transact()
            .await?
            .into_result()?;
        extra_a.push(account);
    }

    let new_bytes = std::fs::read(new_wasm_path())?;
    claim_a.as_account().deploy(&new_bytes).await?.into_result()?;
    claim_a
        .call("migrate")
        .args_json(json!({ "burn_managers": [], "maintainers": [], "staging_managers": [], "upgrade_managers": [] }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    let storage_with_legacy_oracles = claim_a.view_account().await?.storage_usage;

    // Scenario B: same total number of role grants (1 Oracle for setup_oracle +
    // EXTRA_ACCOUNTS Maintainer grants), but the extra accounts are granted
    // through migrate()'s argument path — never added as legacy oracles, so
    // there's no old-set storage for them to leak in the first place.
    let worker_b = near_workspaces::sandbox().await?;
    let root_b = worker_b.root_account()?;
    let setup_oracle_b = create_user(&root_b, "oracle").await?;
    let claim_b = deploy_pre_acl_and_seed_balance_to_burn(&worker_b, &root_b, &setup_oracle_b).await?;

    let mut extra_b_ids = Vec::with_capacity(EXTRA_ACCOUNTS);
    for i in 0..EXTRA_ACCOUNTS {
        let account = create_user(&root_b, &format!("arggrantee{i}")).await?;
        extra_b_ids.push(account.id().clone());
    }

    claim_b.as_account().deploy(&new_bytes).await?.into_result()?;
    claim_b
        .call("migrate")
        .args_json(json!({
            "burn_managers": [],
            "maintainers": extra_b_ids,
            "staging_managers": [],
            "upgrade_managers": [],
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    let storage_without_legacy_oracles = claim_b.view_account().await?.storage_usage;

    // A small constant slack for genuinely incidental differences (e.g. Oracle
    // vs Maintainer role bit encoding, account name length); nowhere near what
    // 19 leaked old-oracle-set entries would cost if unreclaimed.
    let diff = storage_with_legacy_oracles.abs_diff(storage_without_legacy_oracles);
    assert!(
        diff < 200,
        "final storage usage must not scale with how many legacy oracles existed \
         pre-migration if their old-set storage was properly reclaimed \
         (with_legacy_oracles={storage_with_legacy_oracles}, without={storage_without_legacy_oracles}, diff={diff})"
    );

    Ok(())
}
