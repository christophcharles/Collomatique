# Roadmap: greedy group-list generation

*Companion document: `greedy_algorithm.md` details point 1. This roadmap stays
general; points 2, 3 and 4 will be detailed in future sessions.*

## Why this change

Group-list generation is an ILP today (`colloscopes/constraints-groups/`), and
the results are not convincing: slow to reach a good incumbent, and the
solutions themselves are underwhelming. The diagnosis reached during the design
discussion: the current objective is a *step* function — a pair of students
pays once for its first meeting in a size class, and every further meeting is
free — so the model literally cannot see whether meetings concentrate on the
same partners. The ghost (template) grouping exists to patch that blindness by
pre-guessing one grouping and asking every list to resemble it; the whole
optimization is therefore anchored to an unverified greedy guess.

The new direction attacks the root instead:

- a **new objective** that measures partner concentration directly (see below
  and `greedy_algorithm.md`); the ghost becomes unnecessary;
- a **greedy algorithm** as the primary generator: a teacher-quality solution in
  negligible time;
- the **ILP demoted to an optional polish** for users who really want optimized
  lists (not really a teacher concern — a nicety for students, and useful for
  making the later colloscope easier to solve).

## The settled foundation

These decisions are fixed and shared by every point below:

- **One list per `GroupListSpec`**, unchanged: subjects sharing the same
  student set and size range get the same list.
- **Minimal group count** `k = ceil(n / max)`, imposed, and **balanced sizes**
  fixed upfront: `n % k` groups of `ceil(n / k)` then groups of
  `floor(n / k)`, in descending order. Always feasible (proof in the
  algorithm document), monotone descent for free.
- **The objective**: maximize the sum, over students, of the *collision
  probability* of the student's partner distribution — pick two of the
  student's grouping decisions at random, each pointing at a random partner in
  its group; reward the probability that both point at the same person. This
  is a weighted Mirkin/Rand-family objective; exact definition, weighting and
  rationale in `greedy_algorithm.md`. Group lists count once per (period,
  subject) pair using them; kept prefilled lists count as real grouping
  decisions with their own multiplicity.
- **The workflow**: `prefill -> greedy -> optional ILP`. Prefill places the
  obvious students first (whole groups tiled from a single profile) — a
  minimal-energy state on that subset, close to but not necessarily part of
  the global optimum; the greedy places the rest; the ILP, if used at all,
  optimizes either the adjustment layer alone (prefill fixed + greedy as warm
  start) or the whole model (prefill + greedy as warm start).

## Point 1 — the greedy algorithm

Detailed in `docs/plans/greedy_algorithm.md`. Summary:

- New module `colloscopes/constraints-groups/src/greedy.rs` (with submodules,
  tests in `greedy/tests.rs`).
- Input: `GenerationPlan`, extended with a description of the kept lists
  (groups, size range, use count). The greedy ignores `ghost`,
  `canonical_range` and `pinned_pairs`.
- Output: `Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>` — the exact
  payload of `GroupListsUpdateOp::AddGeneratedGroupLists`, mirroring
  `build_group_lists`.

Status: the objective and the prefill/greedy/ILP workflow are settled; the
internals of the greedy pass (student order, placement subroutine) and parts
of the prefill are proposed but not settled — see the status labels in the
algorithm document.

## Point 2 — GUI integration

Today's flow: `generate_dialog` (pick pairs, kept lists, strategy, weights) →
`naming_dialog` (names + model build off-thread) → `run_solver` dialog
(subprocess solve) → `AddGeneratedGroupLists`.

The greedy replaces the solver leg: it is fast enough to run without the
solver dialog and without a subprocess. The generate dialog keeps the pair and
kept-list selection; the strategy/weights/canonical-range controls belong to
the ILP path and their fate is tied to point 3. To be detailed in a future
session.

## Point 3 (optional) — ILP redesign

Redesign the ILP model around the same objective, so "send it to the ILP" is a
strict refinement of the greedy, not a different taste:

- Same collision objective, linearized with per-pair level binaries and
  ordering rows. This costs more pair machinery than today's `SharedPair`; the
  cost is accepted because the ILP is now the optional path.
- The three hard constraint families are unchanged.
- **Retired**: the ghost (`ghost.rs`, `GenerationPlan::ghost`),
  `canonical_range`, `pinned_pairs` (kept lists enter the objective directly),
  the incremental epochs for this model, and the generate dialog's
  canonical-range control.
- **Two solve modes.** Fast mode: the prefilled assignments are fixed in the
  model, the greedy's placements seed the rest as warm start, and the ILP
  only optimizes the adjustment layer — a massive dimension reduction, better
  than the epoch heuristic it replaces, but inheriting any contrived
  placement the frozen pure groups force. Full mode: nothing fixed, the
  complete prefill + greedy solution as warm start — the only mode that can
  also revisit the pure groups themselves.
- The greedy's full solution can seed the solver through the existing
  `warm_start` plumbing (`StrategySubprocess::spawn` →
  `set_mip_start`).

The irony is welcome: the redesigned model is *simpler* than today's even
though the objective is heavier. To be detailed in a future session.

## Point 4 — Python API

There is deliberately no `group_lists.add_generated` door in the Python API
while generation is unsettled (`a7c434d0`). Once points 1–2 have stabilized,
expose the generation request, the greedy, and the resulting update op through
the API. To be detailed in a future session.
