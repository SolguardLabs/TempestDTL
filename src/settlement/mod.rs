use serde::{Deserialize, Serialize};

use crate::accounts::AccountBook;
use crate::amount::Amount;
use crate::error::{TempestError, TempestResult};
use crate::events::{EventKind, EventLog};
use crate::ids::{Digest, TicketId};
use crate::ledger::{Journal, JournalOp};
use crate::liquidity::LiquidityBook;
use crate::routes::{RouteTicket, TicketStatus};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettlementReceipt {
    pub ticket: TicketId,
    pub slot: u64,
    pub source_amount: Amount,
    pub destination_amount: Amount,
    pub hops: usize,
    pub digest: Digest,
}

#[derive(Clone, Debug, Default)]
pub struct SettlementEngine;

impl SettlementEngine {
    pub fn settle_ticket(
        slot: u64,
        ticket: &mut RouteTicket,
        accounts: &mut AccountBook,
        liquidity: &mut LiquidityBook,
        journal: &mut Journal,
        events: &mut EventLog,
    ) -> TempestResult<SettlementReceipt> {
        if ticket.status == TicketStatus::Settled {
            return Err(TempestError::TicketAlreadySettled);
        }
        if slot > ticket.admission.expires_at_slot {
            return Err(TempestError::TicketExpired);
        }

        accounts.debit(&ticket.payer, &ticket.asset_in, ticket.amount_in)?;
        journal.record(
            slot,
            JournalOp::Debit,
            Some(ticket.payer.clone()),
            ticket.asset_in.clone(),
            ticket.amount_in,
            Some(ticket.id.clone()),
        );

        liquidity.commit_path(&ticket.hops)?;

        accounts.credit(&ticket.beneficiary, &ticket.asset_out, ticket.amount_out)?;
        journal.record(
            slot,
            JournalOp::Credit,
            Some(ticket.beneficiary.clone()),
            ticket.asset_out.clone(),
            ticket.amount_out,
            Some(ticket.id.clone()),
        );

        ticket.status = TicketStatus::Settled;
        ticket.settled_at_slot = Some(slot);

        let digest = Digest::stable(
            "settlement",
            &[
                ticket.id.as_str(),
                &slot.to_string(),
                ticket.asset_in.as_str(),
                ticket.asset_out.as_str(),
                &ticket.amount_in.raw().to_string(),
                &ticket.amount_out.raw().to_string(),
                &ticket.hops.len().to_string(),
            ],
        );
        events.push(
            slot,
            EventKind::TicketSettled,
            ticket.hops.last().map(|hop| hop.market.clone()),
            Some(ticket.id.clone()),
            "ticket settled",
        );

        Ok(SettlementReceipt {
            ticket: ticket.id.clone(),
            slot,
            source_amount: ticket.amount_in,
            destination_amount: ticket.amount_out,
            hops: ticket.hops.len(),
            digest,
        })
    }
}
