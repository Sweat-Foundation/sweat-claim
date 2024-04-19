#![cfg(test)]

use std::future::IntoFuture;

use anyhow::Result;
use claim_model::api::RecordApiIntegration;
use near_sdk::json_types::U128;
use near_workspaces::types::Gas;
use nitka::measure::{
    measure::scoped_command_measure,
    outcome_storage::OutcomeStorage,
    utils::{pretty_gas_string, values_diff},
};

use crate::{prepare::IntegrationContext, prepare_contract};

#[ignore]
#[tokio::test]
async fn measure_record_batch_for_hold_test() -> Result<()> {
    let multiplier: u64 = 50;

    let measured = scoped_command_measure(
        (0..5).map(|i| i * multiplier as usize).collect::<Vec<_>>(),
        measure_record_batch_for_hold,
    )
    .await?;

    for me in &measured {
        println!("{} - {}", me.0, pretty_gas_string(me.1))
    }

    let gas: Vec<_> = measured.into_iter().map(|a| a.1.as_gas()).collect();

    let diff: Vec<_> = values_diff(gas.clone())
        .into_iter()
        .map(|val| pretty_gas_string(Gas::from_gas(val)))
        .collect();

    dbg!(&diff);

    let diff: Vec<_> = values_diff(gas.clone())
        .into_iter()
        .map(|val| pretty_gas_string(Gas::from_gas(val / multiplier)))
        .collect();

    dbg!(&diff);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn single_record_batch_for_hold() -> Result<()> {
    let gas = measure_record_batch_for_hold(300).await?;

    dbg!(&gas.as_tgas());

    Ok(())
}

async fn measure_record_batch_for_hold(count: usize) -> Result<Gas> {
    let mut context = prepare_contract(None, None).await?;

    let oracle = context.manager().await?;

    let records: Vec<_> = (0..count)
        .map(|i| {
            (
                format!("acc_{i}sdasaddsaadsdasdsadsa").try_into().unwrap(),
                U128(i as u128),
            )
        })
        .collect();

    let (gas, _) = OutcomeStorage::measure_total(
        &oracle,
        context
            .sweat_claim()
            .record_batch_for_hold(records)
            .with_user(&oracle)
            .into_future(),
    )
    .await?;

    Ok(gas)
}
