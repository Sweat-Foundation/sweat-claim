use claim_model::{
    api::AssetApi,
    asset::{Asset, AssetVersioned},
    AssetSymbol,
};
use near_sdk::{near_bindgen, AccountId};

use crate::{Contract, ContractExt};

#[near_bindgen]
impl AssetApi for Contract {
    fn get_assets(&self) -> Vec<(AssetSymbol, Asset)> {
        self.extra_tokens
            .iter()
            .map(|(key, value)| (key.clone(), Asset::from(value.clone())))
            .collect()
    }

    fn register_asset(&mut self, asset: AssetSymbol, account_id: AccountId) {
        self.assert_oracle();

        self.extra_tokens.insert(asset, AssetVersioned::new(account_id));
    }
}
