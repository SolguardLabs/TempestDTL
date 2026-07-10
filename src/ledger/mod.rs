use serde::{Deserialize, Serialize};

use crate::amount::Amount;
use crate::ids::{AccountId, AssetId, Digest, TicketId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum JournalOp {
    Debit,
    Credit,
    PoolDebit,
    PoolCredit,
    RouteFee,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub index: u64,
    pub slot: u64,
    pub op: JournalOp,
    pub account: Option<AccountId>,
    pub asset: AssetId,
    pub amount: Amount,
    pub ticket: Option<TicketId>,
    pub digest: Digest,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Journal {
    entries: Vec<JournalEntry>,
}

impl Journal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &mut self,
        slot: u64,
        op: JournalOp,
        account: Option<AccountId>,
        asset: AssetId,
        amount: Amount,
        ticket: Option<TicketId>,
    ) {
        let index = self.entries.len() as u64;
        let digest = Digest::stable(
            "journal",
            &[
                &index.to_string(),
                &slot.to_string(),
                &format!("{op:?}"),
                account.as_ref().map(AccountId::as_str).unwrap_or("pool"),
                asset.as_str(),
                &amount.raw().to_string(),
                ticket.as_ref().map(TicketId::as_str).unwrap_or("none"),
            ],
        );
        self.entries.push(JournalEntry {
            index,
            slot,
            op,
            account,
            asset,
            amount,
            ticket,
            digest,
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    pub fn digest(&self) -> Digest {
        let parts: Vec<String> = self
            .entries
            .iter()
            .map(|entry| entry.digest.to_string())
            .collect();
        let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
        Digest::stable("journal-root", &refs)
    }
}
