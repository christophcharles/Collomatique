# Roadmap: greedy group-list generation

*Companion document: `greedy_algorithm.md` details point 1. This roadmap stays
general; points 2 and 4 will be detailed in future sessions.*

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
  (groups, use count). The ILP-era fields the greedy ignored — `ghost`,
  `canonical_range`, `pinned_pairs` — retired with point 3.
- Output: `Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>` — the exact
  payload of `GroupListsUpdateOp::AddGeneratedGroupLists`, mirroring
  `build_group_lists`.

Status: the objective, the prefill/greedy/ILP workflow, the student order,
the claim tie convention and the arithmetic (`f64`) are settled; the one
genuinely open question is the prefill coverage rule (§6.3 of the algorithm
document), implemented as written for now but kept easy to change.

## Point 2 — GUI integration

Today's flow: `generate_dialog` (pick pairs, kept lists, strategy, weights) →
`naming_dialog` (names + model build off-thread) → `run_solver` dialog
(subprocess solve) → `AddGeneratedGroupLists`.

The greedy replaces the solver leg: it is fast enough to run without the
solver dialog and without a subprocess. The generate dialog keeps the pair and
kept-list selection; the strategy/weights/canonical-range controls belong to
the ILP path and their fate is tied to point 3. To be detailed in a future
session.

## Point 3 (optional) — ILP redesign — **done**

Landed in `56124cef..19951a7d` on `greedy_group_lists`. "Send it to the ILP" is
now a strict refinement of the greedy rather than a different taste: at any
placement, the model's objective equals the greedy's score of that placement to
the last digit.

- Same collision objective, linearized by **expanding the square exactly**
  instead of with per-pair level binaries and ordering rows — the first
  deviation below. Two extra families: `Together { a, b, list, group }`, the
  "these two share this group" site binary, and `Coincide`, its pairwise
  product across two lists. Both are one-sided from above and pulled tight by
  the maximize. This costs more pair machinery than the old `SharedPair`; the
  cost is accepted because the ILP is now the optional path.
- One enumeration (`src/pairs.rs`) is the single source of truth for three
  readings — what is declared, what the objective weighs it at, and what the
  warm start values it as. Nothing else keeps them in lockstep, so nothing
  else may enumerate.
- Built with `with_maximize`, the constant term carried inside the objective's
  `LinExpr` rather than dropped as an offset: the model reports the greedy's
  number, not merely its argmax.
- **Retired**: the ghost (`ghost.rs`, `GenerationPlan::ghost`),
  `canonical_range` and its election, `pinned_pairs` (kept lists enter the
  objective directly, as constant mass), the size classes and the
  `class_weight` decay that priced meetings by size class, `ObjectiveWeights`,
  `SharedPair`/`co_occurrences`, the incremental epochs for this model, and the
  generate dialog's canonical-range and weight controls. `build_model(plan,
  frozen)` is the final signature.
- **Two solve modes**, as planned and already wired: fast mode fixes the
  prefilled assignments in the model and lets the ILP optimize the adjustment
  layer alone — a massive dimension reduction, but inheriting any contrived
  placement the frozen pure groups force; full mode fixes nothing and takes the
  complete prefill + greedy solution as warm start, the only mode that can
  revisit the pure groups themselves.
- The greedy's full solution seeds the solver through the existing `warm_start`
  plumbing (`StrategySubprocess::spawn` → `set_mip_start`).

The irony held: the redesigned model is *simpler* than the one it replaces even
though the objective is heavier — the retirement commit alone removed about ten
times the code it added.

### Two deviations from the sketch above

**Exact quadratic expansion, not level binaries with ordering rows.** Level
binaries compute the per-pair contribution exactly only when all of a pair's
shared sites carry the *same* mass. The license case (`greedy_algorithm.md`
§2.4 — a colle trio and a size-12 tutorial in one student's profile) is
precisely the unequal-mass case, and it is the case the objective exists to get
right. So the square is expanded outright: for binaries `(c + Σ mᵢzᵢ)² = c² +
Σ (2c·mᵢ + mᵢ²) zᵢ + 2 Σ_{i<j} mᵢmⱼ zᵢzⱼ`, and pairwise products suffice.

There are also **no symmetry-breaking ordering rows**, for a separate reason:
the greedy numbers its groups in claim order, not lexicographically, so such
rows would reject the warm start — and they can conflict with the frozen pins
of fast mode.

**Group sizes are equality rows, so "the three hard constraint families are
unchanged" is no longer true.** The `students_per_group` min/max pair is
replaced by one equality per group, `Σ_s x[l,s,g] == τ[l,g]`, at the balanced
targets. This is deliberate: constant τ is what makes every mass a constant
coefficient, and without it the objective is not linearizable at all. It costs
nothing elsewhere — the balanced sizes are part of the settled foundation
above, always feasible, and the greedy satisfies them by construction, so the
warm start stays valid.

### The anti-drift net

The two objectives are written independently — a running expansion of squares
in the model, a direct sum of squared partner distributions in the greedy — and
share only the mass formula. What guarantees they never drift is an equality
asserted at three scales. In the small,
`objective::tests::objective_matches_the_greedy_ground_truth` runs a battery of
hand plans — multi-tier lists, overlapping lists, kept lists, a kept-only
student, a multiplicity-0 spec. At document scale, the same equality holds on a
frozen copy of a real document (`tests/examples_build.rs`, which is a *copy* on
purpose: the subject is the code, so its context must not move with
`examples/`). And under a real CBC solve, the optimal value itself is checked
in `the_optimum_is_reached_at_a_tight_configuration`. `placement_objective` is
the ground truth all three compare against.

## Point 4 — Python API

There is deliberately no `group_lists.add_generated` door in the Python API
while generation is unsettled (`a7c434d0`). Once points 1–2 have stabilized,
expose the generation request, the greedy, and the resulting update op through
the API. To be detailed in a future session.
