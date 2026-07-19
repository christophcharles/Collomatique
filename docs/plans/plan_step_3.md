# Step 3 — old-checker completeness survey (session plan + audit record)

**Status:** survey ran July 19 2026. **Doc-only step**: no code is touched, even where a
finding would suggest a change — findings are recorded (§5) and decided on separately.
File/line references are against the tree at commit `0a1041b6` (July 19 2026); the
file + function/variant names are the stable part.

## 1. Goal and method

The design doc's original step-3 text aimed the audit at the *new* checker ("anything the
new checker misses … at the end the new checker is ground truth"). That was wrong. The
**old** checker (`InnerData::check_invariants`, `lib.rs:175`) is the *reference oracle* of
the step-4 differential fuzz — the fuzz asserts "old rejects iff new reports non-empty", so
a fuzz disagreement is only meaningful if the old checker is known-complete. This survey
certifies exactly that, by answering two questions:

1. **Ops ⊆ old checker** (§2): does the elementary-op code enforce any *invariant* — a
   property every valid `InnerData` must satisfy — that the old checker does not check? Such
   a check would let invalid fuzz states silently pass the reference.
2. **Missed by everything** (§4): is there any invariant checked *nowhere* — neither in the
   op code nor in the old checker?

Checks of the *transition* rather than the state are out of scope by design (the §4
carve-out of the design doc): no-clobber, op-target existence, position bounds, anchor
targeting, empty-first protocol preconditions, immutability guards. They are inventoried in
§3 so that every checker-absent op check is *explicitly* accounted for, not silently skipped.

The two audit arrows compose. The July-19 review pass recorded in the design doc §8
established **old ⊆ new** (each of the old checker's 57 conditions maps to a
`LogicError` / `DanglingFk` site / `Convergence` variant). This survey establishes
**ops ⊆ old**. Together: every invariant enforced anywhere in the crate is visible to both
checkers — which is precisely what the step-4 verdict differential requires.

Method: two independent code sweeps (one enumerating the old checker's conditions, one
enumerating every check in the 16 `apply_*` paths and the shared validators), then a
row-by-row reconciliation, then a field-by-field walk of the data model as the
"missed by everything" backstop, cross-checked against the design doc's Appendix A.1
(28 ID relationships) and A.2 (value/shape checks).

### Structure facts the tables rely on

- Every `apply_*` is validate-before-mutate (no phase split, no rollback); an error leaves
  `InnerData` untouched. After every successful apply, `lib.rs:340` runs the full old
  checker as a panic net, so op/checker drift would surface as a production panic — *if*
  the checker knows the invariant. That conditional is what this survey discharges.
- `GlobalUpdate` (`lib.rs:335`) gates on the full old checker directly — trivially covered.
- Add/Update payload validation goes through the **same** `validate_*` helpers that the
  checker's `check_*_data_consistency` families call. For those checks, op/checker drift is
  structurally impossible; they collapse to one row per helper in Table 1.

## 2. Table 1 — every invariant-guarding op check → its old-checker twin

### 2.1 Shared validators (drift structurally impossible)

The op path and the whole-model checker call the same function; columns give both call
sites.

| Validator (conditions bundled) | Op call sites | Checker call site |
|---|---|---|
| `validate_student_internal` (excluded periods resolve) | StudentAdd `students.rs:90`, StudentUpdate `:162` | `check_students_data_consistency` `colloscope_params.rs:438` |
| `validate_subject_internal` (excluded periods resolve) | SubjectAddAfter `subjects.rs:336`, SubjectUpdate `:476` | `check_subjects_data_consistency` `colloscope_params.rs:359` |
| `validate_teacher_internal` (subjects resolve + have interrogations) | TeacherAdd `teachers.rs:80`, TeacherUpdate `:112` | `check_teachers_data_consistency` `colloscope_params.rs:398` |
| `validate_slot_internal` (teacher resolves, teacher teaches subject, week pattern resolves, subject resolves + has interrogations, slot fits in day) | SlotAddAfter `slots.rs:403`, SlotUpdate `:539` | `check_slots_data_consistency` `colloscope_params.rs:581-591` |
| `validate_incompat_internal` (subject + week pattern resolve; deliberately no has-interrogations requirement, see `incompats.rs:28-37`) | IncompatAdd `incompats.rs:96`, IncompatUpdate `:115` | `check_incompats_data_consistency` `colloscope_params.rs:646` |
| `validate_group_list_internal` (+`_filling_`: prefill count matches names, no duplicate prefilled student, prefilled/excluded student ids resolve) | GroupListAdd `group_lists.rs:372`, GroupListUpdate `:535`, GroupListSetFilling `:620` | `check_group_lists_data_consistency` `colloscope_params.rs:746-749` |
| `validate_settings` (per-student keys resolve) | SettingsUpdate `settings.rs:68` | `check_settings_data_consistency` `colloscope_params.rs:776-778` |
| `validate_balancing` (per-subject keys resolve + have interrogations) | BalancingUpdate `balancing.rs:98` | `check_balancing_data_consistency` `colloscope_params.rs:953-958` |
| `validate_pairing_rule_internal` (parts' subjects differ, resolve; excluded periods resolve) | PairingAdd `pairings.rs:106`, PairingUpdate `:125` | `check_pairings_data_consistency` `colloscope_params.rs:940` |
| `validate_slot_pairing_rule_internal` (parts' slots differ, resolve, share a subject; excluded periods resolve) | SlotPairingAdd `slot_pairings.rs:97`, SlotPairingUpdate `:121` | `check_slot_pairings_data_consistency` `colloscope_params.rs:922` |
| `validate_week_pattern` (excluded weeks resolve) | WeekPatternAdd `week_patterns.rs:120`, WeekPatternUpdate `:174` | `check_week_pattern_data_consistency` `colloscope_params.rs:985-988` |
| `validate_group_list_placements` (student not excluded, resolves, group number in bounds) | ColloscopeSetGroupList `colloscopes.rs:345` | `validate_against_params` `colloscopes.rs:211` |

### 2.2 Hand-written guards (the drift-able surface — each row individually reconciled)

"Twin" = the old-checker condition that fires on the state the guard prevents.

**Student ops** (`students.rs`, `apply_student`):

| Guard (op, site) | Prevents | Twin |
|---|---|---|
| Remove: no colloscope placement `students.rs:106-113` | dangling student key in a colloscope group-list row | `ColloscopeError::InvalidStudentId` `colloscopes.rs:245` |
| Remove: not excluded by any group list `:120` | dangling id in `Automatic{excluded_students}` | `InvalidGroupList` via `colloscope_params.rs:681-687` |
| Remove: not in any prefilled group `:126` | dangling id in `Prefilled{groups[].students}` | `InvalidGroupList` via `colloscope_params.rs:673-679` |
| Remove: no assignment references the student `:134-144` | dangling id in an assignments row | `InvalidStudentIdInAssignments` `colloscope_params.rs:473-477` (guard scans only non-excluded periods — complete on valid input, since an assignment on an excluded period is already invalid per `:479`) |
| Remove: no settings entry `:147` | dangling settings key | `InvalidStudentIdInSettings` `colloscope_params.rs:776` |
| Update: no assignment on newly-excluded periods `:168-181` | assigned student not present for period | `AssignedStudentNotPresentForPeriod` `colloscope_params.rs:479-481` |

**Period ops** (`periods.rs`, `apply_period`):

| Guard | Prevents | Twin |
|---|---|---|
| Remove: no colloscope row on the period's weeks `periods.rs:628-641` | dangling week key in colloscope | `ColloscopeError::InvalidWeekId` `colloscopes.rs:148` (vacuous after the `PeriodStillHasWeeks` protocol guard `:614-621`; commented as such in code) |
| Remove: no subject excludes it `:644-651` | dangling period in `Subject.excluded_periods` | `InvalidSubject` via `colloscope_params.rs:359` |
| Remove: no student excludes it `:654-659` | dangling period in `Student.excluded_periods` | `InvalidStudent` via `colloscope_params.rs:438` |
| Remove: no pairing rule excludes it `:662-668` | dangling period in `PairingRule.excluded_periods` | `InvalidPairingRule` via `colloscope_params.rs:819-822` |
| Remove: no slot-pairing rule excludes it `:670-682` | dangling period in `SlotPairingRule.excluded_periods` | `InvalidSlotPairingRule` via `colloscope_params.rs:883-886` |
| Remove: no assignments row `:686-696` | dangling period in assignments key | `InvalidPeriodIdInAssignements` `colloscope_params.rs:456-458` |
| Remove: no association row `:698-709` | dangling period in association key | `WrongPeriodCountInSubjectAssociationsForGroupLists` `colloscope_params.rs:728-730` |

**Week ops** (`periods.rs`, `apply_week`):

| Guard | Prevents | Twin |
|---|---|---|
| Remove: no pattern excludes the week `periods.rs:835-841` | dangling week in `WeekPattern.excluded_weeks` | `InvalidWeekPattern` `colloscope_params.rs:985-988` |
| Remove: no colloscope row on the week `:845-853` | dangling week key in colloscope | `ColloscopeError::InvalidWeekId` `colloscopes.rs:148` |
| Update: interrogations→off blocked while a colloscope row sits on the week `:899-908` | interrogation on an inactive week | `InterrogationOnInactiveWeek` `colloscopes.rs:165` — **covers the week's own flag**: it goes through `is_week_active`, which checks `week_desc.interrogations` (`week_patterns.rs:61`), not just pattern exclusion |
| Move: each row's subject runs on the destination period `:966-982` | slot not running on its week's period | `SlotNotRunningOnPeriod` `colloscopes.rs:159-163` (inline re-implementation — see finding F2) |
| Move: each row's groups fit the destination bound `:983-1000` | group number ≥ association bound | `InvalidGroupNumInInterrogation` `colloscopes.rs:186-192` (same inline re-implementation) |

**Subject ops** (`subjects.rs`, `apply_subject`):

| Guard | Prevents | Twin |
|---|---|---|
| Remove: no balancing override `subjects.rs:388` | dangling subject in balancing key | `InvalidSubjectIdInBalancing` `colloscope_params.rs:953` |
| Remove: no pairing rule references it `:392-396` | dangling subject in a rule part | `InvalidPairingRule` via `colloscope_params.rs:813-818` |
| Remove: no association references it `:398-412` | dangling subject in association key | `InvalidSubjectIdInSubjectAssociations` `colloscope_params.rs:734-737` |
| Remove: subject has no slots `:414-418` | dangling `Slot.subject_id` (+ orphaned ordering row) | `InvalidSlot` via subject-resolve `colloscope_params.rs:512-514`; ordering row also trips `:565-580` |
| Remove: no teacher lists it `:420-426` | dangling subject in `Teacher.subjects` | `InvalidTeacher` via `colloscope_params.rs:375-377` |
| Remove: no incompat references it `:428-436` | dangling `Incompatibility.subject_id` | `InvalidIncompat` via `colloscope_params.rs:613-615` |
| Remove: no assignments row `:440-450` | dangling subject in assignments key | `InvalidSubjectIdInAssignments` `colloscope_params.rs:460-462` |
| Update→no-interrogations: no balancing override `:495` | balancing for interrogation-less subject | `BalancingForSubjectWithoutInterrogations` `colloscope_params.rs:956-958` |
| Update→no-interrogations: no teacher references it `:500-507` | teacher subject without interrogations | `InvalidTeacher` via `colloscope_params.rs:378-380` |
| Update→no-interrogations: no association `:510-524` | association for interrogation-less subject | `SubjectAssociationForSubjectWithoutInterrogations` `colloscope_params.rs:739-741` |
| Update→no-interrogations: zero slots `:528-537` | slots for interrogation-less subject | `SlotsForSubjectWithoutInterrogations` `colloscope_params.rs:565-567` + `InvalidSlot` via `:515-517` |
| Update, newly-excluded period: no assignment there `:552-563` | assignment for subject not running on period | `AssignmentForSubjectNotRunningOnPeriod` `colloscope_params.rs:464-466` |
| Update, newly-excluded period: no association there `:565-577` | association for subject not running on period | `SubjectAssociationForSubjectNotRunningOnPeriod` `colloscope_params.rs:742-744` |
| Update, newly-excluded period: no non-empty colloscope slot there `:583-601` | interrogation whose slot doesn't run on the period | `SlotNotRunningOnPeriod` `colloscopes.rs:159-163` |

(Update→no-interrogations deliberately does *not* re-check pairing/incompat references —
those edges do not require the subject to run interrogations, in both the validators and the
checker. Asymmetry is justified; documented on `Incompatibility::subject_id`.)

**Teacher ops** (`teachers.rs`):

| Guard | Prevents | Twin |
|---|---|---|
| Remove: no slot references the teacher `teachers.rs:95-99` | dangling `Slot.teacher_id` | `InvalidSlot` via teacher-resolve `colloscope_params.rs:498-500` |
| Update: no slot binds the teacher to a dropped subject `:118-136` | slot whose teacher doesn't teach its subject | `InvalidSlot` via `TeacherDoesNotTeachInSubject` `colloscope_params.rs:501-506` |

**Assignment op** (`assignments.rs`, `Assign`):

| Guard | Prevents | Twin |
|---|---|---|
| Subject runs on the period `assignments.rs:106-111` | assignment for excluded subject×period | `AssignmentForSubjectNotRunningOnPeriod` `colloscope_params.rs:464-466` |
| Student present on the period `:119-124` | assigned student excluded from period | `AssignedStudentNotPresentForPeriod` `colloscope_params.rs:479-481` |

(The period/subject/student *existence* checks `:88-117` are input-targeting — a `true`
assign on dangling ids would create a dangling row, so they double as invariant guards;
twins `InvalidPeriodIdInAssignements`/`InvalidSubjectIdInAssignments`/
`InvalidStudentIdInAssignments` all exist. Row canonicality — empty set clears the row — is
maintained at `:133-144`; twin `EmptyAssignmentRow` `colloscope_params.rs:468-470`.)

**Week-pattern ops** (`week_patterns.rs`):

| Guard | Prevents | Twin |
|---|---|---|
| Remove: no slot references the pattern `week_patterns.rs:141-149` | dangling `Slot.week_pattern` | `InvalidSlot` via pattern-resolve `colloscope_params.rs:507-511` |
| Remove: no incompat references the pattern `:151-161` | dangling `Incompatibility.week_pattern_id` | `InvalidIncompat` via `colloscope_params.rs:616-619` |
| Update: no colloscope row on a week the new exclusions silence `:194-211` | interrogation on inactive week | `InterrogationOnInactiveWeek` `colloscopes.rs:165-169` |

**Slot ops** (`slots.rs`):

| Guard | Prevents | Twin |
|---|---|---|
| Remove: no colloscope row on the slot `slots.rs:474-488` | dangling slot key in colloscope | `ColloscopeError::InvalidSlotId` `colloscopes.rs:152-154` |
| Remove: no slot-pairing rule references it `:490-500` | dangling slot in a rule part | `InvalidSlotPairingRule` via `colloscope_params.rs:871-876` |
| Update: colloscope rows stay on weeks active under the new pattern `:545-557` | interrogation on inactive week | `InterrogationOnInactiveWeek` `colloscopes.rs:165-169` |

**Group-list ops** (`group_lists.rs`):

| Guard | Prevents | Twin |
|---|---|---|
| Remove (automatic): no colloscope placement row `group_lists.rs:406-409` | dangling list key in colloscope | `ColloscopeError::InvalidGroupListId` `colloscopes.rs:202-205` |
| Remove: no association references it `:413-423` | dangling list in association value | `InvalidGroupListIdInSubjectAssociations` `colloscope_params.rs:731-733` |
| Update: existing placements stay valid `:455-469` | excluded/over-bound placement | `validate_group_list_placements` `colloscopes.rs:211` (shared, §2.1) |
| Update: interrogation groups stay under the new bound `:473-488` (`check_interrogations_group_bound` `:317-346`) | group number ≥ association bound | `InvalidGroupNumInInterrogation` `colloscopes.rs:186-192` |
| SetFilling auto→prefilled: no colloscope placement row `:574-587` | placements for a prefilled list | `PrefilledGroupListInColloscope` `colloscopes.rs:206-210` |
| SetFilling staying automatic: placements respect new exclusions `:591-611` | excluded student placed | `ExcludedStudentInGroupList` `colloscopes.rs:238-243` |
| AssignToSubject: subject has interrogations `:640-642` | association for interrogation-less subject | `SubjectAssociationForSubjectWithoutInterrogations` `colloscope_params.rs:739-741` |
| AssignToSubject: subject runs on the period `:643-648` | association for excluded subject×period | `SubjectAssociationForSubjectNotRunningOnPeriod` `colloscope_params.rs:742-744` |
| AssignToSubject: existing interrogation groups fit the new list's bound `:671-675` | group number ≥ association bound | `InvalidGroupNumInInterrogation` `colloscopes.rs:186-192` |

**Colloscope ops** (`colloscopes.rs`, `apply_colloscope`):

| Guard | Prevents | Twin |
|---|---|---|
| SetGroupList: list is not prefilled `colloscopes.rs:340-342` | placements for a prefilled list | `PrefilledGroupListInColloscope` `colloscopes.rs:206-210` |
| SetGroupList: placements valid `:345-351` | excluded/dangling/over-bound placement | shared `validate_group_list_placements` (§2.1) |
| SetInterrogation: slot's subject has interrogations & runs on the period `:386-390` | slot not running on period | `SlotNotRunningOnPeriod` `colloscopes.rs:159-163` |
| SetInterrogation: week active for the slot's pattern `:393-397` | interrogation on inactive week | `InterrogationOnInactiveWeek` `colloscopes.rs:165-169` |
| SetInterrogation: group numbers under the association bound `:401-422` | group number ≥ bound | `InvalidGroupNumInInterrogation` `colloscopes.rs:186-192` |

(Row canonicality — empty payload clears the row — is maintained by the sparse writer
surface; twins `EmptyInterrogationRow`/`EmptyGroupListRow` `colloscopes.rs:145/199`.)

**ExportConfig op**: no checks; the config is pure value data (colors, booleans, strings) —
no invariants exist, consistent with the empty `ExportConfigError`.

**Verdict of Table 1: every invariant-guarding op check has an old-checker twin.** No
op-only invariants exist.

## 3. Table 2 — carve-out register (checker-absent by design)

Every op check *not* mapped in Table 1, with its §4-carve-out category. These are
properties of the transition or of the op's inputs, not of the resulting state — the old
checker is *correct* not to know them.

| Category | Checks |
|---|---|
| **No-clobber** (fresh id is fresh; prevents data erasure + preserves reversibility) | `*IdAlreadyExists` in every Add: student `students.rs:87`, teacher `teachers.rs:77`, subject `subjects.rs:327`, period `periods.rs:549/572`, week `periods.rs:804`, week pattern `week_patterns.rs:110`, slot `slots.rs:400`, incompat `incompats.rs:87`, group list `group_lists.rs:357`, pairing `pairings.rs:97`, slot pairing `slot_pairings.rs:87` |
| **Op-target existence** (`Invalid*Id` on the entity being updated/removed — the op must name a live target; the *resulting* state carries no trace) | student `students.rs:101/163`; period `periods.rs:601`; week `periods.rs:830/885/933`; subject `subjects.rs:370/383/477`; teacher `teachers.rs:91/113`; week pattern `week_patterns.rs:131/178`; slot `slots.rs:439/463/520`; incompat `incompats.rs:107/117`; group list `group_lists.rs:387/441/550`; pairing `pairings.rs:117/127`; slot pairing `slot_pairings.rs:107/125` |
| **Parameter targeting** (op inputs that must resolve for the op to be meaningful; where a bad input *would* leave a trace, Table 1 lists the twin) | `AddAfter` anchors: period `periods.rs:582`, week `periods.rs:776`, subject `subjects.rs:338-345`, slot `slots.rs:406-412` + same-subject anchor `PreviousSlotIsNotInRightSubject` `slots.rs:413-417`; `Assign` coordinates `assignments.rs:88-117`; `SetInterrogation` coordinates `colloscopes.rs:373-381`; `SetGroupList` target `colloscopes.rs:326-334`; `AssignToSubject` coordinates `group_lists.rs:635-639/649-657/660-668`; `WeekMove` destination `periods.rs:937-945` |
| **Position bounds** | `InvalidPosition` `periods.rs:812/955`; `PositionOutOfBounds` `subjects.rs:364`, `slots.rs:448-456` |
| **Empty-first protocol preconditions** (op-ordering discipline; the state they demand is valid either way) | `PeriodStillHasWeeks` `periods.rs:614-621` (skipping it would orphan weeks — the checker's mirror sweep `colloscope_params.rs:1054-1057` catches that state); `RemainingFilling` `group_lists.rs:396-404`; `NonEmptyGroupsWhenReducing` `group_lists.rs:501-507` |
| **Immutability** | `CannotChangeSubject` `slots.rs:531-537` (a slot's subject is fixed at creation; changing it would require an ordering-sidecar move the op surface deliberately doesn't offer) |
| **Payload shape at the op boundary** | `PrefillGroupCountMismatch` `group_lists.rs:562-568` — dual-listed: it also has the invariant twin `colloscope_params.rs:664-668` (and the op re-validates via `validate_group_list` at `:620`), so nothing is lost |

## 4. Table 3 — field-coverage sweep (the "missed by everything" backstop)

Every field of `InnerData`, walked; for each id-bearing or shape-bearing field, the covering
old-checker condition — or the mechanism that makes a check unnecessary. Value-only fields
(names, strings, colors, costs, booleans, soft params, time values with valid-by-type
shapes) are marked *value-only* and carry no invariant.

**Id namespaces**: `Parameters::all_ids` (`colloscope_params.rs:300-316`) enumerates all 11
id-owning kinds — students, periods, weeks, subjects, teachers, week patterns, slots,
incompats, group lists, pairings, slot pairings — feeding the top-level duplicate-id sweep
(`lib.rs:163-176`). The colloscope and export config own no ids (colloscope keys borrow
params ids; dangling ones are its validation's business). Complete.

| Field | Coverage |
|---|---|
| `Parameters.periods.first_week` | value-only (`Option<WeekStart>`) |
| `Parameters.periods.{ordered_period_list, week_map}` (private) | list↔map mirror checked ×4: week exists, `period_id` agrees, no duplicate listing, no orphan (`colloscope_params.rs:1041-1057`); within-period *ordering* is encapsulated (§6c of the design doc) — compound mutators only |
| `Week.{interrogations, annotation}` | value-only |
| `Subject.excluded_periods` | resolve: `colloscope_params.rs:359` |
| `Subject.parameters` | value-only throughout — verified id-free: name, `Option<SubjectInterrogationParameters>` (two `NonEmptyRangeInclusive` fields — empty ranges unrepresentable by type, step-2 stage 1 —, `NonZeroMinutes` duration, `SubjectPeriodicity`/`WeekBlock` hold only counts/delays) |
| `Teacher.desc` | value-only (`PersonWithContact`) |
| `Teacher.subjects` | resolve + has-interrogations: `colloscope_params.rs:398` |
| `Student.desc` / `Student.excluded_periods` | value-only / resolve: `colloscope_params.rs:438` |
| `Assignments.map` key `(PeriodId, SubjectId)` | resolve ×2: `colloscope_params.rs:456/460`; convergence subject-runs-on-period `:464` |
| `Assignments.map` value `BTreeSet<StudentId>` | resolve `:473`, student-present-for-period `:479`, canonical non-empty `:468` |
| `WeekPattern.name` / `.excluded_weeks` | value-only / resolve: `colloscope_params.rs:985` (may reference non-interrogation weeks — deliberate, documented at `week_patterns.rs:31-40`) |
| `Slots.{slot_map, ordering}` (private) | sidecar mirror checked: row subject has interrogations `:565`, row non-empty `:568`, slot exists `:572`, `subject_id` agrees `:575`, listed once `:578`, no orphan `:596`; within-subject ordering encapsulated |
| `Slot.subject_id` / `.teacher_id` / `.week_pattern` | resolve `colloscope_params.rs:512/498/507`; convergence teacher-teaches-subject `:501`, subject-has-interrogations `:515` |
| `Slot.start_time` (+ subject duration) | day-overflow: `colloscope_params.rs:518-522` |
| `Slot.{extra_info, cost}` | value-only |
| `Incompatibility.subject_id` / `.week_pattern_id` | resolve `colloscope_params.rs:613/616` (no has-interrogations requirement — deliberate, documented `incompats.rs:28-37`) |
| `Incompatibility.{name, slots}` | value-only |
| `GroupList.params` | name value-only; `students_per_group` empty-unrepresentable by type; `group_names` shape consumed by the bound checks below |
| `GroupList.filling` | Prefilled: count matches `group_names` `colloscope_params.rs:664`, no duplicate student `:670`, students resolve `:673`; Automatic: excluded students resolve `:683` |
| `GroupLists.subjects_associations` key/value | key resolves ×2 `colloscope_params.rs:728/734`; convergence has-interrogations `:739`, runs-on-period `:742`; value list resolves `:731` |
| `Settings.global` / `Limits` | value-only |
| `Settings.students` keys | resolve: `colloscope_params.rs:776` |
| `PairingRule.{antecedent, consequent}.subject_id` | resolve `colloscope_params.rs:813/816`; parts differ `:808` |
| `PairingRule.excluded_periods` / `.{should_have, soft}` | resolve `:819` / value-only |
| `SlotPairingRule.{antecedent, consequent}.slot_id` | resolve `colloscope_params.rs:871/874`; parts differ `:866`; same subject `:877` |
| `SlotPairingRule.excluded_periods` / rest | resolve `:883` / value-only |
| `Balancing.global` / `BalancingOptions` | value-only |
| `Balancing.subjects` keys | resolve + has-interrogations: `colloscope_params.rs:953/956` |
| `Colloscope.interrogations` key `(SlotId, WeekId)` | resolve: week `colloscopes.rs:148`, slot `:152`; convergence slot-runs-on-period `:159`, week-active `:165` |
| `Colloscope.interrogations` value `BTreeSet<u32>` | bound vs association `colloscopes.rs:186` (no association ⇒ bound 0 ⇒ any group invalid, `:171-185`); canonical non-empty `:145` |
| `Colloscope.group_lists` key | resolve `colloscopes.rs:202`; not prefilled `:206` |
| `Colloscope.group_lists` value `BTreeMap<StudentId, u32>` | student not excluded `:238`, resolves `:245`, group in bounds `:249`; canonical non-empty `:199` |
| `ExportConfig.*` | value-only throughout; no ids, no invariants, no checks anywhere — consistent |

Cross-checks: all 28 Appendix-A.1 relationships appear above; every Appendix-A.2 check is
either present (group bounds, pair predicates, side-constraints), dissolved by the step-1
reshapes (dense counts), or lives in the two encapsulated mirrors. The documented deliberate
non-checks (design doc §8 step 3) are confirmed: incompat-no-interrogations, encapsulated
mirrors' internal ordering, id-issuer high-water at `Data` level.

**Verdict of Table 3: no invariant is checked nowhere.**

## 5. Findings

**No gaps.** The old checker is complete with respect to everything the op code enforces
and everything the data model can express — it is fit to serve as the step-4 fuzz reference.
Observations for the record (no action in this step):

- **F1 — vacuous guard**: `NotEmptyPeriodInColloscope` (`periods.rs:628-641`) can never
  fire after the `PeriodStillHasWeeks` guard; already commented in code as defensive
  redundancy. Disappears with all preconditions at step 5.
- **F2 — inline duplication**: `WeekMove` (`periods.rs:966-1000`) re-implements the
  checker's slot-runs-on-period + group-bound logic against the destination period instead
  of calling shared helpers — the one hand-rolled spot with genuine drift *risk*. Not
  refactored: step 5 deletes all preconditions; recorded here so any pre-step-5 touch of
  that code knows to re-verify the pairing.
- **F3 — check-order oddity**: `GroupListAssignToSubject` tests the subject before the
  period exists (`group_lists.rs:635/649`); a dangling period reaches the
  `SubjectDoesNotRunOnPeriod` test first, harmlessly. Error-variant choice quirk only.
- **F4 — justified asymmetry**: `SubjectUpdate`(interrogations→off) re-checks
  balancing/teachers/associations/slots but not pairing/incompat references — correct,
  those edges don't require interrogations (see Table 1 note).
- **F5 — colloscope placements need no association**: a `Colloscope.group_lists` row may
  reference a list not (yet) associated to any subject×period. Ops and both checkers agree
  this is valid (placements can be prepared before associating); noted so the step-6
  resolution map doesn't "fix" it.

## 6. Design-doc corrections made in this step

`docs/plans/invariant_cascade_design.md` §8 step 3 rewritten: the goal is certifying the
**old** checker as the reliable step-4 reference (the "new checker becomes ground truth"
sentence was wrong and is gone); the July-19 review pass is reframed as the *old ⊆ new*
arrow of the two-arrow argument, this survey being the *ops ⊆ old* arrow.

At close, this plan is retired from the tree per the house pattern (pinned via `git show`),
with the delivered-state summary recorded in the design doc §8.
