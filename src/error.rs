use crate::state::State;
use crate::types::Timestamp;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TransitionError {
    #[error("invalid transition: cannot apply {event} while in {from:?}")]
    InvalidTransition { from: State, event: &'static str },

    #[error("timeout too early: now {now} is before deadline {deadline}")]
    TimeoutTooEarly { now: Timestamp, deadline: Timestamp },

    #[error("cannot time out an attempt with no timeout configured")]
    NoTimeoutConfigured,
}
