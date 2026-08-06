# FIXME: the conductor's default strategy is not shut off on first solution

Observed while running the conductor with **both** warm start and incremental
enabled: when one of them produces a first solution, the queued
`DefaultStrategy` does not appear to stop. Worse, a *new* default strategy
seems to get launched instead.

This is a report from use, not a diagnosis — nothing has been traced yet. The
starting point is `strategies/src/strategies/conductor.rs`, where the workers
are queued (`ConductorStrategy::warm_start_config`, `incremental_config`,
`default_config`) and where the reaction to an incumbent lives.

To pin down before fixing:

- Whether the first default worker really keeps running, or only *looks* like
  it does in the log/UI.
- Whether the second default worker is a deliberate requeue (after the
  incumbent, to prove optimality from a warm start) that is simply not
  cancelling its predecessor, or an outright bug in the queueing.
- Whether the same thing happens with warm start alone and with incremental
  alone, which would say whether the combination is what breaks it.

Any fix should come with a regression test at the conductor level, since this
is exactly the kind of scheduling bug that comes back.
