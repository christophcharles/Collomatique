# TODO: the stop relay answers a Progress RPC before the consumer has seen it

`SolverSubprocess`'s RPC handler (subprocesses/src/ilp_solver.rs:127-150) answers
each `SolverMsg::Progress` from `stop_flag` (line 144). The callback it runs
first, on line 141, only pushes the event into an unbounded channel
(subprocesses/src/subprocess_solve_backend.rs:65-72). The consumer that actually
decides — `on_progress`, and `handle.stop()` on its `false` — runs later, on the
async task (same file, 102-105). So the answer to event N is sent before anything
has looked at event N: a stop asked for at N takes effect at N+1 at the earliest,
and at N+k if the channel has backed up.

Found while tracing why incremental epochs ran past their after-incumbent limit
and distance cutoff (Aug 2026). It was **not** the cause — the events were being
dropped inside collo-cbc long before they reached here (`70dc27f6`) — and the
100 ms rate limit in rpc-engine's `solve_ilp` (`538253d1`) stops the channel
backing up, so the lag is now about one event. Recorded because the structure is
unchanged: the relay is still one event behind, and a slow consumer still widens
that.

The fix is to answer from the consumer's own return value instead of from a flag
the consumer sets asynchronously. That means either the RPC answer waits on the
async task, or the decision moves onto the reader thread.
