# Step 1 implementation plan — reshape InnerData: remove the dense copies

**Scope:** step 1 of `docs/plans/invariant_cascade_design.md` in full (bugfix → decoupling →
1a → 1c → 1b → 1d), on branch `consolidate_state`. Supersedes the survey draft; all design
decisions are resolved below (none deferred). Each phase = one or more commits; standing
gates run at every commit, milestone gates at 1a and 1d.

---

## 0. Decisions ledger (all settled with user, July 16 2026)

| # | Decision |
|---|---|
| 1 | **Week patterns store the exception set**: `WeekPattern { name, excluded_weeks: BTreeSet<WeekId> }`. Absent = active (current default). |
| 2 | **Colloscope goes flat (Option A)**: `Table<(SlotId, WeekId), BTreeSet<u32>>` + `Table<GroupListId, BTreeMap<StudentId, u32>>`; the period layer dissolves. One-field wrappers (`ColloscopeInterrogation`, `ColloscopeGroupList`) are dropped. |
| 3 | **Canonical-absent everywhere**: no empty rows in assignments, colloscope interrogations, colloscope group lists. Enforced at op sites, asserted by the checker. Keeps `InnerData::Eq` honest. |
| 4 | **`WeekId` is preserved across `CutPeriod`/`MergeWithPreviousPeriod`** (re-parenting, not delete+recreate): colloscope cells and pattern exclusions survive cut/merge. |
| 5 | **Sequencing: bugfix → decoupling → 1a → 1c → 1b → 1d** (1c pulled before 1b to de-noise the subjects.rs diff). |
| 6 | **Python glue keeps the dense-view contract** (minimum-effort rule): pyclass shapes unchanged, dense views *computed* in glue from the sparse core reusing shared helpers. Reference scripts (`scripts/`, `extra-scripts/`) untouched. All of it is throwaway scaffolding for the upcoming Python API rework. |
| 7 | **Periods container** = slots precedent: private `ordered_period_list: OrderedTable<PeriodId, Vec<WeekId>>` + `week_map: Table<WeekId, Week>` with `Week.period_id` authoritative FK; compound `pub(crate)` mutators; `Lookup<WeekId> → Week`, `Lookup<PeriodId> → Vec<WeekId>`. |
| 8 | **New `WeekOp` op family** (`AddFront/AddAfter/Remove/Update/Move`); `PeriodOp` slims to `AddFront/AddAfter/Remove` (a period is created empty; weeks are added by week ops). |
| 9 | Post-1d colloscope ops are **upserts**: `SetInterrogation(SlotId, WeekId, BTreeSet<u32>)`, `SetGroupList(GroupListId, BTreeMap<StudentId, u32>)`; empty payload = remove row. |
| 10 | `constraints-colloscopes` keeps `GlobalWeek` internally; one canonical `WeekId ↔ GlobalWeek` map built at model entry. |
| 11 | Storage format frozen; every phase byte-stable by construction (decode synthesizes `WeekId`s in period-walk order; encode projects back). |
| 12 | `excluded_weeks` is **not** canonicalized against the week's `interrogations` flag (a file may store `false` on a non-interrogation week; preserve the bit for byte-stability). Merged activity = `week.interrogations ∧ ¬excluded`. |

House rules honored throughout: test-first bug fixes; Edit tool only (no sed); no
`serde(default)`-style silent defaults; pyclass mirrors + contract scripts updated in the
same change (here: mirrors preserved by computed views); `foo.rs + foo/` module style;
`Table` types never leave `state-colloscopes`.

---

## Phase 0 — bugfix: `UpdatePeriodWeekCount` cleaning loop (2 commits)

**Bug** (`ops/src/general_planning.rs:357`): inside `get_next_cleaning_op`, the shrink
branch is only reachable when `*week_count < old_week_count` (guard at `:343`), yet the
colloscope-scan loop is:

```rust
for week in old_week_count..*week_count {   // start > end ⇒ always empty
```

so the cleaning op for interrogations on removed weeks never fires and shrinking a period
with colloscope content surfaces as a hard `NotCompatibleSlotInColloscope` error
(`state-colloscopes/src/periods.rs:507`) instead of an auto-clean. The week-pattern cleanup
just below (`:375-402`) uses correct bounds — isolated swap.

**Commit 0.1 — regression test (committed alone, verified failing).** New file
`ops/tests/found_bugs.rs` (first test file in the crate; mirrors the state-colloscopes
`tests/found_bugs.rs` convention). Build a doc via elementary ops: one period (≥2 weeks),
one subject with interrogation params, teacher, slot; put a non-empty interrogation on the
*last* week (`ColloscopeOp::UpdateInterrogation`); then drive the composite
`GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period, 1)` through the ops-layer entry
point that consults `get_next_cleaning_op` and assert a cleaning op *is* produced (today:
`None` → the test fails). Keep the assertion on observable behavior (a cleaning op exists
and empties the doomed cell), not on the op's exact shape, so it survives phase 5/7 rewrites.

**Commit 0.2 — the fix**:

```rust
for week in *week_count..old_week_count {
```

plus re-run of the regression test and the ops crate suite. Nothing else rides along.

---

## Phase 1 — decoupling commit: consumers stop *relying* on denseness (1 commit)

Behavior-preserving conversions, only for sites that (i) survive the reshapes and (ii) have
a params-side source of truth today. Sites whose job *is* the dense skeleton
(`new_empty_from_params`, `update_slot_for_week_pattern`, storage densification, the
delete-blocking scans) are deliberately untouched — they get deleted by their phases.

1. **`apply_assignment` stops probing the key for "subject runs on period"**
   (`state-colloscopes/src/assignments.rs`, the `SubjectDoesNotRunOnPeriod` /
   `SubjectDoesNotRunOnPeriod`-adjacent checks around `:107-120`): consult
   `subject.excluded_periods` (+ subject existence) directly. Same errors, same order.
2. **Week-pattern-length proxies → `periods.count_weeks()`**: grep-driven sweep for
   `.weeks.len()` used as the global week count (known live logic:
   `WeekPattern.add_weeks/remove_weeks` callers pass period-walk offsets — untouched;
   the sweep targets *reads* treating pattern length as schedule length, e.g. any
   gtk4/python/ops site sizing a UI or loop off `pattern.weeks.len()`).
3. **gtk4 assignments display seeds from params**
   (`gtk4/src/editor/assignments/assignments_display.rs:255-257`): replace the
   `.expect("Subject id should be valid at this point")` on the per-period map with
   iteration driven by params-side subjects and `.unwrap_or_default()` for the student
   set — identical rendering today, sparse-proof tomorrow.
4. **`ops/` assignment reads**: already `.iter()`-based and sparse-safe (verified) — audit
   only, no changes expected; note findings in the commit message.

Explicitly **not** converted: `period_map.get(..).expect(..)` and `interrogations[w]`
sites — under the current architecture those `.expect`s are documentation of a guaranteed
invariant; blanket defensive `.get()` would hide real violations. They are re-cut in 1d.

Also in this commit (cheap riders): delete the stale multi-colloscope comment block
(`state-colloscopes/src/lib.rs:174-178`) and the unused `FromDataError`
(`lib.rs:263-275`) after a final `grep` confirms no external user.

---

## Phase 2 — 1a: assignments sparse (1 commit)

**Target:** same type, new contract — a `(period, subject)` row exists **iff** its student
set is non-empty.

```rust
/// Sparse junction table keyed by `(period, subject)`: a row is present exactly
/// when at least one student is assigned. Absent row = nobody assigned (the
/// canonical form: ops never leave an empty row behind). Whether a subject runs
/// on a period is *not* encoded here — consult `Subject::excluded_periods`.
pub struct Assignments {
    pub map: Table<(PeriodId, SubjectId), BTreeSet<StudentId>>,
}
```

All in one commit (checker flip and decode seeding cannot be split without a red
intermediate state):

1. **Op canonicalization** (`assignments.rs`, `apply_assignment`):
   `Assign(p, student, s, true)` inserts the row if absent, then inserts the student;
   `Assign(p, student, s, false)` removes the student and **removes the row if now empty**.
   Validation order unchanged (period id, subject id, student id, runs-on-period from
   Phase 1, student-present-on-period). Reverse op unchanged (`Assign` with flipped bool).
2. **Delete the 6 fan-out sites**: `periods.rs:212-218`, `:264-270`, `:413-423`;
   `subjects.rs:365-377`, `:501-513`, `:702-727`. The period/subject **Remove guards stay**
   (blocking on non-trivial assignments) but simplify to "any row with this
   period/subject exists" — under canonical-absent, row existence ⇔ non-trivial.
3. **Checker re-cut** (`colloscope_params.rs:466-511`,
   `check_assignments_data_consistency`) — from count-based to row-based, modeled on the
   `subjects_associations` checker:

```rust
fn check_assignments_data_consistency(&self, period_ids: &BTreeSet<PeriodId>)
    -> Result<(), InvariantError>
{
    for (period_id, subject_id, students) in self.assignments.iter() {
        if !period_ids.contains(&period_id) {
            return Err(InvariantError::InvalidPeriodIdInAssignements);
        }
        let Some(subject) = self.subjects.find_subject(subject_id) else {
            return Err(InvariantError::InvalidSubjectIdInAssignments);
        };
        if subject.excluded_periods.contains(&period_id) {
            return Err(InvariantError::AssignmentForSubjectNotRunningOnPeriod);
        }
        if students.is_empty() {
            return Err(InvariantError::EmptyAssignmentRow);   // canonical form
        }
        for student_id in students { /* exists + present-on-period, as today */ }
    }
    Ok(())
}
```

   `WrongSubjectCountInAssignments` is deleted; two new variants
   (`AssignmentForSubjectNotRunningOnPeriod`, `EmptyAssignmentRow`) added. (This is still
   the *old* coordinate-free vocabulary — precise coordinates arrive in step 2; don't
   gold-plate here.)
4. **Storage**: delete the decode densification (`storage/src/decode/spec2.rs:414-432`,
   keep the sparse-row ingestion + validation `:433-451`, now also *rejecting/skipping*
   nothing new — an explicitly-empty row in a file decodes to an absent row, which is what
   `neutral_rows_decode_identically_to_their_absence` already pins). Encode `:288-292`
   keeps its empty-skip (now a canonical-form truism; keep the guard, retarget the
   comment). Byte-stable by construction.
5. **Tests**: rewrite `derived_key_sets_are_completed`
   (`storage/tests/spec2_format.rs:557-598`) to pin the *sparse* contract
   (`assignments.map.len() == 0` for the skeleton file); property-harness generator: where
   `gen_op` picks `(period, subject)` targets off `assignments` keys, re-target from
   params (periods × non-excluded subjects); oracle change is (3) itself.
6. **Python glue** (`python/src/glue/params.rs:89-102`): the outer period map is already
   seeded; also seed the inner map from non-excluded subjects:

```rust
for (period_id, _) in periods {
    let inner = assignments.entry(period_id).or_default();
    for (subject_id, subject) in subjects {
        if subject.excluded_periods.contains(&period_id) { continue; }
        inner.insert(subject_id, BTreeSet::new());
    }
}
for (p, s, students) in data.assignments.iter() { /* overwrite */ }
```

   Pyclass shape unchanged ⇒ both import scripts and the export example provably
   unaffected.

**Milestone gate ★**: full workspace tests, 500-seed harness, hogwarts pristine load,
byte-stability suite; user runs the 3 contract scripts + gtk4 smoke.

---

## Phase 3 — 1c: sparse slots `ordering` sidecar (1 commit)

**Target:** an `ordering` row exists iff the subject has ≥1 slot (no more empty-vec rows).
All inside the existing encapsulation boundary:

- `insert_slot_at` creates the row on first slot (`or_default()`); `remove_slot` drops the
  row when the vec empties; **delete `add_subject_entry`/`remove_subject_entry`**
  (`slots.rs:268-281`) and their 4 call sites (`subjects.rs:363/500/652/655`) — the last
  cross-entity fan-out into `Slots`.
- `from_subject_rows` (`slots.rs:110-126`): skip empty rows (format spec §4.7 says they're
  "valid but redundant" — decode normalizes them away; byte-stable because encode
  `build_slots` already drops empty rows, `encode/spec2.rs:338-342`).
- Checker (`check_slots_data_consistency`, `colloscope_params.rs:569-625`): replace the
  keyset-equality (`WrongSubjectCountInSlots`) with per-row checks — ordering row's
  subject exists **and has interrogations** (new `SlotsForSubjectWithoutInterrogations`
  variant, joining the existing side-constraint family), row non-empty (canonical form),
  permutation/orphan checks unchanged.
- Callers that assumed `slots_for_subject`/`slot_count_for_subject` return `Some` for
  every interrogation subject now treat `None` as empty — notably
  `ColloscopePeriod::new_empty_from_params` (`colloscopes.rs:225-228`,
  `.expect("Subjects should have slots")`) and its `validate_against_params` twin
  (`:256-259`).

---

## Phase 4 — 1b: `WeekId` (2 commits, B1 + B2)

### Commit B1 — weeks become entities; patterns stay `Vec<bool>` for one more commit

**New types** (`state-colloscopes/src/periods.rs`, `ids.rs`):

```rust
#[derive(..., EntityId)]
#[entity(Week)]
pub struct WeekId(u64);                      // NewId gains a Week variant;
                                             // IdIssuer gains get_week_id()

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join)]
pub struct Week {
    /// Period this week belongs to (authoritative; the ordering sidecar groups it here)
    #[fk(name = period)]
    pub period_id: PeriodId,
    pub interrogations: bool,
    pub annotation: Option<non_empty_string::NonEmptyString>,
}

pub struct Periods {
    pub first_week: Option<collomatique_time::WeekStart>,
    /// Period order + per-period week order (private, slots-style encapsulation)
    ordered_period_list: OrderedTable<PeriodId, Vec<WeekId>>,
    week_map: Table<WeekId, Week>,
}
```

Mirror invariant (`week_map` keyset == ∪ ordering vecs; `week.period_id` == owning period)
is encapsulated exactly like `Slots`: private fields, compound `pub(crate)` mutators
(`insert_period_at`, `remove_period` (must be week-empty), `insert_week_at(period, pos,
Week)`, `remove_week(WeekId)`, `move_week(WeekId, PeriodId, pos)`, `update_week`), a
decode builder `from_period_rows(...) -> Result<Self, DuplicatedWeekIdError>`, and a read
surface (`period_ids()`, `weeks_of(PeriodId) -> &[WeekId]`, `find_week`, `count_weeks`,
`walk() -> impl Iterator<Item = (PeriodId, WeekId)>` — the canonical global order,
replacing every hand-rolled accumulate-`len()` loop). New
`check_periods_data_consistency` validates the mirror under the old architecture (same
role `check_slots_data_consistency` plays for slots). `Lookup<WeekId> → Week`;
`Lookup<PeriodId>` re-targets from `Vec<WeekDesc>` to `Vec<WeekId>`; `ids.rs:41`
`#[entity(...)]` updated accordingly. `WeekDesc` survives only as the *op payload / glue
DTO* `{ interrogations, annotation }` (no period FK), reused by `WeekOp` and the pyclass.

**Op re-cut** (`ops.rs`, `periods.rs`):

```rust
pub enum PeriodOp   { AddFront, AddAfter(PeriodId), Remove(PeriodId) }   // created empty
pub enum WeekOp {
    AddFront(PeriodId, WeekDesc),        // annotate issues the WeekId
    AddAfter(WeekId, WeekDesc),
    Remove(WeekId),
    Update(WeekId, WeekDesc),            // status/annotation change
    Move(WeekId, PeriodId, usize),       // re-parent, preserving the id (decision 4)
}
```

Transitional maintenance inside `apply_week` (dies in B2/1d respectively):
- *patterns* (until B2): `AddFront/AddAfter` → `add_weeks(global_pos, 1)`; `Remove` →
  guarded `can_remove_weeks` (`NonTrivialWeekPattern`) then `remove_weeks`; `Move` →
  remove-at-old + insert-at-new **carrying the bool value** (semantics preview of B2).
- *colloscope* (until 1d): splice one cell per affected slot vec at the week's per-period
  position; `Remove`/`Move`-out guarded on cell emptiness as today; `Move` into a period
  where the slot doesn't exist (subject excluded there) requires the moved cells empty —
  same guard family. `Update(interrogations: false)` keeps today's
  `check_empty_on_removed_weeks`-style guard.

**Composite ops** (`ops/src/general_planning.rs:234-248` enum unchanged for gtk4's sake):
`AddNewPeriod(n)` = `PeriodOp::Add` + n × `WeekOp::AddFront/AddAfter`;
`UpdatePeriodWeekCount` = adds/removes tail weeks; `UpdateWeekStatus/Annotation` =
`WeekOp::Update`; `CutPeriod(p, k)` = `PeriodOp::AddAfter(p)` + `Move` for each tail week
(ids preserved — colloscope cells and pattern bits now *survive* a cut, deleting the
save/clean machinery at `general_planning.rs:1387-1445`); `MergeWithPreviousPeriod` =
`Move` every week + `PeriodOp::Remove`. The cleaning-op scans re-cut mechanically (they
already iterate weeks; they now iterate `weeks_of(period)`).

**Consumers re-cut in B1** (all mechanical; positional helpers replaced by `walk()`):
- `Parameters::merge_pattern`/`get_merged_pattern` (`colloscope_params.rs:42-70`) and
  `Slot::build_pattern_for_new_period` (`slots.rs:77-99`) — walk-based, same output.
- Colloscope validate/resize helpers (`colloscopes.rs:125-192`, `319-484`) — index via
  `weeks_of(period)` positions instead of accumulate-`len()`.
- storage: decode reads positional `GeneralPlanning.periods[].weeks`, synthesizes
  `WeekId`s in walk order (fresh ids above the file's max id, seeded into the `IdIssuer`
  like every id); encode projects `walk()` back to positional rows. Colloscope
  interrogation `week: u32` ↔ `WeekId` via the same walk (decode `week_table` at
  `decode/spec2.rs:730-736` becomes `Vec<WeekId>`). **Bytes identical.**
- constraints-colloscopes: `tools.rs:20-100`, `convert.rs`, `helpers.rs`, `extras.rs`
  re-source their period/week walks from `walk()`/`weeks_of`; `GlobalWeek` and all
  windowing logic untouched (decision 10). The `WeekId ↔ GlobalWeek` map is
  `walk().enumerate()`.
- gtk4 `general_planning`/`colloscope_display`/period displays: emit the same composite
  ops; week rows carry `WeekId` alongside display position (mechanical).
- python glue: `Period { id, weeks_status }` rebuilt from `weeks_of` + `week_map`
  (pyclass shape unchanged); `count_weeks` etc. unchanged.

### Commit B2 — week patterns become the exception set

```rust
pub struct WeekPattern {
    pub name: String,
    /// Weeks the pattern *disables*. Absent = active (the trivial value).
    /// May reference non-interrogation weeks (bit preserved for byte-stability).
    pub excluded_weeks: BTreeSet<WeekId>,
}
```

- **Delete**: `add_weeks`/`clean_weeks`/`remove_weeks`/`can_remove_weeks`
  (`week_patterns.rs:48-92`), every lockstep-splice call in period/week ops, the length
  invariant (`BadWeekPatternLength`, `validate_week_pattern_internal`,
  `check_week_pattern_data_consistency` length check → replaced by a dangling-`WeekId`
  sweep over `excluded_weeks`). Invariant #8 is gone.
- `WeekOp::Remove` guard re-cut: blocked while any pattern excludes the week
  (`NonTrivialWeekPattern`, same UX as today — the *bit* is the data being protected);
  `Move` needs no pattern work at all (membership travels with the id).
- `merge_pattern` collapses into the single shared helper that 1d and the glue also use:

```rust
impl Parameters {
    /// One definition of "slot can have an interrogation on week".
    pub fn is_week_active(&self, week_id: WeekId, pattern: Option<WeekPatternId>) -> bool {
        let week = self.periods.resolve(week_id);
        week.interrogations
            && pattern.is_none_or(|p| !self.resolve(p).excluded_weeks.contains(&week_id))
    }
}
```

- storage: encode = `walk()` emitting `!excluded.contains(id)` per position; decode =
  insert ids for `false` bits. Round-trip identity on both bit values ⇒ byte-stable
  (decision 12: no canonicalization against `interrogations`).
- gtk4 pattern dialog + python `WeekPattern { weeks: Vec<bool> }` glue: complement at the
  boundary via the same projection (pyclass shape unchanged).

---

## Phase 5 — 1d: colloscope sparse (3 commits, D0/D1/D2 — the phase-D/E migration pattern)

### Commit D0 — prep on the old shape

Add the read/write surface consumers will use, implemented against the *current* dense
shape, and move consumers onto it:
- `Colloscope::interrogation(&self, slot: SlotId, week: WeekId) -> Option<&BTreeSet<u32>>`,
  `interrogations_for_slot(slot)`, `iter()` (row iterator), `group_list(GroupListId)`,
  plus `is_empty`-style predicates re-expressed on the surface.
- Re-cut `ops/`'s ~16 dense walk sites (6 files, heaviest `general_planning.rs`,
  `group_lists.rs`), gtk4's 2 colloscope view files, and `constraints-colloscopes`
  `convert.rs:8-70/:105-127` onto these accessors (using `WeekId` from 1b). After D0, no
  consumer outside `state-colloscopes` + storage touches `period_map`/`slot_map`/
  `interrogations` directly.

### Commit D1 — the swap

```rust
pub struct Colloscope {
    /// Assigned groups per (slot, week); row present iff non-empty (canonical form)
    interrogations: Table<(SlotId, WeekId), BTreeSet<u32>>,
    /// Student→group placements per non-prefilled group list; row iff non-empty
    group_lists: Table<GroupListId, BTreeMap<StudentId, u32>>,
}
```

- **Ops** become upserts (decision 9): `SetInterrogation(slot, week, groups)` /
  `SetGroupList(list, placements)`; empty = remove row; reverse = `Set…` with the prior
  payload (or empty). Input validation kept from today's `apply_colloscope`
  (`colloscopes.rs:717-761`): ids resolve, `is_week_active(week, slot.week_pattern)`,
  group numbers `< group_names.len()` of `subjects_associations[(week.period,
  slot.subject)]` (absent association ⇒ no groups allowed), list non-prefilled, students
  eligible — minus all cell-existence preconditions.
- **`validate_against_params` re-cut** to per-row checks (the tier-3 residue, written
  once, `Join`-flavored):

```rust
for ((slot_id, week_id), groups) in self.interrogations.iter() {
    let Some(slot) = params.slots.find_slot(slot_id) else { return Err(InvalidSlotId(..)) };
    let Some(week) = params.periods.find_week(week_id) else { return Err(InvalidWeekId(..)) };
    let subject = params.resolve(slot.subject_id);
    if subject.excluded_periods.contains(&week.period_id) { /* SlotNotRunningOnPeriod */ }
    if !params.is_week_active(week_id, slot.week_pattern)  { /* InterrogationOnInactiveWeek */ }
    if groups.is_empty()                                   { /* EmptyInterrogationRow */ }
    /* group-number bound via subjects_associations[(week.period_id, slot.subject_id)] */
}
/* group_lists rows: id resolves, non-prefilled, students valid/eligible, bounds, non-empty */
```

- **Delete outright**: `new_empty_from_params` (all three levels),
  `update_slot_to_match_week_pattern`/`update_slot_for_week_pattern`,
  `check_empty_on_removed_weeks`, every colloscope fan-out site in period/week/slot/
  subject/week-pattern/group-list ops (the ~330 lines), and the B1 transitional splices.
  The **Remove-guards survive re-cut**: "period/slot/week/group-list still referenced" =
  "a row with this id exists" (range scan on the composite key for slots; filtered scan
  for weeks) — strictly less code than the skeleton walks they replace.
- **Storage**: decode `reconstruct_colloscope` (`decode/spec2.rs:719-797`) becomes
  near-identity (row-in, row-out, keeping the placement validations as decode trust-
  boundary errors); encode `build_colloscope` (`encode/spec2.rs:542-606`) iterates rows,
  projecting `WeekId` → global index. Byte-stable; `derived_key_sets_are_completed`
  colloscope arm re-pinned to sparse.
- **Property harness**: generator re-targets interrogation ops from params
  (periods × slots × active weeks via `is_week_active`), not the skeleton
  (`generator.rs:147-166`); oracle is the re-cut validate. found_bugs cell tests
  (`:84-151`, `:158-301`) keep scenarios; upsert semantics removes their skeleton
  dependence.

### Commit D2 — cleanup sweep

Dead helpers, dead error variants (`WrongPeriodCountInColloscopeData`,
`WrongSlotCountInPeriodInColloscopeData`, `WrongInterrogationCountForSlot…`,
`InterrogationOnNonInterrogationWeek`/`Missing…` — superseded by the new row vocabulary),
doc comments, and the **python glue dense view** (decision 6): a computed
`Colloscope`-shaped pyclass built from params + rows + `is_week_active` (~50 lines,
`python/src/glue/colloscopes.rs`), keeping `period_map`/`slot_map`/`interrogations`
pyclass shapes byte-for-byte so `custom_export_xlsx.py` runs unchanged.

**Milestone gate ★** (= end of step 1): 500-seed harness, byte-stability + hogwarts,
user runs 3 contract scripts + gtk4 smoke.

---

## Verification (every commit unless noted)

1. `cargo build --workspace` + `cargo test --workspace` (no clippy per house rule).
2. Property harness at 100 seeds (in `cargo test -p collomatique-state-colloscopes`);
   **500 seeds at the two ★ milestones**.
3. Storage byte-stability: `spec2_format.rs` re-serialize tests +
   `populated_round_trip.rs` + `all_examples_load_pristine` (hogwarts, zero caveats).
4. ★ milestones, user-run: the 3 Python contract scripts (`import.py`,
   `import_pronote_web_2026_05_06.py`, `custom_export_xlsx.py` — expected unchanged
   output) and a gtk4 app smoke (edit weeks/patterns/colloscope cells, undo/redo,
   save/reload).
5. No new dependencies anywhere ⇒ no `Cargo.lock` change ⇒ no Nix `cargoHash` refresh.

## Risks & watch items

- **B1 is the widest commit** (op re-cut + every positional walk). Mitigation: `walk()`
  centralizes the order; constraints-colloscopes regression fixtures
  (`build_model_regression.rs` etc.) pin the solver-facing behavior.
- **Transitional colloscope maintenance in B1** (splice-per-week-op) is throwaway code —
  keep it minimal and lean on the existing `update_slot_for_week_pattern` helpers; 1d
  deletes it weeks later.
- **Decode `WeekId` synthesis** must allocate above the file's max id before the
  `IdIssuer` scan — same pattern as every synthesized id today; the duplicate-id checker
  covers mistakes.
- **Error-variant churn** (new `InvariantError`/`ColloscopeError` variants, deleted count
  variants) touches gtk4/python error display strings — sweep `match`es in the same
  commits; the big vocabulary collapse still belongs to step 5, don't gold-plate.
