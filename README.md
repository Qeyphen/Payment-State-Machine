# Payment Attempt State Machine

A library that models a payment attempt as an explicit state machine, with idempotent event handling and JSON snapshot recovery.

## Setup
Clone the project and build:

```bash
cargo build
```

## Tests

```bash
cargo test
```

## Design decisions, crate choices, tradeoffs

### How the machine works

Reserve holds credit with the counterparties, commit settles it:

```
Created --Reserve--> Reserved --Commit--> Committed
```

If nobody commits before the deadline, the attempt expires and the held credit has to go
back:

```
Reserved --Timeout--> TimedOut --Refund--> Refunded
```

`Fail` is allowed from `Created` or `Reserved`. `Committed`, `Refunded` and `Failed` are
terminal and reject anything that arrives afterwards.

`TimedOut` isn't terminal, on purpose. When a reservation expires the money is still
held, and "the deadline passed" and "the funds came back" are two different facts that
happen at different times. Keeping them apart means an attempt stuck in `TimedOut` shows
up as expired-but-not-yet-cleaned-up instead of being invisible.

`Timeout` is only legal from `Reserved`, because a `Created` attempt hasn't reserved
anything and so has nothing to time out, and only when `now >= timeout_at`. Timing out an
attempt with no deadline is an error rather than quietly allowed, otherwise anything could
be expired by anyone.

The timestamp is passed into `apply_event` rather than read from the system clock inside
it. That keeps the transitions pure and lets the timeout tests just pass in a number
instead of sleeping.

### Idempotency

I assumed at-least-once delivery: the same event can turn up twice and must not apply
twice.

Every event carries an id, and the attempt keeps the set of ids it has already applied.
`apply_event` checks that set before it validates the transition, so a redelivered event
returns the current state instead of failing as an illegal transition. The id is only
recorded once the transition is known to be legal, so an event that got rejected (a
`Timeout` that showed up early) can be retried later with the same id. The set is
serialised with the attempt, so dedup still works after a restart. That last part is what
makes recovery safe rather than just possible.

`state` and `applied_events` are private, so the only way to move an attempt is through
`apply_event`.

### Crates

`serde` and `serde_json` for the snapshot, since the derive is exactly what the
requirement asks for. `thiserror` for `TransitionError`, because this is a library and
callers should be able to match on the error rather than only print it (`anyhow` would be
the pick in a binary). Nothing else; the machine itself is plain std.

### What I traded away for time

- `amount` is a plain `u64`. In real money code I'd newtype it with checked arithmetic.
  Nothing here does arithmetic on it, so I left it alone.
- `Fail` carries a `String` reason. A typed enum would be matchable and countable, but a
  string was quicker.
- `next_state` lists the legal pairs and rejects the rest, instead of matching all 30
  state/event combinations. Writing them all out would give a compile error if someone
  adds a state and forgets a case.
- A duplicate event returns the current state, not the state the first application produced. A per-event result cache would fix that, at the cost of doubling what I store.
- The applied-ids set is never pruned. Fine for a short-lived attempt, not forever.
- The snapshot has no schema version, so an old snapshot won't survive a struct change.
- Tests cover the required paths, but not all 30 state/event pairs, and there's no property test.
- Nothing is validated on construction: an empty route or a zero amount is accepted.
