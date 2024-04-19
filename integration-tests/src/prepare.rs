use claim_model::{
    api::{AuthApiIntegration, ClaimContract, ConfigApiIntegration, InitApiIntegration},
    Duration,
};
use near_sdk::json_types::U128;
use near_workspaces::Account;
use nitka::misc::ToNear;
use sweat_model::{StorageManagementIntegration, SweatApiIntegration, SweatContract};

const FT_CONTRACT: &str = "sweat";
const SWEAT_CLAIM: &str = "sweat_claim";

pub const CLAIM_PERIOD: Duration = 30 * 60;
pub const BURN_PERIOD: Duration = 3 * 60 * 60;

pub type Context = nitka::context::Context<near_workspaces::network::Sandbox>;

pub trait IntegrationContext {
    async fn manager(&mut self) -> anyhow::Result<Account>;
    async fn alice(&mut self) -> anyhow::Result<Account>;
    fn sweat_claim(&self) -> ClaimContract;
    fn ft_contract(&self) -> SweatContract;
}

impl IntegrationContext for Context {
    async fn manager(&mut self) -> anyhow::Result<Account> {
        self.account("manager").await
    }

    async fn alice(&mut self) -> anyhow::Result<Account> {
        self.account("alice").await
    }

    fn sweat_claim(&self) -> ClaimContract {
        ClaimContract {
            contract: &self.contracts[SWEAT_CLAIM],
        }
    }

    fn ft_contract(&self) -> SweatContract {
        SweatContract {
            contract: &self.contracts[FT_CONTRACT],
        }
    }
}

pub async fn prepare_contract(
    claim_period: Option<Duration>,
    burn_period: Option<Duration>,
) -> anyhow::Result<Context> {
    let mut context = Context::new(&[FT_CONTRACT, SWEAT_CLAIM], true, "build-integration".into()).await?;
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
