pub mod account_record;
pub mod api;
pub mod event;

use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
};

pub type UnixTimestamp = u32;
pub type AccrualIndex = u32;
pub type TokensAmount = u128;
pub type Duration = u32; // Period in seconds

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum ClaimAvailabilityView {
    /// Claim is available. Wrapped number is the amount of claimable entries.
    Available(u16),
    /// Claim is not available. Wrapped tuple is the timestamp the last claim
    /// and the duration of the claim period.
    Unavailable((UnixTimestamp, Duration)),
    /// User is not registered in the contract.
    Unregistered,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct ClaimResultView {
    pub total: U128,
}

impl ClaimResultView {
    pub fn new(total: u128) -> Self {
        Self { total: U128(total) }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct BurnStatus {
    pub min_claimable_ts: Option<UnixTimestamp>,
    pub claim_period_refreshed_at: UnixTimestamp,
    pub burn_period: Duration,
}
