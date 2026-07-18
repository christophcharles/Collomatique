# Step 2 session plan — the precise checker beside the old one

**Status:** agreed July 18 2026 (branch `consolidate_state`). Implements step 2 of the
7-step roadmap in `invariant_cascade_design.md` §8, written against the post-step-1 data
model (Appendix B) and the reworked refs registry (`b2085d24` direct-only registry,
`9fb9b8cb` per-kind site enums + `Reference` edge).

Stages below are units of *work*, not necessarily single commits — a stage may land as
several commits if it turns out wide (the step-1 B1/D1 precedent).

---

## 1. Goal and non-goals

Write a *second* invariant checker without removing the first. The old
`InnerData::check_invariants` (`lib.rs:171`) stays authoritative and untouched, except
the mechanical fallout of stage 1 and the stage-6 canonical-absent backfill (which
makes it the *complete* ground truth for roadmap step 3); the new checker ships fully
tested but **unwired** —
no production caller. Wiring is later steps' business: completeness audit (step 3),
differential fuzz (step 4), op-error vocabulary (step 5), cascade (step 6).

The new checker returns **all** broken invariants, deduplicated, in canonical order,
with full coordinates — and it separates two situations that are different in kind:

- **Fixable**: the *data* references something it shouldn't. Fixed by removing or
  clearing the referencing data — this is what the step-6 cascade automates.
- **Logic error**: the *code* (or a hand-forged file) produced a state no elementary op
  can legitimately reach. Not fixable; the cascade will panic on it, decode will hard-error.

Non-goals: no change to the property harness (`property_ops.rs` stays on the old
oracle), no Serde on the new checker types (step 5 revisits when the vocabulary becomes
UI-visible), no `references_to_*` retirement (design doc §7 flags them "likely retire";
that call belongs to a later step), no resolution map (step 6).

## 2. Decisions ledger

1. **`Result` upper layer.** The fixable/unfixable binary is a control-flow boundary
   (every consumer either repairs/reports or panics), and logic errors undermine the
   meaningfulness of the fixable sweep (joins panic, walk order lies). So:

   ```rust
   impl InnerData {
       pub fn broken_invariants(&self)
           -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>>;
   }
   ```

   Tier-2 checks run **first and short-circuit** (`Err`). `Ok(empty)` = valid.
   Doc-comment framing: *Ok = the code is sound; the payload is what the data needs
   fixed.* Step-6 payoff: the resolution map becomes total over `FixableInvariant` —
   the §5 "no map entry → PANIC" arm becomes unrepresentable.

2. **Three-way classification, decided mechanically** (see §3): `DanglingFk` /
   `LogicError` / `Convergence`, with `FixableInvariant = DanglingFk | Convergence`.

3. **Canonical order = derive order.** Both payload sets are `BTreeSet`; `Ord` is
   derived, declaration order is the canonical order (implements §10's "natural
   candidate"; step 6 may still reorder variants — that is a variant-order edit, not a
   mechanism change). `DanglingFk` is declared before `Convergence` so that when a row
   is both dangling and convergence-broken, `min()` picks the precise row-removal fix
   over the lossy one.

4. **Type-encapsulated invariants are not checked.** The periods list↔map mirror and
   the slots ordering↔table mirror are maintained *inside* `Periods` / `Slots` (private
   fields, all mutation through the compound `pub(crate)` helpers — `periods.rs:24`,
   `slots.rs:26`). A badly-written elementary op cannot desync them, so the whole-model
   checker does not sweep for them. (The old checker's mirror sweeps are pre-encapsulation
   vestiges; they stay in the old checker, which we don't touch.) Corollary: layer C may
   trust the mirrors unconditionally.

5. **Stage 1 extends decision 4 to the empty-range shapes**: the four
   empty-range invariants become unrepresentable via a validated non-empty-range
   newtype. The remaining cross-field value shapes (`PrefillGroupCountMismatch`,
   `DuplicatedStudentInPrefilledGroups`, parts-share-an-id ×2) stay as `LogicError`
   variants: encapsulating them means privatizing `GroupList`/`PairingRule`/
   `SlotPairingRule` behind smart constructors — a public-API redesign that step 5
   rechurns anyway. Do that churn once, at step 5.

6. **`SlotOverflowsDay` is Convergence, not LogicError**: the check
   (`colloscope_params.rs:551`) joins `slot.start_time` with the *subject's*
   interrogation duration — a legitimate `UpdateSubject` lengthening the duration can
   break a midnight-adjacent slot.

7. **Legacy bridge is a method, not `From`, and it is total**:
   `to_legacy(&self) -> InnerDataError` on both `LogicError` and `FixableInvariant`.
   Totality is bought by stage 6, which backfills the old checker with the two
   missing colloscope canonical-absent checks (roadmap step 3 treats the old checker
   as ground truth — it should actually be complete). The codomain is
   `InnerDataError`, not `InvariantError`, because the old vocabulary is three-armed:
   `DuplicatedId` maps to the top-level `DuplicateIds`, and every colloscope-side
   condition (11 fixable mappings + the 2 backfilled checks) lives on the
   `ColloscopeError` arm, which `InvariantError` cannot express.

8. **Differential property — three requirements** (not variant equality: the old
   checker returns its *first* error in its own ad-hoc order, the new vocabulary is
   richer). With `InnerDataError::is_necessarily_logic_error(&self) -> bool` classifying the old
   vocabulary, assert for every fixture:
   1. *Verdict always agrees*: old `is_ok()` ⇔ new is `Ok(∅)`.
   2. *Logic errors agree*: if new is `Err(L)` and old is `Err(e)` with
      `e.is_necessarily_logic_error()`, then `e ∈ to_legacy(L)`. (Lenient when `e` is not
      logic-classified — in a compound state old may trip a fixable error first.)
   3. *Fixable side is exact*: if new is `Ok(F)` non-empty, old is `Err(e)` and
      `e ∈ to_legacy(F)`.

   `is_necessarily_logic_error` returns `true` only for variants whose *every* possible cause is
   tier-2 — six: `DuplicateIds`, `P(DuplicatedId)`, `P(EmptyAssignmentRow)`,
   `P(EmptySlotsRow)`, `C(EmptyInterrogationRow)`, `C(EmptyColloscopeGroupListRow)`.
   Mixed-cause coarse variants (`InvalidPairingRule` = parts-share-subject *or*
   dangling id, `InvalidGroupList`, `InvalidSlotPairingRule`) classify `false`: a
   `true` there would demand membership that rightly fails in compound states where
   old trips the fixable cause. (`P(InvalidWeek)`/mirror vestiges are unreachable;
   classify `false`.)

9. **Unit tests live in-crate** (`#[cfg(test)] mod tests`, pattern per
   `settings.rs:77`): corrupting state cannot go through ops (ops preserve invariants —
   that is the point), so tests need crate-internal field access. Forged ids via
   `unsafe { Id::new(n) }`; populated fixtures via the testgen bootstrap where useful
   (`collomatique-testgen-colloscopes` is already a dev-dependency).

10. **Naming**: `broken_invariants`, `FixableInvariant`, `LogicError`, `Convergence`,
    `NonEmptyRangeInclusive`. New module `state-colloscopes/src/invariants.rs`
    (foo.rs style), re-exported from `lib.rs`.

## 3. Classification rule

- **`DanglingFk(Reference)`** — the edge is in the refs registry
  (`InnerData::for_each_reference`, `refs.rs:530`) and its target id does not resolve.
  Some registry edges are additionally type-guaranteed (`WeekPeriodFk`: the `Periods`
  encapsulation makes a dangling week→period impossible); the sweep keeps them anyway —
  it is generic over the registry — they simply never fire.
- **`LogicError`** — truth decidable from a row's *own value* (or, for `DuplicatedId`,
  a whole-document id-uniqueness property): no *other* entity's state can flip it, so
  no legitimate op can produce it by side effect. Nothing that follows a reference
  belongs here.
- **`Convergence`** — a predicate over *existing* edges that legitimate ops can break
  indirectly (`UpdateSlot` changes a slot's subject → slot-pairing subject agreement;
  `UpdateSubject` turns interrogations off → every "subject has interrogations"
  referrer; `UpdateSubject` lengthens the duration → day overflow). The cascade
  resolves these lossily (clear the now-invalid data).

## 4. The vocabulary (final variant sets)

All types derive `Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord` and `thiserror`
`Display` (useful panic/report messages). No Serde (decision, §2.1/§1).

### `LogicError` (9 variants, declaration order = canonical order)

| Variant | Detects | Old-checker anchor |
|---|---|---|
| `DuplicatedId(u64)` | a raw id used twice across the shared u64 namespace | `check_no_duplicate_ids` `colloscope_params.rs:1112`, `lib.rs:159` |
| `EmptyAssignmentsRow(PeriodId, SubjectId)` | stored assignments row with empty student set | `:501` |
| `EmptySlotsRow(SubjectId)` | stored slots-ordering row with empty slot list | `:601` |
| `EmptyInterrogationRow(SlotId, WeekId)` | stored colloscope interrogation row with empty group set — **new**, today write-time-only (`colloscopes.rs` `set_interrogation`) | backfilled into the old checker in stage 6 |
| `EmptyColloscopeGroupListRow(GroupListId)` | stored colloscope group-list row with empty placement map — **new** (`set_group_list`) | backfilled in stage 6 |
| `PrefillGroupCountMismatch(GroupListId)` | prefilled groups count ≠ `group_names.len()` | `:709` |
| `DuplicatedStudentInPrefilledGroups(GroupListId)` | a student placed in two prefilled groups | `:715` |
| `PairingRulePartsShareSubject(PairingRuleId)` | `antecedent.subject_id == consequent.subject_id` | `:854` |
| `SlotPairingRulePartsShareSlot(SlotPairingRuleId)` | `antecedent.slot_id == consequent.slot_id` | `:912` |

Deliberately absent (and why):

- Periods mirror shapes, slots mirror shapes — type-encapsulated (decision 4).
- Empty-range shapes — unrepresentable after stage 1 (decision 5).
- `SlotsForSubjectWithoutInterrogations` as a *row* condition — implied by canonical
  form (a row exists iff the subject has ≥1 slot) plus the per-slot
  `Convergence::SlotForSubjectWithoutInterrogations`; its legacy mapping lives there.
- "Row for subject without slots" — that *is* `EmptySlotsRow`.

### `Convergence` (16 variants)

| Variant | Predicate (all "skip if a prerequisite ref dangles") |
|---|---|
| `SlotTeacherDoesNotTeachSubject(SlotId)` | slot's teacher's `subjects` lacks slot's subject (`:534`) |
| `TeacherSubjectWithoutInterrogations(TeacherId, SubjectId)` | teacher references a subject with `interrogation_parameters: None` (`:411`) |
| `SlotForSubjectWithoutInterrogations(SlotId)` | slot's subject has interrogations off (`:548`) |
| `SlotOverflowsDay(SlotId)` | `SlotWithDuration::new(start, subject.duration)` is `None` (`:551`) |
| `AssignmentForSubjectNotRunningOnPeriod(PeriodId, SubjectId)` | row's subject excludes the row's period (`:497`) |
| `AssignedStudentNotPresentForPeriod { period, subject, student }` | assigned student excludes the period (`:512`) |
| `AssociationForSubjectWithoutInterrogations(PeriodId, SubjectId)` | association's subject has interrogations off (`:785`) |
| `AssociationForSubjectNotRunningOnPeriod(PeriodId, SubjectId)` | association's subject excludes the period (`:788`) |
| `BalancingForSubjectWithoutInterrogations(SubjectId)` | balancing entry for interrogations-off subject (`validate_balancing`) |
| `PairedSlotsNotInSameSubject(SlotPairingRuleId)` | the two paired slots' subjects differ (`:923`) |
| `InterrogationSlotNotRunningOnPeriod(SlotId, WeekId)` | slot's subject excludes the week's period (`colloscopes.rs` validate) |
| `InterrogationOnInactiveWeek(SlotId, WeekId)` | `!params.is_week_active(week, slot.week_pattern)` |
| `InterrogationGroupOutOfBounds(SlotId, WeekId)` | an assigned group number ≥ the associated group list's `group_names.len()` |
| `ColloscopeGroupListPrefilled(GroupListId)` | colloscope row for a prefilled group list |
| `ColloscopeStudentExcluded(GroupListId, StudentId)` | placed student is in the automatic filling's `excluded_students` |
| `ColloscopeStudentGroupOutOfBounds(GroupListId, StudentId)` | student's group number ≥ `group_names.len()` |

No colloscope-row-level "subject without interrogations" variant: the per-slot flag
covers the condition, and in the old check order the params layer fires first there too,
so differential membership holds.

### `FixableInvariant`

```rust
pub enum FixableInvariant {
    DanglingFk(Reference),     // first: min() prefers the precise fix
    Convergence(Convergence),
}
```

No `dangling()` unwrap helper: that belonged to the earlier design where the sibling
variant was an unfixable logic error. With the `Result` split, every consumer of
`FixableInvariant` (the step-6 resolution map foremost) matches both variants
exhaustively — there is no caller entitled to panic on one of them.

## 5. Stages

### Stage 1 — non-empty ranges by construction

Make the four empty-range invariants unrepresentable, so they never enter the checker.

- **New type** (new module `state-colloscopes/src/non_empty_range.rs`, re-export in
  `lib.rs`):

  ```rust
  #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(try_from = "RangeInclusive<T>", into = "RangeInclusive<T>")]
  pub struct NonEmptyRangeInclusive<T: Ord + Clone>(RangeInclusive<T>);

  impl<T: Ord + Clone> NonEmptyRangeInclusive<T> {
      pub fn new(range: RangeInclusive<T>) -> Option<Self>; // None iff empty
  }
  // Deref<Target = RangeInclusive<T>> for reads; Into<RangeInclusive<T>>.
  ```

  Serde is required because the state-side entity structs derive `Serialize`
  (`subjects.rs:28`); the validating `try_from` deserialize keeps the honest-decode
  rule (no silent defaulting). Representation is identical to `RangeInclusive`.

- **Field swaps** (5 fields, 4 error shapes):
  `Subject` `students_per_group` (`subjects.rs:68`), `groups_per_interrogation` (`:86`),
  `interrogation_count_in_year` (`:151`), `interrogation_count_in_block` (`:216`);
  `GroupListParameters.students_per_group` (`group_lists.rs:197`). `Default` impls
  (`subjects.rs:231`, `group_lists.rs:206`) wrap their literals with
  `new(..).expect(..)` (statically non-empty).

- **Remove the now-unreachable checks and error variants**: the range tail of
  `validate_subject_internal` (`colloscope_params.rs:343-370`),
  `validate_group_list_params_internal` (`:690-697`), and
  `SubjectError::{StudentsPerGroupRangeIsEmpty, GroupsPerInterrogationRangeIsEmpty,
  InterrogationCountRangeIsEmpty}`, `GroupListError::StudentsPerGroupRangeIsEmpty`.
  Audit consumers matching those variants (gtk4/python expected to hold wildcard arms;
  fix any exact matches).

- **Storage**: the on-disk format structs are separate (`storage/src/format/subjects.rs:29`
  `Range<NonZeroU32>` with `min`/`max` JSON) — **bytes untouched**. The decode-side
  `range()` conversion helpers (`decode/spec2.rs:405`, `:672`, plus the
  interrogation-count sites) become fallible: an empty range in a file maps to a decode
  error (extend the decode error vocabulary as needed — hard error, never a default).
  Encode converts by deref — trivial.

- **Other construction sites** (mechanical): gtk4 editors
  (`gtk4/src/editor/group_lists.rs`, subject editors), `testgen-colloscopes/src/synth.rs`,
  `storage/tests/populated_round_trip/builder.rs`, state-colloscopes tests. Test code
  uses `new(..).unwrap()`.

- **Verification**: workspace build + tests; the byte-stability suite
  (`spec2_format.rs`, `populated_round_trip`, `all_examples_load_pristine` — hogwarts
  pristine, zero caveats) is the proof that the format is untouched. This stage is the
  likeliest to split (newtype + state-crate swap first, cross-crate fallout second).

### Stage 2 — the vocabulary types

- `refs.rs`: add `PartialOrd, Ord` to `Reference` and the eight `*RefSite` enums
  (id types already `Ord`; no `Hash` — `BTreeSet` needs only `Ord`).
- New `invariants.rs` with `LogicError`, `Convergence`, `FixableInvariant`,
  thiserror messages, module docs stating the classification rule of §3
  and the canonical-order contract.
- Tests: ordering pins — `DanglingFk < Convergence`; `LogicError` declaration order.
- No behavior change anywhere.

### Stage 3 — the dangling sweep (layer B)

- `InnerData::broken_invariants` first version: build the eight per-kind existence
  sets (periods via `periods.period_ids()`, weeks via `week_ids()`, subjects from
  `ordered_subject_list`, teachers/students/week-patterns/group-lists from their maps,
  slots via `slot_ids()`), then run `for_each_reference`; every edge whose target is
  absent yields `DanglingFk(reference)`. Always `Ok` at this stage.
- Dedup/order come free from `BTreeSet`. Two-sided rows produce one entry per id
  occurrence by construction (the registry's unit of account).
- Tests, per referenced kind with at least one site each (fixtures corrupt fields
  directly, or build `Slots`/`Periods` via their public `from_*_rows` constructors with
  forged ids where fields are private):
  - each family site yields exactly the expected `Reference`;
  - a two-sided row with *both* key components dangling yields two entries
    (assignments `(period, subject)`, colloscope `(slot, week)`);
  - clean bootstrap state ⇒ `Ok(empty)`.

### Stage 4 — logic errors (layer A, the `Err` path)

- Insert before the sweep: collect **all** `LogicError`s; non-empty ⇒ `Err(set)`.
  Checks (all trivial): duplicate-id sweep over `ids()` reporting each colliding raw
  id; canonical-absent over `assignments.iter()`, `slots.ordering_entries()`,
  `colloscope.iter()`, `colloscope.group_lists_iter()`; prefill count/duplicate-student
  on each group list; the two parts-share-an-id predicates.
- Tests: one corrupted fixture per variant (colloscope rows forged through
  crate-internal access, bypassing the canonicalizing setters — that is precisely the
  bug being simulated); multiple simultaneous logic errors all reported; the
  short-circuit (a state with both a logic error and a dangling ref returns `Err`
  without the dangling entry).

### Stage 5 — convergence (layer C)

- Hand-written walks mirroring the old semantics exactly (the stage-7 differential is
  the referee): slots (teacher-teaches, subject-has-interrogations, day overflow),
  teachers (`subjects` entries), assignments rows (subject-runs, student-present),
  association rows, balancing keys, slot pairings (subject agreement), colloscope
  interrogation rows (runs-on-period, active week, group bounds via the associated
  group list) and group-list rows (prefilled, excluded student, per-student bounds).
- Discipline: **skip, not unwrap**, when a prerequisite ref dangles
  (`let Some(x) = … else { continue }`) — the `DanglingFk` entry already reports it;
  layers B and C coexist on the `Ok` side. Type-guaranteed structure (mirrors,
  canonical form is layer-A-guaranteed) may be trusted.
- Where the old code consults the association to bound interrogation group numbers
  (`colloscopes.rs:168`), replicate its handling of a missing association.
- Tests: one fixture per variant; the skip-on-dangling case (corrupt a ref *and* the
  predicate behind it → only the `DanglingFk` entry, no panic).

### Stage 6 — old-checker parity (colloscope canonical-absent backfill)

- Add the two missing checks to the *old* architecture so it is actually the complete
  ground truth roadmap step 3 will audit against: `ColloscopeError` gains
  `EmptyInterrogationRow(SlotId, WeekId)` and `EmptyGroupListRow(GroupListId)`
  (naming kept in old style), and `Colloscope::validate_against_params` rejects
  stored empty rows. (`ColloscopeError`, not `InvariantError` — colloscope
  invariants live on that arm of `InnerDataError`.)
- Zero observable behavior change on real data: ops canonicalize at write time and
  the spec-2 codec routes through the canonicalizing sparse surface, so the states
  these checks reject are unreachable through every public path. Only in-crate
  corruption tests can exercise them — write those.
- Verify the existing suites stay green untouched (property harness,
  byte-stability, pristine examples).

### Stage 7 — legacy bridge + differential tests

- `LogicError::to_legacy(&self) -> InnerDataError` and
  `FixableInvariant::to_legacy(&self) -> InnerDataError` per the table in §6 —
  both total, thanks to stage 6.
- `InnerDataError::is_necessarily_logic_error(&self) -> bool` per decision 8 (true for exactly
  the six all-causes-tier-2 variants).
- Differential unit test over *every* fixture from stages 3–6 plus clean states and
  a few deliberate compound states (logic + fixable corruption together), asserting
  the three requirements of decision 8. The compound states specifically pin the
  lenient branch: old may report a fixable error first while new is `Err` — verdicts
  must still agree.

## 6. Legacy conversion table

`P(x)` = `InnerDataError::Params(InvariantError::x)`, `C(x)` =
`InnerDataError::ColloscopeError(ColloscopeError::x)`.

**`DanglingFk`, by site** (verified against the old check order during implementation —
the listed variant is the one the old checker actually emits first for that condition):

| Site | Legacy |
|---|---|
| `Period@WeekPeriodFk` | `P(InvalidWeek)` (type-guaranteed; never fires) |
| `Period@SubjectExcludedPeriods` | `P(InvalidSubject)` |
| `Period@StudentExcludedPeriods` | `P(InvalidStudent)` |
| `Period@PairingRuleExcludedPeriods` | `P(InvalidPairingRule)` |
| `Period@SlotPairingRuleExcludedPeriods` | `P(InvalidSlotPairingRule)` |
| `Period@AssignmentsKey` | `P(InvalidPeriodIdInAssignements)` |
| `Period@AssociationEntry` | `P(WrongPeriodCountInSubjectAssociationsForGroupLists)` (`:774`) |
| `Week@WeekPatternExcludedWeek` | `P(InvalidWeekPattern)` |
| `Week@ColloscopeInterrogation` | `C(InvalidWeekId)` |
| `Subject@TeacherSubjects` | `P(InvalidTeacher)` |
| `Subject@SlotSubject` | `P(SlotsForSubjectWithoutInterrogations)` — the missing subject is not in the filtered set at `:598`, so that check fires before `validate_slot_internal` |
| `Subject@IncompatSubject` | `P(InvalidIncompat)` |
| `Subject@PairingRuleAntecedent` / `Consequent` | `P(InvalidPairingRule)` |
| `Subject@BalancingSubjectKey` | `P(InvalidSubjectIdInBalancing)` |
| `Subject@AssignmentsKey` | `P(InvalidSubjectIdInAssignments)` |
| `Subject@AssociationEntry` | `P(InvalidSubjectIdInSubjectAssociations)` |
| `Teacher@SlotTeacher` | `P(InvalidSlot)` |
| `Student@GroupListPrefilledStudent` / `ExcludedStudent` | `P(InvalidGroupList)` |
| `Student@SettingsStudentKey` | `P(InvalidStudentIdInSettings)` |
| `Student@AssignmentsStudent` | `P(InvalidStudentIdInAssignments)` |
| `Student@ColloscopeGroupListStudent` | `C(InvalidStudentId)` |
| `WeekPattern@SlotWeekPattern` | `P(InvalidSlot)` |
| `WeekPattern@IncompatWeekPattern` | `P(InvalidIncompat)` |
| `Slot@SlotPairingRuleAntecedent` / `Consequent` | `P(InvalidSlotPairingRule)` |
| `Slot@ColloscopeInterrogation` | `C(InvalidSlotId)` |
| `GroupList@AssociationEntry` | `P(InvalidGroupListIdInSubjectAssociations)` |
| `GroupList@ColloscopeGroupListKey` | `C(InvalidGroupListId)` |

**`Convergence`:**

| Variant | Legacy |
|---|---|
| `SlotTeacherDoesNotTeachSubject` | `P(InvalidSlot)` |
| `TeacherSubjectWithoutInterrogations` | `P(InvalidTeacher)` |
| `SlotForSubjectWithoutInterrogations` | `P(SlotsForSubjectWithoutInterrogations)` |
| `SlotOverflowsDay` | `P(InvalidSlot)` |
| `AssignmentForSubjectNotRunningOnPeriod` | `P(AssignmentForSubjectNotRunningOnPeriod)` |
| `AssignedStudentNotPresentForPeriod` | `P(AssignedStudentNotPresentForPeriod)` |
| `AssociationForSubjectWithoutInterrogations` | `P(SubjectAssociationForSubjectWithoutInterrogations)` |
| `AssociationForSubjectNotRunningOnPeriod` | `P(SubjectAssociationForSubjectNotRunningOnPeriod)` |
| `BalancingForSubjectWithoutInterrogations` | `P(BalancingForSubjectWithoutInterrogations)` |
| `PairedSlotsNotInSameSubject` | `P(InvalidSlotPairingRule)` |
| `InterrogationSlotNotRunningOnPeriod(s, w)` | `C(SlotNotRunningOnPeriod(s, w))` |
| `InterrogationOnInactiveWeek(s, w)` | `C(InterrogationOnInactiveWeek(s, w))` |
| `InterrogationGroupOutOfBounds(s, w)` | `C(InvalidGroupNumInInterrogation(s, w))` |
| `ColloscopeGroupListPrefilled(g)` | `C(PrefilledGroupListInColloscope(g))` |
| `ColloscopeStudentExcluded(g, s)` | `C(ExcludedStudentInGroupList(g, s))` |
| `ColloscopeStudentGroupOutOfBounds(g, s)` | `C(InvalidGroupNumForStudentInGroupList(g, s))` |

**`LogicError`:**

| Variant | Legacy |
|---|---|
| `DuplicatedId(_)` | `InnerDataError::DuplicateIds` (the top-level check fires before params') |
| `EmptyAssignmentsRow` | `P(EmptyAssignmentRow)` |
| `EmptySlotsRow` | `P(EmptySlotsRow)` |
| `EmptyInterrogationRow(s, w)` | `C(EmptyInterrogationRow(s, w))` — stage-6 backfill |
| `EmptyColloscopeGroupListRow(g)` | `C(EmptyGroupListRow(g))` — stage-6 backfill |
| `PrefillGroupCountMismatch` | `P(InvalidGroupList)` |
| `DuplicatedStudentInPrefilledGroups` | `P(InvalidGroupList)` |
| `PairingRulePartsShareSubject` | `P(InvalidPairingRule)` |
| `SlotPairingRulePartsShareSlot` | `P(InvalidSlotPairingRule)` |

## 7. End-of-step verification

- Per stage: `cargo build --workspace` + `cargo test -p collomatique-state-colloscopes`;
  `cargo fmt --all`. No clippy.
- Stage 1 additionally: full workspace tests + the byte-stability suite + pristine
  examples (decode path changed).
- End of step: `cargo test --workspace`; `Cargo.lock` expected unchanged (no new deps
  ⇒ no Nix cargoHash refresh).
- Design-doc §8 step-2 markers and this plan's retirement happen at the step's
  close-out, per the step-1 convention.
