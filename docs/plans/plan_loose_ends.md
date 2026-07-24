# Loose ends before step 5: periods/weeks split + group-list consolidation

> **First implementation step**: copy this plan to `docs/plans/plan_loose_ends.md` (durable
> session plans live there) and commit it. Update its checkboxes as commits land.

## Context

Step 4 of `docs/plans/invariant_cascade_design.md` is closed. Before step 5 (rewiring
production onto `force_apply` + the new checker), two op-surface warts are removed:

1. **`PeriodStillHasWeeks`** — in the new framework, removing a period with weeks should
   just leave dangling `Week::period_id` FKs for the cascade, like every other removal.
   It can't today because the week *ordering* is stored as the `OrderedTable` row value
   (`ordered_period_list: OrderedTable<PeriodId, Vec<WeekId>>`): removing the period row
   destroys the ordering, so the broken state is unrepresentable. Fix: adopt the
   Slots/Subjects model wholesale — **Weeks become their own module** (twin of `slots.rs`),
   with `week_map: Table<WeekId, Week>` + a sparse `ordering: Table<PeriodId, Vec<WeekId>>`
   sidecar that is Weeks-internal data ("ordering within the *would-be* period"); `Periods`
   shrinks to `first_week` + a public `OrderedTable<PeriodId, ()>` (pure existence + order,
   mirroring `Subjects.ordered_subject_list`). No new registry edge: the ordering-row key
   has double duty with the per-week `WeekPeriodFk` (same argument that removed
   `SlotsOrderingKey`).
2. **`RemainingFilling` / `PrefillGroupCountMismatch` / `NonEmptyGroupsWhenReducing`** —
   artifacts of splitting one value across `Update(params)` / `SetFilling(filling)`. Fix:
   the elementary op payload becomes the whole **`GroupList`**, sealed (private fields,
   validating constructor à la `NonEmptyRangeInclusive`), making the count-mismatch and
   duplicate-student states unrepresentable (this is C.3's smart-constructor churn, pulled
   forward from step 5). The high-level `ops/` API (GUI actions) is **frozen** — its
   `SetFilling` op survives, translated to a low-level `Update`.

Design doc anchors: D.3 parks the empty-first trio for "step 5 decides"; we decide now.
Both checkers stay wired only in tests; the differential fuzz is the gate for every commit.

**Standing rules honored throughout**: `force_apply` fixes nothing / no consistency
prechecks added; parallel API variants stay uniform independent copies; F2 — any touch of
`WeekMove` code re-verifies its inline guards against `SlotNotRunningOnPeriod` /
`InvalidGroupNumInInterrogation`; no `mod.rs` (new file is `weeks.rs`); no `serde(default)`;
Edit tool only, no sed.

---

## Part P — Periods/weeks (3 commits)

### Commit P1 — field reshape inside `periods.rs` (no behavior change) — DONE (5b416763)

`state-colloscopes/src/periods.rs`:

```rust
pub struct Periods {
    pub first_week: Option<collomatique_time::WeekStart>,
    /// Ordered list of periods — existence and display order only. A period
    /// owns nothing else; week data and per-period week ordering live in the
    /// weeks sidecar (module split lands in commit P3).
    pub ordered_period_list: OrderedTable<PeriodId, ()>,   // public, mirrors Subjects

    // still here until P3, but already the final shape:
    week_map: Table<WeekId, Week>,
    /// Per-period ordered week ids. Sparse: a row exists exactly when the
    /// period has ≥ 1 week (canonical absent, like the slots ordering).
    ordering: Table<PeriodId, Vec<WeekId>>,
}
```

- `ids.rs`: `PeriodId` becomes `#[entity(())]`; `colloscope_params.rs:181-186`
  `impl Lookup<PeriodId>` → `type Entity = ()`, `self.periods.ordered_period_list.get(&id)`.
- **Read-surface re-cut** (slots naming; keep old names where semantics are unchanged):
  - unchanged: `period_ids`, `period_count`, `is_empty`, `period_id_at`,
    `find_period_position`, `walk`, `week_ids`, `find_week`, `week_position`, `week_id_at`,
    `count_weeks`, `global_week_position`, `find_period_position_and_first_week`,
    `find_period_position_and_total_number_of_weeks`, `get_first_week_and_length_for_period`
    — internals now read `ordering` (`.get(&id)` → absent row = empty: use
    `.map(Vec::as_slice).unwrap_or(&[])` where the period is known to exist).
  - `weeks_of` / `weeks_vec_of` / `week_count_of`: keep `Option` = "period id invalid"
    semantics by gating on `ordered_period_list.contains(&id)` first, then reading the
    sidecar with empty-default. (Consumers keep compiling; the slots-style
    `None = no row` semantics arrive with the P3 rename.)
  - **delete** `find_period(id) -> Option<&Vec<WeekId>>` (no borrowable `Vec` for a
    week-empty period). Its callers go through `weeks_of`/`contains`. Re-cut
    `tests/read_api.rs`: the pointer-identity pin moves to
    `resolve::<WeekId> == find_week`; period resolution pins `lookup(id) == Some(&())`.
  - `from_period_rows`: same signature; builds the three structures, **dropping empty
    ordering rows** (sparse canonical form), mirroring `Slots::from_subject_rows`
    (`slots.rs:85-105`). Storage decode/encode untouched in P1.
  - add `#[cfg(test)] forge_ordering_row(&mut self, PeriodId, Vec<WeekId>)` (twin of
    `slots.rs:112-115`).
- **Compound mutators** (`insert_week_at`, `remove_week_entry`, `move_week_entry`)
  maintain the sparse form exactly like `insert_slot_at`/`remove_slot`
  (`slots.rs:261-294`): create the row on demand at first week, drop it when the last
  week leaves. `apply_period` Add arms insert `()` instead of `Vec::new()`; Remove arm's
  `remove_at` unchanged (a week-empty period has no sidecar row to clean — guard still
  ensures week-empty here).
- **Old checker** — `colloscope_params.rs:1037-1061` `check_periods_data_consistency`
  re-cut on the model of `check_slots_data_consistency` (`:544-603`):
  ```rust
  let period_ids: BTreeSet<PeriodId> = self.periods.period_ids().collect();
  let mut ordered_ids = BTreeSet::new();
  for (period_id, order) in self.periods.ordering_entries() {
      if !period_ids.contains(&period_id) {
          return Err(InvariantError::InvalidWeek);   // newly representable dangle (fires from P2 on)
      }
      if order.is_empty() {
          return Err(InvariantError::EmptyWeeksRow); // new variant, canonical-absent
      }
      // per-id: exists in week_map, week.period_id == period_id, no duplicate → InvalidWeek
  }
  // no orphan weeks: every week_map entry covered → InvalidWeek
  ```
  Add `InvariantError::EmptyWeeksRow`.
- **New checker** (`invariants.rs`):
  - `LogicError::EmptyWeeksRow(PeriodId)` (declare after `EmptySlotsRow`), sweep over
    `periods.ordering_entries()` next to the `EmptySlotsRow` sweep (`:255-259`);
    `to_legacy` → `InvariantError::EmptyWeeksRow`; add to `is_necessarily_logic_error`
    (`:751-756`); corruption fixture via `forge_ordering_row`.
  - `dangling_refs` (`:311-312`): build the weeks existence set from the week *table*
    (`week_entries()` keys), not `week_ids()` (which walks the ordering) — honors the
    stated "entities' own tables" principle once rows can dangle.
- Tests: existing suites + differential fuzz (100 seeds) — must be green with zero
  behavioral diff (guard still in place).

### Commit P2 — drop `PeriodStillHasWeeks` from the force path (behavioral) — DONE (af543578)

- `periods.rs`: delete `PeriodPrecheckError::PeriodStillHasWeeks` (`:517-519`) and the
  guard in `force_apply_period` Remove (`:879-889`). **Checked `apply_period` keeps its
  guard** (`:660-668`) — it retires wholesale at step 5; stripping it now would only turn
  the case into a `check_invariants` panic. The asymmetry is exactly the 4.2 fuzz
  carve-out: a checked-rejected `ForceValid` probe must land **broken** — and it does
  (dangling `WeekPeriodFk` per week; ordering row + `week_map` rows untouched, since
  `force_apply` fixes nothing and Remove's mutation is just `remove_at(position)`).
- Doc-comment updates: `invariants.rs:307-309` and `dangling_to_legacy`'s
  `WeekPeriodFk => InvalidWeek` arm (`:617-619`) lose their "never fires" claim (mapping
  itself is already correct: old checker fires `InvalidWeek` from the re-cut consistency
  check, which runs first in `check_invariants`); `refs.rs:61-67` note;
  `PeriodOp::Remove` doc (`ops.rs:87-90`); `PeriodPrecheckError` rustdoc.
- Add an invariants fixture: force-remove a period with weeks (a *real op* now reaches
  the state — no forgery needed) → new checker `Ok({DanglingFk(Period@WeekPeriodFk)…})`,
  old checker `Err(InvalidWeek)`, `assert_differential` passes.
- `testgen-colloscopes/generator.rs`: fix the stale comment in `gen_period`
  (`:343-346` — checked apply still bounces; force no longer does). Verify the
  `ForceRemove` corruption arm (`gen_corruption_op`, `:1110-1186`) can select a
  week-non-empty period; if it can't, extend it so the new broken landing is exercised.
  Corruption commentary at `:1207` updated.
- Gate: differential fuzz 100 seeds green; check the honesty guards still hold
  (≥25% broken landings, every kind lands broken once).

### Commit P3 — module split: `periods.rs` + new `weeks.rs` (mechanical) — DONE (split into 8 green commits P3.1–P3.8, e66f62fe..286c242f)

New `state-colloscopes/src/weeks.rs`, twin of `slots.rs`:

```rust
pub struct Weeks {
    week_map: Table<WeekId, Week>,
    ordering: Table<PeriodId, Vec<WeekId>>,   // sparse, Weeks-internal grouping data
}
```

- Move from `periods.rs`: `Week`, `WeekDesc`, `WeekError`, `WeekPrecheckError`,
  `apply_week` / `force_apply_week` (+ helpers), the compound mutators, the week read
  surface, `forge_ordering_row`, `from_period_rows` (renamed
  `Weeks::from_period_rows(entries) -> Result<Self, DuplicatedWeekIdError>`, dropping
  empty rows). `periods.rs` keeps `Periods { pub first_week, pub ordered_period_list }`,
  `PeriodError`/`PeriodPrecheckError`, `apply_period`/`force_apply_period`, and gains
  `Periods::from_ordered_ids(first_week, Vec<PeriodId>) -> Result<Self, DuplicatedPeriodIdError>`.
- `Parameters` (`colloscope_params.rs:25-39`) gains `pub weeks: weeks::Weeks` (order the
  field right after `periods`).
- Method homes (slots naming; single-container methods on `Weeks`, cross-container
  composites on `Weeks` taking `&Periods` — the `WeekPatterns::is_week_active(&Periods,…)`
  precedent — plus `Parameters` delegations for hot ones):
  | old (`Periods::…`) | new |
  |---|---|
  | `find_week`, `week_position`, `week_id_at` | `Weeks::` same names |
  | `weeks_of(id)` | `Weeks::weeks_for_period(id) -> Option<impl Iterator<Item=(&WeekId,&Week)>>` — `None` = **no row** (slots semantics); existence via `periods.ordered_period_list.contains` |
  | `weeks_vec_of` | `Weeks::weeks_desc_vec_for_period -> Option<Vec<WeekDesc>>` |
  | `week_count_of` | `Weeks::week_count_for_period -> Option<usize>` (`None` = no row; callers wanting bounds use `.unwrap_or(0)` after an existence check) |
  | `walk`, `week_ids`, `count_weeks`, `global_week_position`, `find_period_position_and_first_week`, `find_period_position_and_total_number_of_weeks`, `get_first_week_and_length_for_period` | `Weeks::…(&self, periods: &Periods)`; `walk` iterates `periods.ordered_period_list.keys()` × sidecar rows (a dangling row is simply not walked). `Parameters::{walk_weeks, count_weeks, week_ids}` delegations |
  | `week_entries`, `ordering_entries` | `Weeks::` pub(crate), plus pub(crate) `week_ids_table_order()` for the checker existence set |
  | — | `Weeks::periods_with_weeks()`, `Weeks::is_empty()` (twin `subjects_with_slots`/`is_empty`) |
- `WeekPatterns::is_week_active(periods: &Periods, …)` (`week_patterns.rs:52-67`) →
  takes `weeks: &Weeks`; `Parameters::is_week_active` / `is_interrogation_possible`
  delegate through `self.weeks`. `Lookup<WeekId>` reads `self.weeks.find_week`.
- In-crate call-site re-home: `params.periods.week_position(…)` → `params.weeks.…` etc.
  (slots.rs:513, colloscope guards, refs.rs `walk_weeks` `:307-314`, checker sweeps,
  `check_weeks_data_consistency` (renamed), annotate in `ops.rs`). `WeekOp`/
  `AnnotatedWeekOp` payload shapes unchanged.
- **F2 checkpoint**: after moving `move_week`/`force_move_week`, re-verify the inline
  destination checks still pair with `SlotNotRunningOnPeriod` /
  `InvalidGroupNumInInterrogation` (D.4-F2).
- Consumer churn (mechanical re-homing; wire format untouched):
  - `storage/src/decode/spec2.rs:338-371`: build both containers
    (`Periods::from_ordered_ids` + `Weeks::from_period_rows`);
    `encode/spec2.rs:151-165`: `weeks:` from `params.weeks.weeks_for_period(pid)` with
    `None` → empty vec. `populated_round_trip` must stay byte-identical.
  - `constraints-colloscopes` (`convert.rs:92`, `helpers.rs:58,67`, `extras.rs:208`,
    `misc/limits.rs:13`, …): `walk()`/`count_weeks()` calls gain the second container or
    move to the `Parameters` delegations; the `WeekId ↔ GlobalWeek` map at model entry
    is the natural place.
  - `gtk4` (`editor/general_planning.rs`, `week_patterns/dialog.rs`,
    `colloscope/colloscope_display.rs`, …), `ops/src/general_planning.rs:424`,
    `ops/src/group_lists.rs:734`, `python/src/glue/params.rs`, `xlsx/src/colloscope_sheet.rs`,
    `testgen` (`generator.rs:101-102,150,1043`): same mechanical mapping per the table.
- Gate: full workspace suites + differential fuzz 100 seeds; `populated_round_trip`.

---

## Part G — Group lists (2 commits)

### Commit G1 — seal `GroupList` (validated constructor; ops unchanged)

`state-colloscopes/src/group_lists.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References)]
#[serde(try_from = "RawGroupList", into = "RawGroupList")]
pub struct GroupList {
    params: GroupListParameters,          // private
    #[fk]
    filling: GroupListFilling,            // private
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GroupListBuildError {
    #[error("prefilled group count {actual} does not match the group name count {expected}")]
    PrefillGroupCountMismatch { expected: usize, actual: usize },
    #[error("student {0:?} appears in two prefilled groups")]
    DuplicatedStudentInPrefilledGroups(StudentId),
}

impl GroupList {
    /// Checks the value-internal facts only (state-dependent facts — student
    /// existence — stay with the checker/walker as dangling FKs).
    pub fn new(params: GroupListParameters, filling: GroupListFilling)
        -> Result<Self, GroupListBuildError> { … }
    pub fn params(&self) -> &GroupListParameters { … }
    pub fn filling(&self) -> &GroupListFilling { … }
    pub fn is_prefilled(&self) -> bool { … }        // existing helper, now on accessors
    pub fn into_parts(self) -> (GroupListParameters, GroupListFilling) { … }
}
// Default: manual impl (default params + Automatic{} — always consistent).
// RawGroupList: private { params, filling } serde mirror; TryFrom calls new()
// (honest-decode, the NonEmptyRangeInclusive precedent, non_empty_range.rs:16-47).
```

`GroupListParameters`, `GroupListFilling`, `PrefilledGroup` stay public-field DTOs (the
invariant is *cross-field* on the pair; gtk4 dialog literals stay legal).

- In-crate: `apply_group_list`/`force_apply_group_list` construct via `new()`
  (same-module code may keep maintaining the pair in the Update reshape, but prefer
  `new(...).expect(...)` where the count is maintained by construction). `SetFilling`'s
  count guard (`:605-611`) now delegates: `GroupList::new(old.params().clone(),
  filling.clone())` mapping `BuildError::PrefillGroupCountMismatch` →
  `GroupListError::PrefillGroupCountMismatch` (variant survives until G2).
- **Delete as unrepresentable** (C.3 fulfilled):
  - `LogicError::{PrefillGroupCountMismatch, DuplicatedStudentInPrefilledGroups}`
    (`invariants.rs:100-105`), their sweep (`:275-284`), `to_legacy` arm (`:585-588`),
    fixtures.
  - old-checker `validate_group_list_filling_internal` (`colloscope_params.rs:657-690`):
    drop the count + duplicate-student checks, keep student-existence;
    `GroupListError::DuplicatedStudentInPrefilledGroups` deleted.
  - testgen: `LogicRecipe::{GlobalPrefillMismatch, SetFillingDupStudent}`
    (`generator.rs:1537-1633`) deleted — `ForceLogic` keeps `GlobalDup` /
    `PairingSameSubject` / `SlotPairingSameSlot`, so the honesty guards stand; the
    direct `group_list.filling = …` write (`:1602`) disappears with the recipe.
- Mechanical read-site churn (`.params` → `.params()`, `.filling` → `.filling()`) across
  constraints-colloscopes, gtk4, ops, xlsx, python glue, storage encode, testgen —
  representative sites: `constraints-colloscopes/src/vars.rs:52,103`,
  `gtk4/src/editor/group_lists/prefill_dialog.rs:317-322`, `ops/src/group_lists.rs:42-245`,
  `xlsx/src/per_student_groups_sheet.rs:66,85`, `storage/src/encode/spec2.rs:400-424`,
  `testgen generator.rs:708-710,764`. Grep `\.params\b` / `\.filling\b` on `GroupList`
  bindings for the full list.
- The one external constructor: `storage/src/decode/spec2.rs:646-701` →
  `GroupList::new(params, filling)` with `GroupListBuildError` mapped to a hard decode
  error (honest-decode; a bad file was previously rejected later "by layer 3" — the
  rejection just moves earlier).
- Gate: workspace suites + differential fuzz 100 seeds.

### Commit G2 — consolidate the elementary op surface (drop `SetFilling`)

`ops.rs`:

```rust
pub enum GroupListOp {
    Add(group_lists::GroupList),
    Remove(GroupListId),
    Update(GroupListId, group_lists::GroupList),
    AssignToSubject(PeriodId, SubjectId, Option<GroupListId>),
}
pub enum AnnotatedGroupListOp {
    Add(GroupListId, group_lists::GroupList),      // undo-of-Remove carries everything naturally
    Remove(GroupListId),
    Update(GroupListId, group_lists::GroupList),   // reverse = Update(id, old value)
    AssignToSubject(PeriodId, SubjectId, Option<GroupListId>),
}
```

`annotate` (`ops.rs:1038-1073`): `Add(gl)` → issue id, `Add(id, gl)`; the "filling only
carries information when reversing a Remove" note dies.

- `apply_group_list` re-cut:
  - **Add**(id, gl): no-clobber + `validate_group_list` (student existence) + insert.
    Reverse `Remove(id)`.
  - **Remove**(id): target existence; colloscope placement-row scan
    (`NotEmptyGroupListInColloscope`, now for both filling kinds — a row is invalid for
    prefilled lists anyway); association scan (`RemainingAssociatedSubjects`).
    **`RemainingFilling` deleted** — the row (incl. filling) goes atomically. Reverse
    `Add(id, old)`.
  - **Update**(id, new): target existence; `validate_group_list(new)`; then the merged
    colloscope guard set, driven by `(old.is_prefilled(), new.is_prefilled())` — the
    union of today's Update (`:495-531`) and SetFilling (`:613-654`) guards:
    ```text
    auto → prefilled : colloscope row must be absent (NonEmptyColloscopeGroupListWhenPrefilling)
    prefilled → auto : nothing (absent row = empty placements)
    auto → auto      : validate_group_list_placements(id, placements, new.params(), new.filling(), students)
                       → NotCompatibleGroupListInColloscope   [now against the complete new pair]
    any              : check_interrogations_group_bound for every associated (period,subject)
                       vs new.params().group_names.len()      [from today's Update arm]
    ```
    **`NonEmptyGroupsWhenReducing` and the truncate/extend reshaping deleted** — the op
    carries a complete consistent value. Reverse `Update(id, old)`.
  - **AssignToSubject**: unchanged.
- `force_apply_group_list` re-cut as the uniform thin copy (strip/keep rule): Add keeps
  no-clobber; Remove/Update keep target existence only; AssignToSubject keeps its
  existing coordinate-existence trio. `GroupListPrecheckError` shrinks to
  `{InvalidGroupListId, GroupListIdAlreadyExists, InvalidSubjectId, InvalidPeriodId}`;
  `GroupListError` drops `RemainingFilling`, `NonEmptyGroupsWhenReducing`,
  `PrefillGroupCountMismatch`.
- **`ops/` crate — API frozen, behavior preserved** (`ops/src/group_lists.rs`):
  - `GroupListsUpdateOp` (`:262-279`) and all high-level error enums unchanged; the
    cleaning-op machinery (`:440-580`, which pre-empties fillings/associations) stays
    as-is in this commit (zero visible change in history granularity; simplification is
    a possible follow-up).
  - Translators (`apply_no_cleaning`, `:821-1000`):
    `AddNewGroupList` → `GroupListOp::Add(GroupList::new(params, Filling::default())
    .expect("automatic filling is always consistent"))`.
    `UpdateGroupList` → fetch old; replicate the truncate/extend reshaping here
    (grow pads `PrefilledGroup::default()`, shrink truncates — the cleaning phase has
    already emptied dropped groups; `assert!` they are empty, preserving today's
    should-never-fire backstop) → `GroupListOp::Update(id, GroupList::new(new_params,
    reshaped).expect("count maintained by construction"))`.
    `SetFilling` → keep the student-id validation; then `GroupListOp::Update(id,
    GroupList::new(old.params().clone(), filling.clone()).expect("caller guarantees
    prefill arity"))` (same panic contract as today's `.expect` on the low-level call).
    `DeleteGroupList` → unchanged except the `RemainingFilling` panic arm disappears
    with the variant.
  - python glue (`glue.rs:1108-1180`) and gtk4 drive the high-level ops — no change
    beyond G1's accessor churn.
- Other issuers of low-level ops re-cut: `ops/tests/found_bugs.rs:132-139`,
  `ops/tests/general_planning_content.rs:133-139`,
  `storage/tests/populated_round_trip/builder.rs:507-546,908-915`.
- testgen `gen_group_list` (`generator.rs:696-820`):
  - invalid arm case 0 (over-count `SetFilling`) is unrepresentable → drop, redistribute
    over the remaining cases.
  - valid arms: `Add` → `GroupList::new(synth params, default).unwrap()`; `Update` →
    build a full `GroupList` (keep count-stable-for-prefilled logic, or rebuild filling
    to match a new count); the `SetFilling` arm becomes an `Update` that keeps params
    and swaps filling via `new(…).unwrap()`. Grep `GroupListOp::` in `generator.rs` for
    any corruption arm still naming `SetFilling` (e.g. `:1369`).
- Gate: workspace suites + differential fuzz 100 seeds + honesty guards.

---

## Closing commit — design-doc amendment

Update `docs/plans/invariant_cascade_design.md`: B.1 (periods/weeks shapes: `Periods`
existence-only + `Weeks` module; sealed `GroupList`), B.2 (`PeriodOp::Remove` no longer
week-empty-gated in force; `GroupListOp` consolidated shape), B.3 (read surface re-home),
C.3 (smart-constructor churn done early for `GroupList`), D.3 (empty-first trio resolved:
all three deleted; `PrefillGroupCountMismatch` unrepresentable), E.3 (precheck enums
shrunk), D.4-F1 note (unchanged — checked apply keeps both guards until step 5), and
retire `plan_loose_ends.md` with a pin. Update auto-memory afterwards.

## Verification

Per commit: `cargo test --workspace` + the differential fuzz
(`state-colloscopes/tests/differential_force_apply.rs`, committed 100-seed config);
P3 additionally pins `populated_round_trip` byte-identity. Final gate (user-run): full
suite + `scripts/smoke`, and a gtk4 app pass over period/week editing and the group-list
dialogs (params, prefill, delete with filling, undo/redo of each).
