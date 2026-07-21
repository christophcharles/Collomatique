# Step 4 — differential fuzz: `force_apply` + two-checker agreement (session plan)

**Status:** open (July 19 2026). Live roadmap = `invariant_cascade_design.md` steps 4–7;
this plan delivers step 4. File/line references are against the tree at `d3dcc4e5`.

## 1. Goal and shape

Steps 2–3 delivered the precise new checker (`InnerData::broken_invariants`,
`invariants.rs:213`) and certified the old checker (`InnerData::check_invariants`,
`lib.rs:175`) as the complete reference oracle (Appendix D). Step 4 earns trust in the new
checker by differential fuzzing: force-apply operations that land in *invalid* states and
assert the two checkers agree via the stage-7 three-part differential
(`assert_differential`, `invariants.rs:821`). The `force_apply` door built here is the
primitive step 5 rewires production onto.

**Fuzz shape — depth-1 probes off a validated walk.** Validated random walk (existing
testgen harness, byte-untouched) → every few ops a corruption probe: snapshot, force *one*
op, run the differential, restore, resume. Rationale: in the step-5 architecture
`force_apply` only ever runs on a *consistent* state, so `{valid state} + {one forced op}`
is the exact target distribution — not a compromise. Long forced chains would fuzz
apply-code robustness on broken states, which is out of contract (mirror desyncs are
deliberate fail-fast panics, design doc Appendix D decision 2). Forcing onto broken states
becomes relevant only for the step-6 cascade's resolution ops and is fuzzed there.

**`force_apply` shape — an independent thin copy.** `force_apply` is a *copy* of the
current `apply` family: it duplicates the existing code, with `apply` left byte-untouched.
No mode flag, no threading through the existing methods — step 5 needs this artifact as-is
(the checked originals are deleted there, so the duplication is short-lived). The copy is
**thin**: invariant guards are deleted; carve-out guards are kept and get their own precise
error enums (§3). Uniformity is total: every domain gets its copy, including
`apply_export_config` whose copy is guard-free — no clever reuse, it would only breed
errors later.

## 2. The strip/keep rule

The step-3 survey (pinned: `git show 26d88024:docs/plans/plan_step_3.md`) classified every
guard in the 16 `apply_*` paths; it is the authoritative row list for this step:

- **Strip-list = Table 1** (§2.1 shared validators + §2.2 hand-written guards): every check
  with an old-checker twin. In the copy, the guard block is deleted — the `validate_*` call
  or the scan + `return Err(...)`.
- **Keep-list = Table 2 / Appendix D.3** (the carve-out register): `*IdAlreadyExists`
  (no-clobber), `Invalid*Id` on the op target, `AddAfter` anchors and position bounds,
  empty-first protocol guards (`PeriodStillHasWeeks`, `RemainingFilling`,
  `NonEmptyGroupsWhenReducing`), `CannotChangeSubject`, and the parameter-targeting checks.
- **Dual-listed rows keep.** Checks appearing in *both* tables — the `Assign` coordinate
  existence checks (`assignments.rs:88-117`) and the `SetFilling` prefill-count boundary
  check (`group_lists.rs:562-568`) — are registered carve-outs (D.3 is the certified step-5
  keep-list), so they stay in the thin copy and in its precheck enum. No fuzz coverage is
  lost: dangling assignment rows are reachable via forced `StudentOp::Remove` /
  `PeriodOp::Remove` / `SubjectOp::Remove`, and the prefill-mismatch `LogicError` is
  reachable through forced `GroupListOp::Add`/`Update` (whose `validate_group_list` call
  strips).
- **Mutation code is copied verbatim** — including write-time canonicalization (empty-row
  clearing in `Assign`, the sparse colloscope writers). Only guard blocks are deleted. Care
  point: where mutation reuses a binding produced by a *stripped* guard, rebind it from the
  *kept* target-existence lookup; never re-add validation to get a binding back.

Table 1/2 line refs are against `0a1041b6`; only doc commits landed since, so they are
current.

## 3. The precheck error enums

The thin copies do **not** reuse the existing per-domain error enums. Each domain gets a
new enum containing *only* its keep-list — these are exactly the step-5 carve-out error
vocabulary (design doc §4: "carve-out errors are hard errors"), born here:

```rust
/// Precondition errors of the forced student ops — the carve-out subset
/// (step-3 survey Table 2). This is the error surface that survives step 5.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StudentPrecheckError {
    #[error("student id {0:?} already exists")]
    StudentIdAlreadyExists(StudentId),
    #[error("invalid student id {0:?}")]
    InvalidStudentId(StudentId),
}
```

Variant names and `#[error]` messages are copied verbatim from the originals. Per-domain
content (from Table 2; exact variant names confirmed against the originals while
implementing):

| Enum | Variants (keep-list) |
|---|---|
| `StudentPrecheckError` | id-exists, invalid-id |
| `PeriodPrecheckError` | id-exists, invalid-id (target + AddAfter anchor), `PeriodStillHasWeeks`, invalid-position |
| `WeekPrecheckError` | id-exists, invalid-id, AddAfter anchor, Move destination (`periods.rs:937-945`), invalid-position |
| `SubjectPrecheckError` | id-exists, invalid-id, AddAfter anchor (`subjects.rs:338-345`), position-out-of-bounds |
| `TeacherPrecheckError` | id-exists, invalid-id |
| `AssignmentPrecheckError` | the three coordinate `Invalid*Id` checks (`assignments.rs:88-117`, dual-listed → keep) |
| `WeekPatternPrecheckError` | id-exists, invalid-id |
| `SlotPrecheckError` | id-exists, invalid-id, anchor + `PreviousSlotIsNotInRightSubject`, position-out-of-bounds, `CannotChangeSubject` |
| `IncompatPrecheckError` | id-exists, invalid-id |
| `GroupListPrecheckError` | id-exists, invalid-id, `RemainingFilling`, `NonEmptyGroupsWhenReducing`, `PrefillGroupCountMismatch` (dual-listed → keep), AssignToSubject coordinates (`group_lists.rs:635-668`) |
| `PairingPrecheckError` / `SlotPairingPrecheckError` | id-exists, invalid-id |
| `SettingsPrecheckError` / `BalancingPrecheckError` / `ExportConfigPrecheckError` | **empty enums** (no carve-outs exist) — kept for uniformity |
| `ColloscopePrecheckError` | SetGroupList target (`colloscopes.rs:326-334`), SetInterrogation coordinates (`:373-381`) |

Top-level, mirroring the existing `Error` (`lib.rs`) structure 1:1 with `#[from]`
transparency; no `GlobalUpdate` variant — the forced `GlobalUpdate` arm is infallible:

```rust
/// Errors of [Data::force_apply]: only op preconditions (carve-outs), never
/// invariants — those are the caller's business via a checker + rollback.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PrecheckError {
    #[error(transparent)]
    Student(#[from] StudentPrecheckError),
    …                                        // one arm per domain, incl. the empty enums
}
```

## 4. Commits

### Commit 0 — this plan

`docs/plans/plan_step_4.md` (this file).

### Commit 1 — move `assert_differential` out of `#[cfg(test)]`

It lives inside `#[cfg(test)] pub(crate) mod tests` (`invariants.rs:821`), invisible to
integration tests (cfg(test) items don't exist in the lib as compiled for them). Move the
function — body unchanged, keep `#[track_caller]` — to the crate proper in `invariants.rs`:

```rust
#[doc(hidden)]      // test oracle, not part of the semantic API
#[track_caller]
pub fn assert_differential(data: &InnerData) { … }
```

Doc comment: drop the "`pub(crate)` so `colloscopes.rs`…" line, add "exposed for the step-4
differential fuzz and the step-5 canary". The tests module already does `use super::*` —
delete the local def. Update the `colloscopes.rs` stage-6 call-site imports
(`invariants::tests::…` → `invariants::…`). No behavior change; workspace tests green.

### Commit 2 — `force_apply`: thin copies, precheck enums, dispatch

**2a — per-domain `force_apply_*`**, adjacent to each `apply_*`, in all 16 files:
`students.rs`, `periods.rs` (`force_apply_period` + `force_apply_week`), `subjects.rs`,
`teachers.rs`, `assignments.rs`, `week_patterns.rs`, `slots.rs`, `incompats.rs`,
`pairings.rs`, `slot_pairings.rs`, `group_lists.rs`, `settings.rs`, `balancing.rs`,
`colloscopes.rs`, `export_config.rs` (guard-free copy — uniformity). Each returns its
precheck enum. Template (students; strip/keep per §2):

```rust
/// Used internally by [crate::Data::force_apply]
///
/// Thin copy of [Self::apply_student]: carve-out guards kept (returned as
/// [StudentPrecheckError]), invariant guards stripped (step-3 survey Table 1).
/// May leave the state invalid; the caller owns checking and rollback.
pub(crate) fn force_apply_student(
    &mut self,
    student_op: &AnnotatedStudentOp,
) -> std::result::Result<AnnotatedStudentOp, StudentPrecheckError> {
    match student_op {
        AnnotatedStudentOp::Add(new_id, student) => {
            if self.inner_data.params.students.student_map.contains(new_id) {
                return Err(StudentPrecheckError::StudentIdAlreadyExists(*new_id));
            }
            // stripped: validate_student
            self.inner_data.params.students.student_map.insert(*new_id, student.clone());
            Ok(AnnotatedStudentOp::Remove(*new_id))
        }
        AnnotatedStudentOp::Remove(id) => {
            // stripped: colloscope-placement / group-list / assignments / settings scans
            let Some(old_student) = self.inner_data.params.students.student_map.remove(id)
            else {
                return Err(StudentPrecheckError::InvalidStudentId(*id));
            };
            Ok(AnnotatedStudentOp::Add(*id, old_student))
        }
        AnnotatedStudentOp::Update(id, new_student) => {
            // stripped: validate_student + newly-excluded-period assignment scan
            let Some(current_student) =
                self.inner_data.params.students.student_map.get_mut(id)
            else {
                return Err(StudentPrecheckError::InvalidStudentId(*id));
            };
            let old_student = std::mem::replace(current_student, new_student.clone());
            Ok(AnnotatedStudentOp::Update(*id, old_student))
        }
    }
}
```

Per-domain strip summary (authoritative rows = pinned Table 1):

- **students** — strip `validate_student` + the 5 Remove scans (`students.rs:106-149`) +
  the Update newly-excluded-period scan (`:168-181`).
- **periods** (`apply_period`) — strip the 7 Remove reference-scans (`periods.rs:628-709`);
  keep `PeriodStillHasWeeks`, target existence, anchor, positions.
- **weeks** (`apply_week`) — strip the Remove scans (`:835-853`), the Update
  interrogations→off colloscope guard (`:899-908`), both WeekMove semantic guards
  (`:966-1000`, the F2 inline re-implementations); keep target existence, Move destination,
  positions.
- **subjects** — strip `validate_subject` + the 7 Remove scans (`subjects.rs:388-450`) +
  the 4 Update→no-interrogations guards (`:495-537`) + the 3 newly-excluded-period guards
  (`:552-601`); keep no-clobber, target existence, anchor, position.
- **teachers** — strip `validate_teacher` + slot-reference scan (`teachers.rs:95-99`) +
  dropped-subject slot scan (`:118-136`).
- **assignments** — strip the two semantic guards (`assignments.rs:106-124`); keep the
  coordinate existence checks (`:88-117`); canonicalization (`:133-144`) verbatim.
- **week_patterns** — strip `validate_week_pattern` + Remove scans (`:141-161`) + the
  Update silenced-week guard (`:194-211`).
- **slots** — strip `validate_slot` + Remove scans (`slots.rs:474-500`) + the Update
  pattern guard (`:545-557`); keep no-clobber, target existence, anchors,
  `PreviousSlotIsNotInRightSubject`, position, `CannotChangeSubject`.
- **incompats / pairings / slot_pairings** — strip the `validate_*` calls (forced ops thus
  reach `PairingRulePartsShareSubject`-class logic errors); keep no-clobber + target
  existence.
- **group_lists** — strip `validate_group_list*` calls, Remove scans (`:406-423`), Update
  placement/bound guards (`:455-488`), the SetFilling auto→prefilled and exclusion guards
  (`:574-611`), the AssignToSubject semantic guards (`:640-648`, `:671-675`); keep
  no-clobber, target existence, `RemainingFilling`, `NonEmptyGroupsWhenReducing`,
  `PrefillGroupCountMismatch` (`:562-568`), AssignToSubject coordinate existence.
- **settings / balancing** — strip `validate_settings` / `validate_balancing`; nothing
  else exists (empty precheck enums).
- **colloscopes** — strip SetGroupList prefilled+placement guards (`colloscopes.rs:340-351`)
  and the three SetInterrogation semantic guards (`:386-422`); keep coordinate existence
  (`:326-334`, `:373-381`); sparse writers verbatim.
- **export_config** — guard-free copy.

**2b — `Data::force_apply`** in `lib.rs`: copy of the `apply` dispatch (`lib.rs:287-342`)
with three differences — arms call `force_apply_*` and errors land in `PrecheckError`; the
`GlobalUpdate` arm drops the `new_inner_data.check_invariants()?` pre-gate (this *is* the
force door); the trailing panic net `self.check_invariants()` (`lib.rs:340`) is omitted:

```rust
impl Data {
    /// Applies `op` without checking invariants. Carve-out preconditions still
    /// hold (no-clobber, op-target existence, positions/anchors, protocol) and
    /// surface as [PrecheckError]; a failed call leaves the state unchanged.
    /// A successful call may leave the state *invalid*: the caller owns running
    /// a checker and restoring a snapshot on failure. This is the step-5
    /// apply/check/restore primitive; today the step-4 fuzz exercises it.
    pub fn force_apply(&mut self, op: &AnnotatedOp) -> Result<AnnotatedOp, PrecheckError> {
        let backward = match op {
            AnnotatedOp::Student(o) => AnnotatedOp::Student(self.force_apply_student(o)?),
            …
            AnnotatedOp::GlobalUpdate(new_inner_data) => {
                // no check_invariants pre-gate
                let old = std::mem::replace(&mut self.inner_data, new_inner_data.clone());
                AnnotatedOp::GlobalUpdate(old)
            }
        };
        // no panic net: the state is allowed to be invalid here
        Ok(backward)
    }
}
```

**2c — in-crate unit pins** (same commit), one per tricky spot:
- forced `StudentOp::Remove` of a student referenced by an assignment succeeds; the old
  checker then errs `InvalidStudentIdInAssignments`, the new one reports the matching
  `DanglingFk`, and `assert_differential` passes;
- carve-outs still hard-error in force mode (`Add` on an existing id → id-exists; `Update`
  on a dangling target → invalid-id — asserted on the *new* enums) and leave the state
  unchanged;
- forced `GlobalUpdate` with a duplicated id lands and the differential's `Err(LogicError)`
  path fires;
- forced valid op ≡ checked apply (state + reverse) on one hand-built case per intricate
  domain (weeks, group_lists, colloscopes).

### Commit 3 — testgen: corruption generator

In `testgen-colloscopes/src/generator.rs` (reuses the private `Pools`; dangling payload ids
via the existing `dangling()` / `DANGLING_BASE`):

```rust
/// Probe kinds for the step-4 differential fuzz
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorruptionKind {
    /// Remove of an existing (likely referenced) entity → dangling FKs
    ForceRemove,
    /// Update whose payload embeds a dangling id
    ForceRetarget,
    /// Valid-shaped op whose only obstacle was an invariant guard
    ForceSemantic,
    /// Op landing a LogicError state (dup-id GlobalUpdate, prefill mismatch
    /// via GroupListAdd, same-subject pairing part, …)
    ForceLogic,
    /// Plain valid op — forced ≡ checked equivalence probe
    ForceValid,
}

impl CorruptionKind {
    pub const ALL: [CorruptionKind; 5] = […];
    /// Label for OpLog/RunStats (the harness tracks &'static str categories)
    pub fn label(self) -> &'static str { … }
    /// The four kinds expected to be able to break a state
    pub fn corrupting(self) -> bool { !matches!(self, CorruptionKind::ForceValid) }
}

/// Generates one probe op. All ops are carve-out-clean (live targets, fresh
/// ids); all but ForceValid aim at an invariant break.
pub fn gen_corruption_op(rng: &mut ChaCha8Rng, inner: &InnerData) -> (CorruptionKind, Op)
```

Kind content (ids drawn from live `Pools`):
- **ForceRemove** — pick any non-empty pool (student / period / week / subject / teacher /
  week pattern / slot / incompat / group list / pairing / slot pairing), emit its `Remove`.
  Highest-yield dangling-FK source. Periods still holding weeks bounce off the kept
  `PeriodStillHasWeeks` guard — acceptable; the honesty guards keep the yield visible.
- **ForceRetarget** — `StudentUpdate`/`SubjectUpdate` with a dangling excluded-period,
  `TeacherUpdate` with a dangling subject, `SlotUpdate` with a dangling week pattern,
  `IncompatUpdate` with dangling subject/pattern, `SettingsUpdate`/`BalancingUpdate` with a
  dangling key — reusing the per-category invalid-arm payload builders where carve-out-clean.
- **ForceSemantic** — `TeacherUpdate` dropping a subject still bound to the teacher's
  slots; `SubjectUpdate` newly excluding a period holding assignments/associations;
  `WeekUpdate` interrogations→off under a colloscope row; `GroupListUpdate` shrinking
  bounds under placements; `Assign(…, true)` of an excluded student.
- **ForceLogic** — the duplicated-id `GlobalUpdate` clone (existing corruption,
  `generator.rs:1075-1088`); `GroupListAdd` with a prefill-count mismatch; `PairingAdd`
  with both parts on one subject.
- **ForceValid** — delegate to `gen_op(rng, inner, &[], 0.0)`.

### Commit 4 — the fuzz: `state-colloscopes/tests/differential_force_apply.rs`

Drives `Data` directly (like property 4 of `property_ops.rs`), reusing
`harness::for_each_seed` + `harness::bootstrap` (extract `Data` via
`state.get_data().clone()`). House scale:

```rust
const CONFIG: RunConfig = RunConfig { seeds: 100, ops_per_run: 1000, invalid_fraction: 0.15 };
const PROBE_STRIDE: usize = 10;   // one probe every 10 successful walk ops
```

Walk loop = `gen_op` → `data.annotate` → `data.apply`, recorded in `stats` (satisfies
`for_each_seed`'s 17-category coverage guard; probe labels are extra entries). Every
`PROBE_STRIDE` successful walk ops, one probe:

```rust
let (kind, op) = generator::gen_corruption_op(rng, data.get_inner_data());
log.push(kind.label(), &op);
let (annotated, _) = data.annotate(op);
let snapshot = data.clone();   // after annotate: the clone must carry the
                               // already-advanced id issuer, or a checked replay
                               // of an Add probe trips apply's issuer panic net
match data.force_apply(&annotated) {
    Err(_) => {                                 // bounced off a kept carve-out guard
        assert!(data == snapshot, "failed force_apply must leave the state unchanged");
    }
    Ok(reverse) => {
        invariants::assert_differential(data.get_inner_data());     // ← the payoff
        let broken = data.get_inner_data().check_invariants().is_err();
        if !broken {
            // clean landing: the reverse feeds history in step 5 → pin it
            let mut redo = data.clone();
            redo.force_apply(&reverse).expect("reverse of a clean forced op must apply");
            assert!(redo.get_inner_data() == snapshot.get_inner_data(),
                "reverse must restore the pre-state");
        }
        if kind == CorruptionKind::ForceValid {
            // forced ≡ checked: the standing anti-drift pin on the copies.
            // gen_op's valid arm only guarantees a valid-*shaped* op (live ids,
            // well-formed payload), not applicability — so branch on the
            // checked outcome instead of expecting Ok.
            let mut checked = snapshot.clone();
            match checked.apply(&annotated) {
                Ok(checked_rev) => {
                    assert!(checked.get_inner_data() == data.get_inner_data());
                    assert!(checked_rev == reverse);
                }
                // checked apply bounced off a stripped (strip-list) guard, and
                // step 3 certified every stripped guard has an old-checker twin
                // → the landed forced state must be broken.
                Err(_) => assert!(broken),
            }
        }
        if broken { broken_by_kind[kind] += 1; }
    }
}
data = snapshot;                                // production rollback semantics
```

Honesty guards (in-test, accumulated **cross-seed** to stay stable):
- ≥25% of landed probes were actually broken (old checker `Err`) over the whole run;
- every `CorruptionKind` attempted, and each *corrupting* kind landed a broken state at
  least once across all seeds.

Any disagreement the fuzz finds is handled test-first (standing feedback): minimal
regression fixture committed alone (house corruption-fixture style in `invariants.rs`
tests), then the fix.

### Commit 5 — close-out (after the gate)

- Design doc: step 4 marked complete; delivered state recorded as **Appendix E**
  (`force_apply` contract + precheck vocabulary + strip/keep rule, fuzz shape, honesty
  guards).
- Retire this plan per the house pattern (pin via `git show`).
- Memory update (step-4 status; the force_apply-as-copy + precheck-enum decisions).

## 5. Decisions log

1. **Depth-1 probes** off a validated walk — matches the step-5 production distribution;
   deep forced chains deferred to step 6 where resolution-op forcing is in contract.
2. **`force_apply` is an independent thin copy**, not a mode flag — `apply` byte-untouched;
   duplication is short-lived (step 5 deletes the originals). Uniform: every domain copied,
   including guard-free `export_config`.
3. **New precheck error enums** per domain + top-level `PrecheckError` — only the carve-out
   subset; this *is* the step-5 error vocabulary, introduced here.
4. **Dual-listed guards keep** (Assign coordinates, SetFilling prefill boundary) — D.3 is
   the certified keep-list; fuzz coverage unaffected (alternate routes exist).
5. **Anti-drift**: the ForceValid probe continuously asserts forced ≡ checked on valid ops.
   ForceValid ops are valid-*shaped* (gen_op's contract); when checked apply rejects one,
   the pin asserts the forced landing is broken instead of asserting applicability.
6. **`assert_differential` moves to crate scope** (`#[doc(hidden)] pub`) — cfg(test) items
   are invisible to integration tests; body unchanged.
7. **Enums over strings** for probe kinds (`CorruptionKind`), with `label()` bridging into
   the harness's `&'static str` category tracking.

## 6. Verification

- Per commit: `cargo test --workspace` green; `Cargo.lock` untouched (no new deps).
- Existing harnesses (`property_ops.rs`, `property_ops_broken_invariants.rs`,
  `property_build.rs`) byte-untouched and green — seeded trajectories must not shift.
- Once, locally: crank the fuzz to `seeds: 500` to shake out rare probes before settling
  the committed CONFIG (results reported, not committed).
- End-of-step gate: user runs `scripts/smoke` acceptance.
