use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::{TempestError, TempestResult};
use crate::ids::AssetId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    id: AssetId,
    symbol: String,
    decimals: u8,
    settlement_group: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssetBook {
    assets: BTreeMap<AssetId, Asset>,
}

impl Asset {
    pub fn new(
        id: impl Into<AssetId>,
        symbol: impl Into<String>,
        decimals: u8,
        settlement_group: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            symbol: symbol.into(),
            decimals,
            settlement_group: settlement_group.into(),
        }
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn settlement_group(&self) -> &str {
        &self.settlement_group
    }
}

impl AssetBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, asset: Asset) {
        self.assets.insert(asset.id.clone(), asset);
    }

    pub fn get(&self, asset: &AssetId) -> TempestResult<&Asset> {
        self.assets
            .get(asset)
            .ok_or_else(|| TempestError::AssetNotFound(asset.to_string()))
    }

    pub fn contains(&self, asset: &AssetId) -> bool {
        self.assets.contains_key(asset)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn symbols(&self) -> Vec<String> {
        self.assets
            .values()
            .map(|asset| asset.symbol.clone())
            .collect()
    }
}
