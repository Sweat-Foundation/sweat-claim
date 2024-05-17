#[cfg(feature = "release-api")]
use near_sdk::AccountId;
use near_sdk::{json_types::U128, PromiseOrValue};
#[cfg(not(feature = "release-api"))]
use nitka::AccountId;
use nitka_proc::make_integration_version;

use crate::{asset::Asset, AssetSymbol, BurnStatus, ClaimAvailabilityView, ClaimResultView, Duration, UnixTimestamp};

#[cfg(feature = "integration-test")]
pub struct ClaimContract<'a> {
    pub contract: &'a near_workspaces::Contract,
}

/// An API for initializing smart contracts in the context of fungible token operations.
///
/// This API provides a method to initialize the smart contract, primarily for interactions
/// with a specified fungible token contract.
#[make_integration_version]
pub trait InitApi {
    /// Initializes the smart contract with a specified fungible token contract.
    ///
    /// This method sets up the necessary parameters and states required for the
    /// smart contract to operate with the specified fungible token contract.
    ///
    /// # Arguments
    ///
    /// * `token_account_id` - An `AccountId` representing the account address of the fungible
    ///   token contract that this smart contract will interact with.
    ///
    /// # Returns
    ///
    /// Returns an instance of the implementing type.
    fn init(token_account_id: AccountId) -> Self;
}

/// An API for configuring various parameters of the smart contract during its lifetime.
///
/// This API allows for dynamic configuration of certain operational parameters
/// of the smart contract.
#[make_integration_version]
pub trait ConfigApi {
    /// Sets the claim period for the smart contract.
    ///
    /// This method configures the duration (in seconds) that must elapse before a user
    /// is allowed to claim their tokens. A user can claim the tokens once per the defined
    /// `claim_period`.
    ///
    /// # Arguments
    ///
    /// * `period` - A `Duration` representing the length of the claim period in seconds.
    ///
    /// # Panics
    ///
    /// Panics if called by any entity other than the oracle.
    fn set_claim_period(&mut self, period: Duration);

    /// Sets the burn period for the smart contract.
    ///
    /// This method configures the duration (in seconds) that must elapse before the user's
    /// unclaimed tokens are considered for burning by the contract. Tokens that remain unclaimed
    /// after the `burn_period` has passed may be burnt by the contract.
    ///
    /// # Arguments
    ///
    /// * `period` - A `Duration` representing the length of the burn period in seconds.
    ///
    /// # Panics
    ///
    /// Panics if called by any entity other than the oracle.
    fn set_burn_period(&mut self, period: Duration);
}

/// An API for managing authorization of oracles for sensitive operations in the smart contract.
///
/// This API allows managing of oracles, which are accounts authorized to perform
/// sensitive operations.
#[make_integration_version]
pub trait AuthApi {
    /// Adds an oracle to the smart contract.
    ///
    /// Registers an oracle identified by `account_id`, authorizing them for sensitive operations.
    /// This method is private and can only be called by the account where the contract is deployed.
    /// It will panic if an attempt is made to register the same oracle twice.
    ///
    /// # Arguments
    ///
    /// * `account_id` - An `AccountId` representing the oracle to be added.
    ///
    /// # Panics
    ///
    /// Panics if the oracle is already registered.
    fn add_oracle(&mut self, account_id: AccountId);

    /// Removes an oracle from the smart contract.
    ///
    /// Revokes authorization from an oracle identified by `account_id`. This method is private
    /// and can only be called by the account where the contract is deployed. It will panic
    /// if there is no registered oracle with the specified `account_id`.
    ///
    /// # Arguments
    ///
    /// * `account_id` - An `AccountId` representing the oracle to be removed.
    ///
    /// # Panics
    ///
    /// Panics if no oracle with the specified `account_id` is registered.
    fn remove_oracle(&mut self, account_id: AccountId);

    /// Retrieves the list of registered oracles.
    ///
    /// Returns a vector of `AccountId`s representing the oracles currently authorized
    /// for sensitive operations.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<AccountId>` containing the account IDs of the registered oracles.
    fn get_oracles(&self) -> Vec<AccountId>;

    /// Unlocks the specified account.
    ///
    /// This method allows an oracle to unlock an account that may have been locked due to an error
    /// occurring during a cross-contract call. When a cross-contract call fails, the account might
    /// be locked to prevent further operations until the error is resolved.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The ID of the account to be unlocked.
    ///
    /// # Panics
    ///
    /// This method will panic if it is called by someone other than the oracle or if the specified
    /// account is not found.
    fn unlock_account(&mut self, account_id: AccountId);
}

/// An API for burning unclaimed tokens in the smart contract. This is essential for
/// managing the lifecycle of tokens and ensuring that unclaimed tokens are appropriately
/// disposed of after a certain period.
#[make_integration_version]
pub trait BurnApi {
    /// Burns all unclaimed tokens older than `Contract.burn_period`.
    ///
    /// # Returns
    ///
    /// A `PromiseOrValue<U128>` indicating the total amount of tokens burned.
    ///
    /// # Panics
    ///
    /// Panics if called by any entity other than the oracle. Only the oracle has the
    /// authority to initiate the burn process.
    ///
    /// Panics if another service call is running.
    fn burn(&mut self) -> PromiseOrValue<U128>;

    /// Retrieves the burn status for a given account.
    ///
    /// This method returns a `BurnStatus` struct containing data required to calculate
    /// when a user's balance will start evaporating. The `account_id` parameter specifies
    /// the ID of the account for which the burn status is requested.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The ID of the account for which to retrieve burn status.
    ///
    /// # Returns
    ///
    /// A `BurnStatus` struct containing information about the burn status of the account.
    fn get_burn_status(&self, account_id: AccountId) -> BurnStatus;
}

/// An API for recording (updating) user balances in the smart contract.
#[make_integration_version]
pub trait RecordApi {
    /// Records (updates) the balance for a batch of users.
    ///
    /// This method takes a list of pairs, where each pair consists of a user's `AccountId`
    /// and the amount to be added to their balance. It is used for batch processing of
    /// balance updates.
    ///
    /// # Arguments
    ///
    /// * `amounts`: A vector of tuples (`Vec<(AccountId, U128)>`). Each tuple contains an
    ///   `AccountId` representing a user and a `U128` value indicating the amount to be
    ///   added to the user's balance.
    ///
    /// # Panics
    ///
    /// Panics if called by any account other than an oracle.
    fn record_batch_for_hold(&mut self, amounts: Vec<(AccountId, U128)>, asset: Option<AssetSymbol>);
}

/// An API for managing the claiming process of accrued tokens in the smart contract.
#[make_integration_version]
pub trait ClaimApi {
    /// Retrieves the amount of claimable tokens for a specified account.
    ///
    /// This method returns the total number of tokens that have been deferred and are currently
    /// claimable for the given `account_id`.
    ///
    /// # Arguments
    ///
    /// * `account_id`: The `AccountId` for which the claimable balance is being queried.
    ///
    /// # Returns
    ///
    /// Returns a `U128` value indicating the amount of claimable tokens for the provided
    /// `account_id`.
    fn get_claimable_balance_for_account(&self, account_id: AccountId) -> U128;

    /// Checks if the claim is available for a specified account.
    ///
    /// This method verifies whether a claim is currently available for the provided `account_id`.
    /// It returns a `ClaimAvailabilityView` enum, which can be either `Available` if the user
    /// can claim immediately, or `Unavailable` with details about the datetime of the last claim
    /// and the claim period duration. If the user has no registered data in the contract, it
    /// returns `ClaimAvailabilityView.Unregistered`.
    ///
    /// # Arguments
    ///
    /// * `account_id`: The `AccountId` for which claim availability is being checked.
    ///
    /// # Returns
    ///
    /// Returns a `ClaimAvailabilityView` indicating the claim status for the provided
    /// `account_id`.
    fn is_claim_available(&self, account_id: AccountId) -> ClaimAvailabilityView;

    /// Claims all available tokens for the caller.
    ///
    /// This method allows users to claim all tokens that are available to them at the moment
    /// of calling. It should be used when a user wants to retrieve their accrued tokens.
    ///
    /// # Returns
    ///
    /// Returns a `PromiseOrValue<ClaimResultView>` indicating the result of the claim operation.
    ///
    /// # Panics
    ///
    /// Panics if the claim is unavailable at the moment of calling. Users should ensure that
    /// their claim is available using the `is_claim_available` method prior to calling this.
    fn claim(&mut self) -> PromiseOrValue<ClaimResultView>;
}

#[make_integration_version]
pub trait MigrationApi {
    fn migrate() -> Self;

    fn migrate_accounts(&mut self, accounts: Vec<AccountId>);

    fn cleanup(&mut self, keys: Vec<UnixTimestamp>);
}

#[make_integration_version]
pub trait AssetApi {
    fn get_assets(&self) -> Vec<(AssetSymbol, Asset)>;

    fn register_asset(&mut self, asset: AssetSymbol, account_id: AccountId);
}
