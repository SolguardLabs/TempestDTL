use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::amount::Bps;
use crate::error::{TempestError, TempestResult};
use crate::ids::{Digest, MarketId};
use crate::oracle::MarketMove;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OperatingMode {
    Open,
    Paused,
    Recovery,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PauseTrigger {
    pub slot: u64,
    pub market: MarketId,
    pub reason: String,
    pub price_move_bps: Bps,
    pub depth_move_bps: Bps,
    pub digest: Digest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PauseController {
    mode: OperatingMode,
    price_threshold_bps: Bps,
    depth_threshold_bps: Bps,
    volatility_threshold_bps: Bps,
    recovery_delay_slots: u64,
    recovery_started_at: Option<u64>,
    resume_after_slot: Option<u64>,
    affected_markets: BTreeSet<MarketId>,
    last_trigger: Option<PauseTrigger>,
}

impl PauseController {
    pub fn new(
        price_threshold_bps: Bps,
        depth_threshold_bps: Bps,
        volatility_threshold_bps: Bps,
        recovery_delay_slots: u64,
    ) -> Self {
        Self {
            mode: OperatingMode::Open,
            price_threshold_bps,
            depth_threshold_bps,
            volatility_threshold_bps,
            recovery_delay_slots,
            recovery_started_at: None,
            resume_after_slot: None,
            affected_markets: BTreeSet::new(),
            last_trigger: None,
        }
    }

    pub fn mode(&self) -> &OperatingMode {
        &self.mode
    }

    pub fn is_open(&self) -> bool {
        self.mode == OperatingMode::Open
    }

    pub fn affected_markets(&self) -> Vec<MarketId> {
        self.affected_markets.iter().cloned().collect()
    }

    pub fn last_trigger(&self) -> Option<&PauseTrigger> {
        self.last_trigger.as_ref()
    }

    pub fn observe_market(&mut self, market_move: &MarketMove) -> bool {
        let price_hit = market_move.price_move_bps > self.price_threshold_bps;
        let depth_hit = market_move.depth_move_bps > self.depth_threshold_bps;
        let vol_hit = market_move.rolling_volatility_bps > self.volatility_threshold_bps;
        if !(price_hit || depth_hit || vol_hit) {
            return false;
        }

        let reason = if price_hit {
            "price"
        } else if depth_hit {
            "depth"
        } else {
            "volatility"
        };
        self.mode = OperatingMode::Paused;
        self.recovery_started_at = None;
        self.resume_after_slot = Some(market_move.slot + self.recovery_delay_slots);
        self.affected_markets.insert(market_move.market.clone());
        self.last_trigger = Some(PauseTrigger {
            slot: market_move.slot,
            market: market_move.market.clone(),
            reason: reason.to_string(),
            price_move_bps: market_move.price_move_bps,
            depth_move_bps: market_move.depth_move_bps,
            digest: Digest::stable(
                "pause",
                &[
                    market_move.market.as_str(),
                    &market_move.slot.to_string(),
                    &market_move.price_move_bps.raw().to_string(),
                    &market_move.depth_move_bps.raw().to_string(),
                    reason,
                ],
            ),
        });
        true
    }

    pub fn ensure_admission(&self, markets: &[MarketId]) -> TempestResult<()> {
        match self.mode {
            OperatingMode::Open => Ok(()),
            OperatingMode::Paused | OperatingMode::Recovery => {
                let requested: Vec<String> = markets.iter().map(ToString::to_string).collect();
                Err(TempestError::RouterPaused(format!(
                    "admission closed for {}",
                    requested.join(",")
                )))
            }
        }
    }

    pub fn advance(&mut self, slot: u64, calm_window: bool) -> bool {
        match self.mode {
            OperatingMode::Open => false,
            OperatingMode::Paused => {
                if self.resume_after_slot.is_some_and(|resume| slot >= resume) && calm_window {
                    self.mode = OperatingMode::Recovery;
                    self.recovery_started_at = Some(slot);
                    return true;
                }
                false
            }
            OperatingMode::Recovery => {
                let ready = self
                    .recovery_started_at
                    .is_some_and(|started| slot >= started + self.recovery_delay_slots);
                if ready && calm_window {
                    self.mode = OperatingMode::Open;
                    self.recovery_started_at = None;
                    self.resume_after_slot = None;
                    self.affected_markets.clear();
                    return true;
                }
                false
            }
        }
    }

    pub fn mode_label(&self) -> &'static str {
        match self.mode {
            OperatingMode::Open => "open",
            OperatingMode::Paused => "paused",
            OperatingMode::Recovery => "recovery",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amount::Amount;

    fn move_with_price(price_bps: u32) -> MarketMove {
        MarketMove {
            market: MarketId::from("SOL-USDC"),
            slot: 10,
            previous_price_micros: 1_000,
            next_price_micros: 1_000,
            previous_depth: Amount::from_units(1_000),
            next_depth: Amount::from_units(1_000),
            price_move_bps: Bps::new(price_bps).unwrap(),
            depth_move_bps: Bps::new(10).unwrap(),
            rolling_volatility_bps: Bps::new(20).unwrap(),
            version: 2,
        }
    }

    #[test]
    fn enters_pause_on_threshold() {
        let mut guard = PauseController::new(
            Bps::new(500).unwrap(),
            Bps::new(600).unwrap(),
            Bps::new(700).unwrap(),
            3,
        );
        assert!(guard.observe_market(&move_with_price(800)));
        assert_eq!(guard.mode(), &OperatingMode::Paused);
    }

    #[test]
    fn resumes_after_stability() {
        let mut guard = PauseController::new(
            Bps::new(500).unwrap(),
            Bps::new(600).unwrap(),
            Bps::new(700).unwrap(),
            3,
        );
        guard.observe_market(&move_with_price(800));
        assert!(guard.advance(13, true));
        assert_eq!(guard.mode(), &OperatingMode::Recovery);
        assert!(guard.advance(16, true));
        assert_eq!(guard.mode(), &OperatingMode::Open);
    }
}
