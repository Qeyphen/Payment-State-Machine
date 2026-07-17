use serde::{Deserialize, Serialize};

/// Lifecycle states. See the README for the transition table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Created,
    Reserved,
    Committed,
    TimedOut,
    Refunded,
    Failed,
}

impl State {
    pub fn is_terminal(self) -> bool {
        matches!(self, State::Committed | State::Refunded | State::Failed)
    }
}
