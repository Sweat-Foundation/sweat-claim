use claim_model::{
    account_record::{AccountRecordLegacy, AccountRecordVersioned},
    api::InitApi,
    Duration, TokensAmount, UnixTimestamp,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen,
    store::{LookupMap, UnorderedMap, UnorderedSet, Vector},
    AccountId, BorshStorageKey, PanicOnDefault,
};

mod auth;
mod burn;
mod claim;
mod clean;
mod common;
mod config;
mod migration;
mod record;

const INITIAL_CLAIM_PERIOD_MS: u32 = 24 * 60 * 60;
const INITIAL_BURN_PERIOD_MS: u32 = 30 * 24 * 60 * 60;

/// The main structure representing a smart contract for managing fungible tokens.
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    /// The account ID of the fungible token contract serviced by this smart contract.
    ///
    /// This field specifies the associated fungible token contract with which this smart
    /// contract interacts.
    token_account_id: AccountId,

    /// A set of account IDs authorized to perform sensitive operations within the contract.
    ///
    /// `oracles` represents the entities that have the authority to execute critical
    /// functions such as burning tokens. These accounts are trusted and have elevated privileges.
    oracles: UnorderedSet<AccountId>,

    /// The period in seconds during which tokens are locked after being claimed.
    ///
    /// `claim_period` defines the duration for which the tokens remain locked and
    /// untransferable after a user claims them. This lock period helps in managing the
    /// token lifecycle and user claims.
    claim_period: Duration,

    /// The period in seconds after which unclaimed tokens are eligible to be burnt.
    ///
    /// `burn_period` specifies the timeframe after which tokens that haven't been claimed
    /// are considered for burning, helping in regulating the token supply.
    burn_period: Duration,

    /// A ledger storing the timestamps of recordings and the corresponding user accruals.
    ///
    /// `accruals` does not contain account IDs directly but correlates with `AccountRecord`
    /// entries in the `accounts` field. It is essential for tracking token accruals over time.
    /// `AccountRecord` entries in `accounts` map contain pairs of a timestamp pointing to exact
    /// entry in `accruals` and index of particular accrual in corresponding vector.
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
    accruals: UnorderedMap<UnixTimestamp, (Vector<TokensAmount>, TokensAmount)>,

    /// A map containing accrual and service details for each user account.
    ///
    /// `accounts` holds individual records for users, detailing their accrued tokens and
    /// related service information. It works in conjunction with `accruals` to provide a
    /// comprehensive view of each user's token status.
    accounts_legacy: LookupMap<AccountId, AccountRecordLegacy>,

    accounts: LookupMap<AccountId, AccountRecordVersioned>,

    /// Indicates whether a service call is currently in progress.
    ///
    /// `is_service_call_running` is used to prevent double spending by indicating if the
    /// contract is currently executing a service call. This flag ensures the integrity of
    /// token transactions and operations within the contract.
    is_service_call_running: bool,

    balance_to_burn: TokensAmount,
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    AccountsLegacy,
    Accruals,
    _AccrualsEntryLegacy(u32),
    Oracles,
    Accounts,
}

#[near_bindgen]
impl InitApi for Contract {
    #[init]
    fn init(token_account_id: AccountId) -> Self {
        Self::assert_private();

        Self {
            token_account_id,

            accounts_legacy: LookupMap::new(StorageKey::AccountsLegacy),
            accounts: LookupMap::new(StorageKey::Accounts),
            accruals: UnorderedMap::new(StorageKey::Accruals),
            oracles: UnorderedSet::new(StorageKey::Oracles),

            claim_period: INITIAL_CLAIM_PERIOD_MS,
            burn_period: INITIAL_BURN_PERIOD_MS,

            is_service_call_running: false,

            balance_to_burn: 0,
        }
    }
}
