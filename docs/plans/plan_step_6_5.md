# Step 6.5 session plan — monotonicity checking (`PartialOrd` on the document)

**Status: DRAFT — awaiting user sign-off. No commit below has landed.**

This plan is self-sufficient: it contains every design decision, the complete
specification of the partial order, and old/new code for every touched site. It
is written to be implementable mechanically, commit by commit, without consulting
the conversation that produced it. The wider context lives in
`docs/plans/invariant_cascade_design.md` (§8 "Step 6.5" paragraph, Appendix H);
this plan repeats what it needs from there.

---

## 0. Context: what step 6 left open, and what binds this step

Step 6 delivered the cascade: `apply_cascade` (in `state/src/cascade.rs`) applies
a target operation, and when the apply/check/rollback gate rejects it over broken
invariants, asks the resolution map (`Fixable::fix_invariant`, implemented for
`Data` in `state-colloscopes/src/resolution.rs`) for one repair operation, pushes
it on the retry stack, and loops. The engine has **no round fuse** — deliberately,
because no meaningful bound exists. Termination rests entirely on the engraved
map contract:

> States form a partial order with a universal minimal element:
> `Default::default()`, the empty document. Every returned op must land
> **strictly below** the current state in that order.

Step 6 enforced this contract only partially in-flight. The `None` convictions
and the no-op-fix panic catch every *removal-shaped* violation, but the order
itself was never materialized, so a map bug that keeps **growing** the state is
undetectable and makes the cascade loop forever. Step 6.5 closes that hole:

1. define the partial order as a real `PartialOrd` implementation on the
   document (`InnerData`, and `Data` by delegation),
2. require `PartialOrd` on `Fixable` implementors (only there — the generic
   `InMemoryData` trait is untouched),
3. assert in the cascade loop, after every fix that lands, that the new state is
   **strictly below** the pre-fix state — turning a growing or sideways map into
   a loud panic instead of a hang,
4. fuzz the two halves of the contract: (a) `Default::default()` really is below
   every reachable state, and (b) every `fix_invariant` answer is `None` or an
   op whose applied result lands strictly below.

Three constraints bind any implementation. They are ★ user rulings recorded in
the design doc and the project memory:

* **D5.1 — the order is over the document's *content*, not the meaning it
  denotes.** Several conforming map arms strictly shrink the data while
  *widening* the semantics: a subject that stops excluding a dead period now
  applies more broadly; a slot whose `week_pattern` is cleared now runs every
  week. In each case an id was removed and nothing was added, so the document
  strictly decreased. An order that compared meanings would reject these arms
  and break the termination proof. Every rule in §2 below is a rule about
  *content*.
* **Compatibility with `Eq`.** `InnerData` derives `PartialEq, Eq`, and Rust's
  `PartialOrd` contract requires `a == b` exactly when
  `a.partial_cmp(&b) == Some(Ordering::Equal)`. So there is no room for
  equivalence classes coarser than equality: two documents that differ at all —
  same teacher under a different id, same id with a different name — must *not*
  compare `Equal`. They compare incomparable (or strictly ordered, where one is
  an honest content-subset of the other). The design doc's "equivalence classes
  modulo the id issuer if the issuer gets in the way" resolves to nothing
  special: the id issuer lives in `Data` *outside* `InnerData`, and
  `PartialOrd for Data` delegates to `inner_data` exactly as its hand-written
  `PartialEq` already does (`state-colloscopes/src/lib.rs:191-197`). No further
  quotienting is needed or allowed.
* **The minimal element is `Default::default()`** (explicit user ruling from the
  step-6 contract). Fuzz property (a) pins it. This forces one non-obvious
  design choice for the three configuration values whose `Default` is
  non-trivial — see the "flat order" rule in §1.

What this step does **not** do: no storage format change, no change to the op
surface, no change to `ops/`, no gtk4 change, no new dependencies (so no Nix
`cargoHash` refresh). Nothing in production calls the cascade yet; that is still
step 7's decision.

### Commit map

The user sketched four commits. Commit 1 splits naturally in two (a generic
layer in `state/` that is testable on toy types, then the domain
implementations), giving:

| commit | contents | user's sketch |
| --- | --- | --- |
| 0 | this plan document | — |
| 1 | generic comparison layer in `state/` (`partial_order` module, `Table`/`OrderedTable` comparison methods) + unit tests | commit 1 (first half) |
| 2 | `PartialOrd` on `InnerData`, `Data` and every component type in `state-colloscopes/` + unit tests | commit 1 (second half) |
| 3 | `PartialOrd` on the mock types (`QuoteData`, `EvilQuoteData`) + unit tests | commit 2 |
| 4 | `Fixable: PartialOrd` bound, the in-loop strictly-below assertion, two new evil modes and two new engine tests | commit 3 |
| 5 | the two fuzz properties (`property_partial_order.rs`) | commit 4 |

Each commit builds and passes the whole workspace suite on its own.

---

## 1. The order, in one page

We call the order defined here **the document order**. It is built from seven
named building blocks. Every type in §2 is assigned exactly one composition of
these blocks, field by field. The blocks are:

1. **Atom** (discrete order). Comparable only when equal:
   `partial_cmp` is `Some(Equal)` iff the two values are `==`, otherwise `None`.
   Used for every plain scalar and for every compound value that no fix ever
   modifies in place and that carries no removable sub-structure: strings,
   booleans, integers, ids, times, ranges, `SubjectPeriodicity` as a whole,
   `RulePart` as a whole, and so on. A changed atom makes the two documents
   incomparable — which is exactly what the in-loop assertion should treat as a
   map bug.

2. **Option lift.** `None` is strictly below `Some(_)`; two `Some` values
   compare by the inner rule. This applies to **every** `Option` field
   (uniform rule): clearing optional content is removing content, hence a
   strict decrease. The rule is load-bearing for the two optional foreign keys a
   fix actually clears (`Slot::week_pattern`,
   `Incompatibility::week_pattern_id`) and harmless everywhere else.

3. **Set inclusion.** For `BTreeSet<T>`: `Equal` iff equal, `Less` iff strict
   subset, `Greater` iff strict superset, `None` otherwise. **Never** the
   standard library's lexicographic `Ord` — under lexicographic comparison,
   removing an element can make a set *larger* (`{1,3}` sorts after `{1,2,3}`),
   which would break the whole step. This applies to every id set
   (`excluded_periods`, `excluded_weeks`, `excluded_students`, a teacher's
   `subjects`, assignment rows, interrogation cells).

4. **Map inclusion with a value rule.** For a keyed collection (a `Table` or a
   `BTreeMap`): the left side is below the right side iff its key set is
   included in the right's key set **and** every shared key's value is below or
   equal under the given value rule. `Equal` iff key sets are equal and every
   value compares `Equal`. Removing a row is a strict decrease; shrinking a
   row's value is a strict decrease; a row whose value moved "up" or sideways
   makes the maps incomparable. Rows are matched **by id**: a document holding
   the same teacher under a different id is not below, above, or equal — it is
   incomparable (the user's defining example).

5. **Sequence embedding (subsequence).** For an ordered list of ids
   (`Vec<WeekId>`, `Vec<SlotId>`, and the key sequence of an `OrderedTable`):
   `Less` iff the left is a strict subsequence of the right (same relative
   order, elements removed), `Equal` iff identical, `Greater` symmetric,
   `None` otherwise. Removing an element from the middle preserves the order of
   the survivors, so it is a strict decrease; *reordering* is incomparable —
   ordering is user-visible data, and no fix reorders anything.

6. **Same-length pointwise.** For `Vec<PrefilledGroup>` only: incomparable
   unless the lengths match, then the product of the index-wise comparisons.
   The group count of a prefilled group list is pinned to its
   `group_names` length by the sealed constructor (`GroupList::new`), and
   `group_names` sits in an atom field, so two comparable group lists always
   have the same group count; the fix that removes one dead student from one
   group is then a strict pointwise decrease. (A subsequence rule would be
   ill-defined here: matching structured elements is ambiguous.)

7. **Flat with a bottom.** For exactly three types — `Limits`,
   `BalancingOptions`, `ExportConfig` — the order is:
   `Equal` iff equal, `Less` iff the left is the type's `Default::default()`
   value (and the right is not), `Greater` symmetric, `None` otherwise.
   Rationale: these are configuration values. No fix ever modifies one in place
   (the settings/balancing fixes remove whole override *rows*; nothing touches
   the export configuration), so no fix depends on their internal structure.
   But `BalancingOptions::default()` and `ExportConfig::default()` are
   **non-trivial** (`balancing.rs:60-73`, `export_config.rs`): they contain
   `Some` values, `true` flags, named sheets. A structural per-field order could
   therefore never make the default document the minimum — a user who turns
   `stripes_color_enabled` off would produce a document with *less* structural
   content than the default, sitting below it or beside it, and fuzz property
   (a) would be false. The flat order makes each type's default its bottom **by
   definition**, keeps `Default::default() ≤ every reachable state` true, and
   is the strictest possible order elsewhere (any other change is
   incomparable). `Limits::default()` happens to be all-`None`, so a structural
   order would have worked for it alone; it gets the flat order anyway so that
   all three configuration values follow one rule.

8. **Product.** For a struct: the product of its field comparisons. `Equal`
   iff every field is `Equal`; `Less` iff no field is `Greater` or
   incomparable and at least one is `Less`; `None` as soon as one field is
   incomparable or two fields disagree in direction. A struct whose fields are
   all atoms degenerates to an atom — which is why `SubjectPeriodicity`,
   `RulePart`, `GroupListParameters` and friends need no implementation of
   their own.

Two implementation rules apply everywhere:

* **Never `#[derive(PartialOrd)]`, never the standard library's container
  orders.** Both are lexicographic and are wrong for this order (see block 3).
  Every implementation is hand-written from the helpers of commit 1.
* **Destructure `self` completely in every implementation** —
  `let Self { a, b, c } = self;` with no `..` — so that adding a field to a
  struct breaks the compilation of its `partial_cmp` and the order cannot
  silently forget a field (which would break the `Eq`-consistency contract).

---

## 2. The complete specification, type by type

Notation: "atom", "lift(X)", "set-inclusion", "map-inclusion(V)",
"embedding", "pointwise", "flat", "product" refer to §1's blocks.
"map-inclusion(V)" means map inclusion whose value rule is V.

### 2.1 The two roots

| type | rule |
| --- | --- |
| `Data` (`lib.rs`) | delegates to `inner_data.partial_cmp(&other.inner_data)` — the id issuer is ignored, mirroring the existing `PartialEq for Data` |
| `InnerData` (`lib.rs`) | product: `params` × `colloscope` × `export_config` |

### 2.2 `Parameters` and its fourteen fields (`colloscope_params.rs`)

`Parameters` is the product of all fourteen fields, each compared by its own
`PartialOrd` given below.

| type (module) | rule |
| --- | --- |
| `Periods` (`periods.rs`) | product: `first_week` lift(atom `WeekStart`) × `ordered_period_list` embedding of `OrderedTable<PeriodId, ()>` (unit values always `Equal`) |
| `Weeks` (`weeks.rs`, private fields) | product: `week_map` map-inclusion(`Week`) × `ordering` map-inclusion(sequence embedding of `Vec<WeekId>`) |
| `Week` (`weeks.rs`) | product: `period_id` atom × `interrogations` atom × `annotation` lift(atom) |
| `Subjects` (`subjects.rs`) | embedding of `OrderedTable<SubjectId, Subject>` with pointwise `Subject` values |
| `Subject` (`subjects.rs`) | product: `parameters` (`SubjectParameters`) × `excluded_periods` set-inclusion |
| `SubjectParameters` (`subjects.rs`) | product: `name` atom × `interrogation_parameters` lift(atom) — the whole `SubjectInterrogationParameters` (ranges, duration, flag, `periodicity` with all its variants and `WeekBlock` lists) is **one atom**; no fix modifies any of it in place |
| `Teachers` (`teachers.rs`) | map-inclusion(`Teacher`) |
| `Teacher` (`teachers.rs`) | product: `desc` (`PersonWithContact`) × `subjects` set-inclusion |
| `PersonWithContact` (`lib.rs`) | product: `surname` atom × `firstname` atom × `tel` lift(atom) × `email` lift(atom) |
| `Students` (`students.rs`) | map-inclusion(`Student`) |
| `Student` (`students.rs`) | product: `desc` × `excluded_periods` set-inclusion |
| `Assignments` (`assignments.rs`) | `map` map-inclusion(set-inclusion of `BTreeSet<StudentId>`) — key `(PeriodId, SubjectId)` |
| `WeekPatterns` (`week_patterns.rs`) | map-inclusion(`WeekPattern`) |
| `WeekPattern` (`week_patterns.rs`) | product: `name` atom × `excluded_weeks` set-inclusion |
| `Slots` (`slots.rs`, private fields) | product: `slot_map` map-inclusion(`Slot`) × `ordering` map-inclusion(sequence embedding of `Vec<SlotId>`) |
| `Slot` (`slots.rs`) | product: `subject_id` atom × `teacher_id` atom × `start_time` atom × `extra_info` atom × `week_pattern` **lift**(atom) × `cost` atom |
| `Incompats` (`incompats.rs`) | map-inclusion(`Incompatibility`) |
| `Incompatibility` (`incompats.rs`) | product: `subject_id` atom × `name` atom × `slots` atom (the whole `Vec<SlotWithDuration>`) × `minimum_free_slots` atom × `week_pattern_id` **lift**(atom) |
| `GroupLists` (`group_lists.rs`) | product: `group_list_map` map-inclusion(`GroupList`) × `subjects_associations` map-inclusion(atom `GroupListId`) |
| `GroupList` (`group_lists.rs`, sealed) | product: `params` atom (the whole `GroupListParameters` — name, students-per-group range, `group_names`) × `filling` (`GroupListFilling`) |
| `GroupListFilling` (`group_lists.rs`) | `Prefilled`/`Prefilled`: same-length pointwise over `PrefilledGroup`; `Automatic`/`Automatic`: `excluded_students` set-inclusion; mixed variants: incomparable |
| `PrefilledGroup` (`group_lists.rs`) | `students` set-inclusion (single field) |
| `Settings` (`settings.rs`) | product: `global` flat(`Limits`) × `students` map-inclusion(flat `Limits`) |
| `Limits` (`settings.rs`) | **flat** with bottom `Limits::default()` |
| `Pairings` (`pairings.rs`) | map-inclusion(`PairingRule`) |
| `PairingRule` (`pairings.rs`, sealed) | product: `antecedent` atom (`RulePart` whole) × `consequent` atom × `excluded_periods` set-inclusion × `soft` atom |
| `SlotPairings` (`slot_pairings.rs`) | map-inclusion(`SlotPairingRule`) |
| `SlotPairingRule` (`slot_pairings.rs`, sealed) | product: `antecedent` atom (`SlotRulePart` whole) × `consequent` atom × `excluded_periods` set-inclusion × `soft` atom |
| `Balancing` (`balancing.rs`) | product: `global` flat(`BalancingOptions`) × `subjects` map-inclusion(flat `BalancingOptions`) |
| `BalancingOptions` (`balancing.rs`) | **flat** with bottom `BalancingOptions::default()` |

### 2.3 The colloscope and the export configuration

| type (module) | rule |
| --- | --- |
| `Colloscope` (`colloscopes.rs`, private fields) | product: `interrogations` map-inclusion(set-inclusion of `BTreeSet<u32>`) × `group_lists` map-inclusion(map-inclusion(atom `u32`) over `BTreeMap<StudentId, u32>`) |
| `ExportConfig` (`export_config.rs`) | **flat** with bottom `ExportConfig::default()` |

Types that get **no** implementation (they are atoms inside a parent's rule, and
implementing `PartialOrd` on them would only invite misuse):
`SubjectInterrogationParameters`, `SubjectPeriodicity`, `WeekBlock`,
`GroupListParameters`, `RulePart`, `SlotRulePart`, `SoftParam<T>`, `Color`,
`PageOrientation`, `GlobalConfig`, `ColloscopeConfig`, `PerStudentGroupsConfig`,
`PerGroupListConfig`, `WeekDesc`, `NonEmptyRangeInclusive<T>`.

---

## 3. Why this order is correct

### 3.1 Consistency with `Eq`

Every block reports `Some(Equal)` exactly when the two values are `==`: atoms by
definition; the lift because both sides must be `None` or both inner values
equal; inclusion/embedding/pointwise because mutual inclusion of finite
structures forces equality; the flat order by its first arm; and a product of
consistent fields is consistent **provided every field participates** — which the
mandatory full destructuring guarantees at compile time. So the `PartialOrd` /
`PartialEq` contract holds by construction.

### 3.2 Well-foundedness (the termination proof survives)

Every strict decrease under any block either removes an element from a finite
container, moves an `Option` from `Some` to `None`, or moves a flat value onto
its bottom. Each block is therefore well-founded on the finite values the
document holds, and a product of well-founded orders is well-founded (an
infinite strictly-decreasing chain would have to decrease some field infinitely
often). Strict monotonicity of fixes plus a well-founded order is the
termination proof, unchanged from step 6 — this step only *materializes* the
order it was already stated in.

### 3.3 Every arm of the resolution map lands strictly below

The audit was done against the live `resolution.rs` (all 44 `Some(...)` sites).
Every emitted op falls into one of these shapes, and each shape is a strict
decrease under §2:

1. **Row removal** — `WeekOp::Remove`, `SlotOp::Remove`, `IncompatOp::Remove`,
   `PairingOp::Remove`, `SlotPairingOp::Remove`: one key leaves a
   map-inclusion table (for weeks and slots, the entity also leaves the
   ordering sidecar — both fields of the product decrease or stay equal, so the
   product is `Less`).
2. **Row-clearing targeted writes** —
   `AssignmentOp::SetRow(period, subject, BTreeSet::new())`,
   `ColloscopeOp::SetInterrogation(slot, week, BTreeSet::new())`,
   `ColloscopeOp::SetGroupList(group_list, BTreeMap::new())`,
   `GroupListOp::AssignToSubject(period, subject, None)`,
   `SettingsOp::SetStudent(student, None)`,
   `BalancingOp::SetSubject(subject, None)`: canonical-absent storage means the
   row disappears — a map-inclusion key removal.
3. **Value-shrinking targeted writes** — the same `SetRow` /
   `SetInterrogation` / `SetGroupList` ops carrying the current value minus one
   element: the row's key survives and its value strictly decreases under
   set-inclusion (or map-inclusion for the placement map).
4. **Whole-value rewrites minus one element** — `SubjectOp::Update`,
   `StudentOp::Update`, `TeacherOp::Update`, `WeekPatternOp::Update`,
   `PairingOp::Update`, `SlotPairingOp::Update`, `GroupListOp::Update`: the
   rebuilt value differs from the live one only by one removed set element
   (excluded period, excluded week, teacher's subject, prefilled or excluded
   student), so the row's value is strictly below under the entity's product
   rule, all sibling fields comparing `Equal`.
5. **Optional-edge clears** — `SlotOp::Update` with `week_pattern: None`,
   `IncompatOp::Update` with `week_pattern_id: None`: strictly below by the
   Option lift, every other field equal.

No arm creates anything, renames anything, reorders anything, or moves a
configuration value — so no arm ever needs the comparability the atoms and the
flat order deliberately withhold. Conversely, the in-loop assertion holds every
*future* arm to exactly this discipline.

Note the interplay with D5.1 (content, not semantics): shapes 4 and 5 include
the arms that widen the semantics while shrinking the content (dropping a dead
excluded period, clearing a week pattern). Under §2 they are plain decreases,
as required.

### 3.4 `Default::default()` is below every reachable state

`InnerData::default()` has: every table empty (below any table by
map-inclusion/embedding), `first_week == None` (bottom of the lift),
`Settings::default()` (= empty override table + `Limits::default()`, the flat
bottom), `Balancing::default()` (= empty table + `BalancingOptions::default()`,
the flat bottom by definition), an empty colloscope, and
`ExportConfig::default()` (the flat bottom). Every component is at its
component-wise minimum, so the product is the universal minimum among all
documents — reachable or not. Fuzz property (a) checks the reachable half of
this claim end to end.

### 3.5 What the order rejects, on purpose

* Same entity content under a different id: incomparable (rows match by id).
* Same id, different name/time/cost/parameters: incomparable (atoms).
* Reordered subjects, periods, weeks-within-a-period, slots-within-a-subject:
  incomparable (embedding).
* An association or placement retargeted to a different live value:
  incomparable (atom value under map-inclusion).
* One table shrinking while another grows: incomparable (product mixing).
* Any change to a non-default configuration value: incomparable (flat).

Each of these would be a contract violation if a fix produced it, and the
in-loop assertion turns each into a panic.

---

## 4. Commit 0 — this plan

Add `docs/plans/plan_step_6_5.md` (this file), after user review. The design
doc's §8 step-6.5 paragraph already exists and needs no change at this point;
the close-out (§10) updates it once the step is delivered.

---

## 5. Commit 1 — the generic comparison layer in `state/`

### 5.1 New module `state/src/partial_order.rs`

A small combinator library, generic and domain-free. Full intended content
(doc comments abbreviated here; write them out in full):

```rust
//! Building blocks for hand-written `PartialOrd` implementations over
//! document-shaped data (the "document order" of the cascade's monotonicity
//! contract, design doc §8 step 6.5).
//!
//! The standard library's `PartialOrd`/`Ord` on containers is lexicographic
//! and is NOT what a removal-shaped order needs (removing an element can make
//! a set sort *later*). Every implementation of the document order is
//! hand-written from these helpers; `#[derive(PartialOrd)]` is never used.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Product order: combines per-field comparisons. `Equal` is neutral; two
/// fields pulling in opposite directions, or any incomparable field, make the
/// whole product incomparable (`None`).
pub fn combine(fields: impl IntoIterator<Item = Option<Ordering>>) -> Option<Ordering> {
    let mut acc = Ordering::Equal;
    for field in fields {
        match field? {
            Ordering::Equal => {}
            ord if acc == Ordering::Equal => acc = ord,
            ord if ord == acc => {}
            _ => return None,
        }
    }
    Some(acc)
}

/// Discrete order: comparable only when equal.
pub fn discrete<T: PartialEq + ?Sized>(a: &T, b: &T) -> Option<Ordering> {
    (a == b).then_some(Ordering::Equal)
}

/// Flat order with a designated bottom: `Equal` when equal, otherwise the
/// bottom is below everything and every other pair is incomparable.
pub fn flat_with_bottom<T: PartialEq>(a: &T, b: &T, bottom: &T) -> Option<Ordering> {
    if a == b {
        Some(Ordering::Equal)
    } else if a == bottom {
        Some(Ordering::Less)
    } else if b == bottom {
        Some(Ordering::Greater)
    } else {
        None
    }
}

/// Option lift: `None` is the bottom, two `Some` values compare by `inner`.
pub fn option_lift<T>(
    a: &Option<T>,
    b: &Option<T>,
    inner: impl FnOnce(&T, &T) -> Option<Ordering>,
) -> Option<Ordering> {
    match (a, b) {
        (None, None) => Some(Ordering::Equal),
        (None, Some(_)) => Some(Ordering::Less),
        (Some(_), None) => Some(Ordering::Greater),
        (Some(x), Some(y)) => inner(x, y),
    }
}

/// Set inclusion: strict subset is strictly below.
pub fn set_inclusion<T: Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> Option<Ordering> {
    match (a.is_subset(b), b.is_subset(a)) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// Map inclusion with a value rule: `a` is below `b` iff `a`'s keys are
/// included in `b`'s and every shared key's value is below or equal.
pub fn map_inclusion<K: Ord, V>(
    a: &BTreeMap<K, V>,
    b: &BTreeMap<K, V>,
    value_cmp: impl Fn(&V, &V) -> Option<Ordering>,
) -> Option<Ordering> {
    let a_in_b = a.keys().all(|k| b.contains_key(k));
    let b_in_a = b.keys().all(|k| a.contains_key(k));
    let keys = match (a_in_b, b_in_a) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => return None,
    };
    combine(
        std::iter::once(Some(keys)).chain(
            a.iter()
                .filter_map(|(k, va)| b.get(k).map(|vb| value_cmp(va, vb))),
        ),
    )
}

/// Sequence embedding: a strict subsequence (same relative order, elements
/// removed) is strictly below; a reordering is incomparable.
pub fn subsequence<T: PartialEq>(a: &[T], b: &[T]) -> Option<Ordering> {
    fn embeds<T: PartialEq>(small: &[T], big: &[T]) -> bool {
        let mut rest = big.iter();
        small.iter().all(|x| rest.any(|y| y == x))
    }
    match (embeds(a, b), embeds(b, a)) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Less),
        (false, true) => Some(Ordering::Greater),
        (false, false) => None,
    }
}

/// Same-length pointwise product: incomparable unless the lengths match.
pub fn same_length_pointwise<T>(
    a: &[T],
    b: &[T],
    cmp: impl Fn(&T, &T) -> Option<Ordering>,
) -> Option<Ordering> {
    if a.len() != b.len() {
        return None;
    }
    combine(a.iter().zip(b).map(|(x, y)| cmp(x, y)))
}
```

(`embeds(a, b)` with both lengths equal is only true when the slices are equal,
so the `(true, true)` arm really is `Equal`; the greedy scan is the standard
correct subsequence test.)

Declare it in `state/src/lib.rs`. Old code:

```rust
pub mod join;
pub mod refs;
```

New code:

```rust
pub mod join;
pub mod partial_order;
pub mod refs;
```

No root-level `pub use`: call sites use the module path
(`collomatique_state::partial_order::combine` etc.), which keeps the helper
names from colliding with anything.

### 5.2 Comparison methods on the containers (`state/src/tables.rs`)

The tables' backing stores are private, so the inclusion comparisons must be
offered as methods. Add to `impl<I: Key, T> Table<I, T>` (beside `iter`):

```rust
/// Compares two tables under the document order's map inclusion (see
/// [crate::partial_order::map_inclusion]): row removal is strictly below,
/// row-value shrinking (per `value_cmp`) is strictly below, anything else is
/// incomparable.
pub fn partial_cmp_with(
    &self,
    other: &Self,
    value_cmp: impl Fn(&T, &T) -> Option<std::cmp::Ordering>,
) -> Option<std::cmp::Ordering> {
    crate::partial_order::map_inclusion(&self.inner, &other.inner, value_cmp)
}
```

And to `impl<I: OrderedKey, T> OrderedTable<I, T>`:

```rust
/// Compares two ordered tables under the document order: the key sequence by
/// embedding (removal keeps the survivors' relative order; reordering is
/// incomparable) and shared keys' values by `value_cmp`.
pub fn partial_cmp_with(
    &self,
    other: &Self,
    value_cmp: impl Fn(&T, &T) -> Option<std::cmp::Ordering>,
) -> Option<std::cmp::Ordering> {
    let self_keys: Vec<I> = self.keys().collect();
    let other_keys: Vec<I> = other.keys().collect();
    let keys = crate::partial_order::subsequence(&self_keys, &other_keys)?;
    crate::partial_order::combine(std::iter::once(Some(keys)).chain(
        self.iter().filter_map(|(id, value)| {
            other.get(&id).map(|other_value| value_cmp(value, other_value))
        }),
    ))
}
```

(Keys of an `OrderedTable` are unique, so the embedding is unambiguous.)

### 5.3 Commit-1 unit tests

In `partial_order.rs` `#[cfg(test)]` (plain `u64`/`BTreeSet`/`BTreeMap`
fixtures), one test per behavior:

* `combine`: all-equal → `Equal`; a `Less` among equals → `Less`; `Less` and
  `Greater` mixed → `None`; any `None` → `None`; empty iterator → `Equal`.
* `discrete`: equal → `Equal`, different → `None`.
* `flat_with_bottom`: bottom below non-bottom (both directions), two
  non-bottom values incomparable, equal → `Equal`.
* `option_lift`: the four arms, including `Some`/`Some` delegating to the
  inner rule.
* `set_inclusion`: equal, strict subset, strict superset, crossed sets
  (`{1,3}` vs `{2,3}`) → `None`; **the lexicographic trap pinned**: assert
  `set_inclusion({1,3}, {1,2,3}) == Some(Less)` while std's `Ord` would say
  greater.
* `map_inclusion`: key-subset with equal values → `Less`; same keys with one
  value `Less` → `Less`; key-subset but a shared value `Greater` → `None`;
  incomparable value → `None`.
* `subsequence`: middle removal `[1,3]` vs `[1,2,3]` → `Less`; reorder
  `[2,1]` vs `[1,2]` → `None`; equal; empty below anything.
* `same_length_pointwise`: length mismatch → `None`; pointwise decrease →
  `Less`; mixed directions → `None`.

In `tables.rs` `#[cfg(test)]` (reusing `ToyId`):

* `table_partial_cmp_with_row_removal_is_less`,
* `table_partial_cmp_with_value_shrink_is_less` (value rule `set_inclusion`),
* `ordered_table_partial_cmp_with_middle_removal_is_less`,
* `ordered_table_partial_cmp_with_reorder_is_none`,
* `ordered_table_partial_cmp_with_value_change_uses_value_rule`.

---

## 6. Commit 2 — the document order on `InnerData` and `Data`

One `impl PartialOrd` per type of §2, written next to the type it orders, each
with a short doc comment naming the document order and pointing at the design
doc (the durable home once this plan retires). All implementations follow the
same pattern; representative snippets below, the rest are mechanical
repetitions of §2's table.

Import line per touched module (adjust the helper list to what the module
uses):

```rust
use collomatique_state::partial_order::{combine, discrete, option_lift, set_inclusion};
```

(Inside `state-colloscopes`, `collomatique_state` is the existing dependency
path used everywhere else.)

### 6.1 The roots (`state-colloscopes/src/lib.rs`)

Old code (context — the existing hand-written equality, `lib.rs:191-197`):

```rust
impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        self.inner_data == other.inner_data
    }
}

impl Eq for Data {}
```

New code, immediately below:

```rust
/// The document order (design doc §8, step 6.5): delegates to [InnerData],
/// ignoring the id issuer exactly as [PartialEq] does. Two [Data] with equal
/// inner data compare `Equal` even when their issuers differ.
impl PartialOrd for Data {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner_data.partial_cmp(&other.inner_data)
    }
}
```

And for `InnerData` (below its struct definition):

```rust
/// The document order (design doc §8, step 6.5): the product of the three
/// components. `Default::default()` is the universal minimal element; the
/// cascade's resolution map must move every fix strictly downward in this
/// order, and the engine asserts it in-flight.
impl PartialOrd for InnerData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let InnerData {
            params,
            colloscope,
            export_config,
        } = self;
        combine([
            params.partial_cmp(&other.params),
            colloscope.partial_cmp(&other.colloscope),
            export_config.partial_cmp(&other.export_config),
        ])
    }
}
```

`PersonWithContact` also lives in `lib.rs`:

```rust
impl PartialOrd for PersonWithContact {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let PersonWithContact {
            surname,
            firstname,
            tel,
            email,
        } = self;
        combine([
            discrete(surname, &other.surname),
            discrete(firstname, &other.firstname),
            option_lift(tel, &other.tel, |a, b| discrete(a, b)),
            option_lift(email, &other.email, |a, b| discrete(a, b)),
        ])
    }
}
```

### 6.2 `Parameters` (`colloscope_params.rs`)

```rust
impl PartialOrd for Parameters {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let Parameters {
            periods,
            weeks,
            subjects,
            teachers,
            students,
            assignments,
            week_patterns,
            slots,
            incompats,
            group_lists,
            settings,
            pairings,
            slot_pairings,
            balancing,
        } = self;
        combine([
            periods.partial_cmp(&other.periods),
            weeks.partial_cmp(&other.weeks),
            subjects.partial_cmp(&other.subjects),
            teachers.partial_cmp(&other.teachers),
            students.partial_cmp(&other.students),
            assignments.partial_cmp(&other.assignments),
            week_patterns.partial_cmp(&other.week_patterns),
            slots.partial_cmp(&other.slots),
            incompats.partial_cmp(&other.incompats),
            group_lists.partial_cmp(&other.group_lists),
            settings.partial_cmp(&other.settings),
            pairings.partial_cmp(&other.pairings),
            slot_pairings.partial_cmp(&other.slot_pairings),
            balancing.partial_cmp(&other.balancing),
        ])
    }
}
```

### 6.3 Representative entity implementations

`Weeks` and `Week` (`weeks.rs` — private fields, so the impl must live in this
module; `Slots`/`Slot` in `slots.rs` are the exact same shape with
`slot_map`/`ordering` and the `Slot` product of §2):

```rust
impl PartialOrd for Weeks {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let Weeks { week_map, ordering } = self;
        combine([
            week_map.partial_cmp_with(&other.week_map, |a, b| a.partial_cmp(b)),
            ordering.partial_cmp_with(&other.ordering, |a, b| subsequence(a, b)),
        ])
    }
}

impl PartialOrd for Week {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let Week {
            period_id,
            interrogations,
            annotation,
        } = self;
        combine([
            discrete(period_id, &other.period_id),
            discrete(interrogations, &other.interrogations),
            option_lift(annotation, &other.annotation, |a, b| discrete(a, b)),
        ])
    }
}
```

`Slot` (`slots.rs`) — the one entity where the Option lift is load-bearing on a
live fix arm:

```rust
impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let Slot {
            subject_id,
            teacher_id,
            start_time,
            extra_info,
            week_pattern,
            cost,
        } = self;
        combine([
            discrete(subject_id, &other.subject_id),
            discrete(teacher_id, &other.teacher_id),
            discrete(start_time, &other.start_time),
            discrete(extra_info, &other.extra_info),
            option_lift(week_pattern, &other.week_pattern, |a, b| discrete(a, b)),
            discrete(cost, &other.cost),
        ])
    }
}
```

`Subjects` (`subjects.rs`) — the ordered table:

```rust
impl PartialOrd for Subjects {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let Subjects {
            ordered_subject_list,
        } = self;
        ordered_subject_list
            .partial_cmp_with(&other.ordered_subject_list, |a, b| a.partial_cmp(b))
    }
}
```

`GroupList` and `GroupListFilling` (`group_lists.rs` — sealed type, private
fields, impl in-module):

```rust
impl PartialOrd for GroupList {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let GroupList { params, filling } = self;
        combine([
            discrete(params, &other.params),
            filling.partial_cmp(&other.filling),
        ])
    }
}

impl PartialOrd for GroupListFilling {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (
                GroupListFilling::Prefilled { groups: a },
                GroupListFilling::Prefilled { groups: b },
            ) => same_length_pointwise(a, b, |x, y| x.partial_cmp(y)),
            (
                GroupListFilling::Automatic {
                    excluded_students: a,
                },
                GroupListFilling::Automatic {
                    excluded_students: b,
                },
            ) => set_inclusion(a, b),
            _ => None,
        }
    }
}

impl PartialOrd for PrefilledGroup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let PrefilledGroup { students } = self;
        set_inclusion(students, &other.students)
    }
}
```

`Limits` (`settings.rs`) — the flat pattern, identical for `BalancingOptions`
(`balancing.rs`) and `ExportConfig` (`export_config.rs`):

```rust
/// The document order: configuration values compare flat — the type's
/// [Default] value is the bottom, everything else is comparable only to
/// itself. No cascade fix modifies a configuration value in place (fixes
/// remove whole override rows), and the non-trivial defaults of the sibling
/// configuration types rule out a structural per-field order (design doc §8,
/// step 6.5).
impl PartialOrd for Limits {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        flat_with_bottom(self, other, &Limits::default())
    }
}
```

`Colloscope` (`colloscopes.rs`):

```rust
impl PartialOrd for Colloscope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let Colloscope {
            interrogations,
            group_lists,
        } = self;
        combine([
            interrogations.partial_cmp_with(&other.interrogations, |a, b| set_inclusion(a, b)),
            group_lists.partial_cmp_with(&other.group_lists, |a, b| {
                map_inclusion(a, b, |x, y| discrete(x, y))
            }),
        ])
    }
}
```

The remaining implementations — `Periods`, `Subject`, `SubjectParameters`,
`Teachers`, `Teacher`, `Students`, `Student`, `Assignments`, `WeekPatterns`,
`WeekPattern`, `Slots`, `Incompats`, `Incompatibility`, `GroupLists`,
`Settings`, `Pairings`, `PairingRule`, `SlotPairings`, `SlotPairingRule`,
`Balancing`, `BalancingOptions`, `ExportConfig` — follow §2's table with the
same patterns (product via `combine` + full destructuring; tables via
`partial_cmp_with`; sets via `set_inclusion`; options via `option_lift`; flat
via `flat_with_bottom`). For the sealed `PairingRule`/`SlotPairingRule` the
implementation lives in their modules and destructures the private fields:

```rust
impl PartialOrd for PairingRule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let PairingRule {
            antecedent,
            consequent,
            excluded_periods,
            soft,
        } = self;
        combine([
            discrete(antecedent, &other.antecedent),
            discrete(consequent, &other.consequent),
            set_inclusion(excluded_periods, &other.excluded_periods),
            discrete(soft, &other.soft),
        ])
    }
}
```

### 6.4 Commit-2 unit tests

New file `state-colloscopes/src/partial_order_tests.rs`, declared from
`lib.rs` as `#[cfg(test)] mod partial_order_tests;` (the same in-crate pattern
as `resolution/innocent_tests.rs` — the tests forge ids with
`unsafe { XxxId::new(n) }` and reach private fields where needed through the
public ops or the crate-internal mutators). Every test builds a value and a
twin, compares, and asserts the exact `Option<Ordering>`.

Must-cover list (one test each; names indicative):

* `data_ignores_the_id_issuer` — two `Data` built by different op sequences
  ending on the same `InnerData` compare `Some(Equal)`.
* `default_is_below_a_populated_document` — build a document through the gate
  (the `data_with_assignment` recipe from `lib.rs`'s test modules is a good
  base), assert `InnerData::default() <= *data.get_inner_data()` and
  `InnerData::default() < *data.get_inner_data()`.
* `row_removal_is_strictly_below` — remove one teacher from a two-teacher
  `Teachers` value: `Some(Less)`.
* `same_content_under_a_different_id_is_incomparable` — the user's defining
  example: two `Teachers` holding one identical `Teacher` under two different
  ids: `None`.
* `same_id_with_a_different_name_is_incomparable` — same key, different
  `desc.surname`: `None`.
* `reorder_is_incomparable` — two `Subjects` with the same two entries in
  swapped order: `None`; the same for a `Weeks` value whose per-period
  ordering vector is permuted (week entities untouched).
* `middle_removal_in_an_ordered_list_is_strictly_below` — three subjects,
  middle one removed: `Some(Less)`.
* `excluded_period_drop_is_strictly_below` — a `Subject` twin with one
  `excluded_periods` element removed: `Some(Less)` (this is the D5.1
  content-not-semantics pin: the semantics widen, the content shrinks).
* `week_pattern_exclusion_drop_is_strictly_below` — same shape on
  `WeekPattern::excluded_weeks`.
* `optional_edge_clear_is_strictly_below` — a `Slot` twin with
  `week_pattern: None`: `Some(Less)`; a scalar tweak
  (`start_time`) on another twin: `None`.
* `contact_clear_is_strictly_below` — `PersonWithContact` with `tel` cleared:
  `Some(Less)` (pins the uniform Option rule beyond foreign keys).
* `assignment_row_shrink_and_clear` — value minus one student:
  `Some(Less)`; row removed: `Some(Less)`; student swapped for another:
  `None`.
* `association_retarget_is_incomparable` — `subjects_associations` value
  changed to another live `GroupListId`: `None`; entry removed: `Some(Less)`.
* `group_list_prefilled_minus_student_is_strictly_below` — rebuilt via
  `GroupList::new` with one student removed from one group: `Some(Less)`.
* `group_list_variant_change_is_incomparable` — `Prefilled` vs `Automatic`:
  `None`; and two `Prefilled` fillings with different group counts (via
  different `group_names`, so `params` differ too): `None`.
* `colloscope_cell_trim_is_strictly_below` — interrogation cell minus one
  group: `Some(Less)`; placement map minus one student: `Some(Less)`; a
  placement renumbered: `None`.
* `flat_config_bottom` — for each of `Limits`, `BalancingOptions`,
  `ExportConfig`: default below a modified value (`Some(Less)`), two distinct
  non-default values `None`.
* `settings_override_row_removal_is_strictly_below`.
* `pairing_rule_excluded_period_drop_is_strictly_below` and
  `pairing_rule_part_change_is_incomparable`.
* `mixed_directions_are_incomparable` — one document gains a student and loses
  a teacher relative to the other: `None`.

---

## 7. Commit 3 — the document order on the toy types

`state/src/test_utils.rs`. Old code (context — the current derives):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteData {
    pub students: BTreeSet<u64>,
    /// quote id -> author student id
    pub quotes: BTreeMap<u64, u64>,
}
```

New code, added below the `InMemoryData` impl (derives unchanged — the order is
hand-written like everywhere else):

```rust
/// The document order on the toy: students by set inclusion, quotes by map
/// inclusion with atomic authors (re-authoring a quote is incomparable,
/// removing one is strictly below).
impl PartialOrd for QuoteData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let QuoteData { students, quotes } = self;
        crate::partial_order::combine([
            crate::partial_order::set_inclusion(students, &other.students),
            crate::partial_order::map_inclusion(quotes, &other.quotes, |a, b| {
                crate::partial_order::discrete(a, b)
            }),
        ])
    }
}
```

`EvilQuoteData` derives `PartialEq` over both its fields (the inner data and
the mode), so its order must account for both to stay `Eq`-consistent; the mode
never changes during a run, so this costs nothing:

```rust
impl PartialOrd for EvilQuoteData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let EvilQuoteData(data, mode) = self;
        crate::partial_order::combine([
            data.partial_cmp(&other.0),
            crate::partial_order::discrete(mode, &other.1),
        ])
    }
}
```

Commit-3 unit tests (in `test_utils.rs`'s or `partial_order.rs`'s test module —
put them beside the toys):

* removing a quote → `Some(Less)`;
* adding a student → `Some(Greater)`;
* re-authoring an existing quote to another author → `None`;
* removing a student while adding a quote → `None`;
* equal → `Some(Equal)`.

---

## 8. Commit 4 — the `Fixable` bound and the in-flight assertion

### 8.1 The trait bound (`state/src/cascade.rs`)

Old code:

```rust
/// Implemented by data whose broken invariants can be repaired by ops: the
/// resolution map. (`PartialEq` backs the engine's no-op-fix panic.)
pub trait Fixable: InMemoryData + PartialEq {
```

New code:

```rust
/// Implemented by data whose broken invariants can be repaired by ops: the
/// resolution map. (`PartialOrd` materializes the document order of the
/// monotonicity contract; the engine checks every fix against it in-flight.
/// `PartialEq` comes with it and still backs the no-op-fix panic.)
pub trait Fixable: InMemoryData + PartialOrd {
```

(`PartialOrd` has `PartialEq` as a supertrait, so nothing else changes for
implementors; `QuoteData`, `EvilQuoteData` and `Data` all satisfy the new bound
after commits 2 and 3.)

In the `fix_invariant` doc comment, update the forward reference. Old text:

```rust
    /// well-founded, so this contract is the cascade's termination proof —
    /// a map that *grows* the state makes the cascade loop forever (step 6.5
    /// adds a `PartialOrd`-based in-flight check for exactly that).
```

New text:

```rust
    /// well-founded, so this contract is the cascade's termination proof.
    /// The order is materialized by the `PartialOrd` supertrait bound (the
    /// document order, design doc §8 step 6.5), and [apply_cascade] asserts
    /// after every fix that the state landed strictly below the pre-fix
    /// state — a growing or sideways map panics instead of hanging.
```

### 8.2 The engine assertion (`state/src/cascade.rs`)

Add `use std::cmp::Ordering;` to the imports. Old code (the success arm of the
apply match):

```rust
            Ok(backward) => {
                if let Some(before) = before
                    && *data == before
                {
                    panic!(
                        "resolution map violated the strict-monotonicity \
                         contract: fix {front:?} applied as a perfect no-op \
                         (return None when no material is present)"
                    );
                }
                stack.pop();
                applied.push(ReversibleOp {
                    forward: front,
                    backward,
                });
            }
```

New code — the equality check becomes a three-way check on the document order;
only fix ops are held to it, exactly as before (the `before` snapshot is still
taken only when `!is_target`, so a no-op *target* remains a legitimate perfect
no-op):

```rust
            Ok(backward) => {
                if let Some(before) = before {
                    match (*data).partial_cmp(&before) {
                        Some(Ordering::Less) => {}
                        Some(Ordering::Equal) => panic!(
                            "resolution map violated the strict-monotonicity \
                             contract: fix {front:?} applied as a perfect no-op \
                             (return None when no material is present)"
                        ),
                        not_below => panic!(
                            "resolution map violated the strict-monotonicity \
                             contract: fix {front:?} did not land strictly below \
                             the pre-fix state (partial_cmp = {not_below:?})"
                        ),
                    }
                }
                stack.pop();
                applied.push(ReversibleOp {
                    forward: front,
                    backward,
                });
            }
```

Also update the module docs' contract sentence the same way as §8.1 (the module
header currently ends the contract description at the no-op panic; add one
sentence saying every fix is additionally checked to land strictly below the
pre-fix state in the document order).

### 8.3 Two new evil modes (`state/src/test_utils.rs`)

Old code (context — the current mode enum and its doc):

```rust
/// The way an [EvilQuoteData] map misbehaves.
///
/// There is no state-*growing* mode: without a round fuse that scenario is a
/// hang, not a test — it belongs to step 6.5's `PartialOrd` in-flight check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvilMode {
    /// Always "fixes" by removing the invariant's own quote, even when it is
    /// absent — a fix that lands as a perfect no-op.
    Blind,
    /// "Fixes" a dangling quote by removing some *other* existing quote,
    /// answering `None` only once no other quote remains.
    WrongTargetElseNone,
    /// Returns an op that fails the precheck (`RemoveStudent` of an unknown id).
    InvalidFix,
    /// "Fixes" by *creating* a fresh dangling quote, then disowns the invariant
    /// that fresh quote raises.
    CreateThenDisown { fresh_quote: u64, fresh_author: u64 },
}
```

New code — the doc note is replaced (the growing scenario is now a panic, not a
hang) and two modes are added:

```rust
/// The way an [EvilQuoteData] map misbehaves.
///
/// The two step-6.5 modes ([EvilMode::CreateAuthor] and
/// [EvilMode::ReauthorExisting]) violate the contract while *resolving* the
/// invariant: before the in-flight document-order check they would have led
/// the cascade to a quiet, creative `Ok`. Now they panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvilMode {
    /// Always "fixes" by removing the invariant's own quote, even when it is
    /// absent — a fix that lands as a perfect no-op.
    Blind,
    /// "Fixes" a dangling quote by removing some *other* existing quote,
    /// answering `None` only once no other quote remains.
    WrongTargetElseNone,
    /// Returns an op that fails the precheck (`RemoveStudent` of an unknown id).
    InvalidFix,
    /// "Fixes" by *creating* a fresh dangling quote, then disowns the invariant
    /// that fresh quote raises.
    CreateThenDisown { fresh_quote: u64, fresh_author: u64 },
    /// "Fixes" a dangling author by *creating* the missing student — the
    /// state grows, which is exactly what a strictly-decreasing map may never
    /// do, however helpful it looks.
    CreateAuthor { author: u64 },
    /// "Fixes" a dangling author by re-authoring some existing quote to an
    /// existing student — the state moves *sideways* (incomparable), neither
    /// shrinking nor growing.
    ReauthorExisting { quote: u64, author: u64 },
}
```

And in `impl Fixable for EvilQuoteData`, two new match arms:

```rust
            EvilMode::CreateAuthor { author } => Some(QuoteOp::AddStudent(*author)),
            EvilMode::ReauthorExisting { quote, author } => Some(QuoteOp::SetQuote {
                quote: *quote,
                author: *author,
            }),
```

### 8.4 Two new engine tests (`state/src/cascade.rs`, tests module)

Numbered 10 and 11, following the file's existing comment style. Also tighten
test 6's expected string from `"strict-monotonicity"` to
`"applied as a perfect no-op"` so the three panic tests each pin their own
message.

```rust
    // 10. A fix that grows the state: the map "fixes" the dangling author by
    //     creating the missing student. Before step 6.5 this would have landed
    //     a quiet creative Ok; the document-order check panics instead.
    #[test]
    #[should_panic(expected = "did not land strictly below")]
    fn a_growing_fix_panics() {
        let mut data = EvilQuoteData(quote_data(&[1], &[]), EvilMode::CreateAuthor { author: 2 });
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 99,
            author: 2,
        });

        let _ = apply_cascade(&mut data, target);
    }

    // 11. A fix that moves the state sideways: an existing quote is re-authored
    //     to another existing student — nothing removed, nothing added, the
    //     result incomparable with the pre-fix state.
    #[test]
    #[should_panic(expected = "did not land strictly below")]
    fn a_sideways_fix_panics() {
        let mut data = EvilQuoteData(
            quote_data(&[1, 2], &[(10, 1)]),
            EvilMode::ReauthorExisting {
                quote: 10,
                author: 2,
            },
        );
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 99,
            author: 3,
        });

        let _ = apply_cascade(&mut data, target);
    }
```

(In test 10 the fix `AddStudent(2)` applies cleanly and the state
`{students: {1, 2}}` compares `Greater` to `{students: {1}}`. In test 11 the fix
`SetQuote { 10, author: 2 }` applies cleanly — student 2 exists — and the quote
map `{10 → 2}` is incomparable with `{10 → 1}`.)

### 8.5 What validates this commit

The full workspace suite. In particular the 11 `Ok` cascade fixtures
(`state-colloscopes/tests/cascade.rs`) and the two 50-seed × 500-op cascade fuzz
walks (`tests/property_cascade.rs`) now run every legitimate fix through the
strictly-below assertion — this is the end-to-end demonstration that §2's order
accepts every conforming arm. A failure here means the *order* is wrong (too
strict somewhere), not the map; fix the order against §2 and §3.3.

---

## 9. Commit 5 — the two fuzz properties

New file `state-colloscopes/tests/property_partial_order.rs`, the fourth
property harness, built on the same testgen machinery as
`tests/property_cascade.rs` (same imports, same `RunConfig` shape, same
`for_each_seed` / `bootstrap` / `generator::gen_op` / `OpLog` plumbing — copy
the head of that file and keep the walk bookkeeping on the success path exactly
as it is there; moving it perturbs the RNG trajectory).

Configuration, matching the house ruling for step-6-family harnesses (one
hardcoded const, no environment variables, no `#[ignore]` tiers):

```rust
const CONFIG: RunConfig = RunConfig {
    seeds: 50,
    ops_per_run: 500,
    invalid_fraction: 0.15,
};
```

### 9.1 Property (a): the empty document really is the universal minimum

A plain gated walk (`data.annotate` + `data.apply`, as in `property_ops.rs` —
no cascade). After every op that lands, assert the design-doc claim directly:

```rust
/// Design doc §8 step 6.5, fuzz (a): `Default::default()` is the universal
/// minimal element — below every state the gate can reach.
#[test]
fn default_is_below_every_reachable_state() {
    let landed = Cell::new(0usize);

    harness::for_each_seed(
        "default_is_below_every_reachable_state",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            let bottom = InnerData::default();
            let mut snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = next_op(rng, &data, &snapshots, log);
                let (annotated, _) = data.annotate(op);
                let ok = data.apply(&annotated).is_ok();
                stats.record(category, ok);
                if ok {
                    landed.set(landed.get() + 1);
                    assert!(
                        bottom <= *data.get_inner_data(),
                        "InnerData::default() must be below every reachable state",
                    );
                    maybe_snapshot(rng, &data, &mut snapshots);
                }
            }
        },
    );

    assert!(landed.get() > 0, "no op ever landed across all seeds");
}
```

(The `<=` operator is `partial_cmp` ∈ {`Less`, `Equal`} — exactly the claim.
The bootstrap document itself is also covered: the first landing's assertion
compares against a state that includes everything the bootstrap built.)

### 9.2 Property (b): every map answer is `None` or lands strictly below

The same gated walk, but the interesting event is a **rejection over broken
invariants**: at that point `data` is unchanged and valid (the gate rolled
back), which is precisely the state the cascade would consult the map on. For
*every* invariant in the reported set — not only the canonical first pick the
engine would take — ask the map, and when it answers with a fix, land that fix
through the force door on a clone and compare:

```rust
/// Design doc §8 step 6.5, fuzz (b): over generated broken states, every
/// `fix_invariant` answer is `None` or an op whose applied result sits
/// strictly below the pre-fix state — never above, never equivalent. This is
/// also the only systematic exercise of the map's `Some` branches (the
/// innocent-state tests of §9bis systematically cover `None`).
#[test]
fn every_fix_lands_strictly_below() {
    let probed_fixes = Cell::new(0usize);
    let broken_landings = Cell::new(0usize);

    harness::for_each_seed(
        "every_fix_lands_strictly_below",
        &CONFIG,
        |rng, log, stats| {
            let (state, _) = harness::bootstrap(rng);
            let mut data: Data = state.get_data().clone();
            let mut snapshots: Vec<InnerData> = vec![];

            for _ in 0..CONFIG.ops_per_run {
                let (category, op) = next_op(rng, &data, &snapshots, log);
                let (annotated, _) = data.annotate(op);
                match data.apply(&annotated) {
                    Ok(_) => {
                        stats.record(category, true);
                        maybe_snapshot(rng, &data, &mut snapshots);
                    }
                    Err(Error::BrokenInvariants(set)) => {
                        stats.record(category, false);
                        broken_landings.set(broken_landings.get() + 1);
                        for invariant in &set {
                            let Some(fix) = data.fix_invariant(invariant) else {
                                continue;
                            };
                            let mut fixed = data.clone();
                            fixed.force_apply(&fix).expect(
                                "a fix op emitted by the resolution map must \
                                 pass the prechecks",
                            );
                            assert_eq!(
                                fixed.get_inner_data().partial_cmp(data.get_inner_data()),
                                Some(std::cmp::Ordering::Less),
                                "fix {fix:?} for {invariant:?} must land strictly \
                                 below the pre-fix state",
                            );
                            probed_fixes.set(probed_fixes.get() + 1);
                        }
                    }
                    Err(Error::InvalidOp(_)) => stats.record(category, false),
                }
            }
        },
    );

    // Coverage guards (commit-8 lesson: count the specific outcome the test is
    // about, not a proxy). Without them the walk could go green with the map
    // never once answering `Some`.
    assert!(
        broken_landings.get() > 0,
        "no generated op ever broke an invariant across all seeds",
    );
    assert!(
        probed_fixes.get() > 0,
        "the map never answered Some across all seeds — the strictly-below \
         property was never exercised",
    );
}
```

Notes for the implementer:

* `fix_invariant` comes from `collomatique_state::Fixable` (already re-exported
  at the crate root); `force_apply` is the public force door on `Data`.
* Using `force_apply` (not `apply`) is deliberate and mirrors the engine: a fix
  is allowed to land a state that still breaks *other* invariants (mid-cascade
  states); the gate would bounce those and hide exactly the comparison this
  property is about. Prechecks still run, and a precheck failure is a map bug
  (the engine panics on it), hence the `expect`.
* Probing every member of the set — not just `set.first()` — is strictly wider
  than what the in-loop assertion of commit 4 sees, and is the point of having
  this property in addition to `property_cascade.rs`.
* Per-seed run time: the extra work is one `Data` clone per probed fix (a few
  thousand across the whole run, on documents of a few dozen entities) — well
  within the existing harness budget (`property_cascade.rs` runs 7.7 s).

---

## 10. Gate and close-out

**Per-commit gate:** `cargo build --workspace` and the full workspace test
suite (run in the background, output captured once to the scratchpad and
grepped — never run twice).

**End-of-step gate:** the full suite including the three existing property
harnesses at their committed configurations and the new
`property_partial_order.rs`. This step touches no storage bytes, no op
vocabulary, no `ops/` behavior and no gtk4 code, so no contract-script or GUI
surface changed; the standing user-run checks stay at the user's discretion.
No `Cargo.toml`/`Cargo.lock` change is expected (no new dependencies — no Nix
`cargoHash` refresh).

**Close-out (after the user's sign-off that the step is done):**

* update `docs/plans/invariant_cascade_design.md`: mark the §8 step-6.5
  paragraph completed with commit anchors, and record the delivered state as
  **Appendix I** (the §1/§2 order definition, the flat-order decision for the
  three configuration types and its `Default`-minimum rationale, the uniform
  Option rule, the engine assertion, the two new evil modes, the two fuzz
  properties with their measured numbers);
* retire this plan: delete `docs/plans/plan_step_6_5.md` in the close-out
  commit and pin it in the topic memory as `git show <commit>:docs/plans/plan_step_6_5.md`;
* update the topic memory: step 6.5 closed, next = step 7 (the `ops/`
  remaster).
