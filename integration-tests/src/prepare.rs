use std::ops::{Deref, DerefMut};

use anyhow::Result;
use claim_model::{
    api::{AuthApiIntegration, ClaimContract, ConfigApiIntegration, InitApiIntegration},
    Duration,
};
use near_sdk::json_types::U128;
use near_workspaces::Account;
use nitka::misc::{load_wasm, ToNear};
use sweat_model::{StorageManagementIntegration, SweatApiIntegration, SweatContract};

const FT_CONTRACT: &str = "sweat";
const SWEAT_CLAIM: &str = "sweat_claim";

pub const CLAIM_PERIOD: Duration = 30 * 60;
pub const BURN_PERIOD: Duration = 3 * 60 * 60;

pub type Context = nitka::context::Context<near_workspaces::network::Sandbox>;

pub struct ClaimContext {
    context: Context,
    /// Path to claim contract wasm binary
    claim_contract: &'static str,
}

impl Deref for ClaimContext {
    type Target = Context;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl DerefMut for ClaimContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

pub trait IntegrationContext {
    async fn manager(&mut self) -> Result<Account>;
    async fn alice(&mut self) -> Result<Account>;
    async fn redeploy_claim_contract(&mut self, path: &'static str) -> Result<()>;
    fn sweat_claim(&self) -> ClaimContract;
    fn ft_contract(&self) -> SweatContract;
}

impl IntegrationContext for ClaimContext {
    async fn manager(&mut self) -> Result<Account> {
        self.account("manager").await
    }

    async fn alice(&mut self) -> Result<Account> {
        self.account("alice").await
    }

    async fn redeploy_claim_contract(&mut self, path: &'static str) -> Result<()> {
        let wasm = load_wasm(path);
        let contract = self
            .sweat_claim()
            .contract
            .as_account()
            .deploy(&wasm)
            .await?
            .into_result()?;

        let key = self.claim_contract;

        self.contracts.insert(key, contract);

        Ok(())
    }

    fn sweat_claim(&self) -> ClaimContract {
        ClaimContract {
            contract: &self.contracts[self.claim_contract],
        }
    }

    fn ft_contract(&self) -> SweatContract {
        SweatContract {
            contract: &self.contracts[FT_CONTRACT],
        }
    }
}

pub async fn prepare_custom_contract(
    claim_period: Option<Duration>,
    burn_period: Option<Duration>,
    claim_contract: Option<&'static str>,
) -> Result<ClaimContext> {
    let claim_contract = claim_contract.unwrap_or(SWEAT_CLAIM);

    let context = Context::new(&[FT_CONTRACT, claim_contract], true, "build-integration".into()).await?;

    let mut context = ClaimContext {
        context,
        claim_contract,
    };

    let manager = context.manager().await?;
    let alice = context.alice().await?;

    context.ft_contract().new(".u.sweat.testnet".to_string().into()).await?;
    context
        .sweat_claim()
        .init(context.ft_contract().contract.as_account().to_near())
        .await?;

    context.ft_contract().add_oracle(&manager.to_near()).await?;

    context
        .sweat_claim()
        .add_oracle(context.ft_contract().contract.as_account().to_near())
        .await?;
    context.sweat_claim().add_oracle(manager.to_near()).await?;

    context
        .ft_contract()
        .storage_deposit(context.sweat_claim().contract.as_account().to_near().into(), None)
        .await?;

    context
        .ft_contract()
        .storage_deposit(alice.to_near().into(), None)
        .await?;
    context
        .ft_contract()
        .tge_mint(&alice.to_near(), U128(100_000_000))
        .await?;

    context
        .sweat_claim()
        .set_claim_period(claim_period.unwrap_or(CLAIM_PERIOD))
        .with_user(&manager)
        .await?;
    context
        .sweat_claim()
        .set_burn_period(burn_period.unwrap_or(BURN_PERIOD))
        .with_user(&manager)
        .await?;

    Ok(context)
}

pub async fn prepare_contract(claim_period: Option<Duration>, burn_period: Option<Duration>) -> Result<ClaimContext> {
    prepare_custom_contract(claim_period, burn_period, None).await
}
