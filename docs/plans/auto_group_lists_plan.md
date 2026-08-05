# Automatic group-list prefilling — high-level plan

Status: roadmap (2026-08-04, revised after review). This is not an
implementation plan. It splits the work into pieces that can be done one at a
time. Each piece gets its own detailed implementation plan (full prose,
old+new code snippets) when we start it. Branch context:
`group_lists_auto_prefill`.

## 1. Goal

The disabled button "Générer des listes automatiquement"
(`gtk4/src/editor/group_lists.rs:84-93`) will run a *standalone* solver that
builds **prefilled** group lists, before and independently of any colloscope
resolution.

Why: solving group lists jointly with the colloscope (the current "automatic"
group lists) multiplies the two problem complexities (roughly N·M). Solving
them one after the other costs N + M, with N ≪ M. The price is that the group
lists cannot adapt to the colloscope shape. We compensate with the same
heuristics a human uses when prefilling by hand:

- use as few groups as possible (within the students-per-group policy);
- keep groups stable across group lists — proxied by minimizing the number of
  *pairs of students* that share a group anywhere.

The output is a set of new **prefilled** group lists, plus their associations
to `(period, subject)` pairs, applied to the document as one undoable step.

## 2. The model

### 2.1 Group-list specs (the indexing)

The solver does not know about subjects. It builds one group list per distinct
**spec**:

```rust
pub struct GroupListSpec {
    /// Exactly the students that must be placed. Every one gets a group.
    pub students: BTreeSet<StudentId>,
    /// The allowed group size range, taken from the subject's
    /// `SubjectInterrogationParameters.students_per_group`.
    pub students_per_group: NonEmptyRangeInclusive<NonZeroU32>,
}
```

Specs are collected from the user's selection of `(period, subject)` pairs to
rebuild: for each pair, the registered students come from
`Assignments::students(period, subject)`
(`state-colloscopes/src/assignments.rs:33`), the range from the subject's
interrogation parameters (`state-colloscopes/src/subjects.rs:69`). Pairs that
produce the same `(students, range)` key are **deduplicated into one spec**,
hence one shared list. This is a known, accepted limitation: two subjects with
identical students and identical ranges always share a list (in particular one
subject across several periods with no student change). Running the tool
several times works around it when needed.

The number of group *slots* in a spec's model is
`max(1, floor(n / min_size))` where `n = students.len()` — more groups than
`floor(n / min_size)` cannot all satisfy the minimum size. The clamp matters
when a spec has fewer students than the minimum group size (say 2 students,
minimum 3): the bare formula gives 0 slots and the `StudentGroup` domain
would be empty. With the clamp such a spec gets a single, necessarily
undersized group, and the conditional min-size constraint (§2.3) then makes
its model infeasible — the correct signal, since the data genuinely cannot
satisfy the students-per-group policy.
The objective then minimizes how many slots are actually used. (At least
`ceil(n / max_size)` groups are always needed; pre-fixing that many
`GroupHasStudents` to 1 is an optional strengthening, to be decided in the
piece plan.)

### 2.2 Variables

Base variables, mirroring `constraints-colloscopes/src/vars.rs` but simpler:

- `Var::StudentGroup { list: GroupListIdx, student: StudentId }` — integer in
  `0..=slots-1`. Unlike the colloscope crate there is **no −1 value**: every
  student in a spec is registered and must be placed, so the domain itself
  enforces "exactly one group". The `students_have_groups` constraint
  disappears entirely.

(`GroupListIdx` is the index into the deduplicated `Vec<GroupListSpec>`.)

Extras (reified, as in `constraints-colloscopes/src/extras.rs`):

- `StudentInGroup { list, student, group }` — boolean ⟺
  `StudentGroup == group`. Same reification as
  `constraints-colloscopes/src/extras.rs:261-289`.
- `GroupHasStudents { list, group }` — boolean ⟺ some `StudentInGroup` in
  that group is 1. Used by the min-size constraint, the ascending-order
  constraint, and the objective.
- `PairInGroup { a, b, list, group }` (with `a < b`) — boolean ⟺ both
  students sit in that group of that list (the AND of the two
  `StudentInGroup`s).
- `SharedPair { a: StudentId, b: StudentId }` (with `a < b`) — boolean ⟺ the
  pair shares **any** group in **any** list: reified as
  `Σ PairInGroup(a, b, l, g) >= 1` over every list `l` containing both.
  Defined only for pairs that co-occur in at least one *new* spec.

All reifications are **full equivalences**, never one-sided implications that
rely on objective pressure for the missing direction: several solve strategies
strip the objective, and the variables' values must stay correct there.

Kept lists (see §2.5) add no variables. Conceptually, the OR behind
`SharedPair` ranges over the kept lists too — but there the "both in that
group" terms are compile-time **constants**, exactly like the constants the
colloscope crate substitutes for prefilled lists in its reifications
(`constraints-colloscopes/src/extras.rs:291-342`). For a pair already grouped
in a kept list, one constant term is 1: the upper-bound side of the
equivalence becomes trivially true, and the lower-bound side degenerates to
`SharedPair = 1`. Implementation-wise the variable is **pinned to 1** and its
reification constraints are **omitted entirely**: with the constant term in,
every one of them is degenerate (trivially true), so omitting them is exactly
equivalent to writing them — and sends that many fewer constraints to the
solver. The pin is a **degenerate reified definition**, `and_reified(var, ||
vec![])`, which the machinery turns into the single row `indicator = 1`
(`ilp-modeler/src/bundle.rs:530-536`, the idiom of
`constraints-colloscopes/src/extras.rs:637`). It is *not* the `check_fix`
mechanism of `Var::fix_student_group`, as an earlier draft of this paragraph
claimed: the modeler's fixer chain is typed `Fn(&B, &Env) -> Option<f64>`
(`Modeler::add_fixer`) and reaches base variables only, never extras.
Grouping such a pair in a new list
then costs nothing — which is exactly the stability heuristic: the cheapest
solution reuses last period's groupings where the student sets allow it.

### 2.3 Constraints

Per list `l`, per group `g` (mirroring `constraints-colloscopes/src/groups/`):

- **Max size** (from `students_per_group.rs`): `Σ_s StudentInGroup <= max`.
- **Min size, conditional on non-empty**:
  `Σ_s StudentInGroup >= min · GroupHasStudents(g)`.
- **Ascending fill order** (from `groups_filled_by_ascending_order.rs`):
  `GroupHasStudents(g) >= GroupHasStudents(g+1)`. Doubles as symmetry
  breaking; empty groups form a suffix, which the output conversion relies on
  (defensively — see §3).

Not carried over, and why:

- `students_have_groups.rs` — replaced by the variable domain (§2.2).
- `forbidden_groups.rs` — couples groups to interrogation slots/weeks; there
  is no colloscope here.
- `students_per_group_for_subject.rs` — exists because a shared list can serve
  subjects with *different* student sets. Here a spec's student set is by
  construction exactly each covered subject's registered set, and its range is
  the subject range, so the per-subject constraint is vacuous.

The crate defines its own `ConstraintDesc` as a plain descriptive enum, one
variant per constraint family, with **no severity tiers**: the tier machinery
of `constraints-colloscopes/src/types.rs:368-445` feeds the colloscope warning
display in gtk4, which has no counterpart here — we just solve, and every
constraint matters equally.

### 2.4 Objective

Minimize `w_groups · Σ GroupHasStudents + w_pairs · Σ SharedPair`.

The weights are **configurable**: `ObjectiveWeights` is handed to
`build_model` next to the plan, and the generate dialog's "Paramètres avancés"
modal edits it. The default is strongly **group-dominant**, `w_groups = 1000`
and `w_pairs = 1`, which makes the pair term a tie-breaker among the solutions
that already use as few groups as possible.

That default is the result of two amendments. The weights first shipped as
hardcoded constants that were **equal** (`w_groups = w_pairs = 1`), on the
theory that the two terms merely pull in opposite directions (fewer groups
means fuller groups, which creates more pairs inside each list) and would need
tuning from experience. Piece 9 measured that equal weights are worse than "a
starting point to adjust from": they make the two terms cancel *exactly* in the
cases that matter, so the group count is drowned rather than merely outvoted
(the argument is in the piece-9 record in §5). Piece 11 therefore both exposed
the weights and changed the default, rather than only exposing them.

### 2.5 Kept lists

The user can select existing **prefilled** lists whose pairings should count
as already-shared (the "keep semester 1, rebuild semester 2" use case). Only
prefilled lists qualify: automatic lists store no student assignment (their
filling lives in the colloscope, when it exists at all), and the whole point
of this tool is to move toward prefilled lists. Effect on the model: pair
pinning only, as described in §2.2.

### 2.6 Epochs

The incremental strategy should build the small lists first, ordered by
**inclusion of their student sets**: the first epochs hold lists that can be
solved essentially independently; later epochs hold the larger lists that
contain them, and those align their groups with the already-built small ones
through the pair objective.

The algorithm (agreed). A spec's epoch is the height of the longest
strict-inclusion chain below it:

```text
epoch(S) = 0                                        if no spec's set is
                                                    strictly included in S's,
epoch(S) = 1 + max { epoch(T) | T.students ⊊ S.students }   otherwise.
```

Concretely, no recursion is needed. A strict subset always has strictly
fewer students, so processing the specs by ascending student count guarantees
that every strict subset of a spec is already computed when the spec's turn
comes:

```text
sort the specs by students.len(), ascending
for each spec S in that order:
    epoch[S] = 0
    for each already-processed spec T with T.students ⊊ S.students:
        epoch[S] = max(epoch[S], epoch[T] + 1)
```

That is k(k−1)/2 subset tests (`BTreeSet::is_subset`, plus a size comparison
for strictness), with k = the spec count — bounded by the number of selected
`(period, subject)` pairs, so cost is a non-issue. Two specs with equal
student sets (different size ranges) never strictly include each other, so
they simply never relate. Every `StudentGroup` variable of a spec gets its
spec's epoch.

Properties, on an example — German LV2 `{a…f}` and Spanish LV2 `{g…m}`
(disjoint), a sciences subject `{a…t}` containing both, the whole class
`{a…z}`:

- The two LV2 lists contain no other spec → epoch 0. Being disjoint, they
  share no student, so no `SharedPair` spans them: the epoch-0 subproblem
  decomposes into independent blocks — the "solved basically independently"
  intent.
- The sciences list strictly contains both → epoch 1; the whole class →
  epoch 2. Each aligns to the groupings built before it.
- Two *overlapping but incomparable* sets get the same height: neither is
  "smaller", so neither waits for the other. (They were solved jointly until
  piece 12bis, which gives every spec its own epoch — see below; the more
  entangled of the two solves later.)
- "Small first" really means "inclusion-minimal first": a large list that
  contains no other spec also lands in epoch 0, because nothing smaller
  exists for it to wait for.
- Equal student sets with different size ranges: neither strictly includes
  the other → same epoch. Size ranges play no role in the ordering.

The epoch map covers **base variables only** — verified against the
incremental strategy's contract (`strategies/src/strategies/incremental.rs:18-32`):
entries naming non-base variables are silently ignored (piece 0 makes them
*unrepresentable* instead — see §5), base variables absent from the map are
solved in the final epoch (max + 1), and an empty map means a single priming
solve. Extras are handled by the strategy's own surrogate machinery, so
listing every `StudentGroup` variable is both necessary and sufficient.

Until piece 10, the skeleton ships an **empty epoch map**, which by the above
contract is a plain single solve — no placeholder code needed.

Refined in phase C. On realistic documents this ordering produces only two
epochs, and the first one still bundles many lists at once. Piece 12 (§5) first
cut each level into its connected components, but real documents overlap too
much for that: the components fuse into one big block and the level solves as a
single large model anyway. Piece 12bis (§5) therefore gives **every spec an
epoch of its own**, and inside a level runs the least-entangled lists first —
ascending by the number of distinct students the spec shares with the other
specs of that level, then by student count, then by spec index for determinism.
The inclusion ordering itself is untouched: every spec of a level is numbered
before every spec of the next, so the recurrence above still decides *which
lists wait for which*, and the refinement only decides how far the resulting
stages are broken apart. §5 records what this costs — overlapping lists of a
level now solve apart on purpose, so the staged result can differ from the joint
one even at the true optima.

### 2.7 No `SolveConfig` equivalent

The colloscope crate's `SolveConfig` layer (filter/pin/anchor,
`constraints-colloscopes/src/config.rs:261-453`) has no counterpart here. All
the configuration happens *upstream*, in the `GenerationRequest` (§3): the
model is always built whole, in one pass. This is why the loading UI can be
simpler than `loading_dialog.rs` (no 1/3–3/3 filtering phases, just one build
with a streamed log).

Piece 11 (§5) added the two objective weights as a build-time parameter, which
does not contradict this. What §2.7 rules out is the *filter/pin/anchor
machinery* and the staged builds that come with it; a pair of numbers read by
the objective builder is not that layer, and the model is still built whole in
one pass.

## 3. Input and output helpers (in the new crate)

The translation between document state and model must live in
`constraints-groups/`, because spec construction and model indexing are
intertwined.

Input side:

```rust
pub struct GenerationRequest {
    /// (period, subject) pairs to build new lists for.
    pub rebuild: BTreeSet<(PeriodId, SubjectId)>,
    /// Existing prefilled lists whose pairs are pinned as already-shared.
    pub kept_lists: BTreeSet<GroupListId>,
}

pub struct GenerationPlan {
    /// Deduplicated specs, each with the (period, subject) pairs it covers.
    /// The covered pairs are needed for default naming (UI) and for the
    /// final association step.
    pub specs: Vec<(GroupListSpec, BTreeSet<(PeriodId, SubjectId)>)>,
    /// (period, subject) pairs skipped because nobody is registered —
    /// reported so the UI can warn instead of silently dropping them.
    pub skipped: BTreeSet<(PeriodId, SubjectId)>,
    /// Pairs of students fixed to "already shared" by the kept lists.
    pub pinned_pairs: BTreeSet<(StudentId, StudentId)>,
}

pub fn build_generation_plan(
    params: &Parameters,
    request: &GenerationRequest,
) -> GenerationPlan;
```

`pinned_pairs` only contains pairs where both students appear in at least one
spec — other pairs never get a variable.

Output side (the `convert.rs` analog):

```rust
pub fn build_group_lists(
    plan: &GenerationPlan,
    names: &[String],                 // one per spec, from the naming dialog
    config: &ConfigData<Var>,         // base vars only, after filter_transmute
) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>;
```

Each produced `GroupList` is `Prefilled`, with `students_per_group` = the
spec's range, `group_names` = `vec![None; used_groups]` (unnamed groups), and
groups compacted to the non-empty ones (the ascending-order constraint makes
empties a suffix, but the conversion compacts and remaps defensively rather
than assuming it). Built through `GroupList::new` so the sealed invariants
hold.

## 4. Applying the result: a new composite op (settled)

The change is additive and structured — create k lists, associate them —
which matches the shape of the `ops/` composite layer. So: a new variant

```rust
GroupListsUpdateOp::AddGeneratedGroupLists(
    Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>,
)
```

following the `DuplicatePreviousPeriod` precedent
(`ops/src/group_lists.rs:301-396`): one `CascadeSession`, a loop of
`AddNewGroupList` + `AssignToSubject` elementary ops, `commit` collapses it
all into **one undo slot**. It flows through the normal
`EditorInput::UpdateOp` → `dry_apply` → warning dialog path
(`gtk4/src/editor.rs:1183-1232`) — the group-lists page already speaks
`GroupListsUpdateOp`, so no new editor plumbing; fresh `GroupListId`s are
issued by the session, not hand-managed; cascade warnings surface instead of
being bypassed. (The rejected alternative was `Op::GlobalUpdate`, the
colloscope-solver precedent at `gtk4/src/editor.rs:1278-1298` — also one undo
slot, but it bypasses the warning machinery and needs hand-managed ids.)

Associations for the rebuilt `(period, subject)` pairs **overwrite** any
existing entry (that is `AssignToSubject`'s semantics). A previously
associated list may end up orphaned (associated nowhere); it is kept, not
auto-deleted — deleting is the user's call. (An orphaned list is legal state;
see the standing carve-out in the state-consolidation Appendix A.5.)

The modal dialog chain freezes the document during the whole flow (same
guarantee the colloscope chain relies on, `gtk4/src/editor/colloscope.rs:759-761`),
so the plan computed at config time is still valid at apply time.

## 5. The pieces

Ordered as a **walking skeleton**: get the full pipeline running end to end
with a trivial model first, then grow the real model inside it. The UI pieces
are mechanical; doing them against the trivial model derisks the integration
work (generic `run_solver` instantiation, payload wiring, op application)
before the interesting constraint work starts. Once the skeleton stands,
every model piece has a live testbed: run the tool, look at the lists.

### Phase 0 — groundwork

**Piece 0 — type-tighten the incremental epoch payload.** Today
`IncrementalPayload<V>` is keyed by the *problem* variable type
(`V = InternalVar<B, E>` in real use), and entries naming extras or helpers
are silently ignored — the strategy immediately projects the map down to
`HashMap<B, u32>` anyway (`strategies/src/strategies/incremental.rs:299-305`).
Fix: key the payload by the base type, making non-base entries
unrepresentable. The `Strategy` trait's payload is a GAT `type Payload<V>`
instantiated at `InternalVar<B, E>` (`strategies/src/lib.rs:400-416`); it
becomes two-parameter (`type Payload<B, E>` — e.g.
`FindClosestPayload<InternalVar<B, E>>` for strategies that genuinely carry
full configs, `IncrementalPayload<B>` for incremental) or an equivalent
base-projection trait; settled in the piece plan. The IPC format is
untouched: `IncrementalPayloadData` stays a `Vec<Option<u32>>` aligned to
`var_order`, and aligning a base-keyed map against an `InternalVar` order is
still well-defined (`Base(b)` → lookup, anything else → `None`). Ripples:
the `StrategyPayload` union and its serialization impls
(`strategies/src/lib.rs:595-723`), `ConductorPayload` (which embeds the
incremental payload, `strategies/src/strategies/conductor.rs:454-456`), the
`Strategy` impls' associated-type lines, `build_incremental_epochs` in
constraints-colloscopes (which *simplifies*: it is generic over the extra
space today only because of this keying, and becomes a plain
`HashMap<Var, u32>`), and gtk4's `build_incremental_payload`
(`loading_dialog.rs:49-56`). Residual, not typable away: an entry naming a
base variable absent from the model (a stale name) stays representable and
keeps the documented "ignored" behavior.

**Done** — `d8d42ef0` (*strategies: key the incremental epoch payload by the
base variable type*).

### Phase A — the skeleton

**Piece 1 — minimal crate + full translation layer.** New crate
`constraints-groups/` (`collomatique-constraints-groups`), truly minimal on
the model side: the env and spec types, `Var::StudentGroup` with its
`DescribeVar` derive — and **no constraints, no extras, no objective**.
`build_model` / `build_model_with_log` (same log-callback signature as
`constraints-colloscopes/src/builder.rs:14-62`) return the bare model. The
translation layer, however, is built in full: `GenerationRequest`,
`build_generation_plan` (dedup, skipped reporting, pinned-pair extraction),
`build_group_lists` (compaction, `GroupList::new`) — see §3. Solving this
model produces arbitrary, probably absurd, but structurally valid group
lists. The epoch map is empty — a documented single priming solve (§2.6).
The fuzz-build net comes with this piece already — a `property_build.rs` /
`examples_build.rs` analog of
`constraints-colloscopes/tests/property_build.rs` over random
`GenerationRequest`s, cheap against the trivial model, guarding every later
piece. To verify here: that the solver machinery accepts a model with no
constraints and an empty objective.

**Done** — two commits. `d97e2e3d` (*state-colloscopes:
NonEmptyRangeInclusive implements Ord*) is the prerequisite: the dedup map is
keyed by the spec itself, and a spec carries a `students_per_group` range, so
the range needs a total order. It is a hand-written lexicographic order on the
endpoint pair, because the wrapped `std::ops::RangeInclusive` implements `Eq`
and `Hash` but not `PartialOrd`, and it is a storage order only — it says
nothing about set inclusion, and the type's `ContentOrd` stays discrete.
`6335e003` (*constraints-groups: minimal crate with the full translation
layer (piece 1)*) is the crate itself, with the three tests described above.
Two decisions the piece plan settled and the code carries: the slot-count
clamp now recorded in §2.1, and the split between a malformed request (a
`GenerationPlanError` — the config dialog only offers valid choices, so these
are caller bugs) and a pair with no registered students (which merely lands
in `skipped`).

**Piece 2 — config dialog.** `gtk4/src/editor/group_lists/generate_dialog.rs`,
modeled on `gtk4/src/editor/colloscope/config_dialog.rs` (same
window/headerbar/paned/factory skeleton, `update_vec_deque` refresh idiom).
Left pane: one `adw::PreferencesGroup` per period, one `adw::SwitchRow` per
subject *with interrogations* running on that period. Right pane: one
`SwitchRow` per existing **prefilled** list. Bottom frame: the solver-strategy
presets, reusing `conductor_config::Dialog`. Output:
`(GenerationRequest, ConductorStrategy)`. Defaults (agreed): rebuild switches
on for pairs with **no current association**, off otherwise; kept-list
switches all on. This makes the two obvious cases right by default: an empty
document, and a second period with no associations yet.

**Done** — two commits. `c7a2d91a` (*gtk4: the group-lists page holds the
whole colloscope Parameters*) is the mechanical prerequisite: the page stored
five separate clones, and the dialog echoes its parameters back on `Accepted`
the way `colloscope/config_dialog.rs` does, so that what piece 3 builds a model
from is literally what the user configured against — and `build_generation_plan`
takes a whole `&Parameters` anyway. `663c457b` (*gtk4: configuration dialog for
automatic group-list generation (piece 2)*) is the dialog, its two factory
modules, and the `collomatique-constraints-groups` dependency (used for the
`GenerationRequest` type only; no model is built here). On "Valider" the page
stores the strategy and drops the request, with a comment naming piece 3 as the
consumer.

Two deviations from the piece order above, both decided in the piece plan.
First, **the button is enabled here, not in piece 6**: an insensitive button
leaves `GroupListsInput::GenerateClicked` constructed nowhere, which is a
dead-code warning on a `pub enum` in a private module, and an unreachable
dialog never gets clicked and therefore never gets debugged. Strategy
persistence moved along with it, since the dialog needs a strategy to display
on `Show` anyway. Piece 6 shrinks accordingly.

Second, **the request is recomputed from the document on every open; only the
strategy persists**. The colloscope page persists its `SolveConfig` because
that config carries per-run tuning with no cheap default. The generation
request is not like that: its defaults are a function of the current document
("rebuild what has no list yet, keep everything prefilled"), so a persisted
request would mean a subject added between two openings silently arrives
switched off, contradicting the default rule. The strategy does persist, and
resets to `with_parallelism_defaults()` on a new document — the same treatment
`ColloscopeInput::ResetSolveConfig` gives the colloscope page.

Four smaller choices the piece plan settled and the code carries. A period
with no eligible subject gets **no group at all** rather than an empty titled
one, which makes "the left pane is empty" and "there is nothing to rebuild"
the same condition and lets the empty state be one honest sentence. Subjects
with **no registered student are still listed**: the dialog does not read
`Assignments`, and such a pair lands in `GenerationPlan::skipped`, which piece
3's dialog reports — filtering here would duplicate that logic and hide the
warning. **"Valider" is insensitive when nothing is selected for rebuild**,
since an empty `rebuild` set produces an empty plan and an empty model.
Prefilled lists are listed **sorted by `(name, id)`**, the order the
group-lists page itself uses, so the two views agree.

Verification: `gtk4` has no test suite at all — not one `#[test]` under
`gtk4/src` — so this piece is checked by compilation plus a manual
click-through, and saying that plainly beats inventing a relm4 test harness
for a piece whose whole content is a dialog. The compile check is not empty:
a warning-free build is what proves every new enum variant is reachable.

**Piece 3 — naming/build dialog.** New dialog replacing `loading_dialog.rs`
in the chain: one row per spec with an editable name (`adw::EntryRow`) and a
subtitle listing the covered subjects and periods; "Valider" disabled until
the model build finishes; a header toggle button that shows an `adw::Spinner`
while building and `emblem-ok-symbolic` when done (the
`gtk4/src/editor/colloscope.rs:350-439` pattern), and toggles the content to a
`DebugView` streaming the build log (the
`spawn_oneshot_command` + `FnMut(&str)` → `Echo` pattern of
`loading_dialog.rs:190-207`). Default names are built here from the covered
subjects/periods (French strings belong in gtk4); the naming scheme is a
piece-plan detail. Build failure shows the error like `loading_dialog.rs`
does.

**Done** — `49da7238` (*gtk4: naming/build dialog for automatic group-list
generation (piece 3)*): `naming_dialog.rs` and its `spec_row.rs` factory, plus
the page seam moving one step forward (the request is forwarded here, and this
dialog's output is dropped with a comment naming piece 4). Two file references
in the sketch above were off: the toggle-to-`DebugView` pattern actually lives
in `run_solver.rs:366-449`, and the loading dialog is
`gtk4/src/editor/colloscope/loading_dialog.rs`.

Two deviations, both forced by the code rather than by taste. First, **there is
no error state**, contrary to the last sentence above: `build_model_with_log`
is infallible (it panics on internal bugs, which the piece-1 fuzz net guards),
and the only fallible call, `build_generation_plan`, fails solely on caller
bugs — both dialogs are modal, so the document cannot change between the two —
so it gets an `.expect`. Second, **`adw::EntryRow` has no subtitle property**,
so the coverage lives in the row *title* ("Liste pour Maths et Physique
(périodes 1 et 2)"), where it stays visible while the name is edited.

Four choices the piece plan settled. The **default naming scheme** is the
coverage string itself: distinct covered subjects in document order, then the
distinct covered periods as 1-based numbers, joined the French way — "Maths
(période 1)", "Maths et Physique (périodes 1 et 2)". Names are unique without
any dedup pass, because distinct specs cover disjoint pair sets. **"Annuler"
works at any moment, including mid-build**: unlike `loading_dialog`, this
dialog is interactive from the moment it opens (the user edits names while the
build streams), so blocking cancel would be arbitrary; a `build_seq` counter,
incremented on every `Show` and captured by the command closure, discards a
result that arrives after a cancel or a reopen. **The conductor payload is
emitted by this dialog**, mirroring `loading_dialog::DialogOutput::ModelReady`,
with the empty epoch map §2.6 defines as a single priming solve — which leaves
piece 4 as pure dialog wiring. And **the all-skipped case still opens the
dialog**: when every selected pair lands in `skipped`, `plan.specs` is empty,
so nothing is built and "Valider" stays insensitive forever, but the dialog is
the only place the user would ever learn why nothing was generated.

**Piece 4 — launching the resolution.** Instantiate the generic
`run_solver::Dialog<constraints_groups::Var, ExtraVarName, ConstraintDesc>`
(`gtk4/src/editor/run_solver.rs:28`) with its own title, and feed it
`DialogInput::Run(strategy, model, payload)` from the naming/build dialog's
output. The `NewConfig` result is received but not yet applied.

**Done** — `bef11ce8` (*gtk4: launch the solver for automatic group-list
generation (piece 4)*): the page instantiates `SolverDialog` for the groups
model, launches it with its own title ("Génération des listes de groupes"), and
sends `DialogInput::Run(strategy, model, payload)` from the naming dialog's
output. This is the first time the conductor actually runs against this model,
so §2.6's empty-epoch-map contract is executed rather than merely read off the
incremental strategy's documentation. The `NewConfig` result comes back as a new
`GenerationSolveResult` input and is dropped, together with the plan and the
names, with a comment naming pieces 5-6 — storing them here would leave fields
written but never read, and a warning-free build is this page's whole compile
check.

One deviation, forced by a string. The shared solver dialog hardcoded
colloscope-specific copy in its cancel-while-running confirmation ("Toutes les
modifications sur le colloscope seront perdues.", `run_solver.rs`), which would
have shown verbatim during a group-list generation. Its `Init` therefore grew
from a bare title `String` to a `DialogSettings { title, cancel_warning }`
struct, in its own prerequisite commit `dbdf587b` (*gtk4: the solver dialog's
cancel warning is caller-supplied*); the colloscope call site passes the two
previously hardcoded strings, so its behaviour is unchanged. Every other French
string in that dialog is generic and stayed hardcoded.

**Piece 5 — the update op.** `GroupListsUpdateOp::AddGeneratedGroupLists`
(§4), with its precise error enum, following the existing per-variant test
style in `ops/src/group_lists.rs`.

**Done** — `c73d3d7f` (*ops: composite op to add generated group lists (piece
5)*): the variant carries `Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>`
and lands as one undo slot, prechecking the whole payload against the pre-state
and then running `GroupListOp::Add` plus one `AssignToSubject` per coordinate
on the caller's session — the `DuplicatePreviousPeriod` shape. Its
`AddGeneratedGroupListsError` is the union of the two per-variant surfaces the
composite is made of: the add's student sweep, and four of the assignment's
five coordinate checks. The fifth has no input here, since every association
names the list the session has just issued, so there is no
`InvalidGroupListId`. Two things the growth rule kept out: duplicate
coordinates across entries are last-wins rather than an error, and the filling
shape is not checked. The property suite's `gen_group_lists` grew an arm for
the variant (verified reached by a temporary panic in it, not merely weighted
in).

**Piece 6 — final plumbing.** Chain config dialog → naming/build dialog →
solver dialog; on `NewConfig`, `filter_transmute` to base vars,
`build_group_lists`, emit the composite op through the page's normal output.
The button and the persisted strategy already landed in piece 2, so what is
left here is the chaining and the apply.
End state of phase A: the full pipeline works and produces absurd lists. To
check along the way: whether prefilled lists with out-of-range group sizes
trip any checker warning on apply (harmless if so, but the skeleton demos
will show it).

**Done** — `03e902af` (*gtk4: apply the generated group lists (piece 6)*): a
single-file change to `gtk4/src/editor/group_lists.rs`, because the chaining
sentence above was already discharged by pieces 2-4 — what was left was only
the apply. The page grew one field, `pending_generation: Option<(GenerationPlan,
Vec<String>)>`, written in the `GenerationNamingAccepted` arm that launches the
solver and `take()`n in `GenerationSolveResult`. The `take()` carries an
`.expect("a solve result implies a pending generation")` rather than a graceful
fallback: `GenerationSolveResult` can only follow a `DialogInput::Run`, the only
sender of `Run` is the arm that sets the field, and the solver dialog emits at
most one `NewConfig` per run (`run_solver.rs`, `best_solution.take()`). A
cancelled solve leaves a stale `Some`, which is harmless — nothing reads the
field except that arm, which always follows a fresh write — so the cancel arms
stayed empty and `run_solver::DialogOutput` did not grow a `Cancelled` variant
just for hygiene.

The projection is an inline `filter_transmute` keeping `InternalVar::Base`,
copying the colloscope page's `SolveResult` handler rather than calling
`Model::base_data_from_complete_data`: that method needs a `&Model`, and this
page hands the model to the solver dialog and keeps nothing. The match arm is
written generically (`_ => None`) even though `ExtraVarName` is uninhabited
today, so pieces 7-9 change nothing here. `build_group_lists` is then called
with no error handling, since it returns a plain `Vec` and its panics are
solver-or-caller bugs unreachable by construction. The resulting entries go out
through the page's existing `Output = GroupListsUpdateOp`, so the editor side
needed no change at all: it wraps, `dry_apply`s, and commits one undo slot.

**Phase-A check, answered — no warning fires.** The cascade's warning vocabulary
is closed: `CascadeWarning::new` is `pub(crate)` with exactly one caller
(`ops/src/cascade.rs`), fed by `apply_cascade`'s fixes, so every user-visible
warning is a `Fix` produced by `broken_invariants` answering a broken invariant.
That invariant set (`state-colloscopes/src/invariants.rs`) is `DanglingFk |
Convergence`, and none of its predicates compares group cardinality to a range —
`students_per_group` is never read in `invariants.rs` or `resolution.rs` at all.
`GroupList::new` itself only checks that the prefilled group count matches
`group_names.len()` and that no student sits in two groups; a 30-student group
in a 2..=3 subject is a perfectly legal value. Group-size agreement is enforced
only downstream, as an ILP constraint at colloscope-solve time
(`constraints-colloscopes/src/groups/students_per_group*.rs`). So the skeleton's
absurd lists apply silently, which is what phase A wanted: the demo shows the
pipeline, not a wall of warnings.

### Phase B — the real model

**Piece 7 — extra variables.** `StudentInGroup`, `GroupHasStudents`,
`PairInGroup`, `SharedPair`, with their full-equivalence reifications and the
pinned-pair fixing (§2.2). Note: the build-time DFS only expands extras that
are referenced from constraints or objectives, so this piece leaves the built
model unchanged; tests exercise the declarations directly (e.g. through
throwaway constraints in test code).

**Done** — `318e7455` (*constraints-groups: reified extra variables (piece
7)*): a new private `constraints-groups/src/extras.rs` with one builder per
family, merged into a single bundle and applied through the `apply!` macro
copied from the colloscope builder. `VarEnv` grew a `pinned_pairs` field and
three `pub(crate)` accessors (`lists`, `students`, `pinned_pairs`); the public
constructor is unchanged, so `convert.rs` and the property test were not
touched. The builder test still asserts 7 variables — now with a companion
"and zero constraints" assertion — which is the piece's core claim: lazy
expansion means declared-but-unreferenced extras cost nothing.

Three deviations from the wording above, all settled while planning. The
pinned-pair pin is a degenerate reification, not `check_fix` (§2.2 has been
corrected in place: the fixer chain cannot reach extras). `PairInGroup` uses
the single constraint `x_a + x_b >= 2` instead of the sibling crate's
two-constraint AND (`constraints-colloscopes/src/extras.rs:374-390`): both
operands are binary, so it is the same full equivalence, but it takes the
single-constraint fast path and creates no helper column — worth it for by far
the largest family (pairs × groups per list). And the tests reference the
extras through **objectives**, not through throwaway constraints as suggested
above: a user constraint needs a `ConstraintDesc` value, and that enum stays
uninhabited until piece 8. Two mechanisms need no descriptor — `build_full`,
which force-expands everything (used by `declarations_expand_cleanly` to sweep
for undeclared names, cycles and duplicates in one call), and `add_objective`,
which expands what it references.

The four solver tests share an **adversarial-weight** shape: weight-100 terms
on `StudentInGroup` indicators pin the placement (maximizing a
full-equivalence indicator forces the base equality), and weight-±1 terms push
the extra under test toward the *wrong* value. Landing on the semantic value
anyway is precisely a test of the equivalence direction that objective
pressure does not supply. One trap to remember when adding cases: an extra
that only an *assertion* mentions is never expanded, so it is not a variable
of the problem at all — every asserted extra must also appear in the
objective. Both the AND encoding and the pinning branch were mutation-checked
(weakened to `>= 1`, and disabled with `if false &&`); each turned exactly the
intended test red.

**Piece 8 — shape constraints, one class at a time.** Max size, conditional
min size, ascending fill order (§2.3), each as its own commit with its unit
tests. The fuzz net from piece 1 now has real builders to guard.

**Done** — three commits: `2a87cc69` (*constraints-groups: max group size
constraint (piece 8)*), `a3acddaf` (*constraints-groups: conditional min group
size constraint (piece 8)*), `873e5c9b` (*constraints-groups: ascending fill
order constraint (piece 8)*). The layout mirrors the sibling crate: a private
`constraints-groups/src/constraints.rs` aggregator over a `constraints/`
directory holding `students_per_group.rs` (min and max share one per-group
helper and one `count` sum, as they do in
`constraints-colloscopes/src/groups/students_per_group.rs`) and
`groups_filled_by_ascending_order.rs`. `ConstraintDesc` went from uninhabited
to three flat struct variants keyed by `list: GroupListIdx` and a plain `u32`
group, each carrying its numeric bound; no severity tiers and no `From` impls,
per §2.3. `VarEnv` grew two `pub(crate)` accessors, `min_size` and `max_size`,
alongside the existing `slot_count`. The ascending builder needs no `< 2`
guard, unlike the sibling's: `slot_count` is always at least 1, so
`0..count - 1` cannot underflow and is simply empty for a single-slot list.

The undersized-spec signal promised in §2.1 is now real rather than predicted:
with 2 students and a range of 3..=4 the clamp gives one necessarily
undersized slot, the min-size row has no satisfying assignment, and
`Model::solve` returns `None`. `undersized_spec_is_infeasible` asserts exactly
that through the public `build_model`.

The builder's `trivial_model_has_only_base_vars` was rewritten rather than
deleted — its "7 variables, 0 constraints" claim expires with the first
constraint. Its successor, `shape_constraints_are_emitted`, counts user
constraints per family through a deliberately **exhaustive** `match` (so a
future family cannot land without this test growing to count it) and pins the
half of the piece-7 lazy-expansion claim that survives: `PairInGroup` and
`SharedPair` are referenced by nothing until the piece-9 objective and must
stay out of the built model. `tests/solve_smoke.rs` lost its "piece-1
verification" framing and its test became `model_solves_and_converts`; it now
also asserts a *lower* bound on the group count, which the max cap forces.

Two small things about the test harness. A negative weight belongs in the
`LinExpr` (`maximize(1.0, -1.0 * expr)`), not in `maximize`'s `coef`, which
scales the finished `Objective` and therefore flips its sense too
(`ilp/src/objectives.rs:128`). And CBC returns integral variables as floats
carrying tiny numerical error (a 1 came back as `0.9999999999999999`), so
value assertions go through `collomatique_ilp::f64_equals` rather than
`assert_eq!`.

Every commit was mutation-checked and two checks were informative. Dropping
the min row left `min_size_forces_a_companion` green, because with the
originally planned 5 students and a range of 2..=3 over 2 slots the *max*
constraint already forces a companion by pigeonhole — the spec was widened to
2..=4 so that only the minimum can produce the second student, and the test
then went red as intended. Swapping the ascending inequality to
`next.geq(&current)` turned the intended test red but killed two size tests as
collateral: a descending order forbids piling students into group 0, which is
what those tests do. Removing the constraint outright isolates it cleanly —
exactly `ascending_fill_forbids_gaps` and the builder's count assertion turn
red — so both mutations were run.

**Piece 9 — the objective.** Both terms, equal weights (§2.4). Unit tests: a
two-list instance where the optimum provably reuses groupings; a pinning test
showing kept pairs make reuse free. From here the tool produces sensible
lists.

**Done** — commit `a6778b55` (*constraints-groups: the stability objective
(piece 9)*). A private `constraints-groups/src/objective.rs` builds one
`LinExpr` and hands it to a single `with_minimize(1.0, expr)`, mirroring
`constraints-colloscopes/src/misc/interrogation_cost.rs`, including its
`has_terms` guard (only an empty plan has no terms, and it then builds the
same empty model as before). Both weights are `const … : f64 = 1.0`; they live
*inside* the `LinExpr`, never in the `coef`, for the sense-flip reason
recorded under piece 8.

The co-occurrence enumeration moved out of `build_shared_pair` into a
`pub(crate) fn co_occurrences(env)` that both the declaration and the
objective call. This is not tidying: the objective must sum over exactly the
declared `SharedPair` set, because referencing an undeclared extra makes
`Modeler::build` fail and `build_model` panic. One shared function makes that
drift unrepresentable, so no test is needed for it.

Pinned pairs stay in the sum. Their variables are the constant 1, so they only
shift the objective and cannot change the argmin; keeping the sum uniform over
every declared variable avoids a second filtering pass and makes "regrouping a
kept pair is free" literally true of the objective function.

The two prescribed tests were written as specified, but a **third** was needed:
neither of them can catch a deleted groups term, because in both the group
count is forced by the size constraints and the term is a constant.
Isolating that term is subtler than it looks — at equal weights, moving a
student from a singleton into a group of size `s` trades one group for `s` new
pairs, so merging two singletons is exactly cost-neutral. A data point for the
phase-C tuning of piece 11. `groups_term_pulls_toward_fewer_groups` gets around
it with a *pinned* pair, which holds the pairs term constant so only the groups
term distinguishes merging from splitting; ascending fill then makes "together"
mean "both in group 0", so no placement term is needed at all.

Each test carries a small (0.5) adversarial term against the real objective —
the harness applies the objective bundle first, so the fold keeps `Minimize`
and *subtracts* each later `maximize` term. Without those adversaries the
mutations would merely produce ties, and a solver that happens to return the
expected answer is not a green test. With them, every mutation loses strictly:
neutralizing the pairs term reddens both prescribed tests, neutralizing the
groups term reddens only the third (A and B stay green — the point of adding
it), and rerunning the piece-7 `if false &&` on the pinning branch reddens the
pinning tests of both pieces.

One consequence outside the module: the pair extras are now referenced, so the
lazy-expansion half of `shape_constraints_are_emitted` inverted. Instead of
asserting their absence it counts them exactly — 21 `PairInGroup` and 9
`SharedPair` on its two-disjoint-list fixture. `tests/solve_smoke.rs` was left
alone on purpose: its `2 ≤ groups ≤ 3` bounds hold for any feasible solution,
and welding it to today's weights would make a piece-11 retune look like a
regression. The fuzz-build tests needed no change and now cover the objective
path on every random plan for free.

**Piece 10 — epochs.** The inclusion-based ordering of §2.6, replacing the
absent map of piece 1. Unit tests on hand-built spec families: disjoint sets,
nested chains, overlapping incomparable sets, equal sets.

**Done** — commits `bd9eecc1` (*constraints-groups: inclusion-based
incremental epochs (piece 10)*) and `7ad30636` (*gtk4: ship the inclusion-based
epoch payload (piece 10)*). A private `constraints-groups/src/incremental.rs`
exports `build_incremental_epochs`, laid out like the sibling
`constraints-colloscopes/src/incremental.rs` — but with a different signature,
and that is the one real design decision of the piece. The colloscope version
walks the *built model*, because there an epoch is a function of the variable
alone (its week). Here the epoch depends on inclusion between student sets,
which live in the plan, so the function takes `&GenerationPlan`. The model's
base variables are enumerated from exactly those sets (`VarEnv::new(&plan)`
plus the derive's `#[range]` attributes), so the two agree by construction;
`map_names_exactly_the_base_variables` compares the map's key set against
`<Var as DescribeVar>::enumerate` so the agreement cannot drift silently.

No recursion is needed. A strict subset always has strictly fewer students, so
processing the specs by ascending student count guarantees every strict subset
of a spec is already computed when its turn comes, and the whole thing is one
pass with k(k−1)/2 `is_subset` calls. That sort is not a detail:
`GenerationPlan::specs` is ordered by the spec's `Ord`, which is lexicographic
on the student `BTreeSet` and has nothing to do with size, so
`nested_chain_counts_height` uses a deliberately superset-first fixture to pin
it. Strictness is a length comparison guarding the `is_subset` call, since
`BTreeSet::is_subset` accepts equality; the disjoint and overlapping fixtures
were given sets of *unequal* size on purpose, so that a broken subset test
cannot hide behind the length guard.

Beyond the four prescribed families, a **fifth** test was needed:
`height_is_a_max_over_all_strict_subsets`. Neither the chain nor the §2.6
worked example can tell a `max` from a plain assignment, because in both the
deepest subset happens to be visited last. Its fixture — `{1} ⊂ {1,2}`,
an inclusion-minimal but *larger* `{3,4,5}`, and `{1,2,3,4,5}` on top — makes
the ascending-size pass visit the shallow subset last, so last-wins gives 1
where the max gives 2. It is the only test that reddens under that mutation;
all six tabled mutations were run and each reddened its predicted test.

The fuzz-build net now checks the epochs on every random plan, as a *fixpoint*
of §2.6's recursive definition (0 with no strict subset, else 1 + the max over
the strict subsets) rather than by re-running the algorithm. That is a real
check and not a tautology because the recurrence is well-founded on strict
inclusion, so it has a unique solution; the probe also counts one entry per
base variable and asserts every variable of a spec shares its epoch.

On the gtk4 side the naming dialog's `Accept` arm swapped `HashMap::new()` for
the real call. It runs on the UI thread rather than joining the off-thread
model build: it is quadratic in the spec count (itself bounded by the number of
selected pairs) and reads nothing but the plan. This closes phase B — the model
is complete, and the conductor now stages the lists instead of priming once.

### Phase C — polish

Phase C was left as a catch-all in the initial roadmap ("advanced parameters,
Python wiring, epoch tuning, and whatever the first real uses ask for"). It is
now scoped: three pieces are wanted, and one candidate is explicitly ruled out.

**Piece 11 — configurable objective weights, and a new default.** Two related
things: making the weights configurable, and changing what they default to.

Today `constraints-groups/src/objective.rs:20,22` holds `const W_GROUPS: f64 =
1.0` and `const W_PAIRS: f64 = 1.0`. They must become parameters, settable both
at the crate level (a caller passes them in when building the model) and from
the UI, through an advanced dialog modelled on the colloscope one
(`gtk4/src/editor/colloscope/config_dialog/advanced_dialog.rs`): a modal with
one preferences row per weight, opened from the generate dialog. The two
weights are the only thing to expose; there is nothing else. That is a small
surface but an important one, since the weights are what decides whether the
tool produces the lists the user actually wants.

Where the weights live is the piece's first question. They do not belong in
`GenerationRequest`: that struct says which (period, subject) pairs to rebuild
and which lists to keep, it is consumed by `build_generation_plan`, and the
weights would have to be threaded through the plan for no reason. A small
separate struct handed to `build_model` next to the plan is the natural shape.
This is not a return of the `SolveConfig` layer that §2.7 rules out — there are
still no filtering phases and still one build in one pass.

**The default must not stay equal.** Fewer groups matters more than fewer
shared pairs, in every real use, and the piece-9 record above measured how badly
equal weights fail at this: at `w_groups = w_pairs = 1`, moving a student out of
a singleton into a group of size `s` trades one group against `s` new pairs,
which is *exactly* cost-neutral. The group count is not merely outvoted by the
pair term, it is drowned by it. The new default puts two to three orders of
magnitude more weight on the group count (`w_groups = 1000`, `w_pairs = 1` as
the starting point), which makes the pair term what it should be: a tie-breaker
among the solutions that already use as few groups as possible.

A note on how far to push that. Strict lexicographic priority — no pair saving
may *ever* justify one more group — would need `w_groups` to exceed the largest
pair total the instance can reach, which is bounded by `Σ C(n_i, 2)` over the
specs and runs into the thousands as soon as a couple of full-class lists are
involved. Deriving the weight from the instance that way is possible, but it
costs twice: the objective value stops being readable, and the coefficient range
widens, which is exactly what makes an LP relaxation ill-conditioned. A fixed
`1000` is the recommendation — it dominates in every plausible instance while
keeping the model well scaled — and the dialog is there for the user who
disagrees on a particular document.

**Done** — commits `6fbd2e18` (*constraints-groups: configurable objective
weights with a group-dominant default (piece 11)*), `cd5d0eee` (*gtk4: advanced
dialog for the objective weights (piece 11)*) and this record. Everything above
was implemented as specified, including the fixed `1000`: nothing in the
implementation argued for deriving the weight from the instance.

`ObjectiveWeights { w_groups, w_pairs }` is defined in the private `objective`
module and re-exported from `lib.rs`, the pattern already used for
`builder::build_model`. It derives `Debug, Clone, Copy, PartialEq` and has a
**hand-written** `Default` reading two private consts `W_GROUPS_DEFAULT` and
`W_PAIRS_DEFAULT` — the `constraints-colloscopes/src/config.rs` shape, where
`L1_ANCHOR_WEIGHT = 1000.0` feeds a manual `Default` on `SolveConfig`. No
serde: the crate uses none, and the value never crosses IPC, since the weights
are baked into the model before the solver ever sees it. The signatures became
`build_model(plan, weights)` and `build_model_with_log(plan, weights, log)`,
weights before the log so the callback stays last. `VarEnv` was deliberately
left alone: it describes the *variables*, its `new` is public API used by
`tests/property_build.rs`, and the weights are not part of the variable space.

The test decision worth recording: the three piece-9 objective tests **keep
their arithmetic** by passing an explicit `EQUAL` (1/1) rather than being
rewritten against the new default. Every comment in them — the margins, the
"cost-neutral merge" rationale, and `place()`'s weight-100 scale, which 1000
would no longer dwarf — was written against 1/1, and passing 1/1 explicitly
keeps all of it literally true. Two new tests carry the piece instead.
`default_weights_prefer_fewer_groups_over_fewer_pairs` runs
`groups_term_pulls_toward_fewer_groups`'s instance *without* the pinned pair,
so the pair term is live and must lose: two students, sizes 1..=2, together
costs 1000 + 1 = 1001 against a split at 2000 − 0.5, where at `w_groups = 1` the
split would win 1.5 to 2. `explicit_weights_override_the_default` passes
pair-dominant 1/1000 with the adversary rewarding the shared pair, so the
optimum splits; its assertions are on `SharedPair` and `GroupHasStudents`
because which student lands in which group is a tie between the two splits. All
four tabled mutations were run one at a time and each reddened exactly its
predicted test: `W_GROUPS_DEFAULT` back to `1.0`, swapping the two fields in
`Default`, and hardcoding either weight inside `build` (the first three redden
the default test, the last the override test). Every other call site passes
`ObjectiveWeights::default()`, `tests/solve_smoke.rs` included — the piece-9
record's reason for leaving it weight-agnostic still holds, and it now
exercises the new default for free.

Two smaller decisions. The crate commit **carries the one-line gtk4 call-site
fix** (`naming_dialog.rs` passing `ObjectiveWeights::default()`), so the
workspace compiles at every commit; that is the piece-0 precedent (`d8d42ef0`),
and the gtk4 commit then replaces the default with the threaded value. And
parameterization and the default change are one commit rather than two: split,
they would create an intermediate "configurable but still equal" state this
section explicitly rejects, and the default-pinning test would have to be
written twice.

On the UI side the "Paramètres avancés" button sits **right of** the
solver-configuration frame, not inside it — the strategy frame is wrapped in a
horizontal box so the two are siblings, which is the literal shape of
`colloscope/config_dialog.rs`, down to the `frame` + `warning` css classes and
the `configure-symbolic` icon. That screen already draws the visual line
between solver configuration and model parameters, and the weights are model
parameters; consistency between the two screens decided it. The dialog itself
clones the colloscope one (`hidden` + `should_redraw` so the model is pushed
into the widgets only on `Show`, "Annuler" discarding the edits since the next
`Show` re-seeds from the parent) with one departure: each `adw::SpinRow` carries
a subtitle, because the two weights are opaque without a sentence of
explanation. The weights persist on the group-lists page next to `strategy`,
since the use case is iterating — run, look at the lists, re-run with a tweak —
and are forgotten on a new document, so the page input `ResetStrategy` was
renamed `ResetGenerationConfig` now that it resets both (the colloscope analog
`ResetSolveConfig` likewise resets two things under one name).

**Piece 12 — splitting epochs into independent components.** Piece 10 gives the
inclusion ordering of §2.6, and on realistic documents that ordering yields only
two epochs: everything inclusion-minimal, then the whole-class list on top. That
is better than the single solve of phase A, but the first epoch is still one
large model holding many lists that have nothing to do with each other.

The refinement subdivides each epoch into its independent parts. Within one
epoch level, build a graph whose nodes are the specs of that level and whose
edges join two specs whose student sets intersect; every connected component is
an independent block, and every block becomes an epoch of its own. The epochs
are then renumbered by walking the levels in ascending order and giving each
component of a level the next number, so the inclusion ordering is preserved
exactly — every spec of level `k` still solves before every spec of level `k+1`
— and each level is merely cut into pieces. A level with a single component is
left unchanged. Connectivity is computed *inside* a level, never across levels:
over the whole spec set almost everything is one component as soon as a
whole-class list exists, which would defeat the purpose.

Two claims make this worthwhile. The first is separability: two specs in
different components of the same level share no student, hence no `SharedPair`
and no `GroupHasStudents` variable and no constraint, so the objective is
separable over the components and the *true optima* of the components together
are the true optimum of the level. The second is that this should genuinely
help the solver, because branch and bound does not decompose a model into
independent blocks by itself: `k` independent blocks in one model give a single
search whose tree is roughly the product of the `k` small ones, where `k` epochs
give `k` small searches in sequence.

Separability is not, however, a promise that the split leaves the *computed*
result alone. The incremental strategy deliberately does not compute the true
optima: each epoch optimizes only its own margin of the objective, the previous
epochs are held by a soft L1 anchor rather than fixed, and each epoch stops
within its own tolerance (`strategies/src/strategies/incremental.rs`). Splitting
a level therefore changes the sequence of sub-problems, and the order of the
resulting stages changes what comes out. The settled rule: **inside a level the
components run smaller-first, by their number of distinct students.** Two
components of equal size share no student, so neither can see the other and
their relative order is semantically indifferent; the implementation still
tie-breaks on the smallest spec index, purely so runs and tests are
reproducible.

The visible cost is the number of solver invocations, each carrying the
strategy's per-epoch overhead. The recommendation is to accept that and measure,
rather than capping the epoch count pre-emptively: a real document holds on the
order of ten lists, and a per-epoch overhead measured in milliseconds is nothing
against the ILP solves themselves. Should measurement say otherwise, merging the
smallest components back together is the obvious lever.

One consequence to plan for. The fuzz probe added in piece 10
(`constraints-groups/tests/property_build.rs`) asserts §2.6's recurrence as an
exact equality, and the refined map no longer satisfies it. That probe has to be
rewritten around the properties that survive: a strictly included spec gets a
strictly smaller epoch, two specs of the same inclusion height that share a
student land in the same epoch, and two specs sharing an epoch are connected
through shared students — plus the one-entry-per-base-variable count, which is
unaffected.

**Done** — commit `233d96be` (*constraints-groups: split epoch levels into
connected components (piece 12)*). One commit, not two: the fuzz probe would go
red without its rewrite, so the algorithm and the probe had to move together.
Nothing outside `constraints-groups` changed — `build_incremental_epochs` keeps
its signature, so the gtk4 call site and the strategies crate were untouched.

`build_incremental_epochs` is now two passes. The first is piece 10 verbatim,
except that its result is named `heights` and is no longer the epoch, only the
level. The second groups the specs by level, grows the components of a level in
spec-index order, sorts them by `(distinct students, smallest member index)`,
and hands out consecutive numbers — so epoch numbers come out contiguous from
0. Growing a component needs a *multi-merge*: a spec can touch several existing
components at once and must fuse them all, which is why the loop partitions the
component list on `!union.is_disjoint(students)` instead of stopping at the
first hit. Keeping each component's union set makes that test cheap and correct
at the same time: intersecting the union is the same as intersecting some
member, since the union is exactly their union.

Two tests were added and one rewritten. `disjoint_sets_are_epoch_zero` no longer
describes the behaviour at all — its two disjoint specs are now two epochs — so
it became `disjoint_sets_of_a_level_split_smaller_first`.
`smaller_blocks_solve_first_within_a_level` gives three disjoint blocks of
distinct sizes in scrambled plan order, and `connectivity_is_transitive_within_a_level`
puts the bridging spec `{2,3}` *last* in index order so that it must fuse two
already-separate components. `roadmap_example_heights` doubles as the per-level
connectivity pin: the whole-class list intersects both LV2 lists but sits at
another level, and must not merge them.

All four tabled mutations were run and each reddened its predicted test. The
fourth is the one worth recording: breaking the multi-merge (keep only the first
touching component) left the **fuzz probe green** and reddened only
`connectivity_is_transitive_within_a_level`. The random walks never produce a
three-way merge inside one level, so that hand-written fixture is the only guard
on it.

**Piece 12bis — one epoch per spec, least-entangled first.** Piece 12 was
checked against the user's typical use cases and does not help. Its premise —
that a level holds lists with nothing to do with each other — does not survive
contact with real documents: there is simply too much overlap between the group
lists of a level, so the connected components fuse into one big block, the level
solves as a single large model exactly as before, and the split buys nothing.
The piece-12 record above stays as written; it is history, and its reasoning is
sound on the documents it imagined.

So drop the components entirely. **Inside a level, every spec becomes an epoch
of its own** — overlapping or not. The levels keep their order, so all specs of
level `k` are numbered before any spec of level `k+1` and §2.6's recurrence
still decides which lists wait for which; the refinement again only decides how
far a level is broken apart, and now breaks it apart as far as it goes. Inside a
level the ordering rule is: ascending by the number of **distinct students the
spec shares with the other specs of that level**, then by student count (small
lists first), then by spec index for determinism only. The rationale is the same
one that motivates the whole inclusion ordering: a list that shares few students
with the rest of its level is nearly independent, so its solo optimum is close
to what a joint solve would have given it, and solving it first costs almost
nothing; a heavily-shared list is better off later, when the lists it is
entangled with are already anchored and it can align to them through the pair
objective. When all the shared counts of a level are zero, the rule reduces
exactly to piece 12's smaller-blocks-first.

Sharing is counted **inside the level only**, and this is not the same
consideration that made piece 12 compute connectivity per level. Counting
against the whole plan degenerates arithmetically: as soon as a whole-class list
exists, every student of every spec is shared with it, so every spec's count
equals its own size and the ordering collapses into plain size ordering. The
within-level count is also the only one that carries information here — a spec's
coupling to the levels below it is fixed no matter where it sits inside its own
level.

This gives up piece 12's separability claim, deliberately. Two overlapping specs
of a level used to solve jointly and now solve apart, so the staged result can
differ from the joint one even at the true optima — not merely because of the
strategy's anchors and tolerances, as was already the case, but because the
sub-problems no longer add up to the level's problem. That is the accepted
trade-off: tractability first, quality entrusted to the least-shared-first
ordering plus the incremental strategy's L1 anchor (`l1_weight`, default 1000),
which keeps an epoch close to what the previous ones decided. It is worth being
plain about this rather than dressing the split as free.

**Done** — commit `b536e657` (*constraints-groups: one epoch per spec,
least-entangled first (piece 12bis)*). One commit again, for the same reason as
piece 12: the fuzz probe would go red without its rewrite. Nothing outside
`constraints-groups` changed; `build_incremental_epochs` keeps its signature and
`IncrementalPayload` assumes nothing about the epoch count.

Pass 1 is untouched. Pass 2 shrank to a sort: for each level, build the key
`(shared, size, index)` per spec, sort, hand out consecutive numbers. The
component machinery — the union sets, the multi-merge partition, `BTreeSet` and
the `StudentId` import — is gone.

`connectivity_is_transitive_within_a_level` was deleted (there are no components
to merge), `overlapping_incomparable_sets_share_their_epoch` became
`overlapping_incomparable_sets_get_their_own_epochs` with epochs 0 and 1, and
`equal_sets_never_relate` now asserts two epochs — the fixture still pins what it
was written for, that equal sets never *strictly* include each other and so
neither waits for the other. Two tests were added:
`least_shared_specs_solve_first_within_a_level`, where the biggest list solves
first because it is untangled and the smallest solves last because it is the
most entangled, and `sharing_is_counted_within_the_level_only`, where a
whole-class list at another level must not perturb the level-0 ordering. The
probe's component properties were replaced by the ones that define the new
numbering: heights ascend with the epochs, the map is a bijection onto a
contiguous range from 0, and inside a height the epochs ascend by
`(shared-within-level, size)`.

All five tabled mutations reddened their predicted tests. One is worth recording,
and it is the same lesson as piece 12's fourth mutation: counting sharing
globally instead of per level left the **fuzz probe green** and reddened only
`sharing_is_counted_within_the_level_only`. The random walks do not produce a
level whose ordering cross-level sharing would perturb, so that hand-written
fixture is the only guard on the within-level rule.

**Piece 13 — group-list display polish.** The list of group lists
(`gtk4/src/editor/group_lists/group_lists_display.rs`) does not line up. The
name label at lines 82-90 uses `set_size_request: (150, -1)`, which sets a
*minimum* width, so a long list name pushes its row wider than the others and
every column after it stops aligning. The fix is to give that column a fixed
width instead — ellipsize the label at the end, with a maximum width in
characters — and to put the full name in a tooltip so nothing is lost. The
tooltip should be set unconditionally rather than only when the label actually
overflows: GTK does not readily expose whether a given label was truncated, and
a tooltip that appears only sometimes is worse than one that always does. It
must be `#[watch]`ed like the label itself, so that renaming a list updates
both.

**Explicitly out of scope: Python.** The initial roadmap listed "Python wiring"
here because the original todo mentioned it. It is dropped. If group-list
generation ever becomes scriptable, that will come with the rewrite of the
Python API, not as a polish item bolted onto this feature.

## 6. Points settled

- No −1 in the `StudentGroup` domain; no `students_have_groups` analog.
- Group slots per spec: `max(1, floor(n / min_size))`. The clamp keeps the
  variable domain non-empty when a spec has fewer students than the minimum
  group size; that spec is then infeasible once the min-size constraint
  lands, which is the intended signal (§2.1).
- Kept lists restricted to prefilled ones; their only effect is pair pinning.
- All reifications are full equivalences — never rely on objective pressure
  (strategies may strip the objective). Kept lists enter the `SharedPair`
  equivalence as constant terms; a pair already grouped in one is fixed to 1
  and its degenerate reification constraints are omitted, not emitted.
- Piece 0 (before everything else): key the incremental epoch payload by the
  base variable type, so non-base entries become unrepresentable.
- `ConstraintDesc`: plain flat enum, no severity tiers (the tier machinery
  serves the gtk4 colloscope warning display, absent here).
- Objective weights: hardcoded constants, equal to start with; tune later.
- Apply path: the composite `GroupListsUpdateOp::AddGeneratedGroupLists`
  (option A), not `GlobalUpdate`.
- Piece order: walking skeleton first (trivial model + full pipeline), then
  the real model, then polish.
- Overwritten associations may orphan old lists; they are kept.
- Subjects listed in the config dialog: only those with interrogations.
- Config dialog defaults: rebuild on where no association exists, kept lists
  all on. They are recomputed from the document on every open — persisting
  them would contradict the rule itself, since a subject added between two
  openings would arrive switched off. Only the solver strategy persists.
- A list may be both kept and rebuilt over: the new list takes over the
  association (§4) while the old list's pairings still count as
  already-shared. Keeping a list is about its *pairings*, not its
  association, so this needs no guard.
- Generated lists are `Prefilled`, unnamed groups, spec range as list range.
- Epoch algorithm: longest strict-inclusion chain height, computed by
  ascending student count (§2.6). The epoch map lists base variables only;
  an empty map (phase A) is a documented single priming solve.

## 7. Points still open / to verify

- ~~**Piece-1 verification**: solver machinery on a constraint-free,
  objective-free model.~~ **Answered** by
  `constraints-groups/tests/solve_smoke.rs` (commit `6335e003`). The test
  builds one spec — 6 students, range 2..=3, hence 3 slots and 6 integer
  variables with domain `0..=2` — and runs it through `build_model`,
  `Model::solve` with a real `ColloCbcSolver`, and `build_group_lists`. Three
  things could have broken and did not: `Modeler::build` accepts a model with
  no bundle applied at all; an absent objective folds to a constant-0
  minimize that CBC accepts; and a problem with variables but **zero
  constraint rows** is solved rather than rejected. The returned assignment
  converts back into a structurally valid prefilled list — 6 students placed,
  at most 3 groups — so `GroupList::new`'s sealed invariants hold on solver
  output.

  Two adjacent things this does **not** establish, both deferred to their own
  pieces rather than left as open questions here. First, the test drives the
  in-process `Model::solve` path, whose `FeasibleSolution::get_data()` already
  projects down to `ConfigData<Var>`; the gtk4 path instead goes through the
  strategy/subprocess machinery over `InternalVar` and needs a
  `filter_transmute` — done in piece 6 (`03e902af`), which is where the
  skeleton finally got exercised for real. Second, §2.6's claim that an empty
  epoch map means a single priming solve was read off the incremental
  strategy's contract rather than executed — until piece 4 (`bef11ce8`), which
  runs the conductor against this model for real. Piece 10 is where the map
  stops being empty.
- ~~**Phase-A check**: whether absurd (out-of-range-size) prefilled lists trip
  checker warnings on apply.~~ **Answered — they do not.** See the piece-6
  record in §5 for the full argument: the warning vocabulary is exactly the
  `Fix` values the invariant checker can emit, and no invariant looks at group
  sizes. Out-of-range groups apply silently; the constraint exists only inside
  the colloscope solver's model.
