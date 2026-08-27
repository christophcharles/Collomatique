# Roadmap: greedy group-list generation

*Companion document: `greedy_algorithm.md` details point 1. This roadmap stays
general; points 1 and 2 are done, point 3 was built and then retired, and point
4 will be detailed in a future session.*

## Why this change

Group-list generation was an ILP when this was written
(`colloscopes/greedy-groups/`, then named `constraints-groups/`), and the
results were not convincing: slow to reach a good incumbent, and the
solutions themselves underwhelming. The diagnosis reached during the design
discussion: that objective was a *step* function — a pair of students
paid once for its first meeting in a size class, and every further meeting was
free — so the model literally could not see whether meetings concentrate on the
same partners. The ghost (template) grouping existed to patch that blindness by
pre-guessing one grouping and asking every list to resemble it; the whole
optimization was therefore anchored to an unverified greedy guess.

The new direction attacks the root instead:

- a **new objective** that measures partner concentration directly (see below
  and `greedy_algorithm.md`); the ghost becomes unnecessary;
- a **greedy algorithm** as the primary generator: a teacher-quality solution in
  negligible time;
- the **ILP demoted to an optional polish** for users who really want optimized
  lists (not really a teacher concern — a nicety for students, and useful for
  making the later colloscope easier to solve). This last step was carried out
  and then undone: see point 3 for what the polish turned out to be worth.

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
- **The workflow**: `prefill -> greedy`, written at the time as
  `prefill -> greedy -> optional ILP`. Prefill places the obvious students
  first (whole groups tiled from a single profile) — a minimal-energy state on
  that subset, close to but not necessarily part of the global optimum; the
  greedy places the rest. The third leg existed for a while and is gone
  (point 3), so the greedy's answer is the answer.

## Point 1 — the greedy algorithm — **done**

Detailed in `docs/plans/greedy_algorithm.md`. Summary:

- New module `colloscopes/greedy-groups/src/greedy.rs` (with submodules,
  tests in `greedy/tests.rs`). The crate was called `constraints-groups/` then;
  it was renamed once the greedy was all that was left in it.
- Input: `GenerationPlan`, extended with a description of the kept lists
  (groups, use count). The ILP-era fields the greedy ignored — `ghost`,
  `canonical_range`, `pinned_pairs` — retired with point 3.
- Output: `Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>` — the exact
  payload of `GroupListsUpdateOp::AddGeneratedGroupLists`, mirroring
  `build_group_lists`. Point 3 wrapped that vector in a `GreedyOutcome` to
  carry the prefilled seats over to the model; with the model gone, the
  signature is back to the one written here.

The kept-list plumbing landed in `a33a69f7`, the generator itself in
`2ab5e0f5`, and its regression net — every example, plus a random-walk fuzz
build — in `8b06c7a1`. The objective, the prefill/greedy/ILP workflow, the
student order, the claim tie convention and the arithmetic (`f64`) are
settled.

One question stays deliberately open, and being done does not close it: the
prefill coverage rule (§6.3 of the algorithm document) is implemented exactly
as written, as the first attempt, and kept easy to change.

## Point 2 — GUI integration — **done**

The flow before: `generate_dialog` (pick pairs, kept lists, strategy, weights)
→ `naming_dialog` (names + model build off-thread) → `run_solver` dialog
(subprocess solve) → `AddGeneratedGroupLists`.

The greedy took the solver leg (`e1e88578`). `naming_dialog` is now the step
that generates: it builds the plan, runs the greedy off the UI thread while
streaming its log into a `DebugView`, and "Valider" lands that answer as it
stands — no subprocess, no solver dialog. The ILP became a door off the same
answer: "Optimiser les listes de groupes" opened
`naming_dialog/optimize_dialog.rs`, and the polish ran from the very plan the
greedy ran on, with the greedy's own lists as warm start. That door was removed
with point 3 (`7431205b`), together with the optimize and model-build dialogs;
what is left of the chain is generate dialog → naming dialog → validate.

The generate dialog kept the pair and kept-list selection. The controls that
belonged to the ILP path moved to the optimize window when point 3 landed
(`3cab1012`) — the strategy and "Figer le pré-remplissage" — and went with it;
the weight and canonical-range controls were already gone.

## Point 3 (optional) — ILP redesign — **retired**

Built in `56124cef..19951a7d`, used, and then removed in `7431205b..96916ac1`.
The design did what it promised; it was still not worth keeping.

**What was built.** The same collision objective as the greedy, linearized by
expanding the square exactly rather than with per-pair level binaries: two extra
variable families, `Together { a, b, list, group }` ("these two share this
group") and `Coincide` (its pairwise product across two lists), both one-sided
from above and pulled tight by the maximize. Balanced group sizes became
equality rows at the targets, which is what makes every mass a constant
coefficient — without constant targets the objective is not linearizable at
all. One pair enumeration was the single source of truth for what is declared,
what the objective weighs it at, and what the warm start values it as. Two solve
modes: prefilled seats frozen (a large dimension reduction, inheriting whatever
the pure groups force), or nothing frozen with the complete prefill + greedy
solution as warm start. The ghost, `canonical_range`, `pinned_pairs`, the size
classes and `ObjectiveWeights` were retired along the way — that redesign
removed about ten times the code it added.

**It was verified, and the verification held.** The two objectives were written
independently and shared only the mass formula, so an equality was asserted at
three scales: hand-built plans in `objective::tests`, a frozen copy of a real
document in `tests/examples_build.rs`, and the optimal value itself under a real
CBC solve. At any placement the model's objective equalled the greedy's score of
that placement to the last digit. "Send it to the ILP" really was a strict
refinement of the greedy rather than a different taste.

**Why it went.** The refinement did not pay in practice. On a real document the
solve takes a long time, and what comes back is barely better than the greedy's
own answer, when it is better at all. The greedy is already teacher-quality and
costs milliseconds. A second generator that spends minutes to move the objective
by nothing is a maintenance burden with nobody on the other end of it, so the
user's verdict after living with it was to drop it.

**What the removal took.** The optimize and model-build dialogs and their wiring
(`7431205b`, 813 lines), then the model itself (`78e1908d`, 5820 lines): the
builder, the three constraint families, the variable and extra-variable types,
the pair enumeration, the objective, the frozen-placement pins, both
`ConfigData` conversions, the solve smoke test, the anti-drift equality with its
frozen document, and the ILP fuzz walk in `property-tests`. `GreedyOutcome`
collapsed back to the plain vector of point 1, and `placement_objective` — which
was public only so the equality could be asserted from outside — went with it.
What survived is the greedy, the balanced targets, and the two mass constants
(`src/mass.rs`). The crate, being only the greedy now, was renamed
`colloscopes/greedy-groups/` (`96916ac1`).

The full account of the design, including the two deviations from the original
sketch and the anti-drift net in detail, is the previous version of this
section: `git show 2e1233ac:docs/plans/greedy_roadmap.md`. The code is at the
same commit, under `colloscopes/constraints-groups/`.

## Point 4 — Python API — **done**

Shipped in five commits, `22d6bc18..2ed2177a`. The API hole `a7c434d0` left
open is filled; `docs/python/new_api_design.md` §10 is the authority for the
shape, and this section only records what moved where.

Two pieces moved out of the GUI so both front ends share them, rather than
being reimplemented on the Python side where they would drift:
`default_generation_request` — the generate dialog's opening selection — went
to `greedy-groups/src/specs.rs` beside `GenerationRequest`, and the coverage
label — the naming dialog's default row name — went to `ui-text`'s
`rendering.rs` beside the entity renderers. The dialogs now call them.

The API itself: a `GroupListsGenerationRequest` value, a
`GroupListsGenerationResult` pyclass carrying `entries` and `skipped`, the
three doors `doc.default_generation_request()`,
`doc.generate_group_lists(request, *, on_log=None)` and
`doc.group_lists.add_generated(entries)`, and one exception,
`GroupListsGenerationError`, carrying the plan's own sentence. There is no
`names` parameter — the dedup makes the list count unknowable before the plan
exists — and no objective weights, since the greedy has one fixed objective and
the ILP that had tunable ones is retired.

Adjacent, and decided with the same session: the Python `ConductorStrategy`
gained `warm_start_incumbent`, which the Rust struct and the solve dialog have
had all along. It is inert from a script — a script's solve hands the conductor
no ready-made solution — and it is mirrored anyway, so a strategy built in
Python is the application's structure whole (`95ea8eba`).
