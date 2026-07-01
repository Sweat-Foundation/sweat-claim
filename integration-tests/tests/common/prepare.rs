use std::path::PathBuf;

use anyhow::{anyhow, Result};
use near_workspaces::{network::Sandbox, types::NearToken, Account, Contract, Worker};
use serde_json::json;
use tracing::info;

use super::helpers::{init_tracing, storage_deposit};

const INITIAL_USER_BALANCE: NearToken = NearToken::from_near(10);
const ALICE_TGE_MINT: u128 = 100_000_000;

pub const CLAIM_PERIOD: u32 = 30 * 60;
pub const BURN_PERIOD: u32 = 3 * 60 * 60;

const SWEAT_WASM_ENV: &str = "SWEAT_WASM";
const CLAIM_WASM_ENV: &str = "CLAIM_WASM";

/// A booted sandbox with the SWEAT token and the claim contract deployed and wired
/// together: `manager` is an oracle on both, `alice` is funded and registered.
pub struct Context {
    // Held to keep the sandbox alive for the test's lifetime.
    pub worker: Worker<Sandbox>,
    pub sweat: Contract,
    pub claim: Contract,
    pub manager: Account,
    pub alice: Account,
}

pub async fn prepare_contract(claim_period: Option<u32>, burn_period: Option<u32>) -> Result<Context> {
    init_tracing();

    info!("booting sandbox");
    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;

    info!("deploying sweat + claim");
    let sweat = deploy(&worker, sweat_wasm_path(), "sweat", SWEAT_WASM_ENV).await?;
    let claim = deploy(&worker, claim_wasm_path(), "claim", CLAIM_WASM_ENV).await?;

    let manager = create_user(&root, "manager").await?;
    let alice = create_user(&root, "alice").await?;

    // Initialize the SWEAT token. `new`/`add_oracle`/`tge_mint` are owner-gated
    // (predecessor must equal the contract account), so they are signed by the
    // contract account itself via `Contract::call`.
    info!("initializing sweat");
    sweat
        .call("new")
        .args_json(json!({ "postfix": ".u.sweat.testnet" }))
        .transact()
        .await?
        .into_result()?;

    info!("initializing claim");
    claim
        .call("init")
        .args_json(json!({ "token_account_id": sweat.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Oracle wiring: manager may defer on the token and operate the claim contract;
    // the token contract may call `record_batch_for_hold` on the claim contract.
    // `claim`'s own account is the only account with `acl_grant_role` permission
    // (it made itself super-admin in `init`), so these calls must be signed by `claim` itself.
    sweat
        .call("add_oracle")
        .args_json(json!({ "account_id": manager.id() }))
        .transact()
        .await?
        .into_result()?;
    claim
        .call("acl_grant_role")
        .args_json(json!({ "role": "Oracle", "account_id": sweat.id() }))
        .transact()
        .await?
        .into_result()?;
    for role in ["Oracle", "BurnManager", "Maintainer"] {
        claim
            .call("acl_grant_role")
            .args_json(json!({ "role": role, "account_id": manager.id() }))
            .transact()
            .await?
            .into_result()?;
    }

    // Register the claim contract and alice for FT storage, then seed alice.
    storage_deposit(&sweat, claim.id()).await?;
    storage_deposit(&sweat, alice.id()).await?;
    sweat
        .call("tge_mint")
        .args_json(json!({ "account_id": alice.id(), "amount": ALICE_TGE_MINT.to_string() }))
        .transact()
        .await?
        .into_result()?;

    // Configure claim/burn windows (oracle-gated → signed by manager).
    info!("configuring claim/burn periods");
    manager
        .call(claim.id(), "set_claim_period")
        .args_json(json!({ "period": claim_period.unwrap_or(CLAIM_PERIOD) }))
        .transact()
        .await?
        .into_result()?;
    manager
        .call(claim.id(), "set_burn_period")
        .args_json(json!({ "period": burn_period.unwrap_or(BURN_PERIOD) }))
        .transact()
        .await?
        .into_result()?;

    info!("ready");
    Ok(Context {
        worker,
        sweat,
        claim,
        manager,
        alice,
    })
}

fn wasm_path(env_var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(env_var).map(PathBuf::from).unwrap_or(default)
}

fn res_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("res").join(file)
}

fn sweat_wasm_path() -> PathBuf {
    wasm_path(SWEAT_WASM_ENV, res_path("sweat.wasm"))
}

fn claim_wasm_path() -> PathBuf {
    wasm_path(CLAIM_WASM_ENV, res_path("sweat_claim.wasm"))
}

fn read_wasm_bytes(path: PathBuf, label: &str, env_var: &str) -> Result<Vec<u8>> {
    std::fs::read(&path).map_err(|e| {
        anyhow!(
            "failed to read {label} WASM at {} — did you run `make build-integration`? \
             Override the path with the {env_var} env var. ({e})",
            path.display()
        )
    })
}

async fn deploy(worker: &Worker<Sandbox>, path: PathBuf, label: &str, env_var: &str) -> Result<Contract> {
    let bytes = read_wasm_bytes(path, label, env_var)?;
    Ok(worker.dev_deploy(&bytes).await?)
}

/// Reads the currently-built `claim` contract wasm bytes (same path `deploy` uses
/// for the `claim` contract) — for tests that need to stage/deploy raw code
/// rather than just deploying a fresh instance of it.
pub fn claim_wasm_bytes() -> Result<Vec<u8>> {
    read_wasm_bytes(claim_wasm_path(), "claim", CLAIM_WASM_ENV)
}

async fn create_user(root: &Account, name: &str) -> Result<Account> {
    Ok(root
        .create_subaccount(name)
        .initial_balance(INITIAL_USER_BALANCE)
        .transact()
        .await?
        .into_result()?)
}
