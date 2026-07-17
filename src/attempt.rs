use crate::error::TransitionError;
use crate::event::{Event, EventKind};
use crate::state::State;
use crate::types::{EventId, PaymentId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A single payment attempt and its state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentAttempt {
    pub id: PaymentId,
    pub amount: u64,
    /// Counterparties this attempt routes through, in order.
    pub route: Vec<String>,
    pub created_at: Timestamp,
    pub timeout_at: Option<Timestamp>,
    state: State,
    applied_events: HashSet<EventId>,
}

impl PaymentAttempt {
    pub fn new(
        id: PaymentId,
        amount: u64,
        route: Vec<String>,
        created_at: Timestamp,
        timeout_at: Option<Timestamp>,
    ) -> Self {
        PaymentAttempt {
            id,
            amount,
            route,
            created_at,
            timeout_at,
            state: State::Created,
            applied_events: HashSet::new(),
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn has_applied(&self, event_id: &EventId) -> bool {
        self.applied_events.contains(event_id)
    }

    pub fn apply_event(&mut self, event: &Event, now: Timestamp) -> Result<State, TransitionError> {
        if self.applied_events.contains(&event.id) {
            return Ok(self.state);
        }

        let next = self.next_state(&event.kind, now)?;

        self.state = next;
        self.applied_events.insert(event.id.clone());
        Ok(next)
    }

    fn next_state(&self, kind: &EventKind, now: Timestamp) -> Result<State, TransitionError> {
        match (self.state, kind) {
            (State::Created, EventKind::Reserve) => Ok(State::Reserved),
            (State::Reserved, EventKind::Commit) => Ok(State::Committed),
            (State::Reserved, EventKind::Timeout) => {
                let deadline = self
                    .timeout_at
                    .ok_or(TransitionError::NoTimeoutConfigured)?;
                if now < deadline {
                    return Err(TransitionError::TimeoutTooEarly { now, deadline });
                }
                Ok(State::TimedOut)
            }
            (State::TimedOut, EventKind::Refund) => Ok(State::Refunded),
            (State::Created | State::Reserved, EventKind::Fail { .. }) => Ok(State::Failed),
            (from, kind) => Err(TransitionError::InvalidTransition {
                from,
                event: kind.name(),
            }),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
