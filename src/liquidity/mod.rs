use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::{Amount, Bps};
use crate::error::{TempestError, TempestResult};
use crate::ids::{AssetId, Digest, MarketId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HopRequest {
    pub market: MarketId,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HopPlan {
    pub market: MarketId,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
    pub amount_in: Amount,
    pub amount_out: Amount,
    pub fee_amount: Amount,
    pub reserve_in_before: Amount,
    pub reserve_out_before: Amount,
    pub pool_version_at_quote: u64,
    pub digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PathQuote {
    pub hops: Vec<HopPlan>,
    pub amount_in: Amount,
    pub amount_out: Amount,
    pub total_fee: Amount,
    pub digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Pool {
    pub market: MarketId,
    pub asset_a: AssetId,
    pub asset_b: AssetId,
    pub reserve_a: Amount,
    pub reserve_b: Amount,
    pub fee_bps: Bps,
    pub version: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LiquidityBook {
    pools: BTreeMap<MarketId, Pool>,
}

impl HopRequest {
    pub fn new(
        market: impl Into<MarketId>,
        asset_in: impl Into<AssetId>,
        asset_out: impl Into<AssetId>,
    ) -> Self {
        Self {
            market: market.into(),
            asset_in: asset_in.into(),
            asset_out: asset_out.into(),
        }
    }
}

impl Pool {
    pub fn new(
        market: impl Into<MarketId>,
        asset_a: impl Into<AssetId>,
        asset_b: impl Into<AssetId>,
        reserve_a: Amount,
        reserve_b: Amount,
        fee_bps: Bps,
    ) -> Self {
        Self {
            market: market.into(),
            asset_a: asset_a.into(),
            asset_b: asset_b.into(),
            reserve_a,
            reserve_b,
            fee_bps,
            version: 1,
        }
    }

    pub fn contains_pair(&self, asset_in: &AssetId, asset_out: &AssetId) -> bool {
        (&self.asset_a == asset_in && &self.asset_b == asset_out)
            || (&self.asset_b == asset_in && &self.asset_a == asset_out)
    }

    pub fn reserve_for(&self, asset: &AssetId) -> TempestResult<Amount> {
        if &self.asset_a == asset {
            Ok(self.reserve_a)
        } else if &self.asset_b == asset {
            Ok(self.reserve_b)
        } else {
            Err(TempestError::RouteRejected(format!(
                "asset {asset} not part of {}",
                self.market
            )))
        }
    }

    pub fn quote_exact_in(
        &self,
        request: &HopRequest,
        amount_in: Amount,
    ) -> TempestResult<HopPlan> {
        if !self.contains_pair(&request.asset_in, &request.asset_out) {
            return Err(TempestError::RouteRejected(format!(
                "invalid pair for {}",
                self.market
            )));
        }
        if amount_in.is_zero() {
            return Err(TempestError::InvalidAmount);
        }
        let reserve_in = self.reserve_for(&request.asset_in)?;
        let reserve_out = self.reserve_for(&request.asset_out)?;
        if reserve_in.is_zero() || reserve_out.is_zero() {
            return Err(TempestError::InsufficientReserve {
                market: self.market.to_string(),
            });
        }

        let fee_amount = amount_in.apply_bps_down(self.fee_bps)?;
        let amount_after_fee = amount_in.checked_sub(fee_amount)?;
        let numerator = amount_after_fee
            .raw()
            .checked_mul(reserve_out.raw())
            .ok_or(TempestError::InvalidAmount)?;
        let denominator = reserve_in
            .raw()
            .checked_add(amount_after_fee.raw())
            .ok_or(TempestError::InvalidAmount)?;
        let amount_out = Amount::new(numerator / denominator)?;
        if amount_out.is_zero() || amount_out >= reserve_out {
            return Err(TempestError::InsufficientReserve {
                market: self.market.to_string(),
            });
        }

        let digest = Digest::stable(
            "hop-plan",
            &[
                request.market.as_str(),
                request.asset_in.as_str(),
                request.asset_out.as_str(),
                &amount_in.raw().to_string(),
                &amount_out.raw().to_string(),
                &reserve_in.raw().to_string(),
                &reserve_out.raw().to_string(),
                &self.version.to_string(),
            ],
        );

        Ok(HopPlan {
            market: request.market.clone(),
            asset_in: request.asset_in.clone(),
            asset_out: request.asset_out.clone(),
            amount_in,
            amount_out,
            fee_amount,
            reserve_in_before: reserve_in,
            reserve_out_before: reserve_out,
            pool_version_at_quote: self.version,
            digest,
        })
    }

    pub fn commit_planned_hop(&mut self, plan: &HopPlan) -> TempestResult<()> {
        if !self.contains_pair(&plan.asset_in, &plan.asset_out) {
            return Err(TempestError::RouteRejected(format!(
                "invalid settlement pair for {}",
                self.market
            )));
        }
        if self.reserve_for(&plan.asset_out)? < plan.amount_out {
            return Err(TempestError::InsufficientReserve {
                market: self.market.to_string(),
            });
        }

        if self.asset_a == plan.asset_in {
            self.reserve_a = self.reserve_a.checked_add(plan.amount_in)?;
            self.reserve_b = self.reserve_b.checked_sub(plan.amount_out)?;
        } else {
            self.reserve_b = self.reserve_b.checked_add(plan.amount_in)?;
            self.reserve_a = self.reserve_a.checked_sub(plan.amount_out)?;
        }
        self.version += 1;
        Ok(())
    }

    pub fn apply_external_move(&mut self, asset: &AssetId, delta: i128) -> TempestResult<Amount> {
        let reserve = self.reserve_for(asset)?;
        let next = if delta >= 0 {
            reserve.checked_add(Amount::new(delta)?)?
        } else {
            reserve.checked_sub(Amount::new(delta.abs())?)?
        };
        if &self.asset_a == asset {
            self.reserve_a = next;
        } else if &self.asset_b == asset {
            self.reserve_b = next;
        }
        self.version += 1;
        Ok(next)
    }

    pub fn total_depth(&self) -> TempestResult<Amount> {
        self.reserve_a.checked_add(self.reserve_b)
    }

    pub fn summary(&self) -> PoolSummary {
        PoolSummary {
            market: self.market.to_string(),
            asset_a: self.asset_a.to_string(),
            asset_b: self.asset_b.to_string(),
            reserve_a: self.reserve_a.raw(),
            reserve_b: self.reserve_b.raw(),
            version: self.version,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolSummary {
    pub market: String,
    pub asset_a: String,
    pub asset_b: String,
    pub reserve_a: i128,
    pub reserve_b: i128,
    pub version: u64,
}

impl LiquidityBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, pool: Pool) {
        self.pools.insert(pool.market.clone(), pool);
    }

    pub fn get(&self, market: &MarketId) -> TempestResult<&Pool> {
        self.pools
            .get(market)
            .ok_or_else(|| TempestError::MarketNotFound(market.to_string()))
    }

    pub fn get_mut(&mut self, market: &MarketId) -> TempestResult<&mut Pool> {
        self.pools
            .get_mut(market)
            .ok_or_else(|| TempestError::MarketNotFound(market.to_string()))
    }

    pub fn quote_path(
        &self,
        requests: &[HopRequest],
        amount_in: Amount,
    ) -> TempestResult<PathQuote> {
        if requests.is_empty() {
            return Err(TempestError::RouteRejected("empty path".to_string()));
        }
        let mut current_amount = amount_in;
        let mut total_fee = Amount::ZERO;
        let mut hops = Vec::with_capacity(requests.len());

        for request in requests {
            let pool = self.get(&request.market)?;
            let plan = pool.quote_exact_in(request, current_amount)?;
            current_amount = plan.amount_out;
            total_fee = total_fee.checked_add(plan.fee_amount)?;
            hops.push(plan);
        }

        let digest_parts: Vec<String> = hops
            .iter()
            .map(|hop| {
                format!(
                    "{}:{}:{}:{}:{}:{}",
                    hop.market,
                    hop.asset_in,
                    hop.asset_out,
                    hop.amount_in,
                    hop.amount_out,
                    hop.pool_version_at_quote
                )
            })
            .collect();
        let refs: Vec<&str> = digest_parts.iter().map(String::as_str).collect();
        let digest = Digest::stable("path-quote", &refs);

        Ok(PathQuote {
            hops,
            amount_in,
            amount_out: current_amount,
            total_fee,
            digest,
        })
    }

    pub fn commit_path(&mut self, hops: &[HopPlan]) -> TempestResult<()> {
        for hop in hops {
            self.get_mut(&hop.market)?.commit_planned_hop(hop)?;
        }
        Ok(())
    }

    pub fn apply_external_move(
        &mut self,
        market: &MarketId,
        asset: &AssetId,
        delta: i128,
    ) -> TempestResult<Amount> {
        self.get_mut(market)?.apply_external_move(asset, delta)
    }

    pub fn summaries(&self) -> Vec<PoolSummary> {
        self.pools.values().map(Pool::summary).collect()
    }

    pub fn market_ids(&self) -> Vec<MarketId> {
        self.pools.keys().cloned().collect()
    }

    pub fn digest_parts(&self) -> Vec<String> {
        self.pools
            .values()
            .map(|pool| {
                format!(
                    "{}:{}:{}:{}:{}:{}",
                    pool.market,
                    pool.asset_a,
                    pool.asset_b,
                    pool.reserve_a.raw(),
                    pool.reserve_b.raw(),
                    pool.version
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_commits_path() {
        let mut book = LiquidityBook::new();
        book.insert(Pool::new(
            "SOL-USDC",
            "USDC",
            "SOL",
            Amount::from_units(1_000_000),
            Amount::from_units(50_000),
            Bps::new(30).unwrap(),
        ));
        let request = HopRequest::new("SOL-USDC", "USDC", "SOL");
        let quote = book
            .quote_path(&[request], Amount::from_units(10_000))
            .unwrap();
        assert!(quote.amount_out.raw() > 0);
        book.commit_path(&quote.hops).unwrap();
        let pool = book.get(&MarketId::from("SOL-USDC")).unwrap();
        assert_eq!(pool.version, 2);
    }
}
