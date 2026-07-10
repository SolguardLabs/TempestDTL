use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::accounts::AccountBook;
use crate::amount::{Amount, Bps};
use crate::asset::AssetBook;
use crate::error::{TempestError, TempestResult};
use crate::events::{EventKind, EventLog};
use crate::ids::{AccountId, AssetId, Digest, MarketId, TicketId};
use crate::ledger::Journal;
use crate::liquidity::{HopPlan, HopRequest, LiquidityBook, PoolSummary};
use crate::oracle::{MarketMove, OracleBook};
use crate::pause::PauseController;
use crate::risk::{AdmissionSnapshot, RiskEngine};
use crate::settlement::{SettlementEngine, SettlementReceipt};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TicketStatus {
    Prepared,
    Settled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteIntent {
    pub payer: AccountId,
    pub beneficiary: AccountId,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
    pub amount_in: Amount,
    pub max_deviation_bps: Bps,
    pub path: Vec<HopRequest>,
    pub client_order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteTicket {
    pub id: TicketId,
    pub payer: AccountId,
    pub beneficiary: AccountId,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
    pub amount_in: Amount,
    pub amount_out: Amount,
    pub max_deviation_bps: Bps,
    pub admission: AdmissionSnapshot,
    pub hops: Vec<HopPlan>,
    pub quote_digest: Digest,
    pub status: TicketStatus,
    pub settled_at_slot: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutePlanSummary {
    pub ticket: String,
    pub amount_in: i128,
    pub amount_out: i128,
    pub hops: usize,
    pub expires_at_slot: u64,
    pub quote_digest: String,
    pub admission_digest: String,
}

#[derive(Clone, Debug)]
pub struct Router {
    assets: AssetBook,
    accounts: AccountBook,
    liquidity: LiquidityBook,
    oracle: OracleBook,
    pause: PauseController,
    risk: RiskEngine,
    events: EventLog,
    journal: Journal,
    tickets: BTreeMap<TicketId, RouteTicket>,
    slot: u64,
    nonce: u64,
}

impl RouteIntent {
    pub fn market_ids(&self) -> Vec<MarketId> {
        self.path.iter().map(|hop| hop.market.clone()).collect()
    }

    pub fn validate_path_shape(&self) -> TempestResult<()> {
        if self.path.is_empty() {
            return Err(TempestError::RouteRejected("empty path".to_string()));
        }
        if self
            .path
            .first()
            .is_some_and(|hop| hop.asset_in != self.asset_in)
        {
            return Err(TempestError::RouteRejected(
                "first hop asset mismatch".to_string(),
            ));
        }
        if self
            .path
            .last()
            .is_some_and(|hop| hop.asset_out != self.asset_out)
        {
            return Err(TempestError::RouteRejected(
                "last hop asset mismatch".to_string(),
            ));
        }
        for window in self.path.windows(2) {
            if window[0].asset_out != window[1].asset_in {
                return Err(TempestError::RouteRejected(
                    "path continuity mismatch".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl RouteTicket {
    pub fn summary(&self) -> RoutePlanSummary {
        RoutePlanSummary {
            ticket: self.id.to_string(),
            amount_in: self.amount_in.raw(),
            amount_out: self.amount_out.raw(),
            hops: self.hops.len(),
            expires_at_slot: self.admission.expires_at_slot,
            quote_digest: self.quote_digest.to_string(),
            admission_digest: self.admission.digest.to_string(),
        }
    }
}

impl Router {
    pub fn new(
        assets: AssetBook,
        accounts: AccountBook,
        liquidity: LiquidityBook,
        oracle: OracleBook,
        pause: PauseController,
        risk: RiskEngine,
    ) -> Self {
        Self {
            assets,
            accounts,
            liquidity,
            oracle,
            pause,
            risk,
            events: EventLog::new(),
            journal: Journal::new(),
            tickets: BTreeMap::new(),
            slot: 1,
            nonce: 0,
        }
    }

    pub fn slot(&self) -> u64 {
        self.slot
    }

    pub fn advance_slot(&mut self, slots: u64) {
        self.slot = self.slot.saturating_add(slots);
    }

    pub fn mode_label(&self) -> &'static str {
        self.pause.mode_label()
    }

    pub fn assets(&self) -> &AssetBook {
        &self.assets
    }

    pub fn accounts(&self) -> &AccountBook {
        &self.accounts
    }

    pub fn liquidity(&self) -> &LiquidityBook {
        &self.liquidity
    }

    pub fn oracle(&self) -> &OracleBook {
        &self.oracle
    }

    pub fn events(&self) -> &EventLog {
        &self.events
    }

    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    pub fn tickets(&self) -> Vec<RouteTicket> {
        self.tickets.values().cloned().collect()
    }

    pub fn ticket(&self, id: &TicketId) -> TempestResult<&RouteTicket> {
        self.tickets
            .get(id)
            .ok_or_else(|| TempestError::TicketNotFound(id.to_string()))
    }

    pub fn plan_route(&mut self, intent: RouteIntent) -> TempestResult<RouteTicket> {
        intent.validate_path_shape()?;
        let markets = intent.market_ids();
        self.pause.ensure_admission(&markets)?;
        let observations = self.oracle.get_many(&markets)?;
        let admission = self
            .risk
            .admit(self.slot, &observations, intent.max_deviation_bps)?;
        let quote = self.liquidity.quote_path(&intent.path, intent.amount_in)?;
        self.nonce += 1;

        let ticket_parts = [
            self.nonce.to_string(),
            self.slot.to_string(),
            intent.payer.to_string(),
            intent.beneficiary.to_string(),
            intent.asset_in.to_string(),
            intent.asset_out.to_string(),
            intent.amount_in.raw().to_string(),
            quote.amount_out.raw().to_string(),
            quote.digest.to_string(),
            admission.digest.to_string(),
        ];
        let refs: Vec<&str> = ticket_parts.iter().map(String::as_str).collect();
        let ticket_id = TicketId::deterministic(&refs);
        let ticket = RouteTicket {
            id: ticket_id.clone(),
            payer: intent.payer,
            beneficiary: intent.beneficiary,
            asset_in: intent.asset_in,
            asset_out: intent.asset_out,
            amount_in: quote.amount_in,
            amount_out: quote.amount_out,
            max_deviation_bps: admission.max_deviation_bps,
            admission,
            hops: quote.hops,
            quote_digest: quote.digest,
            status: TicketStatus::Prepared,
            settled_at_slot: None,
        };

        self.events.push(
            self.slot,
            EventKind::TicketAccepted,
            markets.first().cloned(),
            Some(ticket_id.clone()),
            "route ticket accepted",
        );
        self.tickets.insert(ticket_id, ticket.clone());
        Ok(ticket)
    }

    pub fn settle_ticket(&mut self, id: &TicketId) -> TempestResult<SettlementReceipt> {
        let mut ticket = self
            .tickets
            .remove(id)
            .ok_or_else(|| TempestError::TicketNotFound(id.to_string()))?;
        let result = SettlementEngine::settle_ticket(
            self.slot,
            &mut ticket,
            &mut self.accounts,
            &mut self.liquidity,
            &mut self.journal,
            &mut self.events,
        );
        self.tickets.insert(ticket.id.clone(), ticket);
        result
    }

    pub fn observe_market(
        &mut self,
        market: &MarketId,
        price_micros: u128,
        depth: Amount,
        liquidity_score: u32,
        rolling_volatility_bps: Bps,
    ) -> TempestResult<MarketMove> {
        let market_move = self.oracle.update(
            market,
            self.slot,
            price_micros,
            depth,
            liquidity_score,
            rolling_volatility_bps,
        )?;
        let entered_pause = self.pause.observe_market(&market_move);
        self.events.push(
            self.slot,
            EventKind::OracleUpdate,
            Some(market.clone()),
            None,
            "oracle updated",
        );
        if entered_pause {
            self.events.push(
                self.slot,
                EventKind::PauseEntered,
                Some(market.clone()),
                None,
                "market guard entered pause",
            );
        }
        Ok(market_move)
    }

    pub fn advance_protection(&mut self, slots: u64, calm_window: bool) {
        self.advance_slot(slots);
        if self.pause.advance(self.slot, calm_window) {
            self.events.push(
                self.slot,
                EventKind::PauseRecovered,
                None,
                None,
                "protection mode changed",
            );
        }
    }

    pub fn apply_external_liquidity_move(
        &mut self,
        market: &MarketId,
        asset: &AssetId,
        delta: i128,
    ) -> TempestResult<Amount> {
        self.liquidity.apply_external_move(market, asset, delta)
    }

    pub fn balances_for_asset(&self, asset: &AssetId) -> BTreeMap<String, i128> {
        self.accounts.balances_for_asset(asset)
    }

    pub fn pool_summaries(&self) -> Vec<PoolSummary> {
        self.liquidity.summaries()
    }

    pub fn state_digest(&self) -> Digest {
        let mut parts = Vec::<String>::new();
        parts.push(self.slot.to_string());
        parts.push(self.mode_label().to_string());
        parts.extend(self.liquidity.digest_parts());
        parts.extend(self.oracle.digest_parts());
        parts.push(self.journal.digest().to_string());
        for ticket in self.tickets.values() {
            parts.push(format!(
                "{}:{}:{}:{}",
                ticket.id,
                ticket.amount_in.raw(),
                ticket.amount_out.raw(),
                ticket.status_label()
            ));
        }
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        Digest::stable("router-state", &refs)
    }

    pub fn reject_note(&mut self, market: Option<MarketId>, note: impl Into<String>) {
        self.events
            .push(self.slot, EventKind::RouteRejected, market, None, note);
    }
}

impl RouteTicket {
    pub fn status_label(&self) -> &'static str {
        match self.status {
            TicketStatus::Prepared => "prepared",
            TicketStatus::Settled => "settled",
        }
    }
}
