// near-sdk 5.x deprecated `store::UnorderedMap`/`UnorderedSet` in favour of
// `IterableMap`/`IterableSet`. We intentionally keep the deprecated types because
// switching would change the on-chain storage layout and require a state migration,
// which is out of scope for this dependency update.
#![allow(deprecated)]

use claim_model::{
    account_record::{AccountRecordLegacy, AccountRecordVersioned},
    api::InitApi,
    Duration, TokensAmount,
};
use near_plugins::{access_control, AccessControlRole, AccessControllable, Upgradable};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, near, near_bindgen,
    store::LookupMap,
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

#[derive(AccessControlRole, Copy, Clone)]
pub enum Roles {
    Oracle,
    BurnManager,
    Maintainer,
    StagingManager,
    UpgradeManager,
}

/// The main structure representing a smart contract for managing fungible tokens.
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Upgradable)]
#[access_control(role_type(Roles))]
#[upgradable(access_control_roles(
    code_stagers(Roles::StagingManager),
    code_deployers(Roles::UpgradeManager),
    duration_initializers(Roles::UpgradeManager),
    duration_update_stagers(Roles::UpgradeManager),
    duration_update_appliers(Roles::UpgradeManager),
))]
#[near_bindgen(contract_state)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    /// The account ID of the fungible token contract serviced by this smart contract.
    ///
    /// This field specifies the associated fungible token contract with which this smart
    /// contract interacts.
    token_account_id: AccountId,

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

    /// Kept only so pre-linear-burn on-chain state still deserializes correctly;
    /// no current code path reads or writes it.
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
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    AccountsLegacy,
    // Renamed, not removed: deleting any of these variants would shift `Accounts`'s
    // Borsh discriminant (its storage-prefix byte), silently orphaning all stored
    // balances. Only referenced by migration/tests.rs now that Contract itself has
    // dropped the `accruals` field.
    _Accruals,
    _AccrualsEntryLegacy(u32),
    _OraclesLegacy,
    Accounts,
}

#[near_bindgen]
impl InitApi for Contract {
    #[init]
    fn init(token_account_id: AccountId) -> Self {
        Self::assert_private();

        let mut contract = Self {
            token_account_id,

            accounts_legacy: LookupMap::new(StorageKey::AccountsLegacy),
            accounts: LookupMap::new(StorageKey::Accounts),

            claim_period: INITIAL_CLAIM_PERIOD_MS,
            burn_period: INITIAL_BURN_PERIOD_MS,

            is_service_call_running: false,

            balance_to_burn: 0,
        };

        contract.acl_init_super_admin(env::current_account_id());

        contract
    }
}
