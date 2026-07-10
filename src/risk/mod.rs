use serde::{Deserialize, Serialize};

use crate::amount::{Amount, Bps};
use crate::catalog;
use crate::error::{TempestError, TempestResult};
use crate::ids::{Digest, MarketId};
use crate::oracle::MarketObservation;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_route_hops: usize,
    pub default_max_deviation_bps: Bps,
    pub default_max_volatility_bps: Bps,
    pub default_min_liquidity_score: u32,
    pub default_min_depth: Amount,
    pub default_ttl_slots: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RiskPreset {
    pub venue: &'static str,
    pub market_code: &'static str,
    pub tier: &'static str,
    pub max_deviation_bps: u32,
    pub max_volatility_bps: u32,
    pub min_liquidity_score: u32,
    pub min_depth_units: i128,
    pub quote_ttl_slots: u64,
    pub reserve_floor_units: i128,
    pub route_weight: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MarketRiskView {
    pub market: MarketId,
    pub price_micros: u128,
    pub depth: Amount,
    pub liquidity_score: u32,
    pub rolling_volatility_bps: Bps,
    pub oracle_version: u64,
    pub max_deviation_bps: Bps,
    pub max_volatility_bps: Bps,
    pub min_liquidity_score: u32,
    pub min_depth: Amount,
    pub ttl_slots: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdmissionSnapshot {
    pub admitted_at_slot: u64,
    pub expires_at_slot: u64,
    pub route_hops: usize,
    pub max_deviation_bps: Bps,
    pub max_volatility_bps: Bps,
    pub markets: Vec<MarketRiskView>,
    pub digest: Digest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskEngine {
    config: RiskConfig,
}

impl RiskConfig {
    pub fn production() -> Self {
        Self {
            max_route_hops: 4,
            default_max_deviation_bps: Bps::new(250).unwrap(),
            default_max_volatility_bps: Bps::new(650).unwrap(),
            default_min_liquidity_score: 70,
            default_min_depth: Amount::from_units(20_000),
            default_ttl_slots: 12,
        }
    }
}

impl RiskPreset {
    pub fn min_depth(self) -> Amount {
        Amount::from_units(self.min_depth_units)
    }

    pub fn reserve_floor(self) -> Amount {
        Amount::from_units(self.reserve_floor_units)
    }

    pub fn deviation_bps(self) -> TempestResult<Bps> {
        Bps::new(self.max_deviation_bps)
    }

    pub fn volatility_bps(self) -> TempestResult<Bps> {
        Bps::new(self.max_volatility_bps)
    }
}

impl RiskEngine {
    pub fn new(config: RiskConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &RiskConfig {
        &self.config
    }

    pub fn view_for_market(
        &self,
        observation: &MarketObservation,
    ) -> TempestResult<MarketRiskView> {
        let preset = catalog::find_preset(observation.market.as_str(), "core");
        let max_deviation_bps = preset
            .map(|preset| preset.deviation_bps())
            .transpose()?
            .unwrap_or(self.config.default_max_deviation_bps);
        let max_volatility_bps = preset
            .map(|preset| preset.volatility_bps())
            .transpose()?
            .unwrap_or(self.config.default_max_volatility_bps);
        let min_liquidity_score = preset
            .map(|preset| preset.min_liquidity_score)
            .unwrap_or(self.config.default_min_liquidity_score);
        let min_depth = preset
            .map(|preset| preset.min_depth())
            .unwrap_or(self.config.default_min_depth);
        let ttl_slots = preset
            .map(|preset| preset.quote_ttl_slots)
            .unwrap_or(self.config.default_ttl_slots);
        Ok(MarketRiskView {
            market: observation.market.clone(),
            price_micros: observation.price_micros,
            depth: observation.depth,
            liquidity_score: observation.liquidity_score,
            rolling_volatility_bps: observation.rolling_volatility_bps,
            oracle_version: observation.version,
            max_deviation_bps,
            max_volatility_bps,
            min_liquidity_score,
            min_depth,
            ttl_slots,
        })
    }

    pub fn admit(
        &self,
        slot: u64,
        observations: &[MarketObservation],
        requested_max_deviation: Bps,
    ) -> TempestResult<AdmissionSnapshot> {
        if observations.is_empty() {
            return Err(TempestError::RouteRejected("empty path".to_string()));
        }
        if observations.len() > self.config.max_route_hops {
            return Err(TempestError::RouteRejected("path too long".to_string()));
        }

        let mut views = Vec::with_capacity(observations.len());
        let mut ttl_slots = self.config.default_ttl_slots;
        let mut max_deviation = requested_max_deviation.min(self.config.default_max_deviation_bps);
        let mut max_volatility = self.config.default_max_volatility_bps;

        for observation in observations {
            let view = self.view_for_market(observation)?;
            if observation.depth < view.min_depth {
                return Err(TempestError::RouteRejected(format!(
                    "depth below floor for {}",
                    observation.market
                )));
            }
            if observation.liquidity_score < view.min_liquidity_score {
                return Err(TempestError::RouteRejected(format!(
                    "liquidity score below floor for {}",
                    observation.market
                )));
            }
            if observation.rolling_volatility_bps > view.max_volatility_bps {
                return Err(TempestError::RouteRejected(format!(
                    "volatility above limit for {}",
                    observation.market
                )));
            }
            ttl_slots = ttl_slots.min(view.ttl_slots);
            max_deviation = max_deviation.min(view.max_deviation_bps);
            max_volatility = max_volatility.min(view.max_volatility_bps);
            views.push(view);
        }

        let market_parts: Vec<String> = views
            .iter()
            .map(|view| {
                format!(
                    "{}:{}:{}:{}",
                    view.market, view.oracle_version, view.price_micros, view.depth
                )
            })
            .collect();
        let mut digest_parts = vec![
            slot.to_string(),
            (slot + ttl_slots).to_string(),
            max_deviation.raw().to_string(),
            max_volatility.raw().to_string(),
        ];
        digest_parts.extend(market_parts);
        let refs: Vec<&str> = digest_parts.iter().map(String::as_str).collect();

        Ok(AdmissionSnapshot {
            admitted_at_slot: slot,
            expires_at_slot: slot + ttl_slots,
            route_hops: views.len(),
            max_deviation_bps: max_deviation,
            max_volatility_bps: max_volatility,
            markets: views,
            digest: Digest::stable("admission", &refs),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_stable_observation() {
        let engine = RiskEngine::new(RiskConfig::production());
        let observation = MarketObservation::new(
            MarketId::from("SOL-USDC"),
            1,
            1_000_000,
            Amount::from_units(100_000),
            95,
            Bps::new(100).unwrap(),
        );
        let snapshot = engine
            .admit(1, &[observation], Bps::new(200).unwrap())
            .unwrap();
        assert_eq!(snapshot.route_hops, 1);
        assert!(snapshot.expires_at_slot > snapshot.admitted_at_slot);
    }

    #[test]
    fn rejects_high_volatility() {
        let engine = RiskEngine::new(RiskConfig::production());
        let observation = MarketObservation::new(
            MarketId::from("SOL-USDC"),
            1,
            1_000_000,
            Amount::from_units(100_000),
            95,
            Bps::new(900).unwrap(),
        );
        assert!(
            engine
                .admit(1, &[observation], Bps::new(200).unwrap())
                .is_err()
        );
    }
}
