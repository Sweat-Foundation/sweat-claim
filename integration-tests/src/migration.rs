use anyhow::Result;
use claim_model::api::ClaimApiIntegration;
use nitka::misc::ToNear;

use crate::prepare::{prepare_custom_contract, IntegrationContext};

#[tokio::test]
async fn migration_example() -> Result<()> {
    let mut context = prepare_custom_contract(None, None, "sweat_claim_before_migration".into()).await?;

    let alice = context.alice().await?;

    // Add some data before migration
    let balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;

    assert_eq!(balance.0, 0);

    // Redeploy claim contract
    context.redeploy_claim_contract("../res/sweat_claim.wasm").await?;

    // Check data after migration
    let balance = context
        .sweat_claim()
        .get_claimable_balance_for_account(alice.to_near())
        .await?;

    assert_eq!(balance.0, 0);

    Ok(())
}
