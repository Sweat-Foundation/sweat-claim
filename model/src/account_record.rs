#![allow(deprecated)]

use near_sdk::{
    borsh,
    borsh::{BorshDeserialize, BorshSerialize},
};

use crate::{AccrualIndex, TokensAmount, UnixTimestamp};

/// Represents the state of a registered account in the smart contract.
///
/// `AccountRecord` maintains the status and history of an individual user's account within
/// the smart contract. It tracks various aspects of the account, such as accrual references,
/// claim history, and operational states.
#[derive(BorshDeserialize, BorshSerialize)]
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

#[derive(BorshDeserialize, BorshSerialize)]
pub enum AccountRecordVersioned {
    V1(AccountRecordV1),
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct AccountRecordV1 {
    pub balance: TokensAmount,
    pub last_top_up_at: UnixTimestamp,
    pub claim_period_refreshed_at: UnixTimestamp,
    pub is_enabled: bool,
    pub is_locked: bool,
}

impl AccountRecordVersioned {
    pub fn from_legacy(account: &AccountRecordLegacy, balance: TokensAmount, last_top_up_at: UnixTimestamp) -> Self {
        Self::V1(AccountRecordV1 {
            balance,
            last_top_up_at,
            claim_period_refreshed_at: account.claim_period_refreshed_at,
            is_enabled: account.is_enabled,
            is_locked: account.is_locked,
        })
    }

    pub fn into_latest(&self) -> &AccountRecordV1 {
        let AccountRecordVersioned::V1(value) = self;
        value
    }
}

impl AccountRecordVersioned {
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
            last_top_up_at: now,
            is_enabled: true,
            is_locked: false,
        }
    }
}
