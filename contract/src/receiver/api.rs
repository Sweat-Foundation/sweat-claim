use claim_model::{api::RecordApi, TokensAmount};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{
    env,
    json_types::U128,
    near_bindgen, require,
    serde::{Deserialize, Serialize},
    serde_json, AccountId, PromiseOrValue,
};

use crate::{Contract, ContractExt};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data", rename_all = "snake_case")]
pub enum FtMessage {
    Record(RecordMessage),
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RecordMessage {
    pub amounts: Vec<(AccountId, U128)>,
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        self.assert_oracle(&sender_id);

        let ft_message: FtMessage = serde_json::from_str(&msg).expect("Unable to deserialize msg");

        match ft_message {
            FtMessage::Record(message) => {
                let asset_symbol = self
                    .get_asset_symbol(env::predecessor_account_id())
                    .expect("Asset is not registered");

                let total: TokensAmount = message.amounts.iter().map(|(_, amount)| amount.0).sum();
                require!(total == amount.0, "Amounts are not equal to the transferred amount");

                self.record_batch_for_hold(message.amounts, Some(asset_symbol));
            }
        }

        PromiseOrValue::Value(0.into())
    }
}
