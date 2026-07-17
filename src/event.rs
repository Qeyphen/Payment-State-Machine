use crate::types::EventId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    Reserve,
    Commit,
    Timeout,
    Refund,
    Fail { reason: String },
}

impl EventKind {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            EventKind::Reserve => "Reserve",
            EventKind::Commit => "Commit",
            EventKind::Timeout => "Timeout",
            EventKind::Refund => "Refund",
            EventKind::Fail { .. } => "Fail",
        }
    }
}

/// An event delivered to an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub kind: EventKind,
}

impl Event {
    pub fn new(id: impl Into<String>, kind: EventKind) -> Self {
        Event {
            id: EventId(id.into()),
            kind,
        }
    }
}
