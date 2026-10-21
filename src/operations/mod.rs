use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::Amount;
use crate::error::TempestResult;
use crate::ids::{AssetId, MarketId};
use crate::routes::{Router, TicketStatus};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MarketPipeline {
    pub market: MarketId,
    pub asset_in: AssetId,
    pub asset_out: AssetId,
    pub prepared_routes: u64,
    pub input_units: Amount,
    pub output_units: Amount,
    pub oldest_admission_slot: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperationsReport {
    pub slot: u64,
    pub mode: String,
    pub prepared_tickets: u64,
    pub settled_tickets: u64,
    pub pool_count: usize,
    pub event_count: usize,
    pub market_pipeline: Vec<MarketPipeline>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OperationsMonitor;

impl OperationsMonitor {
    pub fn snapshot(&self, router: &Router) -> TempestResult<OperationsReport> {
        let mut prepared_tickets = 0u64;
        let mut settled_tickets = 0u64;
        let mut pipeline: BTreeMap<(MarketId, AssetId, AssetId), MarketPipeline> = BTreeMap::new();
        for ticket in router.tickets() {
            match ticket.status {
                TicketStatus::Prepared => prepared_tickets = prepared_tickets.saturating_add(1),
                TicketStatus::Settled => {
                    settled_tickets = settled_tickets.saturating_add(1);
                    continue;
                }
            }
            for hop in &ticket.hops {
                let key = (
                    hop.market.clone(),
                    hop.asset_in.clone(),
                    hop.asset_out.clone(),
                );
                let entry = pipeline.entry(key).or_insert_with(|| MarketPipeline {
                    market: hop.market.clone(),
                    asset_in: hop.asset_in.clone(),
                    asset_out: hop.asset_out.clone(),
                    prepared_routes: 0,
                    input_units: Amount::ZERO,
                    output_units: Amount::ZERO,
                    oldest_admission_slot: ticket.admission.admitted_at_slot,
                });
                entry.prepared_routes = entry.prepared_routes.saturating_add(1);
                entry.input_units = entry.input_units.checked_add(hop.amount_in)?;
                entry.output_units = entry.output_units.checked_add(hop.amount_out)?;
                entry.oldest_admission_slot = entry
                    .oldest_admission_slot
                    .min(ticket.admission.admitted_at_slot);
            }
        }
        Ok(OperationsReport {
            slot: router.slot(),
            mode: router.mode_label().to_string(),
            prepared_tickets,
            settled_tickets,
            pool_count: router.pool_summaries().len(),
            event_count: router.events().len(),
            market_pipeline: pipeline.into_values().collect(),
        })
    }
}
