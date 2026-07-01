# AccessControllable Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the flat `oracles`/`assert_oracle` access model in `sweat_claim` with `near-plugins`'s `AccessControllable` plugin, split into three roles (`Oracle`, `BurnManager`, `Maintainer`), and add exhaustive authorized/unauthorized integration test coverage for every protected method.

**Architecture:** `near-sdk` is bumped to the latest release first (isolated commit, since it drags the toolchain forward). Then `Contract` is annotated with `#[access_control(role_type(Roles))]`; each currently-`assert_oracle`-gated method gets `#[access_control_any(roles(Roles::X))]` instead. A `migrate()` reads the old on-chain state (still containing the `oracles` set) and grants all three new roles to every previously-registered oracle, so existing production oracles keep working immediately after upgrade. Integration tests (in the separate `integration-tests` workspace, see below) exercise every protected method from both an authorized and unauthorized caller.

**Tech Stack:** Rust, `near-sdk` 5.28.x, `near-plugins` 0.5.x, `near-workspaces` 0.22 (integration tests, unrelated toolchain).

## Global Constraints

- `near-sdk` bump target: `5.28.3` (latest as of writing). Verify against `cargo info near-sdk` at implementation time in case a newer patch has shipped — if so, use that instead and adjust `rust-toolchain.toml`/docker image accordingly.
- `near-plugins` version: `0.5.3` (latest; requires `near-sdk >= 5.2`, satisfied).
- Contract's on-chain Borsh state layout must remain readable across the upgrade: the `Contract` struct field removal (`oracles`) is handled via `#[init(ignore_state)] fn migrate()`, and the `StorageKey` enum's `Oracles` variant must NOT be deleted (only renamed with a leading underscore) — deleting it would shift the Borsh discriminant of the `Accounts` variant that comes after it, silently corrupting the storage prefix used for real user balances.
- `integration-tests/` is its own cargo workspace (see project memory) — always `cd integration-tests` before running `cargo test` there, and rebuild the contract wasm first via `make build-integration`.
- Do not implement `claim_model::api::MigrationApi` for this migration — two of its three methods (`migrate_accounts`, `cleanup`) belong to an unrelated, already-removed legacy-account migration (see `git log -p -S "impl MigrationApi"`). Write `migrate()` as a plain inherent method instead of forcing dead implementations of the other two.

---

### Task 1: Archive a pre-ACL contract wasm fixture for the migration test

The migration integration test (Task 11) needs to deploy the *current* (pre-this-branch) contract, wire an oracle the old way, then upgrade to the new contract and confirm the oracle survived. Build that fixture now, before any contract code changes, while `HEAD` is still the old oracle model.

**Files:**
- Create: `integration-tests/res/sweat_claim_pre_acl.wasm` (build artifact, not hand-written)

**Interfaces:** None (build-only task).

- [ ] **Step 1: Build the current contract**

Run: `make build`

Expected: `res/sweat_claim.wasm` is written (this is today's pre-ACL contract).

- [ ] **Step 2: Copy it into a fixtures directory for integration tests**

Run:
```bash
mkdir -p integration-tests/res
cp res/sweat_claim.wasm integration-tests/res/sweat_claim_pre_acl.wasm
```

- [ ] **Step 3: Commit the fixture**

```bash
git add integration-tests/res/sweat_claim_pre_acl.wasm
git commit -m "test: add pre-ACL contract wasm fixture for migration test"
```

---

### Task 2: Bump near-sdk, toolchain, cargo-near CLI, and CI

**Files:**
- Modify: `rust-toolchain.toml`
- Modify: `Cargo.toml` (root workspace)
- Modify: `contract/Cargo.toml`
- Modify: `.github/workflows/test.yml`
- Modify: `.github/workflows/push.yml`

**Interfaces:** None (dependency/config bump only — no contract code changes yet).

- [ ] **Step 1: Bump the toolchain channel**

In `rust-toolchain.toml`, change:
```toml
channel = "1.86"
```
to:
```toml
channel = "1.93"
```

- [ ] **Step 2: Bump the near-sdk floor in the workspace manifest**

In `Cargo.toml` (root), change:
```toml
near-sdk = "5"
```
to:
```toml
near-sdk = "5.28"
```

- [ ] **Step 3: Update the Cargo.lock**

Run: `cargo update -p near-sdk`

Expected: `Cargo.lock` now pins `near-sdk` to `5.28.3` (or newer, if a patch shipped — confirm via `grep -A1 'name = "near-sdk"$' Cargo.lock`).

- [ ] **Step 4: Bump the reproducible-build docker image**

In `contract/Cargo.toml`, under `[package.metadata.near.reproducible_build]`, change:
```toml
image = "sourcescan/cargo-near:0.19.0-rust-1.86.0"
image_digest = "sha256:772638e343baeeea24e49062c7d424274f3441452cc06ce97fc4e5695b19fecc"
```
to:
```toml
image = "sourcescan/cargo-near:0.21.1-rust-1.93.0"
image_digest = "sha256:79b789c81ba19ec30794a6b1046b02dcaf5c055994b22cc7c9af67ab5096b095"
```

(This digest was fetched from Docker Hub during design; re-verify with
`docker manifest inspect sourcescan/cargo-near:0.21.1-rust-1.93.0` if it's been more than a few days.)

- [ ] **Step 5: Bump the cargo-near CLI version in CI**

In `.github/workflows/test.yml` (2 occurrences) and `.github/workflows/push.yml` (1 occurrence), change:
```
https://github.com/near/cargo-near/releases/download/cargo-near-v0.19.0/cargo-near-installer.sh
```
to:
```
https://github.com/near/cargo-near/releases/download/cargo-near-v0.21.1/cargo-near-installer.sh
```

- [ ] **Step 6: Upgrade the local cargo-near CLI to match**

Run: `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/near/cargo-near/releases/download/cargo-near-v0.21.1/cargo-near-installer.sh | sh`

Expected: `cargo near --version` reports `cargo-near-near 0.21.1`.

- [ ] **Step 7: Build and test with the new toolchain**

Run: `make build && cargo test --package sweat_claim`

Expected: builds and all existing unit tests still pass. If compilation fails due to near-sdk API changes between 5.13 and 5.28, fix each error by reading the compiler message and adjusting the specific call site it points at (the exact errors can't be predicted here — near-sdk's changelog at `https://github.com/near/near-sdk-rs/releases` between the two versions is the reference if a fix isn't obvious from the compiler message alone). Re-run this command until it's green.

- [ ] **Step 8: Run lint**

Run: `make lint`

Expected: no new clippy warnings introduced by the bump. Fix any that appear the same way as Step 7.

- [ ] **Step 9: Commit**

```bash
git add rust-toolchain.toml Cargo.toml Cargo.lock contract/Cargo.toml .github/workflows/test.yml .github/workflows/push.yml
git commit -m "chore: bump near-sdk to 5.28, toolchain to 1.93"
```

(If Step 7 required source fixes in `contract/src/**`, `git add` those files too as part of this same commit.)

---

### Task 3: Add near-plugins dependency and the Roles enum

**Files:**
- Modify: `contract/Cargo.toml`
- Create: `contract/src/auth/roles.rs`
- Modify: `contract/src/auth/mod.rs`

**Interfaces:**
- Produces: `crate::auth::Roles` enum with variants `Oracle`, `BurnManager`, `Maintainer`, deriving `near_plugins::AccessControlRole`.

- [ ] **Step 1: Add the dependency**

In `contract/Cargo.toml`, under `[dependencies]`, add:
```toml
near-plugins = "0.5.3"
```

- [ ] **Step 2: Define the Roles enum**

Create `contract/src/auth/roles.rs`:
```rust
use near_plugins::AccessControlRole;

#[derive(AccessControlRole, Copy, Clone)]
pub enum Roles {
    Oracle,
    BurnManager,
    Maintainer,
}
```

- [ ] **Step 3: Register the module and re-export Roles**

`contract/src/auth/mod.rs` currently is:
```rust
pub(crate) mod api;
mod tests;
```
Change to:
```rust
pub(crate) mod api;
pub(crate) mod roles;
mod tests;

pub(crate) use roles::Roles;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build --package sweat_claim`

Expected: succeeds (the enum isn't used anywhere yet, so this just confirms the derive macro compiles against our near-sdk version).

- [ ] **Step 5: Commit**

```bash
git add contract/Cargo.toml contract/src/auth/roles.rs contract/src/auth/mod.rs Cargo.lock
git commit -m "feat: add near-plugins dependency and Roles enum"
```

---

### Task 4: Annotate Contract with AccessControllable, remove the oracles field, bootstrap super-admin

**Files:**
- Modify: `contract/src/lib.rs`

**Interfaces:**
- Consumes: `crate::auth::Roles` (Task 3).
- Produces: `Contract` now implements `near_plugins::AccessControllable`; `contract.acl_init_super_admin(account_id)`, `contract.acl_get_or_init()` are available to later tasks (migration, tests).

- [ ] **Step 1: Update imports**

In `contract/src/lib.rs`, change:
```rust
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    near_bindgen,
    store::{LookupMap, UnorderedMap, UnorderedSet, Vector},
    AccountId, BorshStorageKey, PanicOnDefault,
};
```
to:
```rust
use near_plugins::{access_control, AccessControllable};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    store::{LookupMap, UnorderedMap, Vector},
    AccountId, BorshStorageKey, PanicOnDefault,
};

use crate::auth::Roles;
```
(`UnorderedSet` is dropped here — it's no longer constructed in `lib.rs` — and `env` is added for `env::current_account_id()`.)

- [ ] **Step 2: Remove the oracles field and annotate the struct**

Change:
```rust
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
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
```
to:
```rust
#[access_control(role_type(Roles))]
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    /// The account ID of the fungible token contract serviced by this smart contract.
    ///
    /// This field specifies the associated fungible token contract with which this smart
    /// contract interacts.
    token_account_id: AccountId,

    /// The period in seconds during which tokens are locked after being claimed.
```

- [ ] **Step 3: Preserve the StorageKey discriminant (do not delete the variant)**

Change:
```rust
#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    AccountsLegacy,
    Accruals,
    _AccrualsEntryLegacy(u32),
    Oracles,
    Accounts,
}
```
to:
```rust
#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    AccountsLegacy,
    Accruals,
    _AccrualsEntryLegacy(u32),
    // Renamed, not removed: deleting this variant would shift `Accounts`'s Borsh
    // discriminant (its storage-prefix byte), silently orphaning all stored balances.
    _OraclesLegacy,
    Accounts,
}
```

- [ ] **Step 4: Update init() to drop the oracles field and bootstrap the super-admin**

Change:
```rust
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
```
to:
```rust
#[near_bindgen]
impl InitApi for Contract {
    #[init]
    fn init(token_account_id: AccountId) -> Self {
        Self::assert_private();

        let mut contract = Self {
            token_account_id,

            accounts_legacy: LookupMap::new(StorageKey::AccountsLegacy),
            accounts: LookupMap::new(StorageKey::Accounts),
            accruals: UnorderedMap::new(StorageKey::Accruals),

            claim_period: INITIAL_CLAIM_PERIOD_MS,
            burn_period: INITIAL_BURN_PERIOD_MS,

            is_service_call_running: false,

            balance_to_burn: 0,
        };

        contract.acl_init_super_admin(env::current_account_id());

        contract
    }
}
```

- [ ] **Step 5: Confirm it does not yet compile due to other files (expected)**

Run: `cargo build --package sweat_claim 2>&1 | head -50`

Expected: errors in `common/tests.rs`, `auth/tests.rs`, `common/asserts.rs`, and the various `api.rs` files that still reference `self.oracles` / `self.assert_oracle()` / `contract.add_oracle` etc. This is expected — those are fixed in Tasks 5-8. Confirm the errors are all in those files and not somewhere unexpected (e.g. `claim/api.rs`, `record/api.rs`'s non-auth logic) before moving on.

- [ ] **Step 6: Commit**

```bash
git add contract/src/lib.rs
git commit -m "feat: annotate Contract with AccessControllable, drop oracles field"
```

(This commit intentionally does not compile standalone — it's a checkpoint inside a larger refactor. If your workflow requires every commit to build, squash Tasks 4-8 into one commit at the end instead.)

---

### Task 5: Add the migrate() function

**Files:**
- Create: `contract/src/migration/mod.rs`
- Modify: `contract/src/lib.rs` (register the module)

**Interfaces:**
- Consumes: `Contract` fields as of Task 4 (post-`oracles`-removal); `crate::auth::Roles`; `contract.acl_init_super_admin`, `contract.acl_get_or_init().grant_role_unchecked(role, &account_id)` (from `near_plugins::AccessControllable`, generated in Task 4).
- Produces: `#[private] #[init(ignore_state)] fn migrate() -> Contract`, exported as a wasm method named `migrate`.

- [ ] **Step 1: Write the migration module**

Create `contract/src/migration/mod.rs`:
```rust
#![allow(deprecated)]

use claim_model::{
    account_record::{AccountRecordLegacy, AccountRecordVersioned},
    Duration, TokensAmount, UnixTimestamp,
};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    near_bindgen,
    store::{LookupMap, UnorderedMap, UnorderedSet, Vector},
    AccountId,
};

use crate::{auth::Roles, Contract, ContractExt};

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
struct OldState {
    token_account_id: AccountId,
    oracles: UnorderedSet<AccountId>,
    claim_period: Duration,
    burn_period: Duration,
    accruals: UnorderedMap<UnixTimestamp, (Vector<TokensAmount>, TokensAmount)>,
    accounts_legacy: LookupMap<AccountId, AccountRecordLegacy>,
    accounts: LookupMap<AccountId, AccountRecordVersioned>,
    is_service_call_running: bool,
    balance_to_burn: TokensAmount,
}

#[near_bindgen]
impl Contract {
    #[private]
    #[init(ignore_state)]
    fn migrate() -> Self {
        let old_state: OldState = env::state_read().expect("Failed to read old state");

        let mut contract = Self {
            token_account_id: old_state.token_account_id,
            claim_period: old_state.claim_period,
            burn_period: old_state.burn_period,
            accruals: old_state.accruals,
            accounts_legacy: old_state.accounts_legacy,
            accounts: old_state.accounts,
            is_service_call_running: old_state.is_service_call_running,
            balance_to_burn: old_state.balance_to_burn,
        };

        contract.acl_init_super_admin(env::current_account_id());

        for oracle in old_state.oracles.iter() {
            contract.acl_get_or_init().grant_role_unchecked(Roles::Oracle, oracle);
            contract.acl_get_or_init().grant_role_unchecked(Roles::BurnManager, oracle);
            contract.acl_get_or_init().grant_role_unchecked(Roles::Maintainer, oracle);
        }

        contract
    }
}
```

- [ ] **Step 2: Register the module**

In `contract/src/lib.rs`, change:
```rust
mod auth;
mod burn;
mod claim;
mod clean;
mod common;
mod config;
mod record;
```
to:
```rust
mod auth;
mod burn;
mod claim;
mod clean;
mod common;
mod config;
mod migration;
mod record;
```

- [ ] **Step 3: Build**

Run: `cargo build --package sweat_claim 2>&1 | head -50`

Expected: this module compiles on its own (it only depends on things Task 4 already produced); remaining errors are still the pre-existing ones from Task 4 Step 5 (assert_oracle call sites, tests) — confirm no *new* errors were introduced by this task.

- [ ] **Step 4: Commit**

```bash
git add contract/src/migration/mod.rs contract/src/lib.rs
git commit -m "feat: add migrate() to grant existing oracles the new ACL roles"
```

---

### Task 6: Replace assert_oracle with role-gated attributes on protected methods

**Files:**
- Modify: `contract/src/common/asserts.rs`
- Modify: `contract/src/burn/api.rs`
- Modify: `contract/src/record/api.rs`
- Modify: `contract/src/config/api.rs`
- Modify: `contract/src/clean/api.rs`

**Interfaces:**
- Consumes: `crate::auth::Roles` (Task 3), `near_plugins::access_control_any`.
- Produces: no new interfaces; behavior change only (panic message changes from `"Unauthorized access! Only oracle can do this!"` to the ACL-generated `"Insufficient permissions for method {name}..."`).

- [ ] **Step 1: Remove assert_oracle**

In `contract/src/common/asserts.rs`, remove the `assert_oracle` method entirely. The file should become:
```rust
use near_sdk::{
    env::{current_account_id, predecessor_account_id},
    require, Gas,
};

use crate::{common::remaining_gas, Contract};

impl Contract {
    pub(crate) fn assert_private() {
        require!(current_account_id() == predecessor_account_id(), "Method is private",);
    }
}

pub(crate) fn assert_enough_gas(required: Gas) {
    require!(remaining_gas() >= required, "Not enough gas for further operations");
}
```

- [ ] **Step 2: Gate burn() with BurnManager**

In `contract/src/burn/api.rs`, change the imports:
```rust
use near_sdk::{json_types::U128, near_bindgen, require, AccountId, PromiseOrValue};
```
to:
```rust
use near_plugins::access_control_any;
use near_sdk::{json_types::U128, near_bindgen, require, AccountId, PromiseOrValue};

use crate::auth::Roles;
```
Then change:
```rust
#[near_bindgen]
impl BurnApi for Contract {
    fn burn(&mut self, amount: Option<U128>) -> PromiseOrValue<U128> {
        self.assert_oracle();

        require!(!self.is_service_call_running, "Another service call is running");
```
to:
```rust
#[near_bindgen]
impl BurnApi for Contract {
    #[access_control_any(roles(Roles::BurnManager))]
    fn burn(&mut self, amount: Option<U128>) -> PromiseOrValue<U128> {
        require!(!self.is_service_call_running, "Another service call is running");
```

- [ ] **Step 3: Gate record_batch_for_hold() with Oracle**

In `contract/src/record/api.rs`, change the imports:
```rust
use near_sdk::{json_types::U128, near_bindgen, AccountId};
```
to:
```rust
use near_plugins::access_control_any;
use near_sdk::{json_types::U128, near_bindgen, AccountId};

use crate::auth::Roles;
```
Then change:
```rust
#[near_bindgen]
impl RecordApi for Contract {
    fn record_batch_for_hold(&mut self, amounts: Vec<(AccountId, U128)>) {
        self.assert_oracle();

        // Default value can be 0 only in tests.
```
to:
```rust
#[near_bindgen]
impl RecordApi for Contract {
    #[access_control_any(roles(Roles::Oracle))]
    fn record_batch_for_hold(&mut self, amounts: Vec<(AccountId, U128)>) {
        // Default value can be 0 only in tests.
```

- [ ] **Step 4: Gate set_claim_period()/set_burn_period() with Maintainer**

In `contract/src/config/api.rs`, change:
```rust
use claim_model::{api::ConfigApi, Duration};
use near_sdk::{near_bindgen, require};

use crate::{Contract, ContractExt};

#[near_bindgen]
impl ConfigApi for Contract {
    fn set_claim_period(&mut self, period: Duration) {
        self.assert_oracle();
        require!(
            period < self.burn_period,
            "Claim period should be less than burn period"
        );

        self.claim_period = period;
    }

    fn set_burn_period(&mut self, period: Duration) {
        self.assert_oracle();
        require!(period > 0, "Burn period should be greater than 0");
        require!(
            period > self.claim_period,
            "Burn period should be greater than claim period"
        );

        self.burn_period = period;
    }
}
```
to:
```rust
use claim_model::{api::ConfigApi, Duration};
use near_plugins::access_control_any;
use near_sdk::{near_bindgen, require};

use crate::{auth::Roles, Contract, ContractExt};

#[near_bindgen]
impl ConfigApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn set_claim_period(&mut self, period: Duration) {
        require!(
            period < self.burn_period,
            "Claim period should be less than burn period"
        );

        self.claim_period = period;
    }

    #[access_control_any(roles(Roles::Maintainer))]
    fn set_burn_period(&mut self, period: Duration) {
        require!(period > 0, "Burn period should be greater than 0");
        require!(
            period > self.claim_period,
            "Burn period should be greater than claim period"
        );

        self.burn_period = period;
    }
}
```

- [ ] **Step 5: Gate clean() with Maintainer**

In `contract/src/clean/api.rs`, change:
```rust
use claim_model::event::{emit, CleanData, EventKind};
use near_sdk::{near_bindgen, AccountId};

use crate::{Contract, ContractExt};

pub trait CleanApi {
    // Invoked via the near_bindgen-generated wasm export; appears unused on the host build.
    #[allow(dead_code)]
    fn clean(&mut self, account_ids: Vec<AccountId>);
}

#[near_bindgen]
impl CleanApi for Contract {
    fn clean(&mut self, account_ids: Vec<AccountId>) {
        self.assert_oracle();

        for account_id in account_ids.clone() {
```
to:
```rust
use claim_model::event::{emit, CleanData, EventKind};
use near_plugins::access_control_any;
use near_sdk::{near_bindgen, AccountId};

use crate::{auth::Roles, Contract, ContractExt};

pub trait CleanApi {
    // Invoked via the near_bindgen-generated wasm export; appears unused on the host build.
    #[allow(dead_code)]
    fn clean(&mut self, account_ids: Vec<AccountId>);
}

#[near_bindgen]
impl CleanApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn clean(&mut self, account_ids: Vec<AccountId>) {
        for account_id in account_ids.clone() {
```

- [ ] **Step 6: Build**

Run: `cargo build --package sweat_claim 2>&1 | head -80`

Expected: remaining errors are only in `auth/api.rs` (Task 7) and the `tests.rs` files (Task 8).

- [ ] **Step 7: Commit**

```bash
git add contract/src/common/asserts.rs contract/src/burn/api.rs contract/src/record/api.rs contract/src/config/api.rs contract/src/clean/api.rs
git commit -m "feat: replace assert_oracle with role-gated access_control_any"
```

---

### Task 7: Update AuthApi — drop oracle management methods, gate unlock_account with Maintainer

**Files:**
- Modify: `model/src/api.rs`
- Modify: `contract/src/auth/api.rs`

**Interfaces:**
- Produces: `claim_model::api::AuthApi` now only declares `fn unlock_account(&mut self, account_id: AccountId);`. `add_oracle`/`remove_oracle`/`get_oracles` no longer exist anywhere; callers must use `near-plugins`'s generated `acl_grant_role`/`acl_revoke_role`/`acl_get_grantees` instead (role name passed as the string `"Oracle"`, `"BurnManager"`, or `"Maintainer"`).

- [ ] **Step 1: Trim the AuthApi trait**

In `model/src/api.rs`, change the `AuthApi` trait from:
```rust
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
```
to:
```rust
pub trait AuthApi {
    /// Unlocks the specified account.
    ///
    /// This method allows a `Maintainer` to unlock an account that may have been locked due to an
    /// error occurring during a cross-contract call. When a cross-contract call fails, the account
    /// might be locked to prevent further operations until the error is resolved.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The ID of the account to be unlocked.
    ///
    /// # Panics
    ///
    /// This method will panic if the caller does not hold the `Maintainer` role, or if the
    /// specified account is not found.
    fn unlock_account(&mut self, account_id: AccountId);
}
```

- [ ] **Step 2: Update the contract's AuthApi impl**

In `contract/src/auth/api.rs`, change:
```rust
use claim_model::api::AuthApi;
use near_sdk::{env::log_str, near_bindgen, require, AccountId};

use crate::{common::AccountAccessor, Contract, ContractExt};

#[near_bindgen]
impl AuthApi for Contract {
    fn add_oracle(&mut self, account_id: AccountId) {
        Self::assert_private();

        require!(self.oracles.insert(account_id.clone()), "Already exists");
        log_str(&format!("Oracle {account_id} was added"));
    }

    fn remove_oracle(&mut self, account_id: AccountId) {
        Self::assert_private();

        require!(self.oracles.remove(&account_id), "No such oracle");
        log_str(&format!("Oracle {account_id} was removed"));
    }

    fn get_oracles(&self) -> Vec<AccountId> {
        self.oracles.iter().cloned().collect()
    }

    fn unlock_account(&mut self, account_id: AccountId) {
        self.assert_oracle();

        let account = self.accounts.get_account_mut(&account_id);
        account.is_locked = false;
    }
}
```
to:
```rust
use claim_model::api::AuthApi;
use near_plugins::access_control_any;
use near_sdk::{near_bindgen, AccountId};

use crate::{auth::Roles, common::AccountAccessor, Contract, ContractExt};

#[near_bindgen]
impl AuthApi for Contract {
    #[access_control_any(roles(Roles::Maintainer))]
    fn unlock_account(&mut self, account_id: AccountId) {
        let account = self.accounts.get_account_mut(&account_id);
        account.is_locked = false;
    }
}
```

- [ ] **Step 3: Build**

Run: `cargo build --package sweat_claim 2>&1 | head -80`

Expected: remaining errors are only in the various `tests.rs` files (Task 8).

- [ ] **Step 4: Commit**

```bash
git add model/src/api.rs contract/src/auth/api.rs
git commit -m "feat: drop add_oracle/remove_oracle/get_oracles, gate unlock_account with Maintainer"
```

---

### Task 8: Fix unit tests

**Files:**
- Modify: `contract/src/common/tests.rs`
- Modify: `contract/src/auth/tests.rs`
- Modify: `contract/src/record/tests.rs`
- Modify: `contract/src/config/tests.rs`
- Modify: `contract/src/clean/tests.rs`

**Interfaces:**
- Consumes: `contract.acl_get_or_init().grant_role_unchecked(role, &account_id)` (Task 4), `crate::auth::Roles`.

- [ ] **Step 1: Update the init_with_oracle test helper**

In `contract/src/common/tests.rs`, change:
```rust
use claim_model::{api::InitApi, Duration, TokensAmount};
use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId};

use crate::Contract;
```
to:
```rust
use claim_model::{api::InitApi, Duration, TokensAmount};
use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId};

use crate::{auth::Roles, Contract};
```
Then change:
```rust
    pub(crate) fn init_with_oracle() -> (Context, Contract, TestAccounts) {
        let (context, mut contract, accounts) = Self::init();
        contract.oracles.insert(accounts.oracle.clone());

        (context, contract, accounts)
    }
```
to:
```rust
    pub(crate) fn init_with_oracle() -> (Context, Contract, TestAccounts) {
        let (context, mut contract, accounts) = Self::init();

        contract.acl_get_or_init().grant_role_unchecked(Roles::Oracle, &accounts.oracle);
        contract.acl_get_or_init().grant_role_unchecked(Roles::BurnManager, &accounts.oracle);
        contract.acl_get_or_init().grant_role_unchecked(Roles::Maintainer, &accounts.oracle);

        (context, contract, accounts)
    }
```

- [ ] **Step 2: Rewrite auth/tests.rs**

Replace the entire contents of `contract/src/auth/tests.rs` (the old file tested `add_oracle`/`remove_oracle`/`get_oracles`, which no longer exist) with:
```rust
#![cfg(test)]

use claim_model::api::{AuthApi, RecordApi};
use near_plugins::AccessControllable;
use near_sdk::json_types::U128;

use crate::{
    auth::Roles,
    common::{tests::Context, AccountAccessor},
};

#[test]
fn grant_oracle_role_by_super_admin() {
    let (mut context, mut contract, accounts) = Context::init();
    context.switch_account(&accounts.owner);

    let granted = contract.acl_grant_role("Oracle".to_string(), accounts.oracle.clone());
    assert_eq!(Some(true), granted);

    let grantees = contract.acl_get_grantees("Oracle".to_string(), 0, 10);
    assert_eq!(grantees, vec![accounts.oracle.clone()]);
}

#[test]
fn grant_oracle_role_by_non_admin_is_noop() {
    let (mut context, mut contract, accounts) = Context::init();
    context.switch_account(&accounts.alice);

    let granted = contract.acl_grant_role("Oracle".to_string(), accounts.oracle.clone());
    assert_eq!(None, granted);

    let grantees = contract.acl_get_grantees("Oracle".to_string(), 0, 10);
    assert!(grantees.is_empty());
}

#[test]
fn revoke_oracle_role_by_super_admin() {
    let (mut context, mut contract, accounts) = Context::init();
    context.switch_account(&accounts.owner);

    contract.acl_grant_role("Oracle".to_string(), accounts.oracle.clone());
    let revoked = contract.acl_revoke_role("Oracle".to_string(), accounts.oracle.clone());
    assert_eq!(Some(true), revoked);

    let grantees = contract.acl_get_grantees("Oracle".to_string(), 0, 10);
    assert!(grantees.is_empty());
}

#[test]
fn unlock_account_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts.get_or_insert_account_mut(&alice_id).is_locked = true;

    contract.unlock_account(alice_id.clone());

    assert!(!contract.accounts.get_account(&alice_id).is_locked);
}

#[test]
#[should_panic(expected = "Account not found")]
fn unlock_not_existing_account_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.unlock_account(alice_id.clone());
}

#[test]
#[should_panic(expected = "Insufficient permissions")]
fn unlock_account_not_by_maintainer() {
    let (mut context, mut contract, accounts) = Context::init_with_oracle();
    let alice_id = accounts.alice;

    context.switch_account(&accounts.oracle);
    contract.record_batch_for_hold(vec![(alice_id.clone(), U128(1_000_000_000))]);
    contract.accounts.get_or_insert_account_mut(&alice_id).is_locked = true;

    context.switch_account(&alice_id);
    contract.unlock_account(alice_id.clone());
}
```
(`record_batch_for_hold` is called by `accounts.oracle` — the only account with the `Oracle` role in `init_with_oracle` — so the account under test only lacks the `Maintainer` role, isolating exactly what this test is meant to check.)

- [ ] **Step 3: Update the panic-message expectations in record/tests.rs**

In `contract/src/record/tests.rs`, change:
```rust
#[should_panic(expected = "Unauthorized access! Only oracle can do this!")]
fn record_by_not_oracle() {
```
to:
```rust
#[should_panic(expected = "Insufficient permissions")]
fn record_by_not_oracle() {
```

- [ ] **Step 4: Update the panic-message expectations in config/tests.rs**

In `contract/src/config/tests.rs`, both occurrences of:
```rust
#[should_panic(expected = "Unauthorized access")]
```
(above `set_claim_period_by_not_oracle` and `set_burn_period_by_not_oracle`) change to:
```rust
#[should_panic(expected = "Insufficient permissions")]
```

- [ ] **Step 5: Update the panic-message expectation in clean/tests.rs**

In `contract/src/clean/tests.rs`, change:
```rust
#[should_panic(expected = "Unauthorized access")]
fn test_clean_single_account_by_not_oracle() {
```
to:
```rust
#[should_panic(expected = "Insufficient permissions")]
fn test_clean_single_account_by_not_oracle() {
```

- [ ] **Step 6: Build and run unit tests**

Run: `cargo test --package sweat_claim 2>&1 | tail -60`

Expected: all tests compile and pass. If `acl_get_grantees`/`acl_grant_role`/`acl_revoke_role` signatures don't match what's used above (double-check against the generated trait in `near-plugins/src/access_controllable.rs` for the installed `0.5.3` version if this fails), adjust the auth/tests.rs code to match the actual generated signatures.

- [ ] **Step 7: Run lint**

Run: `make lint`

Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add contract/src/common/tests.rs contract/src/auth/tests.rs contract/src/record/tests.rs contract/src/config/tests.rs contract/src/clean/tests.rs
git commit -m "test: update unit tests for the ACL role model"
```

---

### Task 9: Update integration-tests wiring to grant roles instead of add_oracle

**Files:**
- Modify: `integration-tests/tests/common/prepare.rs`

**Interfaces:**
- Consumes: the deployed `claim` contract's `acl_grant_role` method (JSON args: `{"role": "<RoleName>", "account_id": "<id>"}`), callable only by an account with admin permission for that role — the `claim` contract itself, right after `init`, per Task 4's `acl_init_super_admin(current_account_id())`.

- [ ] **Step 1: Replace the claim-side oracle wiring**

In `integration-tests/tests/common/prepare.rs`, change:
```rust
    // Oracle wiring: manager may defer on the token and operate the claim contract;
    // the token contract may call `record_batch_for_hold` on the claim contract.
    sweat
        .call("add_oracle")
        .args_json(json!({ "account_id": manager.id() }))
        .transact()
        .await?
        .into_result()?;
    claim
        .call("add_oracle")
        .args_json(json!({ "account_id": sweat.id() }))
        .transact()
        .await?
        .into_result()?;
    claim
        .call("add_oracle")
        .args_json(json!({ "account_id": manager.id() }))
        .transact()
        .await?
        .into_result()?;
```
to:
```rust
    // Oracle wiring: manager may defer on the token and operate the claim contract;
    // the token contract may call `record_batch_for_hold` on the claim contract.
    // `claim`'s own account is the only account with `acl_grant_role` permission
    // (it made itself super-admin in `init`), so these calls must be signed by `claim` itself.
    sweat
        .call("add_oracle")
        .args_json(json!({ "account_id": manager.id() }))
        .transact()
        .await?
        .into_result()?;
    claim
        .call("acl_grant_role")
        .args_json(json!({ "role": "Oracle", "account_id": sweat.id() }))
        .transact()
        .await?
        .into_result()?;
    for role in ["Oracle", "BurnManager", "Maintainer"] {
        claim
            .call("acl_grant_role")
            .args_json(json!({ "role": role, "account_id": manager.id() }))
            .transact()
            .await?
            .into_result()?;
    }
```
(The `sweat.call("add_oracle", ...)` line is unchanged — `sweat` is the SWEAT token contract, out of scope for this change, and still uses its own oracle model.)

- [ ] **Step 2: Verify the doc comment on Context is still accurate**

The doc comment above `pub struct Context` says `manager` is "an oracle on both" — that's still true (manager now holds all three roles on `claim`), no change needed there.

- [ ] **Step 3: Build the integration test contract and run the existing suite**

Run: `make build-integration && cd integration-tests && cargo test && cd ..`

Expected: all existing integration tests still pass (they don't yet test role separation, but they exercise the happy path through `prepare_contract`, which now uses the ACL wiring).

- [ ] **Step 4: Commit**

```bash
git add integration-tests/tests/common/prepare.rs
git commit -m "test: wire integration test oracle roles via acl_grant_role"
```

---

### Task 10: Exhaustive authorized/unauthorized integration tests per protected method

**Files:**
- Create: `integration-tests/tests/access_control.rs`

**Interfaces:**
- Consumes: `common::prepare::prepare_contract`, `common::panic::PanicFinder` (existing helpers, unchanged).

- [ ] **Step 1: Write the test file**

Create `integration-tests/tests/access_control.rs`:
```rust
use near_workspaces::types::NearToken;
use serde_json::json;

mod common;
use common::{panic::PanicFinder, prepare::prepare_contract};

const INSUFFICIENT_PERMISSIONS: &str = "Insufficient permissions";

#[tokio::test]
#[tracing::instrument]
async fn record_batch_for_hold_by_oracle_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn record_batch_for_hold_by_non_oracle_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn burn_by_burn_manager_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn burn_by_non_burn_manager_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_claim_period_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "set_claim_period")
        .args_json(json!({ "period": 100u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_claim_period_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "set_claim_period")
        .args_json(json!({ "period": 100u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_burn_period_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .manager
        .call(context.claim.id(), "set_burn_period")
        .args_json(json!({ "period": 10_000u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn set_burn_period_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "set_burn_period")
        .args_json(json!({ "period": 10_000u32 }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn clean_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .manager
        .call(context.claim.id(), "clean")
        .args_json(json!({ "account_ids": [context.alice.id()] }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn clean_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;

    let result = context
        .alice
        .call(context.claim.id(), "clean")
        .args_json(json!({ "account_ids": [context.alice.id()] }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn unlock_account_by_maintainer_succeeds() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .manager
        .call(context.claim.id(), "unlock_account")
        .args_json(json!({ "account_id": context.alice.id() }))
        .transact()
        .await?
        .into_result();

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn unlock_account_by_non_maintainer_panics() -> anyhow::Result<()> {
    let context = prepare_contract(None, None).await?;
    context
        .manager
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result()?;

    let result = context
        .alice
        .call(context.claim.id(), "unlock_account")
        .args_json(json!({ "account_id": context.alice.id() }))
        .transact()
        .await?
        .into_result();

    assert!(result.has_panic(INSUFFICIENT_PERMISSIONS));
    Ok(())
}

#[tokio::test]
#[tracing::instrument]
async fn role_holder_without_the_right_role_is_still_rejected() -> anyhow::Result<()> {
    // `manager` holds Oracle/BurnManager/Maintainer (see prepare.rs), so use a
    // freshly-granted single-role account to prove roles don't cross-authorize
    // each other's methods.
    let context = prepare_contract(None, None).await?;
    let root = context.worker.root_account()?;
    let oracle_only = root
        .create_subaccount("oracleonly")
        .initial_balance(NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;
    context
        .claim
        .call("acl_grant_role")
        .args_json(json!({ "role": "Oracle", "account_id": oracle_only.id() }))
        .transact()
        .await?
        .into_result()?;

    // Oracle role holder can record...
    let record_result = oracle_only
        .call(context.claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[context.alice.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();
    assert!(record_result.is_ok());

    // ...but not burn, which requires BurnManager.
    let burn_result = oracle_only
        .call(context.claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(burn_result.has_panic(INSUFFICIENT_PERMISSIONS));

    Ok(())
}
```

- [ ] **Step 2: Run the new tests**

Run: `make build-integration && cd integration-tests && cargo test --test access_control && cd ..`

Expected: all tests pass. If `set_claim_period`/`set_burn_period` fail with `"Claim period should be less than burn period"` or similar business-rule panics rather than reaching the ACL check, adjust the periods used in the "succeeds" tests to satisfy `claim_period < burn_period` given `prepare_contract`'s defaults (`CLAIM_PERIOD = 30 * 60`, `BURN_PERIOD = 3 * 60 * 60`, from `integration-tests/tests/common/prepare.rs`) — e.g. use `period: 100` for claim (well under the 3h burn period) and `period: 10_000` for burn (well over the 30min claim period), which is already what's used above.

- [ ] **Step 3: Commit**

```bash
git add integration-tests/tests/access_control.rs
git commit -m "test: add exhaustive authorized/unauthorized coverage for ACL-protected methods"
```

---

### Task 11: Migration integration test

**Files:**
- Create: `integration-tests/tests/migration.rs`

**Interfaces:**
- Consumes: `integration-tests/res/sweat_claim_pre_acl.wasm` (Task 1), `res/sweat_claim.wasm` (built by `make build-integration`), `common::prepare` helpers (`storage_deposit`, wasm path resolution pattern from `prepare.rs`).

- [ ] **Step 1: Write the migration test**

Create `integration-tests/tests/migration.rs`:
```rust
use std::path::PathBuf;

use anyhow::Result;
use serde_json::json;

mod common;

fn pre_acl_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("res").join("sweat_claim_pre_acl.wasm")
}

fn new_wasm_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("res")
        .join("sweat_claim.wasm")
}

#[tokio::test]
#[tracing::instrument]
async fn oracle_survives_migration_to_acl() -> Result<()> {
    common::helpers::init_tracing();

    let worker = near_workspaces::sandbox().await?;
    let root = worker.root_account()?;

    // Deploy the pre-ACL contract and register an oracle the old way.
    let pre_acl_bytes = std::fs::read(pre_acl_wasm_path())?;
    let claim = worker.dev_deploy(&pre_acl_bytes).await?;

    let sweat_placeholder = root
        .create_subaccount("token")
        .initial_balance(near_workspaces::types::NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;

    claim
        .call("init")
        .args_json(json!({ "token_account_id": sweat_placeholder.id() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    let oracle = root
        .create_subaccount("oracle")
        .initial_balance(near_workspaces::types::NearToken::from_near(5))
        .transact()
        .await?
        .into_result()?;

    claim
        .call("add_oracle")
        .args_json(json!({ "account_id": oracle.id() }))
        .transact()
        .await?
        .into_result()?;

    // Upgrade to the ACL-based contract and migrate state.
    let new_bytes = std::fs::read(new_wasm_path())?;
    claim.as_account().deploy(&new_bytes).await?.into_result()?;
    claim
        .call("migrate")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // The oracle should still be able to call all three role-gated actions.
    let record_result = oracle
        .call(claim.id(), "record_batch_for_hold")
        .args_json(json!({ "amounts": [[oracle.id(), "1000"]] }))
        .transact()
        .await?
        .into_result();
    assert!(record_result.is_ok(), "Oracle role not migrated: {record_result:?}");

    let burn_result = oracle
        .call(claim.id(), "burn")
        .args_json(json!({ "amount": null }))
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(burn_result.is_ok(), "BurnManager role not migrated: {burn_result:?}");

    let config_result = oracle
        .call(claim.id(), "set_claim_period")
        .args_json(json!({ "period": 100u32 }))
        .transact()
        .await?
        .into_result();
    assert!(config_result.is_ok(), "Maintainer role not migrated: {config_result:?}");

    Ok(())
}
```

- [ ] **Step 2: Run it**

Run: `make build-integration && cd integration-tests && cargo test --test migration && cd ..`

Expected: passes. If `deploy` on `claim.as_account()` isn't the right near-workspaces API for redeploying an already-`dev_deploy`ed contract in `0.22`, check `Account::deploy` vs `Contract::as_account().deploy` in the `near-workspaces` docs for the installed version and adjust — the intent is: same account, new wasm bytes, then call `migrate`.

- [ ] **Step 3: Commit**

```bash
git add integration-tests/tests/migration.rs
git commit -m "test: verify oracles survive migration to the ACL role model"
```

---

### Task 12: Full verification pass

**Files:** None (verification only).

- [ ] **Step 1: Run the full unit test suite**

Run: `make test`

Expected: all pass.

- [ ] **Step 2: Run lint**

Run: `make lint`

Expected: clean.

- [ ] **Step 3: Run the full integration suite**

Run: `make integration`

Expected: all pass, including the new `access_control.rs` and `migration.rs` files and the existing `burn.rs`, `claim.rs`, `direct_calls.rs`, `gas.rs`.

- [ ] **Step 4: Confirm the reproducible build still works**

Run: `make build-in-docker`

Expected: succeeds and produces `res/sweat_claim.wasm` using the new `sourcescan/cargo-near:0.21.1-rust-1.93.0` image. (`make hash` / `scripts/check-contract-hash.sh` were removed in Task 2's follow-up fix — cargo-near 0.21.1 embeds the git commit SHA into the wasm's NEP-330 metadata, making a byte-identical hash comparison across commits structurally impossible. No local hash-check step replaces this; run `make dock` to refresh `res/sweat_claim.wasm` and `git add` it if the docker build produces new bytes.)
