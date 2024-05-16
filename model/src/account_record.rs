#![allow(deprecated)]

use std::collections::HashMap;

use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
};

use crate::{get_burn_rate, AccrualIndex, AssetSymbol, Duration, TokensAmount, UnixTimestamp};

/// Represents the state of a registered account in the smart contract.
///
/// `AccountRecord` maintains the status and history of an individual user's account within
/// the smart contract. It tracks various aspects of the account, such as accrual references,
/// claim history, and operational states.
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct AccountRecordLegacy {
    /// A list of references to accrual entries in `Contract.accruals`.
    ///
    /// `accruals` contains pairs of timestamps and indices that link to specific accrual
    /// records in the contract's accruals ledger. These references are used to calculate
    /// and verify the user's accrued token amount.
    ///
    /// Here is an illustration of the connection:
    /// ```text
    ///        Contract.accruals:
    ///        ...
    ///        1705066289: ([0.1, 2.3, 5.3, 2.0, 4.3], 14)
    ///  ┌───> 1705066501: ([1.2, 3.4, 8.7, 9.6], 22.9)
    ///  │     ...                      ↑
    ///  │                              │
    ///  │     AccountRecord.accruals:  │
    ///  │     [(1705066501, 2)]        │
    ///  └────────────┘      └──────────┘
    /// ```
    pub accruals: Vec<(UnixTimestamp, AccrualIndex)>,

    /// Indicates whether the user is authorized to use the contract's features.
    ///
    /// Currently, `is_enabled` is not actively used but is prepared for future releases.
    /// It can be used to enable or disable access to contract functionalities for this
    /// particular account.
    pub is_enabled: bool,

    /// The timestamp of the last event that resets claim period.
    /// It can be either creation of the record or claim operation performed by the account.
    ///
    /// `claim_period_refreshed_at` holds an `UnixTimestamp` that records either the time when
    /// the record was created or when the user last claimed their tokens.
    /// It is used to determine eligibility for future claims.
    pub claim_period_refreshed_at: UnixTimestamp,

    /// Indicates whether there is an active operation on the user's balance.
    ///
    /// `is_locked` is used to signal if the account is currently engaged in an operation
    /// that affects its balance, such as a claim process. This is important for ensuring
    /// the integrity of account operations and preventing concurrent modifications.
    pub is_locked: bool,
}

impl AccountRecordLegacy {
    pub fn new(now: UnixTimestamp) -> Self {
        Self {
            accruals: Vec::new(),
            is_enabled: true,
            claim_period_refreshed_at: now,
            is_locked: false,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub enum AccountRecordVersioned {
    V1(AccountRecordV1),
    V2(AccountRecordV2),
}

/// Represents the state of a registered account in the smart contract.
///
/// `AccountRecord` maintains the status of an individual user's account within
/// the smart contract. It tracks various aspects of the account, such as balance,
/// burn status, and operational states.
#[derive(BorshDeserialize, BorshSerialize, Copy, Clone)]
pub struct AccountRecordV1 {
    /// Represents the base balance of a user account.
    ///
    /// This property stores the base balance of a user account, which is the amount of tokens
    /// currently held b the account. The balance decreases over time when evaporation occurs,
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

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct AccountRecordV2 {
    /// Represents the base balance of a user account.
    ///
    /// This property stores the base balance of a user account, which is the amount of tokens
    /// currently held by the account. The balance decreases over time when evaporation occurs,
    /// as specified by the contract rules.
    pub balance: TokensAmount,

    pub extra_balances: HashMap<AssetSymbol, TokensAmount>,

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

impl From<AccountRecordV1> for AccountRecord {
    fn from(value: AccountRecordV1) -> Self {
        Self {
            balance: value.balance,
            extra_balances: HashMap::new(),
            claim_period_refreshed_at: value.claim_period_refreshed_at,
            burn_since: value.burn_since,
            is_enabled: value.is_enabled,
            is_locked: value.is_locked,
        }
    }
}

impl From<AccountRecordVersioned> for AccountRecord {
    fn from(value: AccountRecordVersioned) -> Self {
        match value {
            AccountRecordVersioned::V1(value) => value.into(),
            AccountRecordVersioned::V2(value) => value,
        }
    }
}

impl From<AccountRecord> for AccountRecordVersioned {
    fn from(value: AccountRecord) -> Self {
        Self::V2(value)
    }
}

impl AccountRecordVersioned {
    pub fn new(now: UnixTimestamp) -> Self {
        Self::from(AccountRecord::new(now))
    }

    pub fn from_legacy(account: &AccountRecordLegacy, balance: TokensAmount, burn_since: UnixTimestamp) -> Self {
        Self::V1(AccountRecordV1 {
            balance,
            burn_since,
            claim_period_refreshed_at: account.claim_period_refreshed_at,
            is_enabled: account.is_enabled,
            is_locked: account.is_locked,
        })
    }

    pub fn update_to_latest(&self) -> AccountRecordVersioned {
        match self {
            AccountRecordVersioned::V1(value) => Self::from(AccountRecord::from(*value)),
            AccountRecordVersioned::V2(_) => self.clone(),
        }
    }

    pub fn is_latest(&self) -> bool {
        matches!(self, AccountRecordVersioned::V2(_))
    }
}

pub type AccountRecord = AccountRecordV2;

impl AccountRecord {
    pub fn new(now: UnixTimestamp) -> Self {
        Self {
            balance: 0,
            extra_balances: HashMap::new(),
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
            self.get_burn_rate(burn_period) * u128::from(claimable_window_start - self.burn_since)
        }
        .min(self.balance)
    }

    pub fn get_burn_rate(&self, burn_period: Duration) -> TokensAmount {
        get_burn_rate(self.balance, burn_period)
    }
}
