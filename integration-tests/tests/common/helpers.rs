use anyhow::{anyhow, Result};
use near_workspaces::{network::Sandbox, types::NearToken, AccountId, Contract, Worker};
use serde_json::{json, Value};
use tracing_subscriber::EnvFilter;

use super::prepare::Context;

/// nitka used 240 sandbox blocks per simulated minute; keep the same ratio so the
/// time-based claim/burn windows behave as they did under the old harness.
const BLOCKS_PER_MINUTE: u64 = 240;

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,near_workspaces=warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

/// Advances the sandbox clock by roughly `minutes` simulated minutes.
pub async fn fast_forward_minutes(worker: &Worker<Sandbox>, minutes: u64) -> Result<()> {
    worker.fast_forward(BLOCKS_PER_MINUTE * minutes).await?;
    Ok(())
}

/// The SWEAT payout split: a 5% fee (rounded up) is taken, the rest goes to the user.
pub fn payout(value: u128) -> (u128, u128) {
    let fee = (value * 5).div_ceil(100);
    (value - fee, fee)
}

/// `ft_balance_of` view on the SWEAT token, returned as a number.
pub async fn ft_balance_of(sweat: &Contract, account_id: &AccountId) -> Result<u128> {
    let balance: String = sweat
        .view("ft_balance_of")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?;
    Ok(balance.parse()?)
}

/// `formula` view on the SWEAT token: the raw token amount minted for `steps`.
pub async fn formula(sweat: &Contract, steps_since_tge: u64, steps: u32) -> Result<u128> {
    let amount: String = sweat
        .view("formula")
        .args_json(json!({ "steps_since_tge": steps_since_tge.to_string(), "steps": steps }))
        .await?
        .json()?;
    Ok(amount.parse()?)
}

/// Claimable balance for an account (the non-detailed `Short` variant is a bare string).
pub async fn claimable_balance(claim: &Contract, account_id: &AccountId) -> Result<u128> {
    let balance: String = claim
        .view("get_claimable_balance_for_account")
        .args_json(json!({ "account_id": account_id, "detailed": null }))
        .await?
        .json()?;
    Ok(balance.parse()?)
}

/// `is_claim_available` view; returns the `ClaimAvailabilityView` JSON
/// (`{"type": "available" | "unavailable" | "unregistered", "data": ...}`).
pub async fn claim_availability(claim: &Contract, account_id: &AccountId) -> Result<Value> {
    Ok(claim
        .view("is_claim_available")
        .args_json(json!({ "account_id": account_id }))
        .await?
        .json()?)
}

/// `get_balance_to_burn` view on the claim contract.
pub async fn balance_to_burn(claim: &Contract) -> Result<u128> {
    let balance: String = claim.view("get_balance_to_burn").await?.json()?;
    Ok(balance.parse()?)
}

/// Defers `steps` for `account_id` on the SWEAT token, crediting the claim
/// contract as the holding account — the same mechanism real steps go through
/// on their way to becoming a claimable balance. Signed by `context.manager`,
/// SWEAT's oracle.
pub async fn defer_steps(context: &Context, account_id: &AccountId, steps: u32) -> Result<()> {
    context
        .manager
        .call(context.sweat.id(), "defer_batch")
        .args_json(json!({
            "steps_batch": [[account_id, steps]],
            "holding_account_id": context.claim.id(),
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

/// Grants `role` to `account_id` on the claim contract. Must be signed by the
/// claim contract's own account, since it made itself super-admin in `init`.
pub async fn grant_role(claim: &Contract, role: &str, account_id: &AccountId) -> Result<()> {
    claim
        .call("acl_grant_role")
        .args_json(json!({ "role": role, "account_id": account_id }))
        .transact()
        .await?
        .into_result()?;
    Ok(())
}

/// Registers `account_id` for storage on the SWEAT token (NEP-145).
pub async fn storage_deposit(sweat: &Contract, account_id: &AccountId) -> Result<()> {
    let bounds: Value = sweat.view("storage_balance_bounds").await?.json()?;
    let min = bounds
        .get("min")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("storage_balance_bounds.min missing"))?;
    sweat
        .call("storage_deposit")
        .args_json(json!({ "account_id": account_id }))
        .deposit(NearToken::from_yoctonear(min.parse()?))
        .transact()
        .await?
        .into_result()?;
    Ok(())
}
