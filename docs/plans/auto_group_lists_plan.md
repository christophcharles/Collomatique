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
`SharedPair = 1`. Implementation-wise the variable is **fixed to 1** (via the
`check_fix` mechanism, like `Var::fix_student_group` in
`constraints-colloscopes/src/vars.rs:125-154`) and its reification
constraints are **omitted entirely**: with the constant term in, every one of
them is degenerate (trivially true), so omitting them is exactly equivalent
to writing them — and sends that many fewer constraints to the solver.
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

The weights are plain hardcoded constants, and they start out **equal**
(`w_groups = w_pairs = 1`); they will be adjusted later from experience. The
two terms do pull in opposite directions (fewer groups means fuller groups,
which creates more pairs inside each list), so some tuning is expected —
but that is a tuning question, not a design question. Exposing the weights in
an advanced dialog is part of the polish phase (§5, piece 11).

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
- Two *overlapping but incomparable* sets get the same height and are solved
  jointly — correct, since pair variables couple them and neither is
  "smaller".
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

### 2.7 No `SolveConfig` equivalent

The colloscope crate's `SolveConfig` layer (filter/pin/anchor,
`constraints-colloscopes/src/config.rs:261-453`) has no counterpart here. All
the configuration happens *upstream*, in the `GenerationRequest` (§3): the
model is always built whole, in one pass. This is why the loading UI can be
simpler than `loading_dialog.rs` (no 1/3–3/3 filtering phases, just one build
with a streamed log).

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

**Piece 4 — launching the resolution.** Instantiate the generic
`run_solver::Dialog<constraints_groups::Var, ExtraVarName, ConstraintDesc>`
(`gtk4/src/editor/run_solver.rs:28`) with its own title, and feed it
`DialogInput::Run(strategy, model, payload)` from the naming/build dialog's
output. The `NewConfig` result is received but not yet applied.

**Piece 5 — the update op.** `GroupListsUpdateOp::AddGeneratedGroupLists`
(§4), with its precise error enum, following the existing per-variant test
style in `ops/src/group_lists.rs`.

**Piece 6 — final plumbing.** Chain config dialog → naming/build dialog →
solver dialog; on `NewConfig`, `filter_transmute` to base vars,
`build_group_lists`, emit the composite op through the page's normal output.
The button and the persisted strategy already landed in piece 2, so what is
left here is the chaining and the apply.
End state of phase A: the full pipeline works and produces absurd lists. To
check along the way: whether prefilled lists with out-of-range group sizes
trip any checker warning on apply (harmless if so, but the skeleton demos
will show it).

### Phase B — the real model

**Piece 7 — extra variables.** `StudentInGroup`, `GroupHasStudents`,
`PairInGroup`, `SharedPair`, with their full-equivalence reifications and the
pinned-pair fixing (§2.2). Note: the build-time DFS only expands extras that
are referenced from constraints or objectives, so this piece leaves the built
model unchanged; tests exercise the declarations directly (e.g. through
throwaway constraints in test code).

**Piece 8 — shape constraints, one class at a time.** Max size, conditional
min size, ascending fill order (§2.3), each as its own commit with its unit
tests. The fuzz net from piece 1 now has real builders to guard.

**Piece 9 — the objective.** Both terms, equal weights (§2.4). Unit tests: a
two-list instance where the optimum provably reuses groupings; a pinning test
showing kept pairs make reuse free. From here the tool produces sensible
lists.

**Piece 10 — epochs.** The inclusion-based ordering of §2.6, replacing the
absent map of piece 1. Unit tests on hand-built spec families: disjoint sets,
nested chains, overlapping incomparable sets, equal sets.

### Phase C — polish

**Piece 11 —** advanced parameters (objective weights), Python wiring (the
todo mentions it), epoch-strategy tuning, and whatever the first real uses
ask for. Out of scope for the initial roadmap; not planned further here.

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
  `filter_transmute` — that is piece 4's and piece 6's integration, and the
  skeleton exists precisely so it gets exercised for real. Second, §2.6's
  claim that an empty epoch map means a single priming solve is read off the
  incremental strategy's contract, not yet executed: nothing has run the
  conductor against this model. Piece 4 is where that first happens, and
  piece 10 is where the map stops being empty.
- **Phase-A check**: whether absurd (out-of-range-size) prefilled lists trip
  checker warnings on apply.
