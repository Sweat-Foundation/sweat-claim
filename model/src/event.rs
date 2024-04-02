use near_sdk::{env, json_types::U128, log, serde::Serialize, serde_json, AccountId};

use crate::UnixTimestamp;

pub const PACKAGE_NAME: &str = "sweat_claim";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Debug)]
#[serde(
    crate = "near_sdk::serde",
    tag = "event",
    content = "data",
    rename_all = "snake_case"
)]
pub enum EventKind {
    Burn(BurnData),
    Claim(ClaimData),
    Clean(CleanData),
    Record(RecordData),
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct BurnData {
    pub burnt_amount: U128,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ClaimData {
    pub account_id: AccountId,
    pub claimed: U128,
    pub burnt: U128,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct CleanData {
    pub account_ids: Vec<AccountId>,
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct RecordData {
    pub timestamp: UnixTimestamp,
    pub amounts: Vec<(AccountId, U128)>,
}

impl RecordData {
    pub fn new(timestamp: UnixTimestamp) -> Self {
        Self {
            timestamp,
            amounts: vec![],
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
struct SweatClaimEvent {
    standard: &'static str,
    version: &'static str,
    #[serde(flatten)]
    event_kind: EventKind,
}

impl From<EventKind> for SweatClaimEvent {
    fn from(event_kind: EventKind) -> Self {
        Self {
            standard: PACKAGE_NAME,
            version: VERSION,
            event_kind,
        }
    }
}

pub fn emit(event: EventKind) {
    log!(SweatClaimEvent::from(event).to_json_event_string());
}

impl SweatClaimEvent {
    fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|err| env::panic_str(&format!("Failed to serialize SweatClaimEvent: {err}")))
    }

    fn to_json_event_string(&self) -> String {
        format!("EVENT_JSON:{}", self.to_json_string())
    }
}

#[cfg(test)]
mod test {
    use near_sdk::json_types::U128;

    use crate::event::{BurnData, EventKind, SweatClaimEvent};

    #[test]
    fn event_to_string() {
        assert_eq!(
            strip(
                SweatClaimEvent::from(EventKind::Burn(BurnData {
                    burnt_amount: U128(100_000_000),
                }))
                .to_json_event_string()
                .as_str()
            ),
            strip(
                r#"EVENT_JSON:{
                "standard": "sweat_claim",
                "version": "1.0.0",
                "event": "burn",
                "data": {
                  "burnt_amount": "100000000"
                }}"#
            )
        )
    }

    fn strip(s: &str) -> String {
        let without_newlines: String = s.chars().filter(|&c| c != '\n').collect();
        let mut previous_char = ' ';
        let result: String = without_newlines
            .chars()
            .filter(|&c| {
                let keep = !(c == ' ' && previous_char == ' ');
                previous_char = c;
                keep
            })
            .collect();
        result
    }
}
