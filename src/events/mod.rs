use serde::{Deserialize, Serialize};

use crate::ids::{Digest, MarketId, TicketId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    OracleUpdate,
    RouteQuoted,
    TicketAccepted,
    TicketSettled,
    PauseEntered,
    PauseRecovered,
    RouteRejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub index: u64,
    pub slot: u64,
    pub kind: EventKind,
    pub market: Option<MarketId>,
    pub ticket: Option<TicketId>,
    pub digest: Digest,
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<Event>,
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(
        &mut self,
        slot: u64,
        kind: EventKind,
        market: Option<MarketId>,
        ticket: Option<TicketId>,
        note: impl Into<String>,
    ) {
        let index = self.events.len() as u64;
        let note = note.into();
        let digest = Digest::stable(
            "event",
            &[
                &index.to_string(),
                &slot.to_string(),
                &format!("{kind:?}"),
                market.as_ref().map(MarketId::as_str).unwrap_or("none"),
                ticket.as_ref().map(TicketId::as_str).unwrap_or("none"),
                &note,
            ],
        );
        self.events.push(Event {
            index,
            slot,
            kind,
            market,
            ticket,
            digest,
            note,
        });
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn last(&self) -> Option<&Event> {
        self.events.last()
    }

    pub fn all(&self) -> &[Event] {
        &self.events
    }

    pub fn kind_counts(&self) -> Vec<(String, usize)> {
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for event in &self.events {
            *counts.entry(format!("{:?}", event.kind)).or_default() += 1;
        }
        counts.into_iter().collect()
    }
}
