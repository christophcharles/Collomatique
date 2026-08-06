# FIXME: incremental epochs ignore the incumbent time limit and the distance cutoff

Observed on a real run (run_solver UI): an epoch had several incumbents (so
progress events were flowing), the panel showed |obj − bound| = 7 with a distance
tolerance of 10, and no reconstruction-failure diagnostics appeared in the debug
view — yet the epoch kept solving, and the per-epoch after-incumbent time limit
never fired either.

Ruled out by code tracing (Aug 2026):

- Config plumbing. It is intact end to end: dialog → `IncrementalConfig` →
  `ConductorStrategy::incremental_substrategy` → serde over IPC
  (`StrategyRequest`) → per-epoch `SolveProblemOpts` (incremental.rs) →
  `IlpSolveRequest` → `solve_with_time_limits`. Nothing drops the values.
- Incumbent reconstruction failure (it would print "failed to reconstruct an
  incumbent" and the objective would display as "—").
- "The epoch has no incumbent yet" (incumbents were visibly accumulating).

What makes the joint failure strange: the two mechanisms live in different
processes. The incumbent limit is enforced solver-side (`SolveDeadlines` in
ilp/src/solvers/collo_cbc.rs — armed at the first reconstructed-incumbent event,
checked at every later event). The distance cutoff is strategy-side (the epoch
progress callback in incremental.rs returning false). Incumbent events were
flowing, so both had their trigger and both still failed.

Where to look next:

- The stop relay. The epoch callback's `false` sets `SolverSubprocess`'s stop
  flag (subprocesses/src/ilp_solver.rs), but the reader thread answers each
  Progress RPC from the *flag*, not from the consumer's return value, and the
  progress channel is unbounded. A backlogged consumer (each event does a
  blocking RPC round-trip up to the conductor) keeps answering `true` long after
  the cutoff event was generated.
- Whether the limits actually reach the solver subprocess at runtime: instrument
  `solve_ilp` (rpc-engine) to echo `request.time_limit` and
  `request.incumbent_time_limit` at solve start.
- The `p.incumbent.is_some()` gate in the epoch callback's `good_enough` needs
  the full solution vector. The panel cannot display that field
  (`NoObjectiveSolveProgress` prunes it by design), so its presence at the
  strategy level is unverified; echo it when instrumenting.

Probably not incremental-specific: every strategy's sub-solves use the same
solve path, so whatever breaks here likely affects e.g. `DefaultStrategy`'s
incumbent time limit too.
