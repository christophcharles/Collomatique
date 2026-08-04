# Automatic group-list prefilling — high-level plan

Status: roadmap (2026-08-04). This is not an implementation plan. It splits the
work into pieces that can be done one at a time. Each piece gets its own
detailed implementation plan (full prose, old+new code snippets) when we start
it. Branch context: `group_lists_auto_prefill`.

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

The number of group *slots* in a spec's model is `floor(n / min_size)` where
`n = students.len()` — more groups than that cannot all satisfy the minimum
size. The objective then minimizes how many slots are actually used. (At least
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
- `GroupHasStudents { list, group }` — boolean, ⟺ some `StudentInGroup` in
  that group is 1. Used by the min-size constraint, the ascending-order
  constraint, and the objective.
- `SharedPair { a: StudentId, b: StudentId }` (with `a < b`) — 1 if the pair
  shares **any** group in **any** list. Defined only for pairs that co-occur
  in at least one *new* spec. Since the objective only pushes it down, a
  one-sided implication suffices:
  `SharedPair >= StudentInGroup(a, l, g) + StudentInGroup(b, l, g) - 1`
  for every list `l` containing both and every group `g`. Whether the
  ilp-modeler reification machinery expresses this one-sided form or we use a
  full equivalence is a piece-plan detail.

Kept lists (see §2.5) do not add variables. A pair that already shares a group
in a kept list gets its `SharedPair` **fixed to 1** (via the `check_fix`
mechanism, like `Var::fix_student_group` in
`constraints-colloscopes/src/vars.rs:125-154`). Grouping such a pair in a new
list then costs nothing — which is exactly the stability heuristic: the
cheapest solution reuses last period's groupings where the student sets allow
it.

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

The crate defines its own `ConstraintDesc` with the same severity-tier shape
as `constraints-colloscopes/src/types.rs:368-445` (all constraints are hard;
tiers are blame/relaxation metadata). The exact tier assignment per constraint
is a piece-plan detail.

### 2.4 Objective

Minimize `w_groups · Σ GroupHasStudents + w_pairs · Σ SharedPair`.

The weights are plain hardcoded constants, and they start out **equal**
(`w_groups = w_pairs = 1`); they will be adjusted later from experience. The
two terms do pull in opposite directions (fewer groups means fuller groups,
which creates more pairs inside each list), so some tuning is expected —
but that is a tuning question, not a design question. Exposing the weights in
an advanced dialog is a possible later piece (§5, piece 10).

### 2.5 Kept lists

The user can select existing **prefilled** lists whose pairings should count
as already-shared (the "keep semester 1, rebuild semester 2" use case). Only
prefilled lists qualify: automatic lists store no student assignment (their
filling lives in the colloscope, when it exists at all), and the whole point
of this tool is to move toward prefilled lists. Effect on the model: pair
pinning only, as described in §2.2.

### 2.6 Epochs

The intended ordering (exact algorithm still **to be designed**, see §7):
order the lists by **inclusion of their student sets**, small lists first.
The first epochs should hold mutually disjoint lists that can be solved
essentially independently; later epochs hold the larger lists that contain
them, which then align their groups with the already-built small ones through
the pair objective.

A candidate formalization: build the strict-inclusion DAG over the specs'
student sets and set `epoch(spec) = height of the longest strict-inclusion
chain below it`. Inclusion-minimal specs (whether small or large) land in
epoch 0; a spec strictly containing an epoch-k spec lands at ≥ k+1.
Overlapping-but-incomparable sets share an epoch and are solved together.
Size ranges play no role in the ordering — only student sets do.

Whatever the final scheme, everything-in-epoch-0 (a plain, non-staggered
solve) is the trivial placeholder and fallback; the problem is small enough
for that to be viable from day one.

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

## 4. Applying the result: composite op vs `GlobalUpdate` — **open decision**

Two candidate paths exist; the choice is not settled yet (see §7).

**Option A — a new composite op.** The change is additive and structured —
create k lists, associate them — which matches the shape of the `ops/`
composite layer:

```rust
GroupListsUpdateOp::AddGeneratedGroupLists(
    Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>,
)
```

following the `DuplicatePreviousPeriod` precedent
(`ops/src/group_lists.rs:301-396`): one `CascadeSession`, a loop of
`AddNewGroupList` + `AssignToSubject` elementary ops, `commit` collapses it
all into **one undo slot**. Arguments for it: it flows through the normal
`EditorInput::UpdateOp` → `dry_apply` → warning dialog path
(`gtk4/src/editor.rs:1183-1232`) — the group-lists page already speaks
`GroupListsUpdateOp`, so no new editor plumbing; fresh `GroupListId`s are
issued by the session, not hand-managed; cascade warnings surface instead of
being bypassed.

**Option B — `Op::GlobalUpdate`.** The precedent of the colloscope solver
(`gtk4/src/editor.rs:1278-1298`): clone `InnerData`, insert the new lists and
associations, apply wholesale. One undo slot too, fully validated by the
invariant gate, but it bypasses the cascade/warning machinery and the fresh
ids must be issued by hand.

Whichever option wins, the surrounding semantics are the same. Associations for the rebuilt `(period, subject)` pairs **overwrite** any
existing entry (that is `AssignToSubject`'s semantics). A previously
associated list may end up orphaned (associated nowhere); it is kept, not
auto-deleted — deleting is the user's call. (An orphaned list is legal state;
see the standing carve-out in the state-consolidation Appendix A.5.)

The modal dialog chain freezes the document during the whole flow (same
guarantee the colloscope chain relies on, `gtk4/src/editor/colloscope.rs:759-761`),
so the plan computed at config time is still valid at apply time.

## 5. The pieces

Backend first, UI second. Each piece compiles, passes tests, and is
committable on its own.

**Piece 1 — crate skeleton and core model.** New crate `constraints-groups/`
(`collomatique-constraints-groups`), mirroring the layout of
`constraints-colloscopes/`: an env type wrapping the spec list, `Var` with
`DescribeVar` derive, the `StudentInGroup` / `GroupHasStudents` extras, the
three shape constraints (§2.3), `build_model` / `build_model_with_log`
(same log-callback signature as `constraints-colloscopes/src/builder.rs:14-62`),
and a placeholder `build_incremental_epochs` putting everything in epoch 0.
Objective: the group-count term only. Unit tests on tiny hand-built spec
lists.

**Piece 2 — pair variables.** `SharedPair` extras, their reification, the
pair term of the objective, and pinned-pair fixing. Unit tests: a two-list
instance where the optimum provably reuses groupings; a pinning test showing
kept pairs make reuse free.

**Piece 3 — generation plan and output conversion.** `GenerationRequest`,
`build_generation_plan` (dedup, skipped reporting, pinned-pair extraction from
kept prefilled lists), and `build_group_lists` (compaction, `GroupList::new`).
Unit tests on hand-built `Parameters`, including the dedup and skip cases.

**Piece 4 — inclusion-based epochs.** Replace the placeholder
`build_incremental_epochs` with the student-set-inclusion ordering of §2.6.
This piece starts with its own short design pass to settle the exact
algorithm (candidate: longest strict-inclusion chain height). Unit tests on
hand-built spec families: disjoint sets, nested chains, overlapping
incomparable sets.

**Piece 5 — fuzz-build net.** Mirror
`constraints-colloscopes/tests/property_build.rs` using
`collomatique-testgen-colloscopes`: random valid documents, random
`GenerationRequest`s (random subsets of interrogation-bearing
`(period, subject)` pairs and of prefilled lists), assert
`build_generation_plan` + `build_model` never panic; plus an
`examples_build.rs` analog running a rebuild-everything request over
`examples/*.collomatique`.

**Piece 6 — the apply path.** Contingent on the §4 decision. If option A:
`GroupListsUpdateOp::AddGeneratedGroupLists` with its precise error enum,
following the existing per-variant test style in `ops/src/group_lists.rs`.
If option B: the `GlobalUpdate` assembly instead (a much smaller piece, mostly
folded into piece 9).

**Piece 7 — config dialog.** `gtk4/src/editor/group_lists/generate_dialog.rs`,
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

**Piece 8 — naming/build dialog.** New dialog replacing `loading_dialog.rs`
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

**Piece 9 — solver run and wiring.** Instantiate the generic
`run_solver::Dialog<constraints_groups::Var, ExtraVarName, ConstraintDesc>`
(`gtk4/src/editor/run_solver.rs:28`) with its own title. Chain: button →
generate_dialog → naming/build dialog → `DialogInput::Run(strategy, model,
payload)` → on `NewConfig`, `filter_transmute` to base vars,
`build_group_lists`, then apply through whichever path §4 settles on. Enable
the button at `group_lists.rs:84-93`. Persist the last-used strategy across
invocations (like the colloscope page persists its config/strategy at
`colloscope.rs:166-171`).

**Piece 10 (optional, later) —** advanced parameters (objective weights),
Python wiring (the todo mentions it), and any epoch-strategy tuning. Out of
scope for the initial roadmap; not planned further here.

## 6. Points settled

- No −1 in the `StudentGroup` domain; no `students_have_groups` analog.
- Group slots per spec: `floor(n / min_size)`.
- Kept lists restricted to prefilled ones; their only effect is pair pinning.
- Objective weights: hardcoded constants, equal to start with; tune later.
- Overwritten associations may orphan old lists; they are kept.
- Subjects listed in the config dialog: only those with interrogations.
- Config dialog defaults: rebuild on where no association exists, kept lists
  all on.
- Generated lists are `Prefilled`, unnamed groups, spec range as list range.

## 7. Points still open

- **Apply path** (§4): composite `GroupListsUpdateOp` variant vs
  `Op::GlobalUpdate`. Decides the shape of piece 6.
- **Exact epoch algorithm** (§2.6): inclusion-based, small lists first;
  the precise ordering is designed at the start of piece 4. The candidate on
  the table is longest strict-inclusion chain height.
