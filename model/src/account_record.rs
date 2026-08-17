#![allow(deprecated)]

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

use crate::{get_burn_rate, Duration, TokensAmount, UnixTimestamp};

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum AccountRecordVersioned {
    V1(AccountRecordV1),
}

/// Represents the state of a registered account in the smart contract.
///
/// `AccountRecord` maintains the status of an individual user's account within
/// the smart contract. It tracks various aspects of the account, such as balance,
/// burn status, and operational states.
#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct AccountRecordV1 {
    /// Represents the base balance of a user account.
    ///
    /// This property stores the base balance of a user account, which is the amount of tokens
    /// currently held by the account. The balance decreases over time when evaporation occurs,
    /// as specified by the contract rules.
    pub balance: TokensAmount,

    /// Represents the start of the window for which the balance should be evaporated.
    ///
    /// This property stores the timestamp indicating the start of the window during which the
    /// balance should be evaporated. It serves as the left border of the evaporation window. The
    /// right border is determined by the end of the burn window, calculated as `now - Contract.burn_period`.
    /// This timestamp updates whenever a record or claim operation occurs.
    pub burn_since: UnixTimestamp,

    /// The timestamp of the last event that resets claim period.
    /// It can be either creation of the record or claim operation performed by the account.
    ///
    /// `claim_period_refreshed_at` holds an `UnixTimestamp` that records either the time when
    /// the record was created or when the user last claimed their tokens.
    /// It is used to determine eligibility for future claims.
    pub claim_period_refreshed_at: UnixTimestamp,

    /// Indicates whether the user is authorized to use the contract's features.
    ///
    /// Currently, `is_enabled` is not actively used but is prepared for future releases.
    /// It can be used to enable or disable access to contract functionalities for this
    /// particular account.
    pub is_enabled: bool,

    /// Indicates whether there is an active operation on the user's balance.
    ///
    /// `is_locked` is used to signal if the account is currently engaged in an operation
    /// that affects its balance, such as a claim process. This is important for ensuring
    /// the integrity of account operations and preventing concurrent modifications.
    pub is_locked: bool,
}

impl AccountRecordVersioned {
    pub fn into_latest(&self) -> &AccountRecordV1 {
        let AccountRecordVersioned::V1(value) = self;
        value
    }

    pub fn new(now: UnixTimestamp) -> Self {
        Self::V1(AccountRecordV1::new(now))
    }
}

pub type AccountRecord = AccountRecordV1;

impl AccountRecord {
    pub fn new(now: UnixTimestamp) -> Self {
        Self {
            balance: 0,
            claim_period_refreshed_at: now,
            burn_since: now,
            is_enabled: true,
            is_locked: false,
        }
    }
}

impl AccountRecord {
    pub fn get_balance_to_burn(&self, burn_period: Duration, claimable_window_start: UnixTimestamp) -> TokensAmount {
        if self.claim_period_refreshed_at > claimable_window_start {
            0
        } else {
            self.get_burn_rate(burn_period) * u128::from(claimable_window_start.saturating_sub(self.burn_since))
        }
        .min(self.balance)
    }

    pub fn get_burn_rate(&self, burn_period: Duration) -> TokensAmount {
        get_burn_rate(self.balance, burn_period)
    }
}
