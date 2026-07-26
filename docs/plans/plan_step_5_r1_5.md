# Step-5 commit R1.5 — retire the old-checker test scaffolding (detailed implementation plan)

**Status of the tree this plan is written against:** `cb13427d` (step-5 R1 landed). All file and line references below are against this exact tree and have been verified by reading the files — do not trust the older anchors in `docs/plans/plan_step_5.md` §11 or `plan_step_5_commit_5.md` §6, which were pinned against `aaceda78` and have shifted.

---

## 1. Context — why this commit exists

Step 5 of `docs/plans/invariant_cascade_design.md` rewires production onto the apply/check/rollback gate (`try_apply` = snapshot + `force_apply` + `broken_invariants` + rollback) and then deletes the old world (checked `apply_*`, the old `Error` vocabulary, `InnerData::check_invariants`, the legacy bridge). The deletion happens in commit R2, and R2 must stay a *mechanical* deletion: when R2 removes the old checker and the old `apply`, nothing anywhere in the tree may still call them.

Commit R1 (already landed) removed the last *authoritative* users: the property harnesses now drive `try_apply` with the `broken_invariants` oracle, and the canary and the old differential fuzz are deleted. What remains is **test scaffolding inside `state-colloscopes/src/`** that still exercises the old checker and the legacy bridge on purpose — it was deliberately left untouched by the commit-5 test migration (scope ruling of July 25 2026: "the test-migration commits do not touch the old invariant checks"). R1.5 retires exactly that scaffolding. The three files are:

1. `state-colloscopes/src/invariants.rs` — the fixture test module still runs the old-vs-new differential on every fixture, still asserts the old checker's first error per dangling site, and still unit-tests the legacy bridge itself.
2. `state-colloscopes/src/colloscopes.rs` — three stage-6 corruption tests still assert through `check_invariants` and `assert_differential`.
3. `state-colloscopes/src/lib.rs` — the `force_apply_tests` module still asserts through the old checker, still calls `assert_differential`, and its two anti-drift pins still compare `force_apply` against the old checked `apply`. Additionally, the scaffold helpers of both `force_apply_tests` and `try_apply_tests` still build states through the old checked `apply`.

After R1.5, the old checker and the old `apply` have **zero callers in `state-colloscopes`** outside their own implementations, so R2 can delete them without touching any test logic. (Section 8 of this plan covers a second, newly discovered set of leftover callers in *other* crates — a gap in the parent plan — handled as a sibling commit R1.6.)

Two vocabulary reminders for the implementing model:

- The **new checker** is `InnerData::broken_invariants() -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>>`. `Err(...)` means tier-2 logic errors (short-circuits, fixable breaks are not reported alongside); `Ok(set)` reports the fixable breaks; a valid state is `Ok(∅)`.
- The **old world** being retired from tests: `InnerData::check_invariants() -> Result<(), InnerDataError>` (first-error-wins), the legacy bridge (`to_legacy`, `is_necessarily_logic_error`, `assert_differential` in `invariants.rs`), and the checked `Data::apply`.

House rules that bind this work: use the Edit tool for every change (never `sed` on source files); run each test command once, captured to a scratchpad file, then grep the capture; never argue from a paraphrase — re-read the code at each site before editing it.

---

## 2. Commit R1.5 — file 1: `state-colloscopes/src/invariants.rs`

The test module is `#[cfg(test)] pub(crate) mod tests` starting at line 886. It imports everything via `use super::*` plus explicit `use crate::...` lines, so **no import edits are needed in this file** — `InnerDataError` etc. reach the tests through `super::*` and remain used by the (untouched-until-R2) bridge code above the test module.

Everything *outside* the test module — `to_legacy` (lines ~636-765), `dangling_to_legacy`, `convergence_to_legacy`, `is_necessarily_logic_error` (~817-836), and `assert_differential` (~840-883) — is left **byte-untouched**. After R1.5 these are caller-free, which is fine: they are `pub` (no dead-code warnings) and R2 deletes them wholesale. Do not "helpfully" delete them here; that would make R2 non-mechanical relative to its plan.

### 2.1 The fixture wrapper (lines 928-938)

Old code:

```rust
    /// Runs [assert_differential] on `data`, then returns its
    /// [crate::InnerData::broken_invariants]. Every fixture below asserts on the
    /// checker *through this wrapper*, so the differential contract is verified
    /// on each fixture's state without touching the fixtures themselves.
    #[track_caller]
    fn broken_invariants(
        data: &InnerData,
    ) -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>> {
        assert_differential(data);
        data.broken_invariants()
    }
```

New code — drop the differential call and the now-useless `#[track_caller]` (the function no longer panics), keep the wrapper itself so the hundreds of fixture call sites stay untouched:

```rust
    /// Shorthand for [crate::InnerData::broken_invariants]: every fixture below
    /// asserts on the checker through this wrapper. (Until step-5 R1.5 it also
    /// ran the old-vs-new differential on each fixture's state; the old checker
    /// retired with step 5.)
    fn broken_invariants(
        data: &InnerData,
    ) -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>> {
        data.broken_invariants()
    }
```

Every fixture's exact-set assert (`assert_eq!(broken_invariants(&data), ...)`) survives verbatim — the parent plan is explicit about this. Do not touch any fixture body in stages B through 6.

### 2.2 The per-site legacy coverage section (lines 2681-2703 and its 18 callers)

The section comment (lines 2681-2691) currently reads:

```rust
    // ---- Stage 7: per-site legacy coverage ----
    //
    // One single-corruption fixture per DanglingFk site not already exercised by
    // a stage-3/5 fixture, each pinning both the exact new output *and* the exact
    // old first-error. Together with `assert_differential` (run by the
    // `broken_invariants` wrapper), the two assertions are the operational proof
    // of that §6 table row: the old checker's first error for the single dangle
    // is exactly `to_legacy` of the reported reference. `Period@WeekPeriodFk`
    // became representable when the force path dropped the `PeriodStillHasWeeks`
    // guard, so it now has a fixture
    // (`dangling_period_from_forced_removal_maps_to_legacy`).
```

Replace it with a comment describing what the section pins *now* — per-site coverage of the dangling-reference sweep:

```rust
    // ---- Stage 7: per-site dangling coverage ----
    //
    // One single-corruption fixture per DanglingFk site not already exercised by
    // a stage-3/5 fixture, each pinning the exact new-checker output. (Until
    // step-5 R1.5 these fixtures also pinned the old checker's first error per
    // site — the operational proof of the legacy-bridge tables; that half
    // retired with the old checker.) `Period@WeekPeriodFk` became representable
    // when the force path dropped the `PeriodStillHasWeeks` guard, so it has a
    // fixture (`dangling_period_from_forced_removal_is_reported`).
```

The helper (lines 2693-2703):

```rust
    /// Asserts that `data` has exactly the one dangling reference `reference`
    /// (new checker) and that the old checker's first error is `old`. The
    /// `broken_invariants` wrapper additionally runs the full differential.
    #[track_caller]
    fn assert_dangling_maps(data: &InnerData, reference: Reference, old: InnerDataError) {
        assert_eq!(
            broken_invariants(data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(reference)])),
        );
        assert_eq!(data.check_invariants(), Err(old));
    }
```

becomes:

```rust
    /// Asserts that `data` has exactly the one dangling reference `reference`.
    #[track_caller]
    fn assert_single_dangling_fk(data: &InnerData, reference: Reference) {
        assert_eq!(
            broken_invariants(data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(reference)])),
        );
    }
```

The helper is renamed because its old name (`assert_dangling_maps`, "the dangle *maps* to this legacy error") describes exactly the half being deleted. Its **18 callers** (all tests between lines 2714 and 3208) each need two edits: the call renamed, and the third argument (an `InnerDataError::...` expression, usually spanning two to four lines including the trailing comma) deleted. The tests are also renamed from the `_maps_to_legacy` suffix to `_is_reported`, since no legacy mapping is asserted anymore. The full list, in file order:

| old test name | new test name |
|---|---|
| `dangling_period_in_subject_exclusions_maps_to_legacy` | `dangling_period_in_subject_exclusions_is_reported` |
| `dangling_period_from_forced_removal_maps_to_legacy` | `dangling_period_from_forced_removal_is_reported` |
| `dangling_period_in_pairing_rule_maps_to_legacy` | `dangling_period_in_pairing_rule_is_reported` |
| `dangling_period_in_slot_pairing_rule_maps_to_legacy` | `dangling_period_in_slot_pairing_rule_is_reported` |
| `dangling_period_in_association_maps_to_legacy` | `dangling_period_in_association_is_reported` |
| `dangling_period_in_assignments_key_maps_to_legacy` | `dangling_period_in_assignments_key_is_reported` |
| `dangling_week_in_interrogation_key_maps_to_legacy` | `dangling_week_in_interrogation_key_is_reported` |
| `dangling_subject_in_incompat_maps_to_legacy` | `dangling_subject_in_incompat_is_reported` |
| `dangling_subject_in_pairing_antecedent_maps_to_legacy` | `dangling_subject_in_pairing_antecedent_is_reported` |
| `dangling_subject_in_pairing_consequent_maps_to_legacy` | `dangling_subject_in_pairing_consequent_is_reported` |
| `dangling_subject_in_balancing_maps_to_legacy` | `dangling_subject_in_balancing_is_reported` |
| `dangling_subject_in_association_maps_to_legacy` | `dangling_subject_in_association_is_reported` |
| `dangling_group_list_in_association_maps_to_legacy` | `dangling_group_list_in_association_is_reported` |
| `dangling_student_in_prefilled_group_maps_to_legacy` | `dangling_student_in_prefilled_group_is_reported` |
| `dangling_student_in_excluded_set_maps_to_legacy` | `dangling_student_in_excluded_set_is_reported` |
| `dangling_student_in_assignments_cell_maps_to_legacy` | `dangling_student_in_assignments_cell_is_reported` |
| `dangling_student_in_colloscope_group_list_maps_to_legacy` | `dangling_student_in_colloscope_group_list_is_reported` |
| `dangling_week_pattern_in_incompat_maps_to_legacy` | `dangling_week_pattern_in_incompat_is_reported` |

There are no name collisions with the earlier stage-B tests (those have no suffix and target different sites; verified against the file). One representative caller edit, the first test (lines 2714-2739), old:

```rust
    #[test]
    fn dangling_period_in_subject_exclusions_maps_to_legacy() {
        // ... scaffold unchanged ...
        assert_dangling_maps(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::SubjectExcludedPeriods(subject),
            },
            InnerDataError::Params(InvariantError::InvalidSubject),
        );
    }
```

New:

```rust
    #[test]
    fn dangling_period_in_subject_exclusions_is_reported() {
        // ... scaffold unchanged ...
        assert_single_dangling_fk(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::SubjectExcludedPeriods(subject),
            },
        );
    }
```

Every scaffold body stays byte-identical; only the test name, the helper name, and the deleted last argument change. Apply the same pattern to all 18. Note that `dangling_period_from_forced_removal_maps_to_legacy` (line 2742) has a doc-style comment mentioning force semantics — that comment is accurate and stays.

### 2.3 The compound-state tests (lines 3211-3404)

The section comment (lines 3211-3216):

```rust
    // ---- Stage 7: compound states ----
    //
    // States with more than one corruption, pinning the differential contract's
    // *lenient* branch (decision 8, requirement 2) and membership under
    // multiplicity (requirement 3). Each asserts the exact new *and* old output,
    // then runs `assert_differential` explicitly to document the branch taken.
```

becomes:

```rust
    // ---- Stage 7: compound states ----
    //
    // States with more than one corruption, pinning the checker's short-circuit
    // precedence (layer-A logic errors beat fixable breaks) and exact multi-entry
    // sets under multiplicity.
```

Each of the five tests keeps its scaffold and its `broken_invariants` exact assert, and **loses** its `data.check_invariants()` assert, its trailing `assert_differential(&data)` call, and the old-checker sentences in its comment. Test by test:

**`compound_row_both_empty_and_not_running` (line 3219).** Delete lines 3253-3259 (the `assert_eq!(data.check_invariants(), Err(InnerDataError::Params(InvariantError::AssignmentForSubjectNotRunningOnPeriod)))` and the `assert_differential(&data);`). The leading comment currently explains the old checker's sweep order and the lenient branch; shrink it to:

```rust
        // One assignments row that is *both* empty (a layer-A logic error) and
        // on a subject that excludes the period (a convergence). The checker
        // short-circuits on the logic error.
```

**`compound_logic_error_with_earlier_fixable` (line 3262).** Rename to `compound_logic_error_with_unrelated_fixable` — "earlier" described the old checker's sweep order (teachers before assignments), which no longer exists. Delete the `check_invariants` assert (lines 3291-3294) and `assert_differential(&data);` (line 3295). New leading comment:

```rust
        // A two-corruption state: an empty assignments row (layer-A logic error)
        // and, unrelated, a dangling subject in a teacher's `subjects`. The
        // checker short-circuits on the logic error and does not report the
        // dangle alongside it.
```

**`compound_duplicate_id_with_dangling_ref` (line 3298).** Delete `assert_eq!(data.check_invariants(), Err(InnerDataError::DuplicateIds));` (line 3322) and `assert_differential(&data);` (line 3323). Shrink the leading comment to:

```rust
        // A raw id shared by a student and a teacher (logic error) *plus* a
        // dangling period in that student's exclusions. Layer A short-circuits
        // on the id collision.
```

**`compound_two_fixable_breaks` (line 3326).** Delete lines 3363-3367 (`check_invariants` assert and `assert_differential`). Shrink the comment to:

```rust
        // Two independent dangling references — a teacher in a slot and a
        // student in `settings` — so the `Ok` payload has two entries.
```

**`compound_convergence_with_dangling` (line 3370).** Delete lines 3399-3403. Shrink the comment to:

```rust
        // A clean fixture twisted into a day-overflowing slot (a convergence)
        // with an added dangling student in `settings`. The checker reports
        // both, as `Ok`.
```

### 2.4 The legacy-bridge unit tests (lines 3406-3547)

Delete the whole block: the section comment ("---- Stage 7: legacy bridge ----", lines 3406-3412) and the two tests `is_necessarily_logic_error_classification` (lines 3414-3447) and `to_legacy_payload_plumbing` (lines 3449-3547). The file then ends with `compound_convergence_with_dangling` followed by the module's closing brace (currently line 3548). These tests exist solely to unit-check `to_legacy` and `is_necessarily_logic_error`, both of which lose their last runtime caller in this very commit and are deleted in R2.

---

## 3. Commit R1.5 — file 2: `state-colloscopes/src/colloscopes.rs`

The test module is lines 555-612. Three stage-6 tests assert forged (publicly unreachable) colloscope rows through the **old** checker and run the differential. They become new-checker asserts on the same forged states. The forged-state scaffolds are byte-untouched.

### 3.1 Imports (line 558-559)

Old:

```rust
    use crate::ids::Id;
    use crate::{InnerData, InnerDataError};
```

New (`InnerDataError` would otherwise become an unused import and warn; the refs types are needed by the third test):

```rust
    use crate::ids::Id;
    use crate::refs::{Reference, SlotRefSite, WeekRefSite};
    use crate::{FixableInvariant, InnerData, LogicError};
```

(`FixableInvariant` and `LogicError` are re-exported at the crate root, `state-colloscopes/src/lib.rs:69`. `BTreeSet`/`BTreeMap`, `SlotId`/`WeekId`/`GroupListId`, and `ColloscopeError` all arrive through the module's `use super::*;`, and `ColloscopeError` is no longer referenced after this rewrite — that is fine, it comes via the glob, not an explicit import.)

### 3.2 `empty_interrogation_row_rejected` (lines 563-577)

Old:

```rust
    /// Stage-6 backfill: a stored empty interrogation row — unreachable through
    /// any public path — is rejected by the old checker.
    #[test]
    fn empty_interrogation_row_rejected() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::new());
        assert_eq!(
            data.check_invariants(),
            Err(InnerDataError::ColloscopeError(
                ColloscopeError::EmptyInterrogationRow(slot, week)
            ))
        );
        crate::invariants::assert_differential(&data);
    }
```

New — the same forged state is a tier-2 logic error under the new checker (an empty row violates the canonical-absent storage contract), and logic errors short-circuit, so the assert is an exact single-entry `Err` set:

```rust
    /// Stage-6 backfill: a stored empty interrogation row — unreachable through
    /// any public path — is a tier-2 logic error.
    #[test]
    fn empty_interrogation_row_rejected() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::new());
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::EmptyInterrogationRow(
                slot, week
            )]))
        );
    }
```

### 3.3 `empty_group_list_row_rejected` (lines 580-593)

Same pattern. The old assert on `Err(InnerDataError::ColloscopeError(ColloscopeError::EmptyGroupListRow(group_list)))` plus the differential call becomes:

```rust
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::EmptyColloscopeGroupListRow(
                group_list
            )]))
        );
```

with the doc comment's "is likewise rejected" retargeted the same way ("is likewise a tier-2 logic error"). Note the *new* variant name differs from the old one: `LogicError::EmptyColloscopeGroupListRow`, not `EmptyGroupListRow` (verified against the `to_legacy` mapping in `invariants.rs`).

### 3.4 `non_empty_forged_row_reports_dangling_ids` (lines 595-611)

Old:

```rust
    /// Precedence: emptiness fires before id resolution, but a non-empty row
    /// with dangling coordinates still reports the dangling id.
    #[test]
    fn non_empty_forged_row_reports_dangling_ids() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::from([0]));
        assert_eq!(
            data.check_invariants(),
            Err(InnerDataError::ColloscopeError(
                ColloscopeError::InvalidWeekId(week)
            ))
        );
        crate::invariants::assert_differential(&data);
    }
```

New — this is the exact-two-entry upgrade the parent plan calls for. The old checker stopped at its first error (the week id); the new checker reports **both** dangling coordinates, and nothing else: the row is non-empty (no logic error), and every convergence check on the cell (inactive week, slot-runs-on-period, group bounds) *skips* when its slot or week fails to resolve — this skip behavior is already pinned by `interrogation_checks_skip_when_slot_dangles` in `invariants.rs` (line 2490). The site payloads are the opposite coordinate, exactly as pinned by the (deleted) `to_legacy_payload_plumbing` test:

```rust
    /// A non-empty forged row with dangling coordinates reports *both* dangling
    /// ids — and nothing else: the convergence checks on the cell all skip when
    /// the slot or week fails to resolve.
    #[test]
    fn non_empty_forged_row_reports_dangling_ids() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::from([0]));
        assert_eq!(
            data.broken_invariants(),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Week {
                    target: week,
                    site: WeekRefSite::ColloscopeInterrogation { slot },
                }),
                FixableInvariant::DanglingFk(Reference::Slot {
                    target: slot,
                    site: SlotRefSite::ColloscopeInterrogation { week },
                }),
            ]))
        );
    }
```

If this assert fails on the exact set (for instance an extra entry), do not widen the assert to make it pass — stop and re-read the checker's colloscope walk; the two-entry expectation is a worked-out prediction and a mismatch means the prediction or the checker needs understanding first (house rule: on test failure, stop and explain before fixing).

---

## 4. Commit R1.5 — file 3: `state-colloscopes/src/lib.rs`

Two test modules are touched: `force_apply_tests` (lines 643-836) and, for one scaffold helper, `try_apply_tests` (lines 838-952). The production code (including the old `apply` body, `Data::check_invariants`, and the `format_error_set` region) is untouched.

### 4.1 `force_apply_tests` — module doc and imports (lines 644-663)

Old module doc:

```rust
    //! Step-4 commit 2.2 pins for [Data::force_apply]: carve-out guards still
    //! fire (leaving the state unchanged), stripped invariant guards let a
    //! forced op land an *invalid* state that both checkers agree on, and a
    //! forced *valid* op is byte-identical to the checked `apply` (the standing
    //! anti-drift pin on the thin copies). The step-4 fuzz generalises these.
```

New (the differential fuzz died in R1; `property_try_apply.rs` is its successor):

```rust
    //! Step-4 commit 2.2 pins for [Data::force_apply], retargeted at step-5
    //! R1.5: carve-out guards still fire (leaving the state unchanged),
    //! stripped invariant guards let a forced op land an *invalid* state that
    //! [InnerData::broken_invariants] reports, and a forced *valid* op is
    //! byte-identical to the gated [Data::try_apply] (the standing anti-drift
    //! pin on the thin copies). `tests/property_try_apply.rs` generalises these.
```

Imports: delete the line `use crate::invariants::assert_differential;` (line 652) and add `LogicError` to the crate-root import (line 661), which becomes:

```rust
    use crate::{Data, FixableInvariant, InnerData, LogicError, PrecheckError, StudentPrecheckError};
```

### 4.2 The scaffold helper (lines 665-671)

Old:

```rust
    /// Applies a (valid) op through the checked path and returns its annotated
    /// form, so callers can read back the freshly issued ids.
    fn apply(data: &mut Data, op: Op) -> AnnotatedOp {
        let (annotated, _) = data.annotate(op);
        data.apply(&annotated).expect("valid op should apply");
        annotated
    }
```

New — the scaffold builds through the gate, so the old `apply` loses this caller. The helper keeps its name `apply` (it is a local function; after the R3 rename the production method is called `apply` again and the name reads naturally):

```rust
    /// Applies a (valid) op through the gate and returns its annotated form,
    /// so callers can read back the freshly issued ids.
    fn apply(data: &mut Data, op: Op) -> AnnotatedOp {
        let (annotated, _) = data.annotate(op);
        data.try_apply(&annotated).expect("valid op should apply");
        annotated
    }
```

This is safe: every op the scaffold applies is valid, the gate accepts everything the checked `apply` accepted with the identical resulting state and reverse (the §3 one-directional contract of the parent plan, backed by the step-3 certification and the step-4 `ForceValid` pins, re-verified continuously by the canary until R1).

### 4.3 `force_remove_referenced_student_lands_a_dangling_fk` (lines 700-727)

Three edits. First, the entry-validity assert switches checkers. Old:

```rust
        assert!(
            data.get_inner_data().check_invariants().is_ok(),
            "the built state is valid"
        );
```

New:

```rust
        assert_eq!(
            data.get_inner_data().broken_invariants(),
            Ok(BTreeSet::new()),
            "the built state is valid"
        );
```

Second, the comment above the post-force asserts and the `assert_differential` call. Old (lines 715-717):

```rust
        // Both checkers must see the now-dangling reference, and the three-part
        // differential must agree on it.
        assert_differential(data.get_inner_data());
```

New — delete the `assert_differential` line and shrink the comment to:

```rust
        // The checker must see the now-dangling reference.
```

The existing `matches!`-based assert on `broken_invariants` (a `DanglingFk` is present in the `Ok` set) stays as it is. Third, delete the trailing old-checker assert (lines 723-726):

```rust
        assert!(
            data.get_inner_data().check_invariants().is_err(),
            "the old checker must also reject the dangling state"
        );
```

The two precheck tests that follow (`force_add_on_existing_id_...` and `force_update_on_dangling_target_...`) contain no old-checker references and are untouched.

### 4.4 `force_global_update_drops_the_pre_gate_and_lands_an_invalid_state` (lines 767-800)

The scaffold comment (lines 772-774) references the old `apply`'s pre-gate; reword it so it stays true after R2/R3:

```rust
        // A corrupt inner: a student and a week pattern sharing the same raw id
        // — a duplicated-id LogicError that the try_apply gate rejects and rolls
        // back. `force_apply` has no gate, so the corrupt state lands.
```

The tail (lines 795-799), old:

```rust
        assert!(
            data.get_inner_data().check_invariants().is_err(),
            "the duplicated-id state is invalid"
        );
        assert_differential(data.get_inner_data());
```

New — the natural substitution here is the *exact* new-checker outcome, which is fully predictable (both entities share raw id 1, and layer A short-circuits, matching the existing `compound_duplicate_id_with_dangling_ref` pin in `invariants.rs`):

```rust
        assert_eq!(
            data.get_inner_data().broken_invariants(),
            Err(BTreeSet::from([LogicError::DuplicatedId(1)])),
            "the duplicated-id state is logically impossible"
        );
```

### 4.5 The two anti-drift pins (lines 802-835)

These currently compare `force_apply` against the old checked `apply` on a valid op. They retarget as `try_apply` happy-path pins: a valid op through the gate equals `force_apply` on a twin, in state and reverse. After the R3 rename they read naturally as `apply` happy-path tests (their names contain `try_apply` precisely so R3's textual rename lands them as `..._equals_apply`... in reverse: the *method* rename makes the body read `apply`; the test names below contain `try_apply` so R3 also renames them mechanically).

Old (first pin, lines 802-814):

```rust
    #[test]
    fn forced_valid_student_add_equals_checked_apply() {
        let data = Data::default();
        let (add, _) = data.annotate(Op::Student(StudentOp::Add(Student::default())));

        let mut checked = data.clone();
        let mut forced = data.clone();
        let checked_rev = checked.apply(&add).expect("valid op");
        let forced_rev = forced.force_apply(&add).expect("valid op");

        assert_eq!(checked.get_inner_data(), forced.get_inner_data());
        assert_eq!(checked_rev, forced_rev);
    }
```

New:

```rust
    #[test]
    fn forced_valid_student_add_equals_try_apply() {
        let data = Data::default();
        let (add, _) = data.annotate(Op::Student(StudentOp::Add(Student::default())));

        let mut gated = data.clone();
        let mut forced = data.clone();
        let gated_rev = gated.try_apply(&add).expect("valid op");
        let forced_rev = forced.force_apply(&add).expect("valid op");

        assert_eq!(gated.get_inner_data(), forced.get_inner_data());
        assert_eq!(gated_rev, forced_rev);
    }
```

The second pin, `forced_valid_week_add_equals_checked_apply` (lines 816-835), gets the identical treatment: rename to `forced_valid_week_add_equals_try_apply`, `checked` → `gated`, `checked.apply(&add_week)` → `gated.try_apply(&add_week)`, `checked_rev` → `gated_rev`, and the two `assert_eq!`s reworded accordingly. Its leading comment about weeks exercising the copied helpers (the F2 drift-risk spot) is still accurate and stays.

Be aware of what these pins mean after the retarget: since `try_apply` calls `force_apply` internally, they no longer compare two independent implementations. What they pin is that the gate adds nothing on the happy path — same landed state, same reverse, no residue from the snapshot/check — which is the parent plan's stated intent for them.

### 4.6 `try_apply_tests` — the scaffold helper (lines 857-863)

The sibling module's helper has the same old-`apply` scaffold body:

```rust
    /// Applies a (valid) op through the checked path and returns its annotated
    /// form, so callers can read back the freshly issued ids.
    fn apply(data: &mut Data, op: Op) -> AnnotatedOp {
        let (annotated, _) = data.annotate(op);
        data.apply(&annotated).expect("valid op should apply");
        annotated
    }
```

Same edit as §4.2: `data.apply(...)` → `data.try_apply(...)`, "checked path" → "gate" in the doc comment. Nothing else in `try_apply_tests` changes.

---

## 5. Commit R1.5 — verification and commit

1. Build and run the crate's unit tests once, captured (the three touched modules are all `src/` `#[cfg(test)]` modules, so `--lib` covers them):
   ```
   cargo test -p collomatique-state-colloscopes --lib 2>&1 | tee <scratchpad>/r15_lib_tests.txt
   ```
   then grep the capture for `FAILED`/`test result`. Run it once only.
2. Done-check greps (each must return nothing):
   - `grep -n "assert_differential\|assert_dangling_maps\|_maps_to_legacy" state-colloscopes/src/colloscopes.rs state-colloscopes/src/lib.rs` — and on `invariants.rs`, the only remaining `assert_differential` hits must be the function's own definition/doc (lines ~840-883), none inside the test module.
   - `grep -n "check_invariants" state-colloscopes/src/invariants.rs state-colloscopes/src/colloscopes.rs` — hits only in `invariants.rs`'s non-test bridge/doc code (module doc line 60, `assert_differential`'s body/doc), none in any test.
   - `grep -n "\.apply(" state-colloscopes/src/lib.rs` — no hits (the string `.apply(` does not occur inside `.try_apply(` or `.force_apply(`; the old `Data::apply` *implementation* is `fn apply`, not a call).
3. Commit — this is a lone commit per the migration workflow (deactivation and removal stay separate commits):
   ```
   step-5 R1.5: retire the old-checker test scaffolding
   ```
   Suggested body: notes that invariants.rs fixtures keep every exact-set assert; the 18 per-site tests keep their fixtures and drop the legacy half (renamed `_is_reported`); the stage-6 colloscope tests move to exact new-checker sets; the anti-drift pins retarget onto `try_apply`; and the two in-crate scaffold helpers now build through the gate.

---

## 6. Commit R1.6 (new) — the leftover scaffold callers of old `apply`

**This is a gap discovered while planning R1.5.** The parent plan asserts that after R1 "the old API is intact but caller-free", and its R2 hidden-caller sweep only tracked the dying *error vocabulary*. But R2 also deletes `Manager::apply` and `InMemoryData::apply` themselves, and a fresh workspace sweep (done for this plan, at `cb13427d`) finds **scaffold-only callers of old `Manager::apply` that no planned commit migrates**. Without this commit, R2 does not compile. All of them are happy-path scaffolds (`Ok` expected, panic otherwise) — pure method swaps, no assert rewrites, which is why they slipped through every migration commit keyed on error vocabulary.

The complete list (verified by grep and by reading each site):

1. **`testgen-colloscopes/src/harness.rs:141-151`** — `bootstrap`'s internal closure:
   ```rust
       let new_id = state
           .apply(op, desc.to_string())
           .unwrap_or_else(|e| panic!("bootstrap op `{desc}` failed: {e}"));
   ```
   becomes `.try_apply(op, desc.to_string())`. Also fix the function's doc comment (line 130): "Builds a small but non-degenerate document through the checked op path" → "through the gated op path". This crate is the shared dev-dependency of every property harness; all bootstrap ops are valid, and the gate accepts every valid op with the identical resulting state (parent plan §3, soundness direction), so seeds and walk trajectories are unchanged.
2. **`state-colloscopes/tests/read_api.rs:112`**, **`tests/colloscope_surface.rs:115` and `:123`**, **`tests/refs_registry.rs:138` and `:146`** — scaffold macros of the shape:
   ```rust
       let Ok(Some($variant(id))) = app.apply($op, $msg.into()) else { ... };
   ```
   Each `app.apply(` becomes `app.try_apply(` (and the one bare `.expect` form at `colloscope_surface.rs:123` likewise).
3. **`ops/tests/general_planning_content.rs`** and **`ops/tests/found_bugs.rs`** — the scaffold calls of the form `app.apply(Op::..., desc(...))` / `app_state.apply(Op::..., "...".into())` (roughly nine sites in the first file, ten in the second). **Caution:** these files also contain calls of the shape `something.apply(&mut app_state)` (`general_planning_content.rs:203` and `:273`) — that is the *ops-layer* `UpdateOp::apply`, part of the frozen `ops/` surface, and must **not** be touched. The discriminator is the argument list: `Manager::apply` takes `(op, description)`; the ops-layer `apply` takes `(&mut app_state)`. Re-read each site before editing.
4. **`storage/tests/populated_round_trip.rs:58`** and **`storage/tests/populated_round_trip/builder.rs:88`** — one direct call (the post-reload id-issuer probe) and one helper:
   ```rust
   fn apply(state: &mut AppState<Data, String>, op: Op, desc: &str) -> Option<NewId> {
       state
           .apply(op, desc.to_string())
           .unwrap_or_else(|e| panic!("build_rich_data op `{desc}` failed: {e}"))
   }
   ```
   Both become `.try_apply(...)`. The `:58` site's `assert!(matches!(result, Ok(Some(NewId::StudentId(_)))))` stays as-is (the `Ok` type is unchanged).
5. **`state/src/state.rs`** test module — seventeen calls of the form `state.apply(set(0, 1), "set to 1").expect("valid op")` / `session.apply(...)` / `outer.apply(...)` / `inner.apply(...)` (lines 188-311). All become `.try_apply(...)`. These run over `FakeData`, whose `ApplyError` is the same `FakeError`, so nothing else changes.

**Deliberately NOT touched by R1.6** (the surviving old-API sites, all deleted or reshaped in R2):

- `state/src/traits.rs`: the trait's `fn apply` declaration, `Manager::apply`, and the old-apply twin tests (`apply_changes_data_and_stores_history` line 420, `apply_failing_leaves_state_untouched` line 433). These are the coexistence pins; they die with the API in R2.
- `state/src/test_utils.rs`: `FakeData::apply` (line 61) and the delegation `self.apply(op)` inside `FakeData::try_apply` (line 81). Note for R2: when `fn apply` is removed from the trait, inline the old `apply` body into `try_apply` here.
- `state-colloscopes/src/lib.rs`: the old `Data::apply` implementation and everything it calls.
- `python/src/glue.rs:1363` (`op.apply(&mut *state)`) — ops-layer `UpdateOp::apply`, frozen surface, not the state API.

**Parent-plan bookkeeping, folded into this commit:** in `docs/plans/plan_step_5.md`, add an R1.6 row to the §4 commit table ("R1.6 | migrate remaining scaffold apply callers onto try_apply (gap: parent sweep tracked only the error vocabulary) | testgen-colloscopes, state-colloscopes (tests), ops (tests), storage (tests), state") and a matching short paragraph in §11 between R1.5 and R2, so R2's "old API caller-free" precondition is recorded as actually delivered by R1.5 + R1.6.

**Verification for R1.6**, run once each, captured to the scratchpad and grepped:

```
cargo test -p collomatique-state-colloscopes --test read_api --test colloscope_surface --test refs_registry
cargo test -p collomatique-ops --test general_planning_content --test found_bugs
cargo test -p collomatique-storage --test populated_round_trip
cargo test -p collomatique-state
```

(Adjust the `-p` package names to the actual `Cargo.toml` names — check them, do not guess.) Because `harness.rs` changed, one property harness must also be exercised; `property_ops` at house scale is the heavy suite, so run it once, captured, and do not re-run it:

```
cargo test -p collomatique-state-colloscopes --test property_ops 2>&1 | tee <scratchpad>/r16_property_ops.txt
```

Done-check: a workspace `grep -rn "\.apply(" --include=*.rs` filtered of `try_apply`/`force_apply` must show only the deliberate keep-list above plus unrelated non-Manager `apply` methods in other crates (audit each remaining hit; the two-argument `(op, desc)` shape is the tell for a missed Manager call).

Commit message: `step-5 R1.6: migrate the remaining scaffold apply callers onto try_apply`.

---

## 7. What comes after (not this plan's scope)

R2 (remove) follows, per parent plan §11, and now genuinely finds the old API caller-free. Small notes collected for R2 while reading the code: `FakeData::try_apply` must absorb `apply`'s body (test_utils.rs:81); the traits-tests old-apply twins die; `invariants.rs`'s module doc line 60 ("The id-issuer high-water check lives in `Data::check_invariants`") goes stale when that function dissolves and should be fixed in R2 alongside the already-planned `force_apply_group_list` doc rider.
