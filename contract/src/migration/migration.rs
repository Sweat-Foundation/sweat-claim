#![allow(deprecated)]

use claim_model::account_record::AccountRecordVersioned;
use near_sdk::AccountId;

use crate::Contract;

impl Contract {
    pub(crate) fn migrate_account_if_outdated(&mut self, account_id: &AccountId) {
        if let Some(account) = self.accounts_legacy.get(account_id) {
            let last_top_up_at = account
                .accruals
                .iter()
                .map(|(datetime, _)| datetime)
                .max()
                .map(|value| value.clone())
                .unwrap_or_default()
                .clone();

            let balance = account
                .accruals
                .iter()
                .map(|(datetime, index)| {
                    self.accruals
                        .get(&datetime)
                        .map(|(accruals, _)| accruals.get(index.clone()).unwrap_or(&0).clone())
                        .unwrap_or_default()
                })
                .sum();

            let account = AccountRecordVersioned::from(account, balance, last_top_up_at);
            self.accounts.insert(account_id.clone(), account);

            self.accounts_legacy.remove(account_id);
        }
    }
}
