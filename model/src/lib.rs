use near_sdk::{json_types::U128, near};

pub mod account_record;
pub mod api;
pub mod event;

pub type UnixTimestamp = u32;
pub type TokensAmount = u128;
pub type Duration = u32; // Period in seconds

#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
#[serde(untagged)]
pub enum ClaimableBalanceView {
    Short(U128),
    Detailed { total: U128, available: U128 },
}

impl ClaimableBalanceView {
    pub fn new(total: TokensAmount, available: TokensAmount, detailed: bool) -> Self {
        if detailed {
            Self::Detailed {
                total: total.into(),
                available: available.into(),
            }
        } else {
            Self::Short(available.into())
        }
    }

    pub fn available_balance(&self) -> TokensAmount {
        match self {
            ClaimableBalanceView::Short(amount) => amount.0,
            ClaimableBalanceView::Detailed { available, .. } => available.0,
        }
    }
}

#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ClaimAvailabilityView {
    /// Claim is available. Wrapped number is the amount of claimable entries.
    Available(u16),
    /// Claim is not available. Wrapped tuple is the timestamp the last claim
    /// and the duration of the claim period.
    Unavailable((UnixTimestamp, Duration)),
    /// User is not registered in the contract.
    Unregistered,
}

#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
pub struct ClaimResultView {
    pub total: U128,
}

impl ClaimResultView {
    pub fn new(total: u128) -> Self {
        Self { total: U128(total) }
    }
}

#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
pub struct BurnStatus {
    pub min_claimable_ts: Option<UnixTimestamp>,
    pub claim_period_refreshed_at: UnixTimestamp,
    pub burn_period: Duration,
}

pub trait UnixTimestampExtension {
    fn is_within_period(&self, now: UnixTimestamp, period: Duration) -> bool;
}

impl UnixTimestampExtension for UnixTimestamp {
    fn is_within_period(&self, now: UnixTimestamp, period: Duration) -> bool {
        now - self < period
    }
}

pub fn get_burn_rate(balance: TokensAmount, burn_period: Duration) -> TokensAmount {
    balance.div_ceil(burn_period.into())
}
