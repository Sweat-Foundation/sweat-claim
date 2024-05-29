use std::{
    io::Error,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "release-api")]
use near_sdk::AccountId;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::Serialize,
};
#[cfg(not(feature = "release-api"))]
use nitka::AccountId;

pub type Asset = AssetV1;

#[derive(BorshSerialize, Serialize, Clone)]
pub enum AssetVersioned {
    V1(AssetV1),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Clone)]
pub struct AssetV1 {
    pub account_id: AccountId,
    pub is_enabled: bool,
}
impl BorshDeserialize for AssetVersioned {
    fn deserialize(buf: &mut &[u8]) -> Result<Self, Error> {
        let v1: AssetV1 = BorshDeserialize::deserialize(buf)?;

        Ok(AssetVersioned::V1(v1))
    }
}

impl Deref for AssetVersioned {
    type Target = Asset;

    fn deref(&self) -> &Self::Target {
        match self {
            // Guaranteed by `BorshDeserialize` implementation
            Self::V1(asset) => asset,
        }
    }
}

impl DerefMut for AssetVersioned {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            // Guaranteed by `BorshDeserialize` implementation
            Self::V1(asset) => asset,
        }
    }
}

impl From<AssetVersioned> for Asset {
    fn from(value: AssetVersioned) -> Self {
        match value {
            AssetVersioned::V1(value) => value,
        }
    }
}

impl AssetVersioned {
    pub fn new(account_id: AccountId) -> Self {
        Self::V1(Asset::new(account_id))
    }
}

impl Asset {
    fn new(account_id: AccountId) -> Self {
        Self {
            account_id,
            is_enabled: true,
        }
    }
}
