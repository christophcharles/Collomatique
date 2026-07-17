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

## Phase 0 — bugfix: `UpdatePeriodWeekCount` cleaning loop (2 commits) — **DONE** (`1418d4bf`, `f8e34128`)

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

## Phase 1 — decoupling commit: consumers stop *relying* on denseness (1 commit) — **DONE** (`d3c56e9f`)

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

## Phase 2 — 1a: assignments sparse (1 commit) — **DONE** (`9f4471e2`)

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

## Phase 3 — 1c: sparse slots `ordering` sidecar (1 commit) — **DONE** (`b681cdac`)

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

## Phase 4 — 1b: `WeekId` (B1 **DONE** as six commits `2a0ec129`…`2de37eae`; B2 **DONE** as `fb1793eb`)

### Commit B1 — weeks become entities; patterns stay `Vec<bool>` for one more commit — **DONE**

> **DONE (July 17 2026), landed as six contained commits.** The one-commit B1 proved
> far too wide, so it was split (each commit green, byte-stable, either
> wide-but-shallow or deep-but-narrow). The full split plan — decisions ledger S1–S12
> and per-commit deviation notes — is retired from the tree; pinned at
> `git show 2de37eae:docs/plans/plan_step_1_b1_split.md`.
>
> | # | Commit | Content |
> |---|--------|---------|
> | 0 | `2a0ec129` | Read surface on `Periods` (old shape); `ordered_period_list` privatized; ~50 consumer sites across 8 crates moved onto it |
> | 1 | `fac63ae0` | `WeekId` inline (`Vec<(WeekId, WeekDesc)>` payload); decode synthesis; week ids join `ids()`; undo/redo preserves ids |
> | 2 | `efaf50cc` | `WeekOp` family + `apply_week` with transitional pattern/colloscope maintenance; testgen `gen_week`; targeted unit tests |
> | 3 | `b0da5353` | Composite planning ops emit week ops; cut/merge preserve content; save/clean/restore machinery deleted |
> | 4 | `67c539a1` | `PeriodOp` slimmed to empty-period ops (`Remove` requires week-empty); `apply_week` is the sole week writer |
> | 5 | `2de37eae` | Backend swap: `week_map: Table<WeekId, Week>` + ordering sidecar; `Week` entity with `period_id` FK; mirror checker |

**Landed state — where it refines the original sketch, and what B2/1d build on:**

- **Read surface (final):** `walk() -> impl Iterator<Item = (PeriodId, WeekId, &Week)>`
  (the canonical global order; `walk().enumerate()` = global week index),
  `weeks_of(PeriodId) -> Option<impl Iterator<Item = &Week>>` (descs only, no ids —
  richer than the sketched `&[WeekId]`), `weeks_vec_of -> Option<Vec<WeekDesc>>`
  (op-payload/UI building), `find_week(WeekId) -> Option<&Week>` (owning period via
  `week.period_id`), `week_id_at(period, pos)`, `week_position`, `global_week_position`,
  `period_ids()`, `week_count_of`. `find_period(PeriodId) -> Option<&Vec<WeekId>>` is
  **pub** (pinned by the `read_api` pointer-identity test). `Lookup<WeekId> → Week`,
  `Lookup<PeriodId> → Vec<WeekId>` as planned. `WeekDesc` survives as the FK-less
  op-payload/glue DTO with `Week::desc()`/`Week::from_desc()` converters.
- **`PeriodOp` kept `ChangeStartDate`** (period-adjacent, nowhere better to live);
  final shape `{ ChangeStartDate, AddFront, AddAfter(PeriodId), Remove(PeriodId) }`
  with `Remove` requiring week-empty (`PeriodError::PeriodStillHasWeeks`); the
  `Update` arm is gone (and with it the annotate-can't-see-data id-preservation wart).
- **`WeekOp::Move` carries content** (supersedes the sketch's "Move-out guarded on cell
  emptiness"): pattern bits travel via the new `WeekPattern::move_week`, colloscope
  cells travel verbatim; guards only where content *cannot* travel (dest period lacks
  the slot, or group numbers exceed the dest association bounds). This is what allowed
  deleting `save_then_clean_end_of_period`/`restore_end_of_period` outright — cut/merge
  now preserve content by construction.
- **`WeekOp::Remove` requires trivial state** (every pattern bit `true`, every cell
  empty), so undo re-adds with the original id and restores the exact prior state.
- **Mirror invariant:** `check_periods_data_consistency` (`InvariantError::InvalidWeek`)
  wired right after the duplicate-id check; refs registry gained `RefSite::WeekPeriodFk`,
  walked first in `walk_params_refs`.
- **Storage:** decode pre-scans `max_used_id` over all 10 id-bearing format blocks, then
  synthesizes week ids `max+1, max+2, …` in walk order; encode never writes week ids ⇒
  bytes unconditionally identical. `populated_round_trip` compares **re-encoded bytes**,
  not `InnerData` equality (decode-synthesized ids differ from ops-issued ones by
  design). Colloscope decode's `week_table` stays positional
  (`(PeriodId, week_in_period)` pairs off the surface) — becomes `WeekId`-keyed in 1d.
- **gtk4 stayed positional** (composite `GeneralPlanningUpdateOp` unchanged, so week
  rows carry no `WeekId` yet); `UpdateWeekStatus/Annotation` resolve ids internally via
  `week_id_at(p, w)`.
- **New tests pinning the contract downstream phases rely on:**
  `state-colloscopes/tests/week_ops.rs` (move-preserves-content, both Move guards,
  remove-blocked-by-pattern-bit, undo restores id, update-blocked-by-filled-cell);
  `ops/tests/general_planning_content.rs` (cut preserves the tail's colloscope cell +
  pattern bit; merge-back structure/pattern — **the contract B2/1d rely on**);
  `state-colloscopes/tests/read_api.rs` (`resolve == find_period` by pointer).
- **Transitional code and where it dies:** pattern splices in `apply_week`
  (`add_weeks`/`can_remove_weeks`/`remove_weeks`/`move_week`) and `clean_weeks` in the
  surviving ops-layer cleaning machinery → deleted by **B2**; colloscope cell splices in
  `apply_week` → deleted by **1d**. `can_remove_weeks`/`remove_weeks` asserts were
  relaxed to `len >= first_week` (zero-count removal on an empty last period is a valid
  boundary — merge empties the source before `DeletePeriod`).
- **Gates:** 500-seed harness ran clean after commit 5 (committed `property_ops` config
  stays at 100 seeds). **Outstanding, user-run:** gtk4 smoke (edit weeks/periods,
  cut/merge with a filled colloscope, undo/redo, save/reload) + the 3 contract scripts.

### Commit B2 — week patterns become the exception set — **DONE**

```rust
pub struct WeekPattern {
    pub name: String,
    /// Weeks the pattern *disables*. Absent = active (the trivial value).
    /// May reference non-interrogation weeks (bit preserved for byte-stability).
    pub excluded_weeks: BTreeSet<WeekId>,
}
```

> **DONE (July 17 2026), one commit `fb1793eb`.** Landed as sketched, with the
> refinements noted below. Full workspace suite green, property harness (100
> seeds), byte-stability + `all_examples_load_pristine`, no `Cargo.lock` change.

- **Deleted**: `add_weeks`/`clean_weeks`/`remove_weeks`/`can_remove_weeks`/`move_week`
  (`week_patterns.rs`; `move_week` from B1 commit 2 died here too), every lockstep pattern
  splice in `apply_week` (add/remove/move now do **no** pattern work — membership travels
  with the week id) and in the ops-layer cleaning (`general_planning`/`slots`/
  `week_patterns` now diff exclusion sets by `week_id_at`, no positional `.weeks[..]`),
  and the length invariant (`BadWeekPatternLength`, `validate_week_pattern_internal`, the
  `check_week_pattern_data_consistency` length arm) → replaced by a dangling-`WeekId`
  sweep over `excluded_weeks` (new `WeekPatternError::WeekPatternExcludesInvalidWeek`;
  the ops error variants `Add/UpdateNewWeekPatternError::BadWeekCountInWeekPattern` were
  renamed `WeekPatternExcludesInvalidWeek`). Invariant #8 is gone.
- `WeekOp::Remove` guard re-cut: blocked while any pattern excludes the week
  (`NonTrivialWeekPattern`); `Move` needs no pattern work (membership travels with the id).
- **`merge_pattern` did *not* fully collapse.** `Parameters::is_week_active` (below) is the
  canonical per-week definition, used by `constraints::extract_week_pattern` and the glue.
  But the transitional colloscope maintenance (deleted in 1d) still needs *positional*
  `Vec<bool>`, so two producers were re-expressed on the exclusion set and survive one more
  commit: `merge_excluded(&BTreeSet<WeekId>)` (interrogation ∧ ¬excluded) and
  `week_pattern_active_bits(Option<WeekPatternId>)` (raw ¬excluded, fed to the reworked
  `Slot::build_pattern_for_new_period(new_descs, first_week, active_bits)`).

```rust
impl Parameters {
    /// One definition of "slot can have an interrogation on week".
    pub fn is_week_active(&self, week_id: WeekId, pattern: Option<WeekPatternId>) -> bool {
        let week = self.resolve(week_id); // Parameters: Lookup<WeekId> → Week (no periods.resolve)
        week.interrogations
            && pattern.is_none_or(|p| !self.resolve(p).excluded_weeks.contains(&week_id))
    }
}
```

- **refs.rs deviation (the sketch was silent here):** `RefSite::WeekPatternLengthCoupling`
  stays **period-keyed** (a `period_ref`), with `non_trivial` recomputed as "the pattern
  excludes at least one of this period's weeks" — mirrors the transitive delete guard
  (deleting a period first removes its weeks, each blocked by the per-week
  `NonTrivialWeekPattern`). Remodelling this to a genuine week-ref (`week_ref` visitor +
  `references_to_week`) is deferred to the registry remaster (step 7), not B2. The
  `refs_registry.rs` pin values are unchanged.
- storage (format **frozen**): encode = `walk()` emitting `!excluded.contains(id)` per
  position; decode zips the positional bits against the walk order, excluding weeks whose
  bit is `false` (surplus bits ignored, missing = active). Byte-stable (decision 12: no
  canonicalization against `interrogations`).
- gtk4 pattern dialog + python `WeekPattern { weeks: Vec<bool> }` glue: complement at the
  boundary via the same projection (pyclass shape unchanged). The dialog now holds a
  positional `Vec<bool>` internally, converting on `Show`/`Accept`; the glue gained
  `WeekPattern::into_mem/from_mem(&[WeekId])` + `InternalFile::week_ids_in_order`.
- **Transitional code still alive after B2** (all deleted in 1d): the colloscope cell
  splices in `apply_week` (add/remove/update/move) and the positional producers
  `merge_excluded` / `get_merged_pattern` / `week_pattern_active_bits` /
  `build_pattern_for_new_period`, plus the `check_empty_on_removed_weeks` /
  `update_slot_for_week_pattern` colloscope machinery they feed.
- **Gates:** automated gates all green (see the DONE note). **Outstanding, user-run** (B2 is
  a standing-gate commit, not a ★ milestone): gtk4 smoke (week-pattern dialog: edit bits,
  the all/none/even/odd buttons, save/reload, undo/redo) + the 3 contract scripts.

---

## Phase 5 — 1d: colloscope sparse (D0 **DONE**; D1 **DONE** as six commits, last `0d5cc34b`; D2 **DONE**, one commit)

### Commit D0 — prep on the old shape — **DONE**

Added the read/write surface consumers use, implemented against the *current* dense shape,
and moved every consumer onto it. Delivered:
- Surface on `Colloscope` (`state-colloscopes/src/colloscopes.rs`): `interrogation(&self,
  periods, slot, week) -> Option<&BTreeSet<u32>>`, `interrogations_for_slot(periods, slot)`,
  `iter(periods)` (row iterator), `group_list(id)`, `group_lists_iter()`, and the upsert
  writers `set_interrogation` / `set_group_list` (panic on impossible coords, cleared row on
  empty payload). Canonical sparse view: `None`/missing/`Some(empty)` cells all read absent.
  The `&Periods` argument is transitional — D1 drops it. New tests in
  `tests/colloscope_surface.rs` (5).
- Possibility predicates (permanent): `WeekPatterns::is_week_active(periods, week, pattern)`
  (homed on `WeekPatterns` so gtk4's piece-clones can call it; `Parameters::is_week_active`
  now delegates) and `Parameters::is_interrogation_possible(slot, week)` — the single oracle
  for the dense skeleton's Some-cell rule, reused by every re-derivation.
- Re-cut all ~18 `ops/` dense-walk sites (7 files, heaviest `general_planning.rs`,
  `group_lists.rs`), translating `WeekId → (period, position)` only at op-payload emission
  (op shapes unchanged in D0). `Erase*` composites now skip no-op sub-ops (intended micro
  change).
- Re-cut `constraints-colloscopes/convert.rs` (`build_config`/`build_complete_config`
  zero-fill re-derived from params; `build_colloscope` accumulates D1's row shape then
  commits through the writers), and gtk4's colloscope view (`colloscope_display.rs` gains a
  `WeekPatterns` clone + `WeekId` columns + params-derived possibility; `editor/colloscope.rs`
  re-keys `EditInterrogation` to `(SlotId, WeekId)`, translating back to positional at op
  build; group-list dialog sources via `group_list(id)`).

**Scope pulled forward** (user-confirmed): D0 also moved the **python glue** and the
**testgen generator** onto the surface, so those items no longer belong to D1/D2 (see the
struck-through notes there):
- `python/src/glue/colloscopes.rs`: the `From<mem::Colloscope>` chain became
  `Colloscope::from_mem(&mem::Colloscope, &Parameters)` — a computed dense pyclass built from
  params + surface, byte-for-byte identical (this was **D2's python dense-view item**).
- `testgen-colloscopes/src/generator.rs`: `colloscope_targets` /
  `colloscope_group_list_ids` derived from params (`is_interrogation_possible`, non-prefilled
  lists) instead of the skeleton (this was **D1's generator re-target**).

After D0, no consumer outside `state-colloscopes` + storage touches
`period_map`/`slot_map`/`interrogations`/`colloscope.group_lists` directly. Verified: clean
`cargo build --workspace`, full `cargo test --workspace` green (property_ops 100-seed random
walk, storage 127 byte-stability tests, new surface tests), no `Cargo.lock` change.
**OUTSTANDING user-run gate**: gtk4 colloscope smoke (grid renders identically, cell
edit/erase, undo/redo) + `custom_export_xlsx.py` unchanged output.

### Commit D1 — the swap — **DONE** (six commits, last `0d5cc34b`)

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
- **Property harness**: ~~generator re-targets interrogation ops from params
  (periods × slots × active weeks via `is_week_active`), not the skeleton
  (`generator.rs:147-166`)~~ *(landed in D0)*; oracle is the re-cut validate. found_bugs
  cell tests (`:84-151`, `:158-301`) keep scenarios; upsert semantics removes their skeleton
  dependence.

### Commit D2 — cleanup sweep — **DONE** (one commit)

> **DONE (July 18 2026), one commit** (no split: ~5 files, all edits mechanical and
> mutually independent, no transitional state, no byte/behaviour change). Delivered:
> - Deleted the **10 dead `ColloscopeError` variants** the sparse swap orphaned
>   (`InvalidPeriodId`, the four `Wrong…Count…` variants,
>   `InterrogationOnNonInterrogationWeek`/`MissingInterrogationOnInterrogationWeek`,
>   `InvalidWeekNumberInPeriod`, `NoInterrogationOnWeek`, `MissingNonPrefilledGroupList`).
>   Their only consumer, `ops/src/colloscope.rs`, matches on live variants with `_ =>`
>   wildcards, so deletion touched no consumer code.
> - Re-cut the last positional payload: `InvalidGroupNumInInterrogation(PeriodId, SlotId,
>   usize)` → `(SlotId, WeekId)` (row vocabulary); dropped the now-unused `PeriodId`
>   import.
> - Deleted `Colloscope::new_empty_from_params` (a `Self::default()` shim); its callers
>   (decode, convert, 3 surface tests) now use `Colloscope::default()`.
> - Scrubbed the now-false doc comments (the `&Periods`-transitional surface block; the
>   decode "colloscope skeleton builder"/"interrogation skeleton" comments — the early
>   `check_invariants` call is kept, error ordering is behaviour, and reworded to its true
>   trust-boundary reason).

Dead helpers, dead error variants (`WrongPeriodCountInColloscopeData`,
`WrongSlotCountInPeriodInColloscopeData`, `WrongInterrogationCountForSlot…`,
`InterrogationOnNonInterrogationWeek`/`Missing…` — superseded by the new row vocabulary),
and doc comments. ~~The **python glue dense view** (decision 6): a computed
`Colloscope`-shaped pyclass built from params + rows + `is_week_active`,
`python/src/glue/colloscopes.rs`, keeping `period_map`/`slot_map`/`interrogations` pyclass
shapes byte-for-byte so `custom_export_xlsx.py` runs unchanged~~ *(landed in D0 as
`Colloscope::from_mem`)*.

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

- ~~B1 width / transitional maintenance~~ *(resolved)*: B1 landed as six contained
  commits (see Phase 4); the throwaway splice code sits in `apply_week` and is deleted
  by B2 (patterns) / 1d (cells).
- **Decode `WeekId` synthesis** must allocate above the file's max id before the
  `IdIssuer` scan — same pattern as every synthesized id today; the duplicate-id checker
  covers mistakes.
- **Error-variant churn** (new `InvariantError`/`ColloscopeError` variants, deleted count
  variants) touches gtk4/python error display strings — sweep `match`es in the same
  commits; the big vocabulary collapse still belongs to step 5, don't gold-plate.
