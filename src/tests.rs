use crate::{
    Event, EventId, EventKind, PaymentAttempt, PaymentId, State, Timestamp, TransitionError,
};

const CREATED_AT: Timestamp = 1_000;
const DEADLINE: Timestamp = 2_000;

fn attempt() -> PaymentAttempt {
    PaymentAttempt::new(
        PaymentId("pay-1".into()),
        1250,
        vec!["alice".into(), "bob".into()],
        CREATED_AT,
        Some(DEADLINE),
    )
}

#[test]
fn successful_payment() {
    let mut a = attempt();
    assert_eq!(a.state(), State::Created);

    assert_eq!(
        a.apply_event(&Event::new("e1", EventKind::Reserve), CREATED_AT),
        Ok(State::Reserved)
    );
    assert_eq!(
        a.apply_event(&Event::new("e2", EventKind::Commit), CREATED_AT),
        Ok(State::Committed)
    );
    assert!(a.state().is_terminal());
}

#[test]
fn timeout_then_refund() {
    let mut a = attempt();
    a.apply_event(&Event::new("e1", EventKind::Reserve), CREATED_AT)
        .unwrap();

    assert_eq!(
        a.apply_event(&Event::new("e2", EventKind::Timeout), DEADLINE - 1),
        Err(TransitionError::TimeoutTooEarly {
            now: DEADLINE - 1,
            deadline: DEADLINE
        })
    );
    assert_eq!(a.state(), State::Reserved);

    assert_eq!(
        a.apply_event(&Event::new("e3", EventKind::Timeout), DEADLINE),
        Ok(State::TimedOut)
    );
    assert_eq!(
        a.apply_event(&Event::new("e4", EventKind::Refund), DEADLINE),
        Ok(State::Refunded)
    );
}

#[test]
fn invalid_transitions_are_rejected() {
    // Commit without reserving.
    let mut a = attempt();
    assert_eq!(
        a.apply_event(&Event::new("e1", EventKind::Commit), CREATED_AT),
        Err(TransitionError::InvalidTransition {
            from: State::Created,
            event: "Commit"
        })
    );
    assert_eq!(a.state(), State::Created);

    let mut b = attempt();
    b.apply_event(&Event::new("e1", EventKind::Reserve), CREATED_AT)
        .unwrap();
    b.apply_event(&Event::new("e2", EventKind::Commit), CREATED_AT)
        .unwrap();
    assert_eq!(
        b.apply_event(&Event::new("e3", EventKind::Refund), CREATED_AT),
        Err(TransitionError::InvalidTransition {
            from: State::Committed,
            event: "Refund"
        })
    );
    assert_eq!(b.state(), State::Committed);
}

#[test]
fn fail_from_created_and_reserved() {
    let mut a = attempt();
    let fail = Event::new(
        "e1",
        EventKind::Fail {
            reason: "no route".into(),
        },
    );
    assert_eq!(a.apply_event(&fail, CREATED_AT), Ok(State::Failed));

    let mut b = attempt();
    b.apply_event(&Event::new("e1", EventKind::Reserve), CREATED_AT)
        .unwrap();
    let fail = Event::new(
        "e2",
        EventKind::Fail {
            reason: "peer rejected".into(),
        },
    );
    assert_eq!(b.apply_event(&fail, CREATED_AT), Ok(State::Failed));
}

#[test]
fn duplicate_event_is_idempotent() {
    let mut a = attempt();
    let reserve = Event::new("e1", EventKind::Reserve);

    assert_eq!(a.apply_event(&reserve, CREATED_AT), Ok(State::Reserved));

    assert_eq!(a.apply_event(&reserve, CREATED_AT), Ok(State::Reserved));
    assert_eq!(a.state(), State::Reserved);

    a.apply_event(&Event::new("e2", EventKind::Commit), CREATED_AT)
        .unwrap();
    assert_eq!(a.apply_event(&reserve, CREATED_AT), Ok(State::Committed));
    assert_eq!(a.state(), State::Committed);
}

#[test]
fn snapshot_recovery_resumes_and_keeps_dedup() {
    let mut a = attempt();
    a.apply_event(&Event::new("e1", EventKind::Reserve), CREATED_AT)
        .unwrap();

    let json = a.to_json().unwrap();
    let mut restored = PaymentAttempt::from_json(&json).unwrap();

    assert_eq!(restored, a);
    assert_eq!(restored.state(), State::Reserved);
    assert!(restored.has_applied(&EventId("e1".into())));

    // A replayed event from before the snapshot is still deduplicated.
    assert_eq!(
        restored.apply_event(&Event::new("e1", EventKind::Reserve), CREATED_AT),
        Ok(State::Reserved)
    );

    assert_eq!(
        restored.apply_event(&Event::new("e2", EventKind::Commit), CREATED_AT),
        Ok(State::Committed)
    );
}
