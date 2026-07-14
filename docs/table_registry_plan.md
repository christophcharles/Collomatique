# Table + relationship registry plan (phase 2, item 2)

Status: **agreed detailed plan** (July 13 2026, branch `consolidate_state`); §4.1, §4.2 and
§5 phase A amended July 13 2026 during the A1 session with the settled phase-A design
(validating `OrderedTable`, `References<K>` union parameterization, join machinery pulled
forward into a new A3 session).
Scope: phase 2 item 2 of `docs/state_consolidation_plan.md` — the generic `Table<Id, T>`
containers, the declare-once relationship ("FK") registry, and the staged migration of all
consumer code to the new SQL-like read interface.

This document is the per-item detailed plan that `state_consolidation_plan.md` §6 requires
before implementation ("each item below gets its own detailed plan (and user sign-off) before
implementation"). It supersedes the sketch in §6 item 2 of that document. **Read this whole
document before any session working on this item.** The work spans many sessions; §5 divides
it into commits sized so that each can be one session, each leaving the workspace building and
green.

File/line references are against the tree at commit `de8ed888` (July 13 2026). Line numbers
will rot as the work proceeds; the file + function names are the stable part.

---

## 1. Why

Today every referential relationship between entities ("slots hold a `TeacherId`") is
hand-coded at least three times in `state-colloscopes/`:

1. **Candidate validation** — `validate_*_internal` in `colloscope_params.rs` checks that a
   new/updated entity's outgoing references exist (`SlotError::InvalidTeacherId`, …).
2. **Delete blocking** — each `apply_*` `Remove`/`Update` path scans every container that could
   reference the entity and returns a typed error
   (`TeacherError::TeacherStillHasAssociatedSlots`, …). This is `ON DELETE RESTRICT` by hand.
3. **Whole-model consistency** — `check_*_data_consistency` re-walks the same relationships,
   discarding detail into `InvariantError` (colloscope-side relationships go through a separate
   pass, `Colloscope::validate_against_params` → `ColloscopeError`).

On top of that, `ops/` hand-codes ~21 reverse-lookup loops ("who references X?") in its
cleaning cascades, duplicate-ID scanning exists in three implementations, and the 10 typed ID
newtypes in `ids.rs` are 10 hand-written copies of the same 20 lines (since A2: one-line
`#[derive(EntityId)]`s).

The goal of this item: **declare each relationship once**, derive the mechanical parts
generically, and give consumer code a small SQL-like read interface (keyed lookup, FK
resolution, reverse lookup) instead of raw container-field access. The retired SQLite schema
(`git show 6e18c3d1~1:sqlite-state/src/schema.rs`) is the design reference: its FK set matches
the in-memory relationship inventory below 1:1 (`ON DELETE RESTRICT` ↔ today's delete-blocking
scans, `ON DELETE CASCADE` ↔ today's structural fan-out).

---

## 2. Decisions taken (July 13 2026)

1. **Proc-macro: yes, and the machinery is generic.** A new proc-macro crate
   (`state-derive/`, package `collomatique-state-derive`, re-exported through
   `collomatique-state`) plus the generic *runtime* in `state/`: `Table<I, T>`,
   `OrderedTable<I, T>`, the `Id` trait (moved from `state-colloscopes/src/ids.rs`, where it is
   hand-implemented 10×), and the reference-enumeration traits the generated code targets.
   Rationale: clearer declaration sites, and the rooms-organisation side-project can reuse the
   same structure long-term.
2. **Macro scope rule.** The derive covers the *regular* FK shapes only: plain `Id` field,
   `Option<Id>`, homogeneous collections of ids (`BTreeSet<Id>`, `Vec<Id>`). Irregular shapes —
   references inside enum variants (`GroupListFilling`), dense mirrors whose "referencing
   entity" is a container (assignments, associations, colloscope) — use a **manual-impl escape
   hatch** that composes with generated code (serde-style `skip` + hand impl), NOT an
   ever-richer attribute vocabulary.
3. **Error typing stays local.** `state/` cannot know `PeriodError` etc., so generated code
   emits generic site information; the site → typed-error mappings are declared once per
   relationship in `state-colloscopes`. When item 3 reroutes the checks, delete-blocking keeps
   today's exact typed errors.
4. **Consumer strategy — four steps** (drives the phase structure of §5):
   (a) introduce the new containers with `Deref` compatibility so the old read interface keeps
   compiling; (b) add the SQL-like interface; (c) progressively migrate consumers to it;
   (d) deprecate and remove the `Deref` compatibility layer.
5. **Check-rerouting is item 3, not item 2.** Item 2 *builds* the registry (declarations,
   walker, reverse lookups, SQL-like API). Rerouting candidate validation / delete-blocking /
   consistency through it is part of phase 2 item 3 (invariant consolidation), which gets its
   own plan. The registry design anticipates that consumer (site payloads carry exactly what
   the typed errors need); §6 records the design notes item 3 will need.
6. **`ops/` is touched as little as possible.** The crate has known bugs and large
   simplification potential and will be remastered soon; reshaping it now is wasted work.
   Item 2 only adjusts its *reads* mechanically where the container/API change forces it. No
   cleaning-cascade restructuring, no adoption of reverse lookups there.
7. **The Python read API may change.** `state_consolidation_plan.md` §6.7 ("no Python API
   redesign in this phase") is a guideline, not a rule: the read API naturally follows the new
   state code. The three contract scripts (§7 of the consolidation plan) are updated **in the
   same change** and the user runs them as acceptance. The write API changes later, outside
   phase 2.
8. **Dense mirrors are out of scope.** `assignments`, `group_lists.subjects_associations`,
   `slots`' per-subject ordering key set, and the whole `colloscope` keep their fan-out
   maintenance — centralizing that is item 5. The registry only *reads* them (their
   keys/values are references like any other).

   *Update (pre-phase-C cleanup):* the junction containers were retyped to composite-keyed
   `Table`s ahead of the registry: `assignments.map: Table<(PeriodId, SubjectId),
   BTreeSet<StudentId>>` (dense) and `group_lists.subjects_associations: Table<(PeriodId,
   SubjectId), GroupListId>` (**now sparse** — the old per-period denseness was never
   observable and is retired), plus `settings.students`/`balancing.subjects` →
   `Table<Id, _>` and `slots.ordering` → `Table<SubjectId, Vec<SlotId>>`. Only the
   maintenance-centralization (item 5) still remains; the container shapes are settled.
   The composite-key retype needed the `Table`/`OrderedTable` key bound relaxed from `Id` to
   `Key`/`OrderedKey` (see §4.1).

   *Whole-entry override rule (same cleanup, out of this item's scope but recorded here since it
   rides on the `settings.students`/`balancing.subjects` retype):* those two per-entity override
   tables are **whole-entry** — an entry wins verbatim, `None` fields included, so an override can
   *disable* a globally-enabled limit/option. This is now expressed once by
   `Settings::limits_for(student) -> &Limits` / `Balancing::options_for(subject) -> &BalancingOptions`
   (each `self.<table>.get(&id).unwrap_or(&self.global)`). The `constraints-colloscopes`
   `effective_*` helpers route through these accessors; a prior per-field
   `get(id).and_then(extract).or(global)` fallback silently re-inherited disabled fields (bug,
   fixed with regression tests in `constraints-colloscopes/tests/override_disable_regression.rs`).

---

## 3. Inventory of the current state

### 3.1 Container census

Two shapes dominate; only three containers carry user-visible order.

| Entity | Container (file) | Order matters? |
|---|---|---|
| Periods | `Periods::ordered_period_list: Vec<(PeriodId, Vec<WeekDesc>)>` (`periods.rs:34`) | **yes** — user order, positional ops |
| Subjects | `Subjects::ordered_subject_list: Vec<(SubjectId, Subject)>` (`subjects.rs:21`) | **yes** |
| Slots | `Slots::slot_map: Table<SlotId, Slot>` + `ordering: BTreeMap<SubjectId, Vec<SlotId>>` sidecar (`slots.rs`) — **restructured in commit 3**; `Slot` carries `subject_id`. Was `SubjectSlots::ordered_slots` inside `Slots::subject_map` | **yes** — the per-subject `ordering` is user order; the sidecar is a dense mirror keyed by interrogation-subjects |
| Teachers | `Teachers::teacher_map: BTreeMap<TeacherId, Teacher>` (`teachers.rs`) | no (id-sorted) |
| Students | `Students::student_map: BTreeMap<StudentId, Student>` (`students.rs`) | no |
| Week patterns | `WeekPatterns::week_pattern_map: BTreeMap<…>` (`week_patterns.rs`) | no |
| Incompats | `Incompats::incompat_map: BTreeMap<…>` (`incompats.rs`) | no |
| Group lists | `GroupLists::group_list_map: BTreeMap<…>` (`group_lists.rs`) | no |
| Pairing rules | `Pairings::pairing_rule_map: BTreeMap<…>` (`pairings.rs`) | no |
| Slot pairing rules | `SlotPairings::slot_pairing_rule_map: BTreeMap<…>` (`slot_pairings.rs`) | no |

**Junction / derived containers** (keys fan-out-maintained by parent ops, NOT free tables —
maintenance-centralization is item 5). As of the pre-phase-C cleanup these are composite-keyed
`Table`s: `assignments.map: Table<(PeriodId, SubjectId), BTreeSet<StudentId>>` (dense — one
entry per period × non-excluded subject), `group_lists.subjects_associations:
Table<(PeriodId, SubjectId), GroupListId>` (**sparse** — one entry per actual association).
Plus `slots.ordering: Table<SubjectId, Vec<SlotId>>` (one entry per subject with
interrogations) and the whole `colloscope` (`period_map` per period, `slot_map` per slot,
`interrogations: Vec<Option<…>>` per week, `group_lists` per non-prefilled group list — still
plain `BTreeMap`s).

**Singletons**: `settings` (global `Limits` + `students: BTreeMap<StudentId, Limits>`),
`balancing` (global + `subjects: BTreeMap<SubjectId, BalancingOptions>`), `export_config`
(no ids, no invariants).

### 3.2 The relationship inventory (28 ID-based relationships)

Every relationship below is currently RESTRICT-blocked with a typed error. "Block" = the
delete-blocking scan; "twin" = the whole-model consistency check. Dense-mirror keys block only
when their payload is non-trivial.

**→ Period** (blocks in `apply_period` Remove, `periods.rs:302-435`):

| # | Referencing site | Cardinality | Block error | Twin |
|---|---|---|---|---|
| 1 | `Subject.excluded_periods` | `BTreeSet<PeriodId>` | `PeriodIsReferencedBySubject` (`periods.rs:338`) | `InvalidSubject` via `validate_subject_internal` (`colloscope_params.rs:231`) |
| 2 | `Student.excluded_periods` | `BTreeSet<PeriodId>` | `PeriodIsReferencedByStudent` (`:347`) | `InvalidStudent` (`:343`) |
| 3 | `PairingRule.excluded_periods` | `BTreeSet<PeriodId>` | `PeriodIsReferencedByPairingRule` (`:356`) | `InvalidPairingRule` (`:737`) |
| 4 | `SlotPairingRule.excluded_periods` | `BTreeSet<PeriodId>` | `PeriodIsReferencedBySlotPairingRule` (`:364`) | `InvalidSlotPairingRule` (`:804`) |
| 5 | `assignments.map` key `(period, _)` (dense) | composite key | `PeriodStillHasNonTrivialAssignments` | `InvalidPeriodIdInAssignements` |
| 6 | `subjects_associations` key `(period, _)` (sparse) | composite key | `PeriodStillHasNonTrivialGroupListAssociation` | `WrongPeriodCountInSubjectAssociationsForGroupLists` (now fires on an invalid period in a row) |
| 7 | `colloscope.period_map` key (dense) | map key | `NotEmptyPeriodInColloscope` (`:312`) | `ColloscopeError::WrongPeriodCountInColloscopeData`/`InvalidPeriodId` (`colloscopes.rs:85`) |
| 8 | `WeekPattern.weeks` length coupling | `Vec<bool>` length | `NonTrivialWeekPattern` (`:327`) | `InvalidWeekPattern` (`:895`) |

**→ Subject** (blocks in `apply_subject` Remove/Update, `subjects.rs:412-638`):

| # | Referencing site | Cardinality | Block error | Twin |
|---|---|---|---|---|
| 9 | `Teacher.subjects` | `BTreeSet<SubjectId>` | `SubjectStillHasAssociatedTeachers` (`subjects.rs:448`) | `InvalidTeacher` (`:305`) |
| 10 | `slots.ordering` key (dense) — was `subject_map` key; slots themselves carry `Slot.subject_id` as a regular `#[fk]` | dense-mirror key | `SubjectStillHasAssociatedSlots` | `WrongSubjectCountInSlots` |
| 11 | `Incompatibility.subject_id` | single | `SubjectStillHasAssociatedIncompats` (`:457`) | `InvalidIncompat` (`:522`) |
| 12 | `subjects_associations[p]` inner key | map key | `SubjectStillHasAssociatedGroupList` (`:430`) | `InvalidSubjectIdInSubjectAssociations` (`:653`) |
| 13 | `balancing.subjects` key | map key | `SubjectStillHasBalancingOptions` (`:418`) | `InvalidSubjectIdInBalancing` (`:707`) |
| 14 | `PairingRule.{antecedent,consequent}.subject_id` | 2 × single | `SubjectIsReferencedByPairingRule` (`:422`) | `InvalidPairingRule` (`:731`) |
| 15 | `assignments…subject_map` key (dense) | map key | `SubjectStillHasNonTrivialAssignments` (`:466`) | `InvalidSubjectIdInAssignments` (`:399`) |
| 16 | colloscope slots of the subject | indirect | **Update-only** `SubjectStillHasNonEmptySlotInColloscope` (`:619`) | `ColloscopeError` slot validation (`colloscopes.rs:273`) |

**→ Teacher / → WeekPattern / → Slot**:

| # | Referencing site | Cardinality | Block error | Twin |
|---|---|---|---|---|
| 17 | `Slot.teacher_id` | single | `TeacherStillHasAssociatedSlots(TeacherId, SlotId)` (`teachers.rs:96`); **Update-only** `TeacherStillHasAssociatedSlotsInSubject` (`:123`) | `InvalidSlot` via `validate_slot_internal` (`colloscope_params.rs:431`) |
| 18 | `Slot.week_pattern` | `Option` | `WeekPatternStillHasAssociatedSlots` (`week_patterns.rs:167`) | `InvalidSlot` (`:440`) |
| 19 | `Incompatibility.week_pattern_id` | `Option` | `WeekPatternStillHasAssociatedIncompat` (`:179`) | `InvalidIncompat` (`:525`) |
| 20 | `SlotPairingRule.{antecedent,consequent}.slot_id` | 2 × single | `SlotIsReferencedBySlotPairingRule` (`slots.rs:312`) | `InvalidSlotPairingRule` (`:792`) |
| 21 | `colloscope…slot_map` key (dense) | map key | `NotEmptySlotInColloscope` (`slots.rs:302`) | `ColloscopeError::WrongSlotCountInPeriodInColloscopeData`/`InvalidSlotId` (`colloscopes.rs:267`) |

**→ Student** (blocks in `apply_student` Remove/Update, `students.rs:101-188`):

| # | Referencing site | Cardinality | Block error | Twin |
|---|---|---|---|---|
| 22 | `assignments…subject_map` value sets | `BTreeSet<StudentId>` | `StudentStillHasNonTrivialAssignments` (`students.rs:133`) | `InvalidStudentIdInAssignments`/`AssignedStudentNotPresentForPeriod` (`:406`) |
| 23 | `GroupListFilling::Prefilled{groups[].students}` | set, in enum variant | `StudentIsStillReferencedByPrefilledGroupList` (`:125`) | `InvalidGroupList` (`:596`) |
| 24 | `GroupListFilling::Automatic{excluded_students}` | set, in enum variant | `StudentIsStillExcludedByGroupList` (`:116`) | `InvalidGroupList` (`:603`) |
| 25 | `settings.students` key | map key | `StudentStillHasSettings` (`:150`) | `InvalidStudentIdInSettings` (`:680`) |
| 26 | `colloscope.group_lists[gl].groups_for_students` key | map key | `StudentIsReferencedInColloscopeGroupList` (`:107`) | `ColloscopeError::InvalidStudentId` (`colloscopes.rs:575`) |

**→ GroupList**:

| # | Referencing site | Cardinality | Block error | Twin |
|---|---|---|---|---|
| 27 | `subjects_associations[p][s]` value | single | `RemainingAssociatedSubjects` (`group_lists.rs:397`) | `InvalidGroupListIdInSubjectAssociations` (`:648`) |
| 28 | `colloscope.group_lists` key (dense, non-prefilled only) | map key | `NotEmptyGroupListInColloscope` (`group_lists.rs:385`) | `ColloscopeError::InvalidGroupListId`/`PrefilledGroupListInColloscope`/`MissingNonPrefilledGroupList` (`colloscopes.rs:97`) |

Nothing references `IncompatId`, `PairingRuleId`, or `SlotPairingRuleId`.

### 3.3 Outside the registry: index-based and structural checks

These are *value/shape* checks, not ID-existence checks; they stay hand-coded (unchanged by
this item):

- **Group-number bounds** (indices, not ids): `ColloscopeInterrogation.assigned_groups` and
  `groups_for_students` values vs the associated group list's `group_names.len()`
  (`ColloscopeError::InvalidGroupNumInInterrogation` / `InvalidGroupNumForStudentInGroupList`,
  `check_interrogations_group_bound` in `group_lists.rs:287`).
- **Structural counts** of the dense mirrors: `WrongSubjectCountInAssignments`,
  `WrongSubjectCountInSlots`, `WrongPeriodCountInSubjectAssociationsForGroupLists`, the
  colloscope shape/count/week-structure checks, `BadWeekPatternLength`.
- **Pair-level predicates**: `SameSubjectInBothParts`, `SameSlotInBothParts`,
  `SlotsNotInSameSubject`.
- **Side-constraints attached to references** (the SQL schema encoded these by pointing FKs at
  `subject_interrogation_params` instead of `subjects`): "referenced subject must have
  interrogations" (`TeacherError::SubjectHasNoInterrogation`,
  `BalancingForSubjectWithoutInterrogations`, …), "teacher must teach the slot's subject"
  (`TeacherDoesNotTeachInSubject`), "subject must run on the period"
  (`SubjectAssociationForSubjectNotRunningOnPeriod`). These live next to the existence checks
  today and stay with them; item 3 decides how they ride along.

### 3.4 Consumer blast radius

| Consumer | Reads | Order/position dependence | Insulated? |
|---|---|---|---|
| `ops/` | ~200 container-field reads; 36 `find_*_position` calls; ~25 positional `[pos]` indexings; ~21 hand reverse-lookup loops in `get_next_cleaning_op` | high (positional ops, move up/down) | no |
| `gtk4/` | ~138 field reads across 27 files; 42 `get_inner_data` in `editor.rs`; ~10 positional sites + MoveUp/Down | high | no |
| `constraints-colloscopes` | `build_config` (`convert.rs:8`) walks `ordered_period_list` accumulating week offsets — **period order is correctness-critical** (global week index); ~50 reads, 7 `find_*_position` | periods: hard requirement | no |
| `xlsx/` | ~15 reads; `colloscope_sheet.rs` layout driven by period/subject/slot order | order = sheet layout | no |
| `python/` | one conversion file `python/src/glue/params.rs` (~9 accesses); pyclass mirrors expose subjects/periods/slots as **lists** (order preserved), the rest as dicts | via conversion | yes (until phase D changes the read API deliberately) |
| contract scripts | `import.py` and the Pronote import: no order reliance; `custom_export_xlsx.py`: period/subject/slot order drives layout | via Python shapes | yes |
| `rpc/` | zero field access — `serde_json` of `InnerData` as wire (`rpc/src/lib.rs:64-76`) | none | yes, **if serde representation is preserved** |
| `storage/` spec-2 | `encode/spec2.rs` ~13 container readers; `decode/spec2.rs` ~15 `reconstruct_*` functions | array order for periods/subjects/slots already round-tripped | designed boundary; **on-disk format is frozen and must not change** |

Behavioral (not compile-time) requirement: the `BTreeMap`-backed entities iterate id-sorted
today, and Python dicts, gtk4 lists, xlsx sheets, and ILP-variable insertion all inherit that
determinism. The new `Table` must keep **deterministic id-sorted iteration**.

---

## 4. Target architecture

### 4.1 Generic containers in `state/`

New module `state/src/tables.rs` (style rule: `foo.rs` + `foo/` directory if it grows, never
`mod.rs`), re-exported by `state-colloscopes`.

```rust
// state/src/tables.rs — as shipped (A1, then bound-relaxed in the pre-phase-C cleanup)
pub trait Key: Copy + Ord {}                        // blanket impl for every Copy + Ord type
pub trait OrderedKey: Copy + Eq + std::fmt::Debug {} // blanket impl; Debug for DuplicatedIdError
pub struct Table<I: Key, T> { inner: BTreeMap<I, T> }          // #[serde(transparent)]
pub struct OrderedTable<I: OrderedKey, T> { inner: Vec<(I, T)> } // #[serde(transparent)]
```

The key bound is `Key`/`OrderedKey`, **not `Id`** (relaxed in the pre-phase-C cleanup — see
decision 8's Update): the containers never call `Id::inner()`/`Id::new()`, only key supertraits,
so a minimal `Copy + Ord` (resp. `Copy + Eq + Debug`) bound is enough. Every `Id` newtype
satisfies it via the blanket impls, and so does a tuple of ids — which is exactly what lets the
junction tables use a composite `(PeriodId, SubjectId)` key.

- **Common, representation-independent keyed API** on both: `get(&I) -> Option<&T>`,
  `contains(&I) -> bool`, `keys()` (yields owned `I`), `values()`, `iter()` (yields `(I, &T)`),
  `len()`, `is_empty()`. The inherent `keys()`/`iter()`/`values()` **deliberately shadow** the
  Deref'd `BTreeMap::keys`/`iter`/`values`: since every `I` is `Copy`, they yield an owned id
  (rather than a `&I` reference pair), which is the ergonomic shape consumers want. (An earlier
  draft named the iterator `entries()` to *avoid* shadowing; that was reversed — shadowing with
  owned-id item types is what shipped.)
- **`OrderedTable` position API**: `position_of(&I)`, `get_at(usize)`, and doc-marked-internal
  mutators `insert_at` (fallible, see below), `remove_at`, `replace_value_at`, `move_entry`
  (covers today's `insert(0,…)`, `insert(pos+1,…)`, `remove(pos)`,
  `mem::replace(&mut v[pos].1, …)`, and the ChangePosition remove+insert pairs).
- **Compat window (consumer step (a))**: `Deref<Target = BTreeMap<I, T>>` resp.
  `Deref<Target = [(I, T)]>` keeps the ~350 consumer read sites compiling unchanged. No
  `DerefMut` — mutation happens only through restricted methods whose names shadow the
  `BTreeMap` ones (`insert`, `remove`, `get_mut`, `values_mut`) so in-crate mutation sites are
  textually unchanged. The `Deref` impls are removed in phase E; after that the representation
  is free to change.
- **Serde**: `#[serde(transparent)]`, so the serialized form is exactly today's
  `BTreeMap`/`Vec<(Id, T)>` form. This pins the rpc wire (`serde_json` of `InnerData`) and any
  in-memory serde (`AnnotatedOp::GlobalUpdate`).
- **Per-table primary-key uniqueness is a type invariant** (settled July 13 2026, replacing
  the earlier non-validating-constructors sketch). `Table` gets it structurally from its map
  backend (`insert` keeps `BTreeMap` replace semantics and the `Option<T>` return, so B1 call
  sites stay textually unchanged). `OrderedTable` enforces it: construction is
  `TryFrom<Vec<(I, T)>>` (errors on a duplicated id), `insert_at` is fallible, and
  `Deserialize` is hand-written to route through the checked constructor. Consequence:
  single-table duplicate corruption surfaces at deserialize time (D1 maps it into a proper
  storage decode error). The **global cross-kind** uniqueness check
  (`InnerData::check_no_duplicate_ids` chains all id kinds into one `u64` stream) is not
  expressible per table and stays in `from_inner_data`/`check_invariants`.
- Mutation-visibility (settled July 13 2026): fully opaque inner field, all access through
  methods. Mutators are `pub` (Rust has no crate-friend visibility) but doc-marked
  state-layer-internal; the real boundary is that consumers of `state-colloscopes` only ever
  hold `&Table`/`&OrderedTable`. The `Deref` impls are the *sole* representation leak,
  removed in phase E.

### 4.2 The derive crate

New crate `state-derive/` (package `collomatique-state-derive`, `proc-macro = true`),
re-exported via `collomatique-state`. Three derives (`EntityId`, `References`, and — since the
A3 amendment — `Join`, described with the join machinery below):

- **`#[derive(EntityId)]`** — replaces the 10 hand-written `pub struct XxxId(u64)` + `impl Id`
  blocks in `state-colloscopes/src/ids.rs` (the `Id` trait itself moves to `state/`). Also a
  natural place to generate the `From<XxxId> for NewId` impls if that proves convenient
  (`NewId` itself stays project-specific and hand-written).
- **A references derive** (working name `#[derive(References)]`) generating impls of a generic
  trait in `state/`:

```rust
// state/src/refs.rs — settled July 13 2026 (replaces the earlier References<I: Id> sketch)
pub trait References<K> {
    fn for_each_ref(&self, f: &mut dyn FnMut(K));
}
// K is a target *union* type, instantiated at K = NewId in state-colloscopes (its ten
// From<XxxId> impls already exist). Leaf impls come from #[derive(EntityId)]:
//     impl<K: From<XxxId>> References<K> for XxxId
// Option/Vec/BTreeSet lifts live in state/; derived struct impls are generic over K with
// per-field `FieldTy: References<K>` bounds — so nested #[fk] structs (PairingRule's
// PairingPart) compose with no manual impls and no extra attributes.
```

```rust
// state-colloscopes — usage sketch (worked example: Slot, relationships #10, #17 and #18)
// Since commit 3 the slot carries its subject as a regular FK (#10): slots live in a
// flat `Table<SlotId, Slot>` and the per-subject `ordering` sidecar mirrors this field.
#[derive(References)]
pub struct Slot {
    #[fk] pub subject_id: SubjectId,          // plain Id field (grouping FK, #10)
    #[fk] pub teacher_id: TeacherId,          // plain Id field
    pub start_time: SlotStart,                // no attribute → ignored
    #[fk] pub week_pattern: Option<WeekPatternId>,  // Option<Id>
    pub cost: u32,
    pub extra_info: Option<NonEmptyString>,
}
// generates:
// impl<K> References<K> for Slot
// where TeacherId: References<K>, Option<WeekPatternId>: References<K> { … }
// (walk order = field declaration order)
```

  Covered shapes (decision 2): plain `Id`, `Option<Id>`, `BTreeSet<Id>`/`Vec<Id>`, plus nested
  structs whose type implements `References` (free through the generic bounds). Everything
  else — `GroupListFilling`'s per-variant student sets (#23, #24), map-key references
  (#5–7, #10, #12, #13, #15, #21, #25, #26, #28), the week-pattern length coupling (#8) — is a
  hand-written `impl References<…>` or a container-level walker (see 4.3).

- **Join machinery (pulled forward from C3 into phase A — its interface constrains the derive
  design).** Reference-based only, in `state/src/join.rs`:

```rust
pub trait Joinable {                    // context-independent type level
    type Output<'a> where Self: 'a;
    type Error;
}
pub trait Join<Ctx>: Joinable {         // value level
    fn join<'a>(&'a self, ctx: &'a Ctx) -> Result<Self::Output<'a>, Self::Error>;
}
pub trait Lookup<I> {                   // what a context provides, per id type
    type Entity;
    fn lookup(&self, id: I) -> Option<&Self::Entity>;
}
```

  `#[entity(Teacher)]` on `#[derive(EntityId)]` declares the id→entity association and
  generates the leaf impls (`Output<'a> = &'a Teacher`, `Error = TeacherId` — the dangling id
  is the diagnostic; `Join<Ctx>` bounded by `Ctx: Lookup<TeacherId, Entity = Teacher>`).
  `Option`/`Vec`/`BTreeSet` lifts live in `state/`. `#[derive(Join)]` on a struct generates a
  borrowed `Joined{Name}<'a>` struct (non-`#[fk]` fields appear as `&'a T`; the name is
  overridable via `#[join(output = Name)]`; the view derives `Debug + Clone`, so entity types
  must be `Debug`); it requires
  `#[join(error = Type)]` (with generated `From` bounds — `state/` defines no error type);
  field names are kept as-is, `#[fk(name = ident)]` renames explicitly (no automatic naming).
  `Lookup` impls on `Parameters` and any registry wiring remain C3 work.

Standing note for every commit that adds a crate or dependency: **Cargo.lock changes ⇒ the user
refreshes `collomatique.nix`'s `cargoHash` before committing.**

### 4.3 The registry: sites, walker, reverse lookups

In `state-colloscopes` (new module `refs.rs`; entity-level declarations live next to their
types in the entity modules, same philosophy as the item-6 split):

- **`RefSite`** — one enum (25 variants, shipped in C1a) describing *where* a reference lives;
  the payload is the referencing entity's identity, chosen to carry exactly what today's typed
  errors need (verified against the error payloads: e.g. `TeacherStillHasAssociatedSlots(TeacherId,
  SlotId)` needs the referencing `SlotId`; `SubjectStillHasAssociatedGroupList(SubjectId,
  GroupListId, PeriodId)` needs both association coordinates). Dense-mirror / colloscope sites
  carry a `non_trivial: bool` computed at walk time, so item 3's delete-blocking can skip trivial
  dense sites while consistency checks all of them — reproducing today's asymmetry exactly. The
  shipped variants (see `state-colloscopes/src/refs.rs`):
  - *entity fields*: `SubjectExcludedPeriods`, `StudentExcludedPeriods`,
    `PairingRuleExcludedPeriods`, `SlotPairingRuleExcludedPeriods`, `TeacherSubjects`,
    `SlotSubject`, `SlotTeacher`, `SlotWeekPattern`, `IncompatSubject`, `IncompatWeekPattern`,
    `PairingRulePart` (one variant for both parts — the block error doesn't distinguish them),
    `SlotPairingRulePart`, `GroupListPrefilledStudent`, `GroupListExcludedStudent`,
    `SettingsStudentKey`, `BalancingSubjectKey`,
    `WeekPatternLengthCoupling { week_pattern, non_trivial }` (emitted per week-pattern × period
    pair; `non_trivial = week_count != 0 && !wp.can_remove_weeks(first_week, week_count)`);
  - *dense mirrors / junctions*: `AssignmentsKey { period, subject, non_trivial }`,
    `AssignmentsStudent { period, subject }`, `AssociationEntry { period, subject, group_list }`,
    `SlotsOrderingKey { non_trivial }`;
  - *colloscope*: `ColloscopePeriodKey { non_trivial }`, `ColloscopeSlotKey { period, non_trivial }`,
    `ColloscopeGroupListKey { non_trivial }`, `ColloscopeGroupListStudent`.
  - *documented exclusions*: `SubjectStillHasNonEmptySlotInColloscope` (update-only + indirect via
    slot → subject, handled by item 3 as a wrapper); `slots.ordering` row *values* (pure mirror of
    `slot_map` keys); colloscope group *indices* (not ids).
- **Walker** — composes, in a fixed documented order (the `check_invariants` family order,
  then dense mirrors, then colloscope; within a family, container order; within an entity, field
  declaration order). The exact order is spelled out in the `refs.rs` module doc:
  1–11 the [Parameters] families (subjects, teachers, students, slots, incompats, pairings, slot
  pairings, group lists, `settings.students` keys, `balancing.subjects` keys, week-pattern length
  coupling), then 12 the dense mirrors (`walk_assignments`, `walk_associations`,
  `walk_slots_ordering_keys`), then 13 the colloscope (`walk_colloscope`).
  - C1a (shipped) hand-writes the family walkers inline; C2 rewrites the per-entity bodies to
    `for_each_ref::<NewId>` + kind-dispatch while keeping the same decomposition and pin output;
  - the dense-mirror / colloscope walkers stay hand-written (no entity `References` involved).
  The visitor (`RefVisitor`) has one callback per referenced id kind (period/subject/teacher/
  student/week-pattern/slot/group-list — the seven referenced kinds); every callback defaults to
  a no-op so a filtering visitor implements only what it needs.
  - *C1a status (shipped)*: `RefSite`, `RefVisitor`, the family walkers (`walk_params_refs`) and
    the family-only pin test.
  - *C1b status (shipped)*: the dense-mirror walkers (`walk_assignments`, `walk_associations`,
    `walk_slots_ordering_keys`) and the colloscope walker (`walk_colloscope`); the public
    `InnerData::walk_refs` composing all three; the seven `references_to_*` reverse lookups via a
    single `references_to_impl!` filtering-visitor macro. The temporary `walk_params_refs_for_tests`
    shim was removed — `tests/refs_registry.rs` now drives `walk_refs`/`references_to_*` and pins
    the full ordered output plus every reverse lookup (both `non_trivial` polarities).
  - *C2 status (shipped)*: `#[derive(References)]` + `#[fk]` applied to the ten entity structs
    (`Subject`, `Student`, `Teacher`, `Slot`, `Incompatibility`, `RulePart`, `PairingRule`,
    `SlotRulePart`, `SlotPairingRule`, `GroupList`), with the `#[fk(name = …)]` join renames
    (`subject_id→subject`, `teacher_id→teacher`, `week_pattern_id→week_pattern`, `slot_id→slot`)
    added now (validated-and-ignored by `References`, used by C3b `Join`). `GroupListFilling`
    gets a manual `References` impl (references live inside the enum variants) composing with the
    derive through the generic `K` bound. The eight per-entity family walkers in `refs.rs` now
    drive `for_each_ref::<NewId>` + kind-dispatch (a `match NewId` per entity, `unreachable!` on
    kinds the entity cannot reference); the dense-mirror/colloscope/settings/balancing/week-pattern
    walkers stay hand-written. Acceptance held: the C1b pin test passes **unchanged**, proving the
    derives emit exactly the hand walk's ordered output.
  - *C3a status (shipped)*: the ten `Lookup<XxxId> for Parameters` impls (keyed on each typed id,
    `Entity` matching the id's `#[entity(…)]` type), each delegating to the container accessor
    already in `colloscope_params.rs` (`find_period`/`find_subject`/`find_slot` for the ordered
    tables, `Table::get` for the rest) so lookup borrows straight out of the table — no clone. Two
    inherent helpers on `Parameters`: `lookup<I>` (fallible entry point, shadowing the trait method,
    delegating via fully-qualified `<Self as Lookup<I>>::lookup`) and `resolve<I: Id>` (infallible
    for already-validated data, panics printing the dangling id). New `tests/read_api.rs` builds one
    entity of every kind through the op API and pins: `lookup`/`resolve` return the live borrow
    (pointer identity), a `1 << 40` dangling id resolves to `None` for every kind, and `resolve`
    panics on a dangling id. Kept separate from C3b (the Join derive) so this trivial foundation
    stays an isolated, fully-tested checkpoint. No consumer code touched; no on-disk/wire impact.
  - *C3b status (shipped)*: `#[derive(Join)] #[join(error = NewId)]` added to the nine regular
    entity structs (`Slot`, `Teacher`, `Student`, `Subject`, `Incompatibility`, `RulePart`,
    `PairingRule`, `SlotRulePart`, `SlotPairingRule`) — attributes only, the `#[fk]`/`#[fk(name =
    …)]` markers already existed from C2. Each generates a `Joined*` view (every FK field resolved
    to a borrow of its entity, non-FK fields borrowed as `&'a T`) plus `Joinable`/`Join<Ctx>` impls
    that resolve against any `Ctx: Lookup<…>` — i.e. `Parameters` via C3a. `NewId` is the error out
    of the box (its ten `From<XxxId>` impls + the std `From<NewId>` identity cover the nested
    `RulePart`/`SlotRulePart` cases). Views re-exported from the lib root (item-6 pattern).
    `GroupList` (refs inside the `GroupListFilling` enum) and `WeekPattern`/periods (no/absent FK
    fields) are excluded, as planned. `tests/read_api.rs` extended: ptr-eq borrows for scalar FKs,
    id-sorted `Vec<&Subject>` from the `BTreeSet` FK lift, `Option` FK both ways, nested
    `JoinedRulePart` composition, and a dangling FK → `Err(NewId::TeacherId(_))`. Storage
    byte-stability (`round_trip_identity`) confirms the Join derive adds no serde/on-disk impact.
- **Reverse lookups** (public, the item-2 deliverable):

```rust
impl InnerData {
    pub fn references_to_period(&self, id: PeriodId) -> Vec<RefSite>;
    pub fn references_to_subject(&self, id: SubjectId) -> Vec<RefSite>;
    // … teacher, student, week_pattern, slot, group_list
}
```

### 4.4 The SQL-like read API (the consumer-facing goal)

Consumer step (b). Sketch — final shape settled in the C3 session, guided by what the phase-D
migrations actually need:

- **Typed keyed lookup**: an `Id → Entity` association (e.g. `trait TableLookup<I: Id>` on the
  document/params type, macro- or hand-generated per table) giving
  `params.lookup(teacher_id) -> Option<&Teacher>` — replaces today's ad-hoc `find_*` helpers
  and the `validate_*_id` u64-promotion scans.
- **FK resolution ("auto-join")**: follow a reference to the referenced struct —
  `params.resolve(slot.teacher_id) -> &Teacher` (infallible variant for already-validated data,
  fallible variant for candidate data).
- **Reverse lookup**: `references_to_*` from 4.3 ("who references X?" — for the UI and,
  eventually, the remastered `ops/`).
- **Table/id enumeration**: a declared list of all tables, replacing the hand-chained
  `Parameters::ids()` (`colloscope_params.rs:175-222`) and giving duplicate-ID scanning and
  `IdIssuer` seeding a single source of truth.
- **Ordered accessors**: `OrderedTable`'s position API absorbs the `find_*_position` family
  (the public helpers on `Periods`/`Subjects`/`SubjectSlots` delegate to it, then migrate away
  in phase D).

Reads only — the write path stays elementary ops through `AppState`/`ops/`, unchanged.

---

## 5. Commit roadmap

Each commit ≈ one session, builds green across the workspace, and passes:
`state-colloscopes` property harness (`tests/property_ops*.rs`, 100 seeds) + `found_bugs.rs`
exact-error asserts + storage `populated_round_trip` byte-stability + the `examples/` smoke
test. Milestones marked ★ additionally run the 500-seed slow reference and (per decision 7)
the three contract scripts, run by the user.

### Phase A — generic layer in `state/` (three sessions since the July 13 2026 amendment)

- **A1** (DONE July 13 2026): `state/src/tables.rs` — `Table`, `OrderedTable` (validating,
  per amended §4.1), `Id` trait moved into `state/src/ids.rs` (re-exported from
  `state-colloscopes::ids` so no consumer path changes), full unit tests including
  serde-equivalence tests (`to_value(table) == to_value(btreemap)`, same for ordered vs `Vec`)
  pinning the wire format. Nothing adopted yet — zero fallout. **Cargo.lock changes (serde in
  `state/`): user refreshes the nix `cargoHash`.**
- **A2** (DONE July 13 2026): `state-derive/` crate with `#[derive(EntityId)]` and `#[derive(References)]`
  (per amended §4.2; `#[entity(…)]` parsed but inert until A3); `ids.rs` shrinks to 10
  one-line derives + `IdIssuer` + `NewId`; `state/src/refs.rs` with the `References<K>` trait
  and container lifts. Derive tests are integration tests in `state/tests/` (generated code
  uses absolute `::collomatique_state::` paths) on toy types; real entity structs are C2.
  **Cargo.lock changes: user refreshes the nix `cargoHash`.**
- **A3** (DONE July 13 2026): `state/src/join.rs` (`Joinable`/`Join`/`Lookup` + container lifts, per amended
  §4.2), `#[derive(Join)]`, `#[entity(Type)]` activated in `EntityId` and applied to the ten
  real ids (`PeriodId → Vec<WeekDesc>` — periods have no entity struct today). No `Lookup`
  impls on `Parameters` (C3). Integration tests on a toy world. No new dependencies.

### Phase B — container adoption with `Deref` compat (consumer step (a))

- **B1**: the 7 keyed maps → `Table<…>` (fields keep their names). Mutator names shadow
  `BTreeMap`'s, so in-crate call sites are textually unchanged (audited: `insert` return value
  used at `group_lists.rs:531,621`; `get_mut`+`mem::replace` in the 6 Update paths;
  `values_mut` fan-out in `periods.rs`). Known fallout: `storage/src/decode/spec2.rs`
  `reconstruct_incompats` builds a local `BTreeMap` → `.into()`; two gtk4 dialog fields typed
  `BTreeMap` (`gtk4/src/editor/group_lists.rs:253,351`) → retype to `Table` or deep-clone.
- **B2**: `ordered_period_list` + `ordered_subject_list` → `OrderedTable<…>`. Positional
  mutator rewrites (audited): `periods.rs:210,268,409,486`; `subjects.rs:358,399-409,500,641`.
  Public `find_*_position` helpers delegate to `position_of`; external callers untouched.
- **B3** ★ — **SUPERSEDED by the commit-2/commit-3 slots restructure** (July 14 2026, done).
  Rather than wrapping today's per-subject `Vec` in `OrderedTable`, the slots backend was
  rebuilt as the right relational shape: a flat `slot_map: Table<SlotId, Slot>` (each `Slot`
  carries `subject_id` as a regular FK, #10) plus an explicit `ordering: BTreeMap<SubjectId,
  Vec<SlotId>>` sidecar (dense, user order). `SubjectSlots` is deleted; all mutation goes
  through compound `pub(crate)` helpers so the two structures cannot desynchronize.
  `SlotOp::AddAfter` dropped its `SubjectId` parameter (`AddAfter(Option<SlotId>, Slot)`);
  `SlotOp::Update` rejects a changed subject (`SlotError::CannotChangeSubject`). Commit 2 first
  narrowed every consumer onto a `Slots` read surface (zero behavior change); commit 3 swapped
  the backend behind it. On-disk format frozen (encode joins `ordering`→`slot_map` byte-for-byte;
  decode rebuilds both fields with explicit duplicate-slot-id detection). Phase-B milestone check.

### Phase C — registry + SQL-like functions (consumer step (b))

- **C1**: `refs.rs` — `RefSite` (~26 variants, payloads per 4.3), visitor trait, walker with
  its fixed documented order, dense-mirror walkers, `references_to_*` API. A pin test asserts
  the exact site list and order for each id kind on a small hand-built document (built via ops,
  like `found_bugs.rs`).
- **C2**: `#[derive(References)]` + `#[fk]` attributes applied to all regular relationships;
  manual `References` impls for the irregular ones (`GroupListFilling`, `Teacher.subjects` is
  regular, week-pattern length coupling #8 stays a special walker case). The C1 pin test must
  not change — it is the proof the derive emits the same references the hand walk did.
- **C3** ★: the SQL-like read API (4.4): `Lookup` impls on `Parameters`/`InnerData` (the
  trait itself lands in A3), table enumeration replacing `Parameters::ids()`, ordered
  accessors. New tests; no consumer migrated yet.

### Phase D — consumer migration (consumer step (c))

One crate (or slice) per commit; each replaces raw field access with the SQL-like API and
drops reliance on the `Deref` layer within its scope:

- **D1**: `storage/` encode (`encode/spec2.rs` ~13 readers) + decode (`decode/spec2.rs` ~15
  `reconstruct_*` — constructors from the format's `KeyedVec` rows into `Table`/`OrderedTable`;
  the decode-then-`from_inner_data`-validates split stays). Byte-stability tests prove the
  on-disk format did not move.
- **D2**: `constraints-colloscopes` (the ordered-period week-offset walk keeps its explicit
  order via `OrderedTable` iteration) + `xlsx/`.
- **D3**: `ops/` — **mechanical read adjustments only** (decision 6): swap field reads for the
  new API where the `Deref` removal will require it, nothing else. No cascade restructuring.
- **D4, D5**: `gtk4/`, sliced (editor pages grouped by the ~27 files; two sessions expected).
- **D6** ★: `python/` glue + deliberate read-API changes (decision 7), with the three contract
  scripts updated in the same commit. Acceptance: the user runs `extra-scripts/import.py`,
  `scripts/import_pronote_web_2026_05_06.py`, `scripts/examples/custom_export_xlsx.py`.

### Phase E — remove the compat layer (consumer step (d))

- **E1**: deprecate the `Deref` impls (`#[deprecated]` on a shim if needed) and sweep remaining
  stragglers workspace-wide.
- **E2** ★: delete the `Deref` impls and any BTreeMap-shaped helpers. From here the internal
  representation is free to change (e.g. `OrderedTable` to map + order list) without touching
  consumers. Final milestone: 500-seed property run + contract scripts.

Phases B and C are independent enough that C1 can start before B is fully done if a session
prefers; D depends on C3; E depends on all of D.

---

## 6. Hand-off notes for item 3 (invariant consolidation)

Item 3 will reroute the triplicated checks through the registry built here. Design notes it
will need, recorded now while they are fresh:

- **Candidate validation**: per-entity `References` walks over the *candidate* value + existence
  checks against the tables, mapping site → per-entity error (`SubjectError::InvalidPeriodId`,
  …). Interleaving matters: e.g. `validate_teacher_internal` checks existence *and* the
  has-interrogations predicate per reference in iteration order — two separate passes would
  change which error surfaces first when both problems exist.
- **Delete-blocking**: "first blocker wins", and the winning category must reproduce today's
  per-op check order (`found_bugs.rs` asserts exact error variants). The per-op category orders
  must be transcribed from the `apply_*` bodies at that time (e.g. `apply_period` Remove checks
  colloscope → week patterns → subjects → students → pairings → slot pairings → assignments →
  associations; `apply_student` Remove checks colloscope → group lists (excluded before
  prefilled, per group list) → assignments → settings).
- **Non-1:1 mappings**: the Update-only guards — `TeacherStillHasAssociatedSlotsInSubject`
  (same site as #17, different error, filtered by the update's subject diff),
  `SubjectStillHasNonEmptySlotInColloscope` (#16, period-scoped on the update's exclusion
  diff), student-update assignment checks — either stay hand-coded or get dedicated wrappers
  over the same collected sites.
- **Switchover pattern**: the item-1 canary — one commit computes old and new results side by
  side under `debug_assert_eq!` (the property harness then compares them across millions of
  generated valid+invalid ops), the next commit deletes the old code.
- **Consistency**: keep the per-family structure of `check_invariants` (family order
  unchanged); the registry replaces only the referential loops; structural counts, group-number
  bounds, pair predicates, and side-constraints (3.3) stay.

## 7. Risks

- **Serde/wire drift** (rpc, `GlobalUpdate` ops): pinned by `#[serde(transparent)]` + the A1
  equivalence tests + storage byte-stability. Caught immediately; revert unit is one commit.
- **Method shadowing**: inherent methods deliberately shadow `Deref` targets (`get`, `insert`,
  `remove`). All collisions are compile-visible; the one known behavioral trap
  (`get(usize)` vs `get(&I)` on `OrderedTable`) is addressed by naming the positional accessor
  `get_at` and fixing the two in-crate sites in B3.
- **Borrow-checker friction**: method-mediated mutation borrows only the container field, same
  disjoint-field borrows as today (e.g. iterating `slots` while mutating `colloscope` in
  `periods.rs`). If a site resists, a documented escape hatch on the table type is acceptable
  temporarily.
- **Derive-crate maintenance**: kept small by decision 2 (regular shapes only); the macro is
  exercised by the C1 pin test, so regressions in generated code are caught by ordinary tests.
- **Scope creep**: the walker only *reads* dense mirrors (item 5 owns their maintenance);
  check-rerouting belongs to item 3; `ops/` restructuring belongs to its future remaster.
