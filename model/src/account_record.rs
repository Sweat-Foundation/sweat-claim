#![allow(deprecated)]

use std::{
    collections::HashMap,
    fmt::format,
    ops::{Deref, DerefMut},
};

use near_sdk::{
    borsh,
    borsh::{
        maybestd::io::{Error, ErrorKind::InvalidInput},
        BorshDeserialize, BorshSerialize,
    },
};

use crate::{get_burn_rate, AccrualIndex, AssetSymbol, Duration, TokensAmount, UnixTimestamp};

pub type AccountRecord = AccountRecordVersioned;
pub type AccountRecordLatest = AccountRecordV2;

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

#[derive(BorshSerialize, Clone)]
pub enum AccountRecordVersioned {
    V1(AccountRecordV1),
    V2(AccountRecordV2),
}

/// Custom `BorshDeserialize` implementation is needed to automatically
/// convert old versions to latest version
impl BorshDeserialize for AccountRecordVersioned {
    fn deserialize(buf: &mut &[u8]) -> Result<Self, Error> {
        let variant_idx: u8 = BorshDeserialize::deserialize(buf)?;
        let return_value = match variant_idx {
            0u8 => {
                let v1: AccountRecordV1 = BorshDeserialize::deserialize(buf)?;
                AccountRecordVersioned::V2(v1.into())
            }
            1u8 => AccountRecordVersioned::V2(BorshDeserialize::deserialize(buf)?),
            _ => {
                let msg = {
                    let res = format(format_args!("Unexpected variant index: {variant_idx}",));
                    res
                };
                return Err(Error::new(InvalidInput, msg));
            }
        };
        Ok(return_value)
    }
}

impl Deref for AccountRecordVersioned {
    type Target = AccountRecordLatest;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::V1(_) => unreachable!("Outdated AccountRecord"),
            // Guaranteed by `BorshDeserialize` implementation
            Self::V2(record) => record,
        }
    }
}

impl DerefMut for AccountRecordVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::V1(_) => unreachable!("Outdated AccountRecord"),
            // Guaranteed by `BorshDeserialize` implementation
            Self::V2(record) => record,
        }
    }
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

impl From<AccountRecordV1> for AccountRecordV2 {
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

impl AccountRecord {
    pub fn new(now: UnixTimestamp) -> Self {
        Self::V2(AccountRecordLatest {
            balance: 0,
            extra_balances: HashMap::new(),
            claim_period_refreshed_at: now,
            burn_since: now,
            is_enabled: true,
            is_locked: false,
        })
    }

    pub fn from_legacy(account: &AccountRecordLegacy, balance: TokensAmount, burn_since: UnixTimestamp) -> Self {
        Self::V2(AccountRecordV2 {
            balance,
            extra_balances: HashMap::new(),
            burn_since,
            claim_period_refreshed_at: account.claim_period_refreshed_at,
            is_enabled: account.is_enabled,
            is_locked: account.is_locked,
        })
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
