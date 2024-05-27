use claim_model::{
    account_record::AccountRecord, api::ClaimExtraApi, asset::AssetVersioned, AssetSymbol, ClaimAllResultView,
    TokensAmount,
};
use near_sdk::{
    env, env::panic_str, ext_contract, is_promise_success, json_types::U128, near_bindgen, require, serde_json::json,
    AccountId, Gas, Promise, PromiseOrValue,
};

use crate::{common::AccountAccessor, Contract, ContractExt, DEFAULT_TOKEN_SYMBOL, NEAR_SYMBOL};

#[near_bindgen]
impl ClaimExtraApi for Contract {
    fn claim_extra(&mut self, assets: Option<Vec<AssetSymbol>>) -> PromiseOrValue<ClaimAllResultView> {
        let account_id = env::predecessor_account_id();
        let account = self.accounts.get_account(&account_id);

        require!(account.is_enabled, "Account is disabled");
        require!(!account.is_locked, "Another operation is running");

        let assets: Vec<AssetSymbol> = assets.unwrap_or_else(|| account.get_assets());
        let claimable_assets: Vec<AssetSymbol> = assets.into_iter().filter(|asset| self.is_claimable(asset)).collect();

        if claimable_assets.is_empty() {
            PromiseOrValue::Value(ClaimAllResultView::new())
        } else {
            let account = self.accounts.get_account_mut(&account_id);
            account.is_locked = true;

            let mut result = ClaimAllResultView::new();

            let claimable_assets: Vec<(AssetSymbol, TokensAmount)> = claimable_assets
                .into_iter()
                .map(|asset| {
                    (
                        asset.clone(),
                        account.extra_balances.remove(&asset).expect("Unable to remove asset"),
                    )
                })
                .collect();
            let ((asset_symbol, amount), tail) = claimable_assets.split_first().expect("Unable to split tail");

            self.transfer(&mut result, account_id, asset_symbol.clone(), *amount, tail.to_vec())
                .into()
        }
    }
}

const GAS_FOR_TRANSFER: Gas = Gas(5 * Gas::ONE_TERA.0);
const GAS_FOR_TRANSFER_CALLBACK: Gas = Gas(5 * Gas::ONE_TERA.0);

impl Contract {
    fn is_claimable(&self, asset: &AssetSymbol) -> bool {
        if asset == DEFAULT_TOKEN_SYMBOL || asset == NEAR_SYMBOL {
            return true;
        }

        if let Some(asset) = self.extra_tokens.get(asset) {
            let AssetVersioned::V1(asset) = asset;
            asset.is_enabled
        } else {
            false
        }
    }

    fn transfer(
        &mut self,
        result: &mut ClaimAllResultView,
        receiver_id: AccountId,
        asset_symbol: AssetSymbol,
        amount: TokensAmount,
        tail: Vec<(AssetSymbol, TokensAmount)>,
    ) -> PromiseOrValue<ClaimAllResultView> {
        if asset_symbol == AssetSymbol::from(NEAR_SYMBOL) {
            self.transfer_near(result, receiver_id, amount, tail)
        } else {
            self.transfer_ft(result, receiver_id, asset_symbol, amount, tail)
        }
    }

    fn transfer_ft(
        &mut self,
        result: &mut ClaimAllResultView,
        receiver_id: AccountId,
        asset_symbol: AssetSymbol,
        amount: TokensAmount,
        tail: Vec<(AssetSymbol, TokensAmount)>,
    ) -> PromiseOrValue<ClaimAllResultView> {
        let callback = claim_all_callback::ext(env::current_account_id())
            .with_static_gas(GAS_FOR_TRANSFER_CALLBACK)
            .on_transfer(result, receiver_id.clone(), asset_symbol, amount, tail);

        let args = json!({
            "receiver_id": receiver_id.clone(),
            "amount": amount.to_string(),
            "memo": "",
        })
        .to_string()
        .as_bytes()
        .to_vec();

        Promise::new(self.token_account_id.clone())
            .function_call("ft_transfer".to_string(), args, 1, GAS_FOR_TRANSFER)
            .then(callback)
            .into()
    }

    fn transfer_near(
        &mut self,
        result: &mut ClaimAllResultView,
        receiver_id: AccountId,
        amount: TokensAmount,
        tail: Vec<(AssetSymbol, TokensAmount)>,
    ) -> PromiseOrValue<ClaimAllResultView> {
        let callback = claim_all_callback::ext(env::current_account_id())
            .with_static_gas(GAS_FOR_TRANSFER_CALLBACK)
            .on_transfer(
                result,
                receiver_id.clone(),
                AssetSymbol::from(NEAR_SYMBOL),
                amount,
                tail,
            );

        Promise::new(receiver_id).transfer(amount.into()).then(callback).into()
    }
}

#[ext_contract(claim_all_callback)]
trait ClaimAllCallbacks {
    fn on_transfer(
        &mut self,
        result: &mut ClaimAllResultView,
        receiver_id: AccountId,
        asset_symbol: AssetSymbol,
        amount: TokensAmount,
        tail: Vec<(AssetSymbol, TokensAmount)>,
    ) -> PromiseOrValue<ClaimAllResultView>;
}

#[near_bindgen]
impl ClaimAllCallbacks for Contract {
    #[private]
    fn on_transfer(
        &mut self,
        result: &mut ClaimAllResultView,
        receiver_id: AccountId,
        asset_symbol: AssetSymbol,
        amount: TokensAmount,
        tail: Vec<(AssetSymbol, TokensAmount)>,
    ) -> PromiseOrValue<ClaimAllResultView> {
        if is_promise_success() {
            result.claimed.insert(asset_symbol, U128(amount));
        } else {
            self.accounts
                .get_account_mut(&receiver_id)
                .extra_balances
                .insert(asset_symbol.clone(), amount);

            result.failed.push(asset_symbol);
        }

        if tail.is_empty() {
            self.accounts.get_account_mut(&receiver_id).is_locked = false;

            PromiseOrValue::Value(result.clone())
        } else {
            let ((asset_symbol, amount), tail) = tail.split_first().expect("Unable to split tail");

            self.transfer(result, receiver_id, asset_symbol.clone(), *amount, tail.to_vec())
        }
    }
}

trait AssetsAccessor {
    fn get_assets(&self) -> Vec<AssetSymbol>;
}

impl AssetsAccessor for AccountRecord {
    fn get_assets(&self) -> Vec<AssetSymbol> {
        let mut assets: Vec<AssetSymbol> = self.extra_balances.keys().cloned().collect();
        assets.push(AssetSymbol::from(NEAR_SYMBOL));

        assets
    }
}
