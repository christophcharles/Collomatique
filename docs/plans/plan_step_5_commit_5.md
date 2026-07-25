# Step 5, commit 5 split — the state-colloscopes test migration (sub-plan)

**Status:** open (July 25 2026). This document replaces the single "commit 5" of `docs/plans/plan_step_5.md` §9 with a sequence of **four test-only commits** (5.1–5.4). File/line references are against the tree at `aaceda78` (step-5 commit 4 landed). Read §0 first: it records a scope ruling that *narrows* what §9 originally asked for.

---

## 0. Scope ruling — the old invariant checks are untouched

**Ruling (July 25 2026): commit 5 does not touch the old invariant checks in any way.** The only thing these four commits do is switch tests that drive ops through the *public surface* from the old checked `apply` onto `try_apply`, and upgrade their assert tails to the new error vocabulary. Any code whose *purpose* is comparing the old and new worlds keeps both, byte-untouched:

- `state-colloscopes/src/invariants.rs` — the fixture wrapper `broken_invariants(&data)` (:944-954) **keeps** its `assert_differential(data)` call; `assert_dangling_maps` **keeps** its old-checker `InnerDataError` argument and assert; the explicit `check_invariants` asserts in compound fixtures and the two legacy-bridge unit tests **stay**.
- `state-colloscopes/src/colloscopes.rs` — the three stage-6 corruption tests (:571-610) stay on their old asserts and `assert_differential` calls.
- `state-colloscopes/src/lib.rs` — the force_apply_tests module (:538-731) keeps its old-checker asserts, its `assert_differential` calls, and the two `forced_valid_*_equals_checked_apply` anti-drift pins comparing against checked `apply`.
- `state-colloscopes/tests/differential_force_apply.rs` — byte-untouched. It dies in R1, not before; until then it is live fuzz coverage.
- `state-colloscopes/tests/canary_try_apply.rs` — byte-untouched. It *must* keep both APIs (that is its job) and dies in R1.
- `state-colloscopes/tests/property_ops.rs` and `tests/property_ops_broken_invariants.rs` — untouched; their oracle swap is R1's job (parent plan §11).

Everything §9 originally scheduled for those `src/` files is **deferred to a new commit R1.5** (between R1 and R2 — see §6 below and parent plan §11).

This gives the sequence an auditable structural property: **all four commits touch only files under `state-colloscopes/tests/`, and no existing test file other than the three named below.** If a `git diff --stat` of any of these commits shows a `src/` path, something went wrong — stop and re-read this section.

---

## 1. The shared mechanical pattern

All three migrated files follow the same two-part recipe. Read this once; the per-commit sections then only spell out the parts that are *not* mechanical.

**Part one — the apply swap.** Every call through the `Manager` trait, `app.apply(...)` / `app_state.apply(...)`, becomes `app.try_apply(...)` / `app_state.try_apply(...)`. Every raw call on `Data`, `data.apply(&annotated)` / `data.apply(&rev)`, becomes `data.try_apply(...)`. Nothing else about those call sites changes: the scaffold shape

```rust
let Ok(Some(NewId::StudentId(student_id))) = app_state.try_apply(
    Op::Student(StudentOp::Add(Student::default())),
    "Add student".into(),
) else {
    panic!("Unexpected result after adding the student");
};
```

is the old shape with one method name substituted. `Manager::try_apply` has the same signature as `Manager::apply` apart from the error type, and every scaffold call succeeds, so the `let Ok(..) = .. else { panic! }` pattern needs no other edit. There is **no** call site in any of the three files that stays on the old API — the done-check per file is that `grep -F ".apply(" <file>` returns nothing after the edit (the string `.apply(` does not occur inside `.try_apply(`).

**Part two — the assert rewrite.** Each file has a handful of tests asserting a *rejection*. Under the old API these matched a per-domain `Error` variant, often through a `matches!` with wildcards. Under the new API the same rejections arrive as either:

- `ApplyError::Precheck(PrecheckError::Domain(...))` for the **kept carve-out guards** — a direct variant-for-variant rewrite, or
- `ApplyError::Invariants(set)` for the **stripped guards** — the op landed via `force_apply`, `broken_invariants` reported the breakage, and the gate rolled back.

For the `Invariants` arm we assert the **exact expected set** with `assert_eq!`, not a `matches!`. This is deliberate and is a strict upgrade: the pre-op state is valid in every one of these tests (it was built through the same gate), so every entry in the set was caused by the op at hand, the expected set is fully computable in advance, and pinning it exactly also pins that *nothing else* broke. Where the old variant carried different payload than the new set entry does (e.g. the old `(subject, period, slot)` triple vs. the new `(slot, week)` pair), the per-commit section explains the payload change.

**Imports.** Each file's `use collomatique_state_colloscopes::{...}` block drops the dead old names (`Error`, per-domain `*Error` enums) and gains the new vocabulary (`ApplyError`, `FixableInvariant`, `Convergence`, `Reference`, the needed `*RefSite` and `*PrecheckError` types). All of these are re-exported at the crate root (lib.rs:69, :85-100, :102-105), so the import blocks below are known-good. `BTreeSet` is already imported in all three files.

**Commit hygiene.** One commit per file, message in the series style (`step-5 commit 5.1: ...`). After each commit, run that file's tests: `cargo test -p collomatique-state-colloscopes --test <file-stem>`. These commits touch no `src/` file, so nothing else in the workspace can move; running the single test binary is the complete check.

---

## 2. Commit 5.1 — `tests/period_consistency_in_subjects.rs`

The smallest file (one rejection assert); it establishes the template.

### 2.1 Imports

Old (lines 1-8):

```rust
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, NewId, NonEmptyRangeInclusive, Op, PeriodOp, Subject, SubjectOp, SubjectParameters,
    SubjectPeriodicity, WeekOp,
    ids::{PeriodId, WeekId},
    subjects::{SubjectInterrogationParameters, WeekBlock},
    weeks::WeekDesc,
};
```

New:

```rust
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ApplyError, Data, FixableInvariant, NewId, NonEmptyRangeInclusive, Op, PeriodOp, PeriodRefSite,
    Reference, Subject, SubjectOp, SubjectParameters, SubjectPeriodicity, WeekOp,
    ids::{PeriodId, WeekId},
    subjects::{SubjectInterrogationParameters, WeekBlock},
    weeks::WeekDesc,
};
```

(Added: `ApplyError`, `FixableInvariant`, `PeriodRefSite`, `Reference`. Nothing removed — this file referenced the old `Error`/`PeriodError` through fully-qualified paths at the assert site, which disappears below.)

### 2.2 The apply swap

Every `.apply(` in the file becomes `.try_apply(`: the two calls in the `add_active_period` helper, and all scaffold calls in the three tests (`add_subject_referencing_period_then_remove_period`, `..._and_then_undo`, `add_subject_referencing_week_then_shrink_week_count_but_keep_said_week`). All keep their `let Ok(..) = .. else` shape.

### 2.3 The one assert rewrite (:79-88)

The test removes a week-empty period that a subject's `excluded_periods` still references. Old code:

```rust
    // Remove second period
    let Err(collomatique_state_colloscopes::Error::Period(period_err)) = app_state.apply(
        Op::Period(PeriodOp::Remove(id2)),
        "Remove unused period".into(),
    ) else {
        panic!("Unexpected result after removing unused period");
    };

    assert_eq!(
        period_err,
        collomatique_state_colloscopes::PeriodError::PeriodIsReferencedBySubject(id2, subject_id)
    );
```

New code:

```rust
    // Remove second period
    let result = app_state.try_apply(
        Op::Period(PeriodOp::Remove(id2)),
        "Remove unused period".into(),
    );

    assert_eq!(
        result,
        Err(ApplyError::Invariants(BTreeSet::from([
            FixableInvariant::DanglingFk(Reference::Period {
                target: id2,
                site: PeriodRefSite::SubjectExcludedPeriods(subject_id),
            })
        ]))),
    );
```

Why this is the expected outcome: `PeriodIsReferencedBySubject` was a *stripped* guard (checked apply scanned the subject list before removing a period). Under the gate, the removal lands, the subject's `excluded_periods` entry for `id2` now dangles, and the checker reports exactly one broken invariant: the period-reference from site `SubjectExcludedPeriods(subject_id)` to target `id2`. The period is week-empty and nothing else references it (the test is built that way precisely so only this reference blocks removal), so the set has exactly this one entry. The old assert pinned one variant; the new one pins the entire failure surface.

Note the shape change: the old code destructured the error with a `let Err(...) = ... else`; the new code binds `result` and `assert_eq!`s the whole `Result` — the `Ok` side (`Option<NewId>`) is `PartialEq + Debug`, so this compiles and gives a better failure message.

**Commit message:** `step-5 commit 5.1: migrate period_consistency tests onto try_apply`

---

## 3. Commit 5.2 — `tests/week_ops.rs`

Three rejection asserts, one of which needs a scaffold change (capturing a previously-discarded id), and one raw-`Data` round-trip test.

### 3.1 Imports

Old (lines 13-18, the root-level list):

```rust
use collomatique_state::{AppState, InMemoryData, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, Error, GroupListOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, SlotOp,
    Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity,
    TeacherOp, WeekError, WeekOp, WeekPatternOp,
```

New:

```rust
use collomatique_state::{AppState, InMemoryData, traits::Manager};
use collomatique_state_colloscopes::{
    ApplyError, ColloscopeOp, Convergence, Data, FixableInvariant, GroupListOp, NewId,
    NonEmptyRangeInclusive, Op, PeriodOp, Reference, SlotOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekOp, WeekPatternOp, WeekRefSite,
```

(Removed: `Error`, `WeekError`. Added: `ApplyError`, `Convergence`, `FixableInvariant`, `Reference`, `WeekRefSite`. The sub-module imports that follow — `group_lists::{...}`, `ids::{...}`, `slots::Slot`, etc. — are unchanged.)

### 3.2 The apply swap

Every `.apply(` becomes `.try_apply(`: the two calls in the `add_period` helper, all scaffold calls in the four scenario tests, and the two raw-`Data` calls in `remove_week_then_undo_restores_identity`:

```rust
    let (annotated, _) = data.annotate(Op::Week(WeekOp::Remove(middle)));
    let rev = data
        .try_apply(&annotated)                                  // was .apply(&annotated)
        .expect("removing the week should succeed");
    data.try_apply(&rev)                                        // was .apply(&rev)
        .expect("the reverse of a successful op must apply");
```

(`InMemoryData` is already imported at the top of the file, so the raw trait calls resolve.)

### 3.3 Assert rewrite 1 — `remove_week_blocked_by_non_trivial_pattern` (:142-171)

The scaffold currently **discards the pattern id**, and the new exact-set assert needs it. Old scaffold:

```rust
    // A pattern that skips the middle week.
    let Ok(Some(NewId::WeekPatternId(_))) = app.apply(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
```

New scaffold — capture the id:

```rust
    // A pattern that skips the middle week.
    let Ok(Some(NewId::WeekPatternId(pattern_id))) = app.try_apply(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
```

Old assert:

```rust
    let result = app.apply(
        Op::Week(WeekOp::Remove(weeks[1])),
        "Remove middle week".into(),
    );
    assert!(
        matches!(
            result,
            Err(Error::Week(WeekError::NonTrivialWeekPattern(w, _))) if w == weeks[1]
        ),
        "removing a week a pattern skips must fail, got {result:?}",
    );
```

New assert:

```rust
    let result = app.try_apply(
        Op::Week(WeekOp::Remove(weeks[1])),
        "Remove middle week".into(),
    );
    assert_eq!(
        result,
        Err(ApplyError::Invariants(BTreeSet::from([
            FixableInvariant::DanglingFk(Reference::Week {
                target: weeks[1],
                site: WeekRefSite::WeekPatternExcludedWeek(pattern_id),
            })
        ]))),
        "removing a week a pattern skips must fail, got {result:?}",
    );
```

`NonTrivialWeekPattern` was a stripped guard. The forced removal lands, and the pattern's `excluded_weeks` entry for `weeks[1]` dangles — one `DanglingFk` from site `WeekPatternExcludedWeek(pattern_id)` to target `weeks[1]`. The old `matches!` wildcarded the pattern id; the new assert pins it, which is why the scaffold must start capturing it.

### 3.4 Assert rewrite 2 — `update_week_to_inactive_blocked_by_filled_cell` (:295-312)

Old:

```rust
    // Turning the week inactive would silence the non-empty cell.
    let result = app.apply(
        Op::Week(WeekOp::Update(weeks[0], WeekDesc::new(false))),
        "Deactivate week".into(),
    );
    assert!(
        matches!(
            result,
            Err(Error::Week(WeekError::NotCompatibleSlotInColloscope(w, s)))
                if w == weeks[0] && s == slot
        ),
        "deactivating a week with a filled cell must fail, got {result:?}",
    );
```

New:

```rust
    // Turning the week inactive would silence the non-empty cell.
    let result = app.try_apply(
        Op::Week(WeekOp::Update(weeks[0], WeekDesc::new(false))),
        "Deactivate week".into(),
    );
    assert_eq!(
        result,
        Err(ApplyError::Invariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::InterrogationOnInactiveWeek(slot, weeks[0]))
        ]))),
        "deactivating a week with a filled cell must fail, got {result:?}",
    );
```

The deactivation lands; the filled interrogation cell at `(slot, weeks[0])` now sits on an inactive week — exactly the convergence fact `InterrogationOnInactiveWeek(slot, weeks[0])`, and nothing else breaks (the cell's slot still runs on the period, the group is still in bounds). Single-entry set.

### 3.5 Assert rewrite 3 — `move_week_blocked_when_destination_lacks_slot` (:523-537)

This is the one **two-entry set** in the whole migration; it doubles as a pin on the F5 bound-saturates-to-0 rule. Old:

```rust
    let result = app.apply(
        Op::Week(WeekOp::Move(moved, period_b, 0)),
        "Move filled week to B".into(),
    );
    assert!(
        matches!(
            result,
            Err(Error::Week(WeekError::NotCompatibleSlotInColloscope(w, s)))
                if w == moved && s == slot
        ),
        "moving a filled week to a period lacking the slot must fail, got {result:?}",
    );
```

New:

```rust
    let result = app.try_apply(
        Op::Week(WeekOp::Move(moved, period_b, 0)),
        "Move filled week to B".into(),
    );
    assert_eq!(
        result,
        Err(ApplyError::Invariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::InterrogationSlotNotRunningOnPeriod(
                slot, moved,
            )),
            FixableInvariant::Convergence(Convergence::InterrogationGroupOutOfBounds(slot, moved)),
        ]))),
        "moving a filled week to a period lacking the slot must fail, got {result:?}",
    );
```

Why *two* entries: the subject is excluded from `period_b`, so after the forced move the filled cell's slot does not run on the destination period (`InterrogationSlotNotRunningOnPeriod`), **and** the destination has no group-list association for the subject, so the group-count bound saturates to 0 and the cell's group number is out of bounds (`InterrogationGroupOutOfBounds`). This is the F5/D.4 rule from the design doc's Appendix D: the bound is computed against the association, and no association means bound 0 — the exact-set assert pins that behavior, which is why we assert both entries rather than a one-entry subset.

**Commit message:** `step-5 commit 5.2: migrate week_ops tests onto try_apply`

---

## 4. Commit 5.3 — `tests/found_bugs.rs`

Five rejection asserts — three stripped guards, two kept carve-outs — plus one raw-`Data` round-trip test.

### 4.1 Imports

Old (lines 9-21):

```rust
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, Error, GroupListError, GroupListOp, NewId, NonEmptyRangeInclusive, Op,
    PeriodOp, SettingsOp, SlotOp, StudentOp, Subject, SubjectInterrogationParameters, SubjectOp,
    SubjectParameters, SubjectPeriodicity, TeacherOp, WeekOp,
    group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::PeriodId,
    settings::{Limits, Settings},
    slots::{Slot, SlotError},
    students::Student,
    teachers::Teacher,
    weeks::WeekDesc,
};
```

New:

```rust
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ApplyError, ColloscopeOp, Convergence, Data, FixableInvariant, GroupListOp,
    GroupListPrecheckError, NewId, NonEmptyRangeInclusive, Op, PeriodOp, PrecheckError, Reference,
    SettingsOp, SlotOp, SlotPrecheckError, StudentOp, StudentRefSite, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekOp,
    group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::PeriodId,
    settings::{Limits, Settings},
    slots::Slot,
    students::Student,
    teachers::Teacher,
    weeks::WeekDesc,
};
```

(Removed: `Error`, `GroupListError`, and `SlotError` from the `slots::` import. Added: `ApplyError`, `Convergence`, `FixableInvariant`, `GroupListPrecheckError`, `PrecheckError`, `Reference`, `SlotPrecheckError`, `StudentRefSite`.)

### 4.2 The apply swap

Every `.apply(` becomes `.try_apply(`: the two calls in the `add_active_period` helper, all scaffold calls in the six tests, and the two raw-`Data` calls in `remove_prefilled_group_list_round_trips_on_reverse`:

```rust
    let (annotated, _new_id) = data.annotate(Op::GroupList(GroupListOp::Remove(group_list_id)));
    let rev = data
        .try_apply(&annotated)                                  // was .apply(&annotated)
        .expect("removing an empty prefilled group list should succeed");
    data.try_apply(&rev)                                        // was .apply(&rev)
        .expect("the reverse of a successfully applied op must apply");
```

(That test already does `use collomatique_state::InMemoryData;` locally, so the raw trait calls resolve.)

### 4.3 Assert rewrite 1 — `remove_student_with_settings_is_rejected` (:75-83)

Old — note this was a **wildcard** match, the weakest assert in the file:

```rust
    let result = app_state.apply(
        Op::Student(StudentOp::Remove(student_id)),
        "Remove student".into(),
    );
    assert!(
        matches!(result, Err(Error::Student(_))),
        "removing a student that still has per-student settings must fail, got {result:?}",
    );
```

New — exact single-entry set:

```rust
    let result = app_state.try_apply(
        Op::Student(StudentOp::Remove(student_id)),
        "Remove student".into(),
    );
    assert_eq!(
        result,
        Err(ApplyError::Invariants(BTreeSet::from([
            FixableInvariant::DanglingFk(Reference::Student {
                target: student_id,
                site: StudentRefSite::SettingsStudentKey,
            })
        ]))),
        "removing a student that still has per-student settings must fail, got {result:?}",
    );
```

The forced removal lands and the per-student settings row keyed by `student_id` dangles — one `DanglingFk` at site `SettingsStudentKey`. The student is referenced nowhere else in this scenario, so the set is exactly this one entry. This is the strict-upgrade case §9 called out: the old test only pinned "some student error"; the new one pins the entire failure.

### 4.4 Assert rewrite 2 — `set_filling_excluding_placed_student_is_rejected` (:157-162)

Old:

```rust
    assert_eq!(
        result,
        Err(Error::GroupList(
            GroupListError::NotCompatibleGroupListInColloscope(group_list_id)
        )),
    );
```

New:

```rust
    assert_eq!(
        result,
        Err(ApplyError::Invariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::ColloscopeStudentExcluded(
                group_list_id,
                placed_student,
            ))
        ]))),
    );
```

The whole-value group-list update lands; the colloscope entry still places `placed_student` in group 0, but the new filling excludes them — the convergence fact `ColloscopeStudentExcluded(group_list_id, placed_student)`. Payload note: the old variant only named the group list; the new entry also names the *student*, which the test has in scope (`placed_student`).

### 4.5 Assert rewrite 3 — `update_shrinking_group_names_below_assigned_group_is_rejected` (:325-330)

Old:

```rust
    assert_eq!(
        result,
        Err(Error::GroupList(
            GroupListError::InvalidGroupInSubjectSlotInColloscope(subject_id, period_id, slot_id)
        )),
    );
```

New:

```rust
    assert_eq!(
        result,
        Err(ApplyError::Invariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::InterrogationGroupOutOfBounds(
                slot_id, week0,
            ))
        ]))),
    );
```

Payload change explained: the old guard reported the `(subject, period, slot)` coordinates of the offending colloscope cell; the new convergence fact is keyed the way the checker walks the colloscope — by `(slot, week)`. The test already computes `week0` (the period's first week id, fetched a few lines above to fill the cell), so the new assert uses it directly. The shrink to 2 groups leaves the cell's group number 2 out of bounds on exactly that one filled cell — single-entry set.

### 4.6 Assert rewrite 4 — `assign_to_subject_with_dangling_group_list_id_errors` (:473-479)

This guard is a **kept carve-out** (dangling op target ⇒ precheck; the state is never touched). Old:

```rust
    assert_eq!(
        result,
        Err(Error::GroupList(GroupListError::InvalidGroupListId(
            group_list_id
        ))),
    );
```

New:

```rust
    assert_eq!(
        result,
        Err(ApplyError::Precheck(PrecheckError::GroupList(
            GroupListPrecheckError::InvalidGroupListId(group_list_id)
        ))),
    );
```

### 4.7 Assert rewrite 5 — `slot_update_changing_subject_is_rejected` (:566-574)

Also a kept carve-out. Old:

```rust
    assert_eq!(
        result,
        Err(Error::Slot(SlotError::CannotChangeSubject(
            slot_id, subject_a, subject_b
        ))),
    );
```

New:

```rust
    assert_eq!(
        result,
        Err(ApplyError::Precheck(PrecheckError::Slot(
            SlotPrecheckError::CannotChangeSubject(slot_id, subject_a, subject_b)
        ))),
    );
```

**Commit message:** `step-5 commit 5.3: migrate found_bugs tests onto try_apply`

---

## 5. Commit 5.4 — new file `tests/property_try_apply.rs`

### 5.1 What this file is and is not

This is the step-5 successor of `differential_force_apply.rs` — the same walk-and-probe fuzz shape, but the probe assertions become **properties of the gate alone**. It is a *new file*; the old differential file is **not deleted here** (§0 — it dies in R1), so randomized coverage never gaps: during the window both files run, one checking old-vs-new agreement, this one checking the gate's own contract.

The three property families:

- **Atomicity** — every `Err` arm leaves the state bit-identical to before the op. `Precheck` never touched it (by construction of the `force_apply_*` copies); `Logic`/`Invariants` were rolled back from the snapshot and must carry a non-empty error set.
- **Honesty** — every `Ok` landing is fully valid (`broken_invariants() == Ok(∅)`), and the returned reverse restores the pre-state exactly (the clean-landing reverse pin, carried over from step 4).
- **Coverage** — cross-seed honesty counters: every `CorruptionKind` was attempted, each corrupting kind produced at least one `Err`, and `ForceLogic` reached the `ApplyError::Logic` tier at least once (post-sealing, `ForceLogic`'s surviving recipes are external-data-shaped — duplicated-id `GlobalUpdate` and friends — and this counter proves that route still fires).

Three design points that differ from the old differential file, each deliberate:

1. **The walk itself commits through `try_apply`**, not through checked `apply`. This file exercises the primitive production runs on, so the walk should be the production path. It is safe with respect to the harness's honesty guards because those guards are one-sided: `for_each_seed` asserts that at least half the generated ops succeed per seed and that every op category is attempted (testgen `harness.rs:107-120`) — and the gate accepts a superset of what old `apply` accepts (the one divergence is the perfect-no-op case, where the gate *accepts* what old apply rejected), so success rates can only go up. There is no "each category must be rejected" guard to trip.
2. **The probe snapshot is taken *after* `annotate`** (like the walk's own `before` clone). `annotate` burns ids whether or not the op lands, in production too — rollback restores the issuer to its post-annotate value, never un-issues history ids. Snapshotting after `annotate` makes the `data == snapshot` atomicity assert compare like with like.
3. **There is no `ForceValid` special arm anymore.** The step-4 differential needed one because its oracle was *agreement with checked apply*, and `gen_op`/`gen_corruption_op`'s valid arms are valid-*shaped* only — so it had to branch on the checked-apply outcome. Here the oracle is the gate's own contract: a `ForceValid` probe either lands `Ok` (and the honesty assert proves the landing is fully valid — whether it changed state or was a perfect no-op, both are correct gate behavior) or is rejected `Err` (and the atomicity assert proves rollback). "Hidden repair" detection — a force copy silently fixing what a stripped guard policed — remains the **canary's** job until R1; this file does not and cannot check it, and its module doc says so.

### 5.2 The full file

```rust
//! Property fuzz over the apply/check/rollback gate ([`Data::try_apply`]).
//!
//! This is the step-5 successor of `differential_force_apply.rs`. The old file
//! *differential-fuzzed* `force_apply` against the two checkers to earn trust in
//! the new checker; that job is done, and the old checker retires with step 5.
//! What survives is the randomized coverage of the exact primitive production
//! now runs on: the gate `try_apply` = snapshot + `force_apply` +
//! `broken_invariants` + rollback. This file re-expresses the same walk-and-probe
//! shape as *properties of the gate alone*:
//!
//! * **atomicity** — every `Err` arm (precheck, logic, invariants) leaves the
//!   state bit-identical to before the op, and carries a non-empty error set on
//!   the two rolled-back arms;
//! * **honesty** — every `Ok` landing is fully valid (`broken_invariants()` is
//!   `Ok(∅)`), and its returned reverse restores the pre-state exactly;
//! * **coverage** — every [`CorruptionKind`] is attempted, each corrupting kind
//!   is rejected at least once, and `ForceLogic` reaches the [`ApplyError::Logic`]
//!   tier at least once (the external-data route the sealing left standing).
//!
//! **Fuzz shape — depth-1 probes off a validated walk.** A validated random walk
//! (the testgen harness, byte-untouched) is interrupted every [`PROBE_STRIDE`]
//! successful ops by a corruption probe: snapshot the state, run *one* op through
//! [`Data::try_apply`], assert the gate properties, then restore the snapshot and
//! resume. In production `try_apply` only ever runs on a consistent state, so
//! `{valid state} + {one gated op}` is the exact target distribution.
//!
//! On failure the harness prints the seed and the full op log so the sequence
//! replays exactly.

use std::cell::Cell;
use std::collections::BTreeSet;

use collomatique_testgen_colloscopes::generator::CorruptionKind;
use collomatique_testgen_colloscopes::rand::Rng;
use collomatique_testgen_colloscopes::{generator, harness};

use collomatique_state::InMemoryData;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{ApplyError, Data, InnerData};

use harness::RunConfig;

/// House scale, matching `property_ops.rs`.
const CONFIG: RunConfig = RunConfig {
    seeds: 100,
    ops_per_run: 1000,
    invalid_fraction: 0.15,
};

/// One corruption probe every this many *successful* walk ops.
const PROBE_STRIDE: usize = 10;

/// Index of `kind` in [`CorruptionKind::ALL`], for the per-kind counters.
fn kind_index(kind: CorruptionKind) -> usize {
    CorruptionKind::ALL
        .iter()
        .position(|k| *k == kind)
        .expect("every kind is in ALL")
}

/// Walk `Data` through the gate (like property 4 of `property_ops.rs`), probing
/// `try_apply` every [`PROBE_STRIDE`] ops and asserting the gate's atomicity and
/// honesty on the resulting (usually rejected) op.
#[test]
fn try_apply_gate_is_atomic_and_honest() {
    // Cross-seed honesty counters (interior mutability: `for_each_seed` takes a
    // `Fn` closure).
    let landed = Cell::new(0usize); // probes that returned Ok
    let rejected = Cell::new(0usize); // probes that returned Err (rolled back)
    let attempted: [Cell<usize>; 5] = std::array::from_fn(|_| Cell::new(0));
    let rejected_by_kind: [Cell<usize>; 5] = std::array::from_fn(|_| Cell::new(0));
    let logic_seen = Cell::new(0usize);

    harness::for_each_seed(
        "try_apply_gate_is_atomic_and_honest",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            let mut inner_snapshots: Vec<InnerData> = vec![];
            let mut since_probe = 0usize;

            for _ in 0..CONFIG.ops_per_run {
                // --- validated walk op through the gate (feeds category coverage) ---
                let (category, op) = generator::gen_op(
                    rng,
                    data.get_inner_data(),
                    &inner_snapshots,
                    CONFIG.invalid_fraction,
                );
                log.push(category, &op);
                let (annotated, _) = data.annotate(op);
                let before = data.clone();

                match data.try_apply(&annotated) {
                    Ok(_) => {
                        stats.record(category, true);
                        if inner_snapshots.len() < 8 && rng.random_bool(0.02) {
                            inner_snapshots.push(data.get_inner_data().clone());
                        }
                    }
                    Err(_) => {
                        stats.record(category, false);
                        assert!(
                            data == before,
                            "a failed walk try_apply must leave the state unchanged",
                        );
                        continue;
                    }
                }

                since_probe += 1;
                if since_probe < PROBE_STRIDE {
                    continue;
                }
                since_probe = 0;

                // --- corruption probe off the current (valid) state ---
                let (kind, op) = generator::gen_corruption_op(rng, data.get_inner_data());
                log.push(kind.label(), &op);
                let i = kind_index(kind);
                attempted[i].set(attempted[i].get() + 1);

                let (annotated, _) = data.annotate(op);
                // Snapshot after annotate (like the walk's `before`): the clone
                // carries the already-advanced id issuer, matching production
                // rollback (which restores the issuer too).
                let snapshot = data.clone();
                match data.try_apply(&annotated) {
                    Err(e) => {
                        rejected.set(rejected.get() + 1);
                        rejected_by_kind[i].set(rejected_by_kind[i].get() + 1);
                        // Atomicity: every error arm rolls back to bit-identical.
                        assert!(
                            data == snapshot,
                            "a rejected try_apply must leave the state unchanged",
                        );
                        match e {
                            // Precheck bounced before any mutation.
                            ApplyError::Precheck(_) => {}
                            ApplyError::Logic(set) => {
                                logic_seen.set(logic_seen.get() + 1);
                                assert!(!set.is_empty(), "a Logic error carries a non-empty set");
                            }
                            ApplyError::Invariants(set) => {
                                assert!(
                                    !set.is_empty(),
                                    "an Invariants error carries a non-empty set",
                                );
                            }
                        }
                    }
                    Ok(reverse) => {
                        landed.set(landed.get() + 1);
                        // Honesty: a landing the gate accepts really is fully valid.
                        assert_eq!(
                            data.get_inner_data().broken_invariants(),
                            Ok(BTreeSet::new()),
                            "try_apply returned Ok but the state is not fully valid",
                        );
                        // The returned reverse restores the pre-state exactly (the
                        // clean-landing reverse pin, carried over from step 4).
                        let mut redo = data.clone();
                        redo.force_apply(&reverse)
                            .expect("reverse of a gated op must apply");
                        assert!(
                            redo.get_inner_data() == snapshot.get_inner_data(),
                            "reverse of a gated op must restore the pre-state",
                        );
                        // ForceValid needs no special arm: without the old checker
                        // there is no "hidden repair" to detect here (the gate only
                        // ever lands fully-valid states, asserted just above). A
                        // valid landing is honest whether it changed state or was a
                        // perfect no-op; the canary still guards force-path drift
                        // until R1.
                    }
                }

                // Production rollback semantics: the probe never persists.
                data = snapshot;
            }
        },
    );

    // --- honesty guards (cross-seed, over the whole run) ---
    assert!(landed.get() > 0, "no corruption probe ever landed a valid state");
    assert!(rejected.get() > 0, "no corruption probe was ever rejected");

    for kind in CorruptionKind::ALL {
        let i = kind_index(kind);
        assert!(
            attempted[i].get() > 0,
            "corruption kind {kind:?} was never attempted across all seeds",
        );
        if kind.corrupting() {
            assert!(
                rejected_by_kind[i].get() > 0,
                "corrupting kind {kind:?} was never rejected across all seeds",
            );
        }
    }

    assert!(
        logic_seen.get() > 0,
        "no ForceLogic probe ever reached the ApplyError::Logic tier across all seeds",
    );
}
```

Implementation notes for the reader of this listing:

- Everything this file uses exists at `aaceda78`: `CorruptionKind::ALL` / `.label()` / `.corrupting()` (testgen `generator.rs:1145-1170`), `harness::for_each_seed` / `bootstrap` / `RunConfig`, `Data::get_inner_data` (lib.rs:615), `Data::force_apply` (lib.rs:504, public), `try_apply` via the `InMemoryData` trait (commit 1), and the `ApplyError` re-export. `collomatique-testgen-colloscopes` is already a dev-dependency of state-colloscopes (the differential fuzz uses it) — **no `Cargo.toml` change**.
- The reverse-restores-pre-state pin compares `get_inner_data()` (not whole `Data`): applying the reverse cannot un-burn the ids the forward op's `annotate` issued, so the issuers legitimately differ. This mirrors how the step-4 differential pinned reverses.
- The `data = snapshot` at the probe's end restores the walk after a *successful* probe too — probes never persist, keeping walk trajectories a function of the seed alone.

**Commit message:** `step-5 commit 5.4: add property_try_apply, the gate-property successor of the differential fuzz`

---

## 6. Deferred material — commit R1.5 (recorded here, executed later)

The following items were part of §9's original commit 5 and are **deliberately not done now** (§0 ruling). They form a new commit **R1.5**, landing between R1 and R2 so that R2 stays a mechanical deletion (parent plan §11 has the matching entry):

- `src/invariants.rs`: the fixture wrapper (:944-954) drops its `assert_differential(data)` call; `assert_dangling_maps` loses its old-checker `InnerDataError` argument and assert (and every caller drops that argument); the explicit `data.check_invariants() == Err(...)` asserts inside compound fixtures (:3455-3458, :3491-3494) and the two legacy-bridge unit tests (:3506-3639) are deleted whole.
- `src/colloscopes.rs` :571-610: the three stage-6 corruption tests become new-checker asserts on the same forged states (`Err({EmptyInterrogationRow})`, `Err({EmptyColloscopeGroupListRow})`, and the forged-row test asserts the exact two-entry `DanglingFk` set); the `assert_differential` calls are dropped.
- `src/lib.rs` force_apply_tests (:538-731): drop the old-checker asserts and `assert_differential` calls; the two `forced_valid_*_equals_checked_apply` anti-drift pins retarget as `try_apply` happy-path pins (a valid op through `try_apply` equals `force_apply`-on-a-twin in state and reverse) — after R3 they read naturally as `apply` happy-path tests.

R1.5 gets its own detailed pass (exact old+new snippets, in this style) when it is reached — the line anchors above are against `aaceda78` and must be re-verified then.

---

## 7. Verification

Per commit (5.1, 5.2, 5.3): build and run that file's test binary —

```
cargo test -p collomatique-state-colloscopes --test period_consistency_in_subjects
cargo test -p collomatique-state-colloscopes --test week_ops
cargo test -p collomatique-state-colloscopes --test found_bugs
```

— plus the per-file done-check `grep -F ".apply(" <file>` → empty. These commits touch nothing outside their one test file, so this is the complete check.

At 5.4: run the new property test once in full (`cargo test -p collomatique-state-colloscopes --test property_try_apply`; 100 seeds × 1000 ops, comparable runtime to the differential fuzz — run it once and capture the output, per house rules).

End of the sequence: the usual user-run acceptance (full workspace suite once, captured to a scratchpad file and grepped) before moving on to commit 6 of the parent plan. The canary and the old differential fuzz are part of that suite and must still be green — untouched files, so any failure there means a commit leaked outside its scope.
