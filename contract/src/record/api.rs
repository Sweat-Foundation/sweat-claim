use claim_model::{
    api::RecordApi,
    event::{emit, EventKind::Record, RecordAmountDetailed, RecordData},
    AssetSymbol,
};
use near_sdk::{env, json_types::U128, near_bindgen, require, AccountId};

use crate::{
    common::{now_seconds, AccountAccessor},
    Contract, ContractExt, NEAR_SYMBOL,
};

#[near_bindgen]
impl RecordApi for Contract {
    #[payable]
    fn record_batch_for_hold(&mut self, amounts: Vec<(AccountId, U128)>, asset: Option<AssetSymbol>) {
        self.assert_oracle();

        if let Some(asset) = asset {
            if asset == NEAR_SYMBOL {
                let total_amount: u128 = amounts.iter().map(|value| value.1 .0).sum();
                require!(
                    total_amount == env::attached_deposit(),
                    "Amounts do not match attached deposit"
                );
            } else {
                require!(self.extra_tokens.contains_key(&asset), "Asset is not supported");
            }

            self.record_extra_token_balances(asset, amounts);
        } else {
            self.record_main_token_balances(amounts);
        }
    }
}

impl Contract {
    fn record_main_token_balances(&mut self, amounts: Vec<(AccountId, U128)>) {
        // Default value can be 0 only in tests.
        let claimable_window_start = self.get_claimable_window_start();
        let mut event_data = RecordData::new(now_seconds());

        for (account_id, amount) in amounts {
            let account = self.accounts.get_or_insert_account_mut(&account_id);
            let balance_to_burn = account.get_balance_to_burn(self.burn_period, claimable_window_start);

            if balance_to_burn > 0 {
                self.balance_to_burn += balance_to_burn;

                account.balance -= balance_to_burn;
                account.burn_since = claimable_window_start;
            }

            account.balance += amount.0;

            event_data.amounts.push((
                account_id.clone(),
                RecordAmountDetailed {
                    credited: amount,
                    burnt: U128(balance_to_burn),
                },
            ));
        }

        emit(Record(event_data));
    }

    fn record_extra_token_balances(&mut self, asset: AssetSymbol, amounts: Vec<(AccountId, U128)>) {
        for (account_id, amount) in amounts {
            let account = self.accounts.get_or_insert_account_mut(&account_id);

            let entry = account.extra_balances.entry(asset.clone()).or_default();
            *entry += amount.0;
        }
    }
}
