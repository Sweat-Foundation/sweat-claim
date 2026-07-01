use serde_json::json;

mod common;
use common::{panic::PanicFinder, prepare::prepare_contract};

#[tokio::test]
#[tracing::instrument]
async fn on_burn_direct_call() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "on_burn")
        .args_json(json!({ "amount_to_burn": "100000" }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Method on_burn is private"));

    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn on_transfer_direct_call() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "on_transfer")
        .args_json(json!({
            "now": 1_702_304_333u32,
            "account_id": context.alice.id(),
            "amount_to_claim": "100000",
            "amount_to_burn": "0",
        }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic("Method on_transfer is private"));

    Ok(())
}
