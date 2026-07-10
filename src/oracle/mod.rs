use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::{Amount, Bps};
use crate::error::{TempestError, TempestResult};
use crate::ids::MarketId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MarketObservation {
    pub market: MarketId,
    pub slot: u64,
    pub price_micros: u128,
    pub depth: Amount,
    pub liquidity_score: u32,
    pub rolling_volatility_bps: Bps,
    pub version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MarketMove {
    pub market: MarketId,
    pub slot: u64,
    pub previous_price_micros: u128,
    pub next_price_micros: u128,
    pub previous_depth: Amount,
    pub next_depth: Amount,
    pub price_move_bps: Bps,
    pub depth_move_bps: Bps,
    pub rolling_volatility_bps: Bps,
    pub version: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OracleBook {
    observations: BTreeMap<MarketId, MarketObservation>,
}

impl MarketObservation {
    pub fn new(
        market: impl Into<MarketId>,
        slot: u64,
        price_micros: u128,
        depth: Amount,
        liquidity_score: u32,
        rolling_volatility_bps: Bps,
    ) -> Self {
        Self {
            market: market.into(),
            slot,
            price_micros,
            depth,
            liquidity_score,
            rolling_volatility_bps,
            version: 1,
        }
    }

    pub fn market(&self) -> &MarketId {
        &self.market
    }

    pub fn with_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }
}

impl OracleBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, observation: MarketObservation) {
        self.observations
            .insert(observation.market.clone(), observation);
    }

    pub fn get(&self, market: &MarketId) -> TempestResult<&MarketObservation> {
        self.observations
            .get(market)
            .ok_or_else(|| TempestError::OracleMissing(market.to_string()))
    }

    pub fn get_many(&self, markets: &[MarketId]) -> TempestResult<Vec<MarketObservation>> {
        markets
            .iter()
            .map(|market| self.get(market).cloned())
            .collect()
    }

    pub fn update(
        &mut self,
        market: &MarketId,
        slot: u64,
        price_micros: u128,
        depth: Amount,
        liquidity_score: u32,
        rolling_volatility_bps: Bps,
    ) -> TempestResult<MarketMove> {
        let previous = self.get(market)?.clone();
        let price_move_bps = movement_bps_u128(previous.price_micros, price_micros)?;
        let depth_move_bps = movement_bps_i128(previous.depth.raw(), depth.raw())?;
        let next = MarketObservation {
            market: market.clone(),
            slot,
            price_micros,
            depth,
            liquidity_score,
            rolling_volatility_bps,
            version: previous.version + 1,
        };
        self.observations.insert(market.clone(), next.clone());
        Ok(MarketMove {
            market: market.clone(),
            slot,
            previous_price_micros: previous.price_micros,
            next_price_micros: price_micros,
            previous_depth: previous.depth,
            next_depth: depth,
            price_move_bps,
            depth_move_bps,
            rolling_volatility_bps,
            version: next.version,
        })
    }

    pub fn digest_parts(&self) -> Vec<String> {
        self.observations
            .values()
            .map(|observation| {
                format!(
                    "{}:{}:{}:{}:{}:{}",
                    observation.market,
                    observation.slot,
                    observation.price_micros,
                    observation.depth.raw(),
                    observation.rolling_volatility_bps.raw(),
                    observation.version
                )
            })
            .collect()
    }

    pub fn markets(&self) -> Vec<MarketId> {
        self.observations.keys().cloned().collect()
    }
}

fn movement_bps_u128(previous: u128, next: u128) -> TempestResult<Bps> {
    if previous == 0 {
        return Bps::new(10_000);
    }
    let diff = previous.abs_diff(next);
    let raw = ((diff.saturating_mul(10_000)) / previous).min(10_000);
    Bps::new(raw as u32)
}

fn movement_bps_i128(previous: i128, next: i128) -> TempestResult<Bps> {
    if previous <= 0 {
        return Bps::new(10_000);
    }
    let diff = previous.abs_diff(next);
    let raw = ((diff.saturating_mul(10_000)) / previous as u128).min(10_000);
    Bps::new(raw as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_price_move() {
        let mut book = OracleBook::new();
        let market = MarketId::from("SOL-USDC");
        book.insert(MarketObservation::new(
            market.clone(),
            1,
            1_000_000,
            Amount::from_units(10_000),
            90,
            Bps::new(100).unwrap(),
        ));
        let mv = book
            .update(
                &market,
                2,
                1_050_000,
                Amount::from_units(9_000),
                82,
                Bps::new(180).unwrap(),
            )
            .unwrap();
        assert_eq!(mv.price_move_bps.raw(), 500);
        assert_eq!(mv.depth_move_bps.raw(), 1_000);
    }
}
