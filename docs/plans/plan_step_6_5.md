# Step 6.5 session plan — monotonicity checking (`ContentOrd` + derive)

**Status: DRAFT — awaiting user sign-off. No implementation commit has landed.**

This is the second version of the plan, with the second review round folded
in. The first version (committed as `ec0dd2a2`, readable via
`git show ec0dd2a2:docs/plans/plan_step_6_5.md`) built the same partial order
out of hand-written `PartialOrd` implementations; it was reworked after review
into the present design: a dedicated **`ContentOrd`** trait, implemented for
the regular shapes by a **derive macro** in the existing
`collomatique-state-derive` crate, with manual implementations only where the
shape is irregular. A second review round then corrected the *methodology* of
the order's definition itself (§0.2, decisions 8–11): the order is intrinsic
to the document type and pre-exists the resolution map; several rules were
re-derived accordingly.

This plan is self-sufficient: it contains every design decision, the complete
specification of the order, and old/new code for every touched site. It is
written to be implementable mechanically, commit by commit. The wider context
lives in `docs/plans/invariant_cascade_design.md` (§8 "Step 6.5" paragraph,
Appendix H).

---

## 0. Context: what step 6 left open, and what binds this step

Step 6 delivered the cascade: `apply_cascade` (in `state/src/cascade.rs`)
applies a target operation, and when the apply/check/rollback gate rejects it
over broken invariants, asks the resolution map (`Fixable::fix_invariant`,
implemented for `Data` in `state-colloscopes/src/resolution.rs`) for one repair
operation, pushes it on the retry stack, and loops. The engine has **no round
fuse** — deliberately, because no meaningful bound exists. Termination rests
entirely on the engraved map contract:

> States form a partial order with a universal minimal element:
> `Default::default()`, the empty document. Every returned op must land
> **strictly below** the current state in that order.

Step 6 enforced this contract only partially in-flight. The `None` convictions
and the no-op-fix panic catch every *removal-shaped* violation, but the order
itself was never materialized, so a map bug that keeps **growing** the state is
undetectable and makes the cascade loop forever. Step 6.5 closes that hole:

1. define the partial order as a real trait implementation on the document
   (`InnerData`, and `Data` by delegation),
2. require that trait on `Fixable` implementors (only there — the generic
   `InMemoryData` trait is untouched),
3. assert in the cascade loop, after every fix that lands, that the new state
   is **strictly below** the pre-fix state — turning a growing or sideways map
   into a loud panic instead of a hang,
4. fuzz the two halves of the contract: (a) `Default::default()` really is
   below every reachable state, and (b) every `fix_invariant` answer is `None`
   or an op whose applied result lands strictly below.

### 0.1 Binding constraints (★ user rulings, recorded in the design doc)

* **D5.1 — the order is over the document's *content*, not the meaning it
  denotes.** Several conforming map arms strictly shrink the data while
  *widening* the semantics: a subject that stops excluding a dead period now
  applies more broadly; a slot whose `week_pattern` is cleared now runs every
  week. In each case an id was removed and nothing was added, so the document
  strictly decreased. An order that compared meanings would reject these arms
  and break the termination proof. Every rule in §3 below is a rule about
  *content*. The trait's name, `ContentOrd`, records this ruling.
* **Compatibility with `Eq`.** `InnerData` derives `PartialEq, Eq`, and the
  order must agree with it: `content_cmp` returns `Some(Ordering::Equal)`
  exactly when the two values are `==`. So there is no room for equivalence
  classes coarser than equality: two documents that differ at all — same
  teacher under a different id, same id with a different name — must *not*
  compare `Equal`. They compare incomparable (or strictly ordered, where one is
  an honest content-subset of the other). The design doc's "equivalence classes
  modulo the id issuer if the issuer gets in the way" resolves to nothing
  special: the id issuer lives in `Data` *outside* `InnerData`, and the order
  on `Data` delegates to `inner_data` exactly as its hand-written `PartialEq`
  already does (`state-colloscopes/src/lib.rs:191-197`).
* **The minimal element is `Default::default()`** — an **axiom** of the
  engraved step-6 contract, not a theorem. Fuzz property (a) pins it, and it
  actively shapes the order: it *forces* the flat rule for the three
  configuration types whose `Default` is structurally non-minimal (§2,
  block 7).

### 0.2 Decisions from the design reviews (July 29 2026)

Settled with the user across the two review rounds:

1. **A derive, not thirty-odd hand-written impls.** The comparison rule
   travels *by type, through trait impls* (the serde model): containers get
   blanket impls once, entity structs get `#[derive(ContentOrd)]`, and the
   derive walks **all** fields by construction — a forgotten field becomes
   impossible rather than merely discouraged, and a new field whose type has no
   impl is a compile error that forces a decision.
2. **A dedicated trait, not `PartialOrd`, and required everywhere.** A derive
   *could* emit `impl PartialOrd`, but the trait the derive dispatches through
   cannot be `PartialOrd`: the standard library already implements it on
   `BTreeSet`/`BTreeMap`/`Vec`/`Option` **lexicographically** (wrong for a
   removal order — under it `{1,3} > {1,2,3}`), those impls cannot be replaced
   (std coherence), and the typed ids must keep their *numeric* `Ord` because
   they are `BTreeMap` keys — Rust requires `PartialOrd` and `Ord` to agree, so
   the ids could never carry a discrete `PartialOrd`. Hence `ContentOrd`, and
   the engine requires it directly: `Fixable: InMemoryData + ContentOrd`. **No
   type anywhere gains a new `PartialOrd`** — the name stays free for genuine
   total-order needs, and nobody can mistake the document order for a sortable
   order.
3. **Blanket `Vec` impl = subsequence.** A wrong default here can only be *too
   strict* (it never blesses a non-removal change), and too-strict fails loud —
   the engine's strictly-below panic, the fixtures, fuzz (b). The blanket is
   also what lets `Vec` fields *inside table values* (the week/slot ordering
   sidecars) participate through trait dispatch, where field attributes cannot
   reach. It is gated on `T: ContentOrd`, so it can never fire on a type that
   was not deliberately enrolled. Fields whose element identity is not
   value-borne override it (decisions 9–11).
4. **Attribute values are plain expressions, not strings.** House style
   (`#[join(error = NewId)]`, `#[fk(name = subject)]`) already writes unquoted
   attribute arguments; syn 2 parses an attribute value as an ordinary
   expression, and the macro splices it in callable position. So
   `#[ord(with = option_lift_discrete)]` and even `#[ord(with = |a, b| …)]`
   are both legal.
5. **The toys stay manual.** Derive-generated code emits absolute
   `::collomatique_state::` paths, which do not resolve inside `state/` itself
   (the same reason the existing derive tests are integration tests). The
   hand-written toy impls also test the engine and the combinators with zero
   dependence on the macro.
6. **The derive gets its own integration test** in
   `state/tests/derive_content_ord.rs`, in the style of `derive_refs.rs` /
   `derive_join.rs`.
7. **A `Vec` analogue of `option_lift_discrete` is provided**:
   `vec_subsequence` (elements matched by plain `==`, i.e. discretely), for
   `#[ord(with = …)]` on a `Vec` whose element type is foreign and cannot be
   enrolled in the trait.
8. **★ The content order *pre-exists* the resolution map** (second review
   round). The order is an intrinsic property of the document type, derived
   from first principles — content inclusion, D5.1, `Eq`-consistency, the
   `Default`-minimum axiom, scalar leaves as atoms. The map is then *obligated*
   to move strictly downward in it. "No fix touches this field" is **never** a
   reason for choosing a rule; such statements are theorems about the map and
   live in the §4.3 audit, not in the definition. (This corrected one rule
   outright: `Incompatibility::slots` is subsequence-ordered, not atomic.)
9. **The identity criterion for `Vec` fields** (second review round). Ask
   where an element's identity lives:
   * with the element's **value** (id lists, time windows) → **subsequence**;
   * with its **position** — indices bound to a sibling structure or
     referenced from outside → **prefix + pointwise** (the `Vec` read as a map
     from an initial segment of indices; map inclusion specialized);
   * in **relations between elements** (a chain) → the list is one composite
     value: an **atom**.
10. **★ `Vec<WeekBlock>` is an atom** (user ruling). The blocks refer to each
    other — each block's `delay_in_weeks` is measured from its predecessor —
    so the list is a relational chain, one composite schedule description, not
    a collection of independently removable items.
11. **Prefilled groups and group names are prefix-ordered** (user correction
    of the earlier same-length rule). Adding a group is adding content, and
    zero groups is the minimum element; removing a *middle* group is an
    identity shift, not a removal — group numbers are referenced *by index*
    from the colloscope's placement maps and interrogation cells — and stays
    incomparable. The `same_length_pointwise` combinator of the earlier draft
    is replaced by `prefix_pointwise`.

### 0.3 What this step does *not* do

No storage format change, no change to the op surface, no change to `ops/`
behavior, no gtk4 change, no new dependencies (`syn`/`quote`/`proc-macro2` are
already dependencies of `collomatique-state-derive`, so no Nix `cargoHash`
refresh). Nothing in production calls the cascade yet; that is still step 7's
decision.

### 0.4 Commit map

| commit | contents |
| --- | --- |
| 0 | this plan (the rework; the first draft is already committed as `ec0dd2a2`) |
| 1 | `state/`: the `ContentOrd` trait, the combinators, the leaf and container impls, the `impl_content_ord_atom!` macro, the `with =` helpers + unit tests |
| 2 | `collomatique-state-derive`: `#[derive(ContentOrd)]` with the three field attributes + integration tests in `state/tests/derive_content_ord.rs` |
| 3 | `state-colloscopes/`: adoption — the id enrollment, the derives with their ten attributes, five manual impls + the semantic unit tests |
| 4 | the toy types: manual `ContentOrd` for `QuoteData` / `EvilQuoteData` + unit tests |
| 5 | the engine: `Fixable: InMemoryData + ContentOrd`, the in-loop strictly-below check, two new evil modes, two new engine tests |
| 6 | the two fuzz properties (`state-colloscopes/tests/property_partial_order.rs`) |

Each commit builds and passes the whole workspace suite on its own. Commit 4
only needs commit 1; commits 3 and 5 need everything before them.

---

## 1. The trait

```rust
/// The document order: a partial order over *content* (design doc §8,
/// step 6.5, and ruling D5.1 — content, not the meaning it denotes).
///
/// This is the order of the cascade's monotonicity contract: states form a
/// partial order whose universal minimal element is `Default::default()`
/// (the empty document), and every resolution-map fix must land strictly
/// below the pre-fix state. [crate::apply_cascade] checks that in-flight.
/// The order is intrinsic to the data type — it is defined from the
/// structure alone, and the resolution map is *held to it*, never the other
/// way around.
///
/// # Laws
///
/// * **Consistency with equality**: `content_cmp` returns
///   `Some(Ordering::Equal)` exactly when the two values are `==` (hence the
///   `PartialEq` supertrait).
/// * **Antisymmetry and transitivity**, as for any partial order.
/// * **Well-foundedness on document data**: every strict decrease removes an
///   element from a finite container, moves an `Option` from `Some` to
///   `None`, or moves a flat configuration value onto its bottom — so there
///   is no infinite strictly-decreasing chain, and strict monotonicity of
///   fixes is a termination proof.
///
/// This is deliberately *not* `PartialOrd`: the standard library implements
/// `PartialOrd` lexicographically on containers (under which removing an
/// element can make a set sort *later*), and the typed ids must keep their
/// numeric `Ord` for use as map keys. A distinct trait keeps both worlds
/// intact and unambiguous.
pub trait ContentOrd: PartialEq {
    /// Compares two values in the document order.
    fn content_cmp(&self, other: &Self) -> Option<Ordering>;

    /// `self` is below or equal to `other` in the document order.
    fn content_le(&self, other: &Self) -> bool {
        matches!(
            self.content_cmp(other),
            Some(Ordering::Less | Ordering::Equal)
        )
    }

    /// `self` is strictly below `other` in the document order.
    fn content_lt(&self, other: &Self) -> bool {
        self.content_cmp(other) == Some(Ordering::Less)
    }
}
```

The comparison rule for a compound value is determined by its type's impl;
struct and enum impls are generated by `#[derive(ContentOrd)]` (commit 2) as
the product of their fields' rules. Three field attributes override the
default dispatch where a type cannot carry an impl (foreign types, the orphan
rule) or where the field's structure demands a different reading than its
type's default:

* `#[ord(atom)]` — compare this field discretely, inline (`==` or
  incomparable); no trait impl needed on the field's type.
* `#[ord(with = <expr>)]` — compare this field with the given expression,
  which must be callable as `fn(&T, &T) -> Option<Ordering>`; a path or an
  inline closure.
* `#[ord(partial_ord)]` — compare this field with its existing
  `PartialOrd::partial_cmp`. An explicit opt-in for a type whose `PartialOrd`
  *is* the right content order; never a default, because of the numeric-order
  traps above.

---

## 2. The order, block by block

The order is defined from first principles (decision 8): structure decomposes
until it bottoms out in scalar leaves, and each structural layer is read *as
content*. Eight named building blocks; §3 assigns every type a composition of
them.

1. **Atom** (discrete order). Comparable only when equal: `Some(Equal)` iff
   `==`, otherwise `None`. In order-theory vocabulary this is the discrete
   partial order — two different values are neither above nor below one
   another. Atoms are the scalar leaves of the document: strings, numbers,
   booleans, times, ids, ranges. A range deserves the explicit argument: its
   *content* is the endpoint pair, and reading `[2..=3] ⊆ [1..=4]` as an
   order would compare the denoted sets — exactly the semantic reading D5.1
   forbids. Text is likewise opaque: a string is a scalar value, not a
   container of characters.
2. **Option lift.** `None` is strictly below `Some(_)`; two `Some` values
   compare by the inner rule. This applies to **every** `Option` field:
   clearing optional content is removing content.
3. **Set inclusion.** `Equal` iff equal, `Less` iff strict subset, `Greater`
   iff strict superset, `None` otherwise.
4. **Map inclusion with a value rule.** Below iff the key set is included and
   every shared key's value is below or equal. Rows are matched **by id**: the
   same teacher under a different id is incomparable (the user's defining
   example); the same id with different content compares by the value rule.
5. **Sequence embedding (subsequence).** `Less` iff the left is a strict
   subsequence of the right — obtainable by *removing elements only*, the
   survivors keeping their relative order. Reordering is incomparable:
   ordering is user-visible data.
6. **Prefix pointwise.** A `Vec` whose positions carry the identity is
   content-wise a **map from indices to values**; map inclusion (block 4)
   specialized to index sets that are initial segments gives: below iff the
   left is at most as long *and* every shared index compares below or equal.
   Appending is adding content (the empty vector is the minimum); shrinking a
   value in place is a decrease; removing a *middle* element shifts the
   identity of every later element and is incomparable.
7. **Flat with a bottom.** For exactly three configuration types — `Limits`,
   `BalancingOptions`, `ExportConfig`: `Equal` iff equal, and the type's
   `Default::default()` value sits strictly below everything else; any other
   pair is incomparable. This rule is **forced by the `Default`-minimum
   axiom**, not chosen for convenience: `BalancingOptions::default()` and
   `ExportConfig::default()` are non-trivial (`balancing.rs:60-73`; the
   export defaults carry enabled flags and named sheets), so the structural
   reading puts states strictly *below* the default (clear
   `teacher_rotation` from the default and content was removed) —
   contradicting the axiom that nothing sits below `Default::default()`. Nor
   can "default ≤ everything" be glued onto the structural order: a state
   structurally below the default would then sit both above and below it,
   breaking antisymmetry. So every order honoring the axiom must make all
   other values incomparable-or-above; flat is the simplest such order.
   (`Limits::default()` happens to be all-`None`, but it follows the same
   rule so the three configuration values behave alike.)
8. **Product.** For a struct: the field-wise combination. `Equal` iff every
   field `Equal`; `Less` iff at least one `Less` and none `Greater` or
   incomparable; `None` as soon as one field is incomparable or two fields
   disagree in direction. A product of atoms degenerates to an atom.

**The identity criterion for `Vec` fields** (decision 9) selects among blocks
5, 6 and 1: identity borne by the element's *value* → subsequence (block 5);
identity borne by the *position* → prefix pointwise (block 6); identity in the
*relations between elements* (a chain) → the whole list is one composite
value, an atom (block 1). The document's instances: the week/slot ordering
sidecars and an incompatibility's time windows are value-borne (subsequence);
prefilled groups and group names are position-borne — group *i* binds to name
*i*, and group numbers are referenced by index from the colloscope
(prefix pointwise); the `WeekBlock` chain is relational — each block's delay
is measured from its predecessor (atom, ★ decision 10).

Two implementation rules apply everywhere: **never `#[derive(PartialOrd)]`,
never the standard library's container orders** (both lexicographic, wrong for
a removal order); and the derive covers all fields by construction, so no
manual impl may compare a strict subset of a type's fields (the
`Eq`-consistency law would break).

---

## 3. The complete specification, type by type

Notation: "atom", "lift(X)", "set-inclusion", "map-inclusion(V)",
"subsequence", "prefix(V)", "flat", "product" refer to §2. The **how** column
says what produces the impl: `derive` (with any attributes), `blanket` (a
container impl from commit 1), `macro` (`impl_content_ord_atom!`), `manual`,
or `—` (no impl; the type is only ever compared inside an atom or flat
boundary).

### 3.1 The two roots

| type | rule | how |
| --- | --- | --- |
| `Data` (`lib.rs`) | delegates to `inner_data`, ignoring the id issuer — mirroring `PartialEq for Data` | manual |
| `InnerData` (`lib.rs`) | product: `params` × `colloscope` × `export_config` | derive |

### 3.2 `Parameters` and everything under it

| type (module) | rule | how |
| --- | --- | --- |
| `Parameters` (`colloscope_params.rs`) | product of its fourteen fields | derive |
| `Periods` (`periods.rs`) | product: `first_week` lift(atom `WeekStart`) × `ordered_period_list` embedding (unit values always `Equal`) | derive; `first_week` gets `#[ord(with = option_lift_discrete)]` (foreign inner type) |
| `Weeks` (`weeks.rs`) | product: `week_map` map-inclusion(`Week`) × `ordering` map-inclusion(subsequence of `Vec<WeekId>`) | derive — the `Table` and `Vec` blankets compose |
| `Week` (`weeks.rs`) | product: `period_id` atom × `interrogations` atom × `annotation` lift(atom) | derive; `annotation` gets `#[ord(with = option_lift_discrete)]` |
| `Subjects` (`subjects.rs`) | embedding of `OrderedTable<SubjectId, Subject>` with pointwise `Subject` values | derive |
| `Subject` (`subjects.rs`) | product: `parameters` × `excluded_periods` set-inclusion | derive |
| `SubjectParameters` (`subjects.rs`) | product: `name` atom × `interrogation_parameters` lift(product) | derive — the `Option` blanket dispatches into the structural `SubjectInterrogationParameters` |
| `SubjectInterrogationParameters` (`subjects.rs`) | product: `students_per_group` atom (range) × `groups_per_interrogation` atom (range) × `duration` atom × `take_duration_into_account` atom × `periodicity` | derive; `duration` gets `#[ord(atom)]` (foreign `NonZeroMinutes`) |
| `SubjectPeriodicity` (`subjects.rs`) | enum: same variant → product of its fields (all atoms), different variants → incomparable | derive; the `blocks` field of `AmountForEveryArbitraryBlock` gets `#[ord(atom)]` — ★ decision 10: the `WeekBlock` chain is one composite value |
| `WeekBlock` (`subjects.rs`) | — (inside the `blocks` atom) | — |
| `NonEmptyRangeInclusive<T>` (`non_empty_range.rs`) | atom — content is the endpoint pair; interval inclusion would be the semantic reading D5.1 forbids | manual (generic impl; the macro cannot express generics) |
| `Teachers` (`teachers.rs`) | map-inclusion(`Teacher`) | derive |
| `Teacher` (`teachers.rs`) | product: `desc` × `subjects` set-inclusion | derive |
| `PersonWithContact` (`lib.rs`) | product: `surname` atom × `firstname` atom × `tel` lift(atom) × `email` lift(atom) | derive; `tel` and `email` get `#[ord(with = option_lift_discrete)]` (foreign `NonEmptyString`) |
| `Students` / `Student` (`students.rs`) | as `Teachers` / `Teacher`, with `excluded_periods` set-inclusion | derive |
| `Assignments` (`assignments.rs`) | `map` map-inclusion(set-inclusion) | derive |
| `WeekPatterns` / `WeekPattern` (`week_patterns.rs`) | map-inclusion(product: `name` atom × `excluded_weeks` set-inclusion) | derive |
| `Slots` (`slots.rs`) | product: `slot_map` map-inclusion(`Slot`) × `ordering` map-inclusion(subsequence of `Vec<SlotId>`) | derive |
| `Slot` (`slots.rs`) | product: `subject_id` atom × `teacher_id` atom × `start_time` atom × `extra_info` atom × `week_pattern` lift(atom) × `cost` atom | derive; `start_time` gets `#[ord(atom)]` (foreign `SlotStart`); `week_pattern` needs no attribute (`Option` blanket + id atom) |
| `Incompats` (`incompats.rs`) | map-inclusion(`Incompatibility`) | derive |
| `Incompatibility` (`incompats.rs`) | product: `subject_id` atom × `name` atom × `slots` **subsequence** × `minimum_free_slots` atom × `week_pattern_id` lift(atom) | derive; `slots` gets `#[ord(with = vec_subsequence)]` — a time window's identity is its value, so removing one is removing content (decision 8 corrected the earlier atom designation); the helper is needed because `SlotWithDuration` is foreign |
| `GroupLists` (`group_lists.rs`) | product: `group_list_map` map-inclusion(`GroupList`) × `subjects_associations` map-inclusion(atom `GroupListId`) | derive |
| `GroupList` (`group_lists.rs`, sealed) | product: `params` × `filling` | derive (in-module; private fields are fine) |
| `GroupListParameters` (`group_lists.rs`) | product: `name` atom × `students_per_group` atom (range) × `group_names` **prefix**(lift(atom)) — un-naming a group is below, renaming is incomparable, truncating is below, a middle removal shifts bindings and is incomparable | derive; `group_names` gets `#[ord(with = \|a, b\| prefix_pointwise(a, b, option_lift_discrete))]` (closure form; the element type is foreign) |
| `GroupListFilling` (`group_lists.rs`) | `Prefilled`/`Prefilled`: **prefix**(`PrefilledGroup`); `Automatic`/`Automatic`: `excluded_students` set-inclusion; mixed variants: incomparable | derive; the `groups` field gets `#[ord(with = vec_prefix)]` — position-borne identity (decision 11); the `Vec` blanket's subsequence would reject the minus-one-student fix (§7.4) |
| `PrefilledGroup` (`group_lists.rs`) | `students` set-inclusion | derive |
| `Settings` (`settings.rs`) | product: `global` flat × `students` map-inclusion(flat) | derive (`Limits` carries the flat impl) |
| `Limits` (`settings.rs`) | flat, bottom `Limits::default()` | manual |
| `Pairings` (`pairings.rs`) | map-inclusion(`PairingRule`) | derive |
| `PairingRule` (`pairings.rs`, sealed) | product: `antecedent` × `consequent` × `excluded_periods` set-inclusion × `soft` atom | derive (in-module) |
| `RulePart` (`pairings.rs`) | product: `subject_id` atom × `should_have` atom — a product of atoms, i.e. effectively an atom, but derived so any future field joins the order | derive |
| `SlotPairings` / `SlotPairingRule` / `SlotRulePart` (`slot_pairings.rs`) | as pairings | derive |
| `Balancing` (`balancing.rs`) | product: `global` flat × `subjects` map-inclusion(flat) | derive |
| `BalancingOptions` (`balancing.rs`) | flat, bottom `BalancingOptions::default()` | manual |

### 3.3 The colloscope and the export configuration

| type (module) | rule | how |
| --- | --- | --- |
| `Colloscope` (`colloscopes.rs`) | product: `interrogations` map-inclusion(set-inclusion of `BTreeSet<u32>`) × `group_lists` map-inclusion(map-inclusion(atom `u32`)) | derive — the blankets compose all the way down |
| `ExportConfig` (`export_config.rs`) | flat, bottom `ExportConfig::default()` | manual |

### 3.4 Enrollment of the ids, and the unenrolled types

In `state-colloscopes/src/ids.rs`, one invocation — ids are scalar reference
tokens with no internal content, so they are atoms wherever they appear as
field *values* (`Slot::teacher_id`, an association's `GroupListId`, …):

```rust
collomatique_state::impl_content_ord_atom!(
    PeriodId, WeekId, SubjectId, TeacherId, StudentId, WeekPatternId,
    SlotId, IncompatId, GroupListId, PairingRuleId, SlotPairingRuleId,
);
```

`NonEmptyRangeInclusive<T>` is generic, which the macro cannot express, so it
gets the one hand-written atom impl (§7.4).

Types with **no** impl at all (only ever compared inside an atom or flat
boundary): `WeekBlock`, `SoftParam<T>`, `Color`, `PageOrientation`,
`GlobalConfig`, `ColloscopeConfig`, `PerStudentGroupsConfig`,
`PerGroupListConfig`, `WeekDesc`, and every `collomatique-time` /
`non_empty_string` type.

---

## 4. Why this order is correct

### 4.1 Consistency with `Eq`

Every block reports `Some(Equal)` exactly when the two values are `==`: atoms
by definition; the lift because both sides must be `None` or both inner values
equal; inclusion/embedding because mutual inclusion of finite structures
forces equality; prefix pointwise because mutual prefix forces equal lengths
and pointwise equality; the flat order by its first arm; and a product of
consistent fields is consistent **provided every field participates** — which
the derive guarantees by construction (it walks the field list of the
definition itself). The law is stated on the trait; the manual impls (`Data`,
the three flat types, the range atom, the toys) are small enough to check by
eye.

### 4.2 Well-foundedness (the termination proof survives)

Every strict decrease under any block removes an element from a finite
container (including a trailing prefix element), moves an `Option` from
`Some` to `None`, or moves a flat value onto its bottom. Each block is
therefore well-founded on the finite values the document holds, and a product
of well-founded orders is well-founded (an infinite strictly-decreasing chain
would have to decrease some field infinitely often). Strict monotonicity of
fixes plus a well-founded order is the termination proof, unchanged from
step 6 — this step only *materializes* the order it was already stated in.

### 4.3 Every arm of the resolution map lands strictly below

The order of §3 is defined without reference to the map (decision 8); this
section is the **audit** that the map, as it exists, satisfies it. Statements
of the form "no fix touches this field" belong here — they are theorems about
today's map, never inputs to the definition, and step 7 may add arms without
touching §3.

Audited against the live `resolution.rs` (all 44 `Some(...)` sites). Every
emitted op falls into one of five shapes, each a strict decrease under §3:

1. **Row removal** — `WeekOp::Remove`, `SlotOp::Remove`, `IncompatOp::Remove`,
   `PairingOp::Remove`, `SlotPairingOp::Remove`: one key leaves a
   map-inclusion table (for weeks and slots the entity also leaves the
   ordering sidecar — both fields decrease or stay equal, so the product is
   `Less`).
2. **Row-clearing targeted writes** —
   `AssignmentOp::SetRow(period, subject, BTreeSet::new())`,
   `ColloscopeOp::SetInterrogation(slot, week, BTreeSet::new())`,
   `ColloscopeOp::SetGroupList(group_list, BTreeMap::new())`,
   `GroupListOp::AssignToSubject(period, subject, None)`,
   `SettingsOp::SetStudent(student, None)`,
   `BalancingOp::SetSubject(subject, None)`: canonical-absent storage means the
   row disappears — a map-inclusion key removal.
3. **Value-shrinking targeted writes** — the same ops carrying the current
   value minus one element: the row's key survives and its value strictly
   decreases under set-inclusion (or map-inclusion for the placement map).
4. **Whole-value rewrites minus one element** — `SubjectOp::Update`,
   `StudentOp::Update`, `TeacherOp::Update`, `WeekPatternOp::Update`,
   `PairingOp::Update`, `SlotPairingOp::Update`, `GroupListOp::Update`: the
   rebuilt value differs from the live one only by one removed set element, so
   the row's value is strictly below under the entity's product rule, all
   sibling fields comparing `Equal`. (The group-list rebuild that removes one
   student from one prefilled group is a pointwise decrease under the prefix
   rule — this is the arm the `vec_prefix` override exists for.)
5. **Optional-edge clears** — `SlotOp::Update` with `week_pattern: None`,
   `IncompatOp::Update` with `week_pattern_id: None`: strictly below by the
   Option lift, every other field equal.

No arm creates anything, renames anything, reorders anything, or moves a
configuration value — so the comparability the atoms and the flat order
withhold is never needed by today's map, and the in-loop assertion holds every
*future* arm to the same discipline. Shapes 4 and 5 include the D5.1 arms that
widen semantics while shrinking content; under §3 they are plain decreases, as
required.

### 4.4 `Default::default()` is below every reachable state

`InnerData::default()` has: every table empty (below anything by
map-inclusion/embedding), `first_week == None` (bottom of the lift),
`Settings::default()` (empty override table + `Limits::default()`, the flat
bottom), `Balancing::default()` (empty table + `BalancingOptions::default()`,
the flat bottom by definition), an empty colloscope, and
`ExportConfig::default()` (the flat bottom). Every component is at its
component-wise minimum — including the prefix rule, whose minimum is the
empty vector — so the product is the universal minimum among all documents,
reachable or not. Fuzz property (a) checks the reachable half end to end.

### 4.5 What the order rejects, on purpose

Same entity content under a different id; same id with different scalar
content; any reordering; a retargeted association or renumbered placement; a
*middle* group removed from a prefilled list (the surviving groups' externally
referenced indices silently re-aim — an identity shift, not a removal); one
table shrinking while another grows; any change between two non-default
configuration values. Each would be a contract violation if a fix produced
it, and the in-loop assertion turns each into a panic.

---

## 5. Commit 1 — the `ContentOrd` layer in `state/`

Everything lands in a new module `state/src/partial_order.rs`; `tables.rs` is
**not** touched (the container impls read `Table`/`OrderedTable` through their
public API).

### 5.1 The module

Contents, in order (doc comments abbreviated here where §1 already gives them
in full; write them out):

```rust
//! The document order: building blocks and the [ContentOrd] trait (design
//! doc §8, step 6.5).
//!
//! The standard library's `PartialOrd`/`Ord` on containers is lexicographic
//! and is NOT what a removal-shaped order needs (removing an element can
//! make a set sort *later*). The document order therefore lives on its own
//! trait, with hand-picked container semantics: sets by inclusion, maps by
//! key-and-value inclusion, sequences by embedding or prefix.
//! `#[derive(ContentOrd)]` (from `collomatique-state-derive`) implements it
//! for regular structs and enums as the product of their fields.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU32, NonZeroU64};

use crate::tables::{Key, OrderedKey, OrderedTable, Table};

pub trait ContentOrd: PartialEq {
    // exactly as §1, with content_le / content_lt default methods
}
```

Then the free-function combinators — the shared vocabulary the blanket impls,
the derive expansion, the manual impls and the attributes all use:

```rust
/// Product order: combines per-field comparisons. `Equal` is neutral; two
/// fields pulling in opposite directions, or any incomparable field, make
/// the whole product incomparable (`None`).
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
    combine(std::iter::once(Some(keys)).chain(
        a.iter()
            .filter_map(|(k, va)| b.get(k).map(|vb| value_cmp(va, vb))),
    ))
}

/// Sequence embedding: a strict subsequence (same relative order, elements
/// removed) is strictly below; a reordering is incomparable. Elements are
/// matched by `==`. The rule for sequences whose elements carry their own
/// identity (id lists, time windows).
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

/// Prefix-pointwise order: the rule for sequences whose *positions* carry
/// the identity — the `Vec` read as a map from an initial segment of
/// indices, i.e. [map_inclusion] specialized. The length comparison plays
/// the key-set role (appending is adding content, the empty vector is the
/// minimum) and shared indices compare pointwise; removing a middle element
/// shifts every later element's identity and comes out incomparable.
pub fn prefix_pointwise<T>(
    a: &[T],
    b: &[T],
    cmp: impl Fn(&T, &T) -> Option<Ordering>,
) -> Option<Ordering> {
    combine(
        std::iter::once(Some(a.len().cmp(&b.len())))
            .chain(a.iter().zip(b).map(|(x, y)| cmp(x, y))),
    )
}
```

(`embeds` with equal lengths is only true when the slices are equal, so the
`(true, true)` arm really is `Equal`; the greedy scan is the standard correct
subsequence test. In `prefix_pointwise`, `zip` stops at the shorter side —
exactly the shared index range — and `combine` merges the length verdict with
the pointwise ones, so `[g1] < [g1, g2]`, `[g1', g2] < [g1, g2]` when
`g1' < g1`, and `[g1, g3]` vs `[g1, g2, g3]` is `None` because position 1
holds `g3` against `g2`.)

### 5.2 The `with =` helpers

The functions field attributes reach for. The first two exist for *foreign*
element types the orphan rule keeps out of the trait (they need only
`PartialEq`); the third is the positional-identity rule for enrolled types:

```rust
/// `Option` lift with a discrete inner comparison — for
/// `#[ord(with = option_lift_discrete)]` on an `Option` whose inner type is
/// foreign (`Option<NonEmptyString>`, `Option<WeekStart>`, …).
pub fn option_lift_discrete<T: PartialEq>(a: &Option<T>, b: &Option<T>) -> Option<Ordering> {
    option_lift(a, b, |x, y| discrete(x, y))
}

/// Sequence embedding with elements matched discretely (by `==`) — the `Vec`
/// analogue of [option_lift_discrete], for `#[ord(with = vec_subsequence)]`
/// on a `Vec` whose element type is foreign. (For an *enrolled* element type
/// the blanket `Vec` impl already gives exactly this behavior.)
pub fn vec_subsequence<T: PartialEq>(a: &Vec<T>, b: &Vec<T>) -> Option<Ordering> {
    subsequence(a, b)
}

/// Prefix-pointwise comparison through [ContentOrd] — for
/// `#[ord(with = vec_prefix)]` where element identity is positional
/// (prefilled groups) and the subsequence default would be wrong.
pub fn vec_prefix<T: ContentOrd>(a: &Vec<T>, b: &Vec<T>) -> Option<Ordering> {
    prefix_pointwise(a, b, ContentOrd::content_cmp)
}
```

### 5.3 The leaf and container impls

Scalar atoms, via a private macro:

```rust
macro_rules! impl_atoms {
    ($($t:ty),* $(,)?) => { $(
        impl ContentOrd for $t {
            fn content_cmp(&self, other: &Self) -> Option<Ordering> {
                discrete(self, other)
            }
        }
    )* };
}

impl_atoms!(
    (), bool, char,
    u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize,
    String, NonZeroU32, NonZeroU64,
);
```

The exported macro for downstream *local* types (the orphan rule allows a
foreign trait on a local type, so `state-colloscopes` enrolls its ids with
this):

```rust
/// Enrolls local types into the document order as atoms (discretely
/// compared: equal or incomparable). For foreign types use `#[ord(atom)]`
/// on the field instead; for generic types write the impl by hand.
#[macro_export]
macro_rules! impl_content_ord_atom {
    ($($t:ty),* $(,)?) => { $(
        impl $crate::partial_order::ContentOrd for $t {
            fn content_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                $crate::partial_order::discrete(self, other)
            }
        }
    )* };
}
```

The container blankets — the heart of the dispatch design:

```rust
impl<T: ContentOrd> ContentOrd for Option<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        option_lift(self, other, ContentOrd::content_cmp)
    }
}

impl<T: Ord> ContentOrd for BTreeSet<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        set_inclusion(self, other)
    }
}

impl<K: Ord, V: ContentOrd> ContentOrd for BTreeMap<K, V> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        map_inclusion(self, other, ContentOrd::content_cmp)
    }
}

/// Sequence embedding — the value-borne-identity reading, which is the
/// common case reached through trait dispatch (id lists inside table
/// values). Deliberately a blanket: a wrong fit can only be too strict
/// (never blesses a non-removal change), and it is gated on
/// `T: ContentOrd` so it never fires on a type not deliberately enrolled.
/// Where identity is positional, override the field with
/// `#[ord(with = vec_prefix)]`; where the list is a relational chain, with
/// `#[ord(atom)]` (§2, the identity criterion).
impl<T: ContentOrd> ContentOrd for Vec<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        subsequence(self, other)
    }
}

impl<I: Key, T: ContentOrd> ContentOrd for Table<I, T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_in_other = self.keys().all(|k| other.contains(&k));
        let other_in_self = other.keys().all(|k| self.contains(&k));
        let keys = match (self_in_other, other_in_self) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => return None,
        };
        combine(std::iter::once(Some(keys)).chain(
            self.iter()
                .filter_map(|(k, v)| other.get(&k).map(|w| v.content_cmp(w))),
        ))
    }
}

impl<I: OrderedKey, T: ContentOrd> ContentOrd for OrderedTable<I, T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_keys: Vec<I> = self.keys().collect();
        let other_keys: Vec<I> = other.keys().collect();
        let keys = subsequence(&self_keys, &other_keys)?;
        combine(std::iter::once(Some(keys)).chain(
            self.iter()
                .filter_map(|(k, v)| other.get(&k).map(|w| v.content_cmp(w))),
        ))
    }
}
```

(Both table impls use only the public read API — `keys`, `contains`, `iter`,
`get` — so `tables.rs` stays untouched. `OrderedTable` keys are unique, so the
embedding is unambiguous, and a shared key set in a different order yields
`None` from the key comparison before any value is consulted.)

Note the `Vec` blanket satisfies the `ContentOrd: PartialEq` supertrait
because `T: ContentOrd` implies `T: PartialEq`; the `BTreeSet` blanket gets it
from `T: Ord`.

### 5.4 Wiring in `state/src/lib.rs`

Old code:

```rust
pub mod join;
pub mod refs;
```

and

```rust
pub use join::{Join, Joinable, Lookup};
```

New code:

```rust
pub mod join;
pub mod partial_order;
pub mod refs;
```

and

```rust
pub use join::{Join, Joinable, Lookup};
pub use partial_order::ContentOrd;
```

(The combinators and helpers stay behind the module path —
`collomatique_state::partial_order::…` — only the trait gets a root
re-export, mirroring `Fixable`.)

### 5.5 Commit-1 unit tests

In `partial_order.rs` `#[cfg(test)]`, on plain `u64` / `String` fixtures:

* `combine`: all-equal → `Equal`; a `Less` among equals → `Less`; `Less` and
  `Greater` mixed → `None`; any `None` → `None`; empty iterator → `Equal`.
* `discrete`, `flat_with_bottom` (bottom below, two non-bottom values
  incomparable), `option_lift` (all four arms).
* `set_inclusion`: equal / strict subset / strict superset / crossed →
  `None`; **the lexicographic trap pinned**: `set_inclusion` of `{1,3}` vs
  `{1,2,3}` is `Some(Less)` while std's `Ord` sorts `{1,3}` *after*
  `{1,2,3}` — assert both facts side by side, as documentation.
* `map_inclusion`: key-subset with equal values → `Less`; same keys, one
  value `Less` → `Less`; key-subset but a shared value `Greater` → `None`.
* `subsequence`: `[1,3]` vs `[1,2,3]` → `Less`; reorder `[2,1]` vs `[1,2]` →
  `None`; equal; empty below anything.
* `prefix_pointwise`: truncation `[1]` vs `[1,2]` → `Less`; equal length
  with one pointwise decrease → `Less`; **the middle-removal pin**: `[1,3]`
  vs `[1,2,3]` with a discrete element rule → `None` (contrast with
  `subsequence`, which says `Less` on the same input — the two rules differ
  exactly on identity); mixed directions → `None`; empty below anything.
* Blanket dispatch smoke tests: `Option<u32>` (`None < Some(3)`,
  `Some(3)` vs `Some(4)` → `None`), `Vec<u32>` subsequence,
  `BTreeMap<u64, u32>` (value renumber → `None`), and `Table` /
  `OrderedTable` with a local test id type (row removal → `Less`, reorder →
  `None`, value change dispatching into the value's impl).

---

## 6. Commit 2 — `#[derive(ContentOrd)]` in `collomatique-state-derive`

### 6.1 Registration

New file `state-derive/src/content_ord.rs`; in `state-derive/src/lib.rs`, next
to the existing entries:

```rust
#[proc_macro_derive(ContentOrd, attributes(ord))]
pub fn derive_content_ord(input: TokenStream) -> TokenStream {
    content_ord::derive(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
```

And in `state/src/lib.rs`, the derive joins the existing re-export (derive
macros and traits live in different namespaces, so the name `ContentOrd` can
be both — the `Join` trait/derive pair is the in-house precedent). Old code:

```rust
pub use collomatique_state_derive::{EntityId, Join, References};
```

New code:

```rust
pub use collomatique_state_derive::{ContentOrd, EntityId, Join, References};
```

### 6.2 What the derive accepts and what it generates

Accepted inputs: **non-generic structs with named fields** (including empty
ones) and **non-generic enums whose variants have named fields or are unit
variants**. Everything else — generics (the `Join` precedent), tuple structs,
tuple variants, unions — is rejected with a spanned `syn::Error` naming the
restriction. All our targets are non-generic named shapes.

Field rule resolution: at most one `#[ord(…)]` attribute per field (a second
is a spanned error), parsed as:

```rust
enum FieldRule {
    /// No attribute: dispatch through the trait.
    Default,
    /// `#[ord(atom)]`: inline discrete comparison.
    Atom,
    /// `#[ord(partial_ord)]`: use the field type's existing `PartialOrd`.
    PartialOrd,
    /// `#[ord(with = <expr>)]`: call the expression (path or closure).
    With(syn::Expr),
}
```

parsed by a small `syn::parse::Parse` impl: an identifier, which must be
`atom`, `partial_ord`, or `with` followed by `=` and an `Expr`. Anything else
is a spanned error listing the three forms. Note `with` takes a **plain
expression, not a string** — `#[ord(with = option_lift_discrete)]` and
`#[ord(with = |a, b| …)]` both parse.

Per-field generated comparison, for a struct field `x` (fully-qualified paths
throughout, per the house lesson on derive hygiene; `ContentOrd::content_cmp`
is called in function position so the receiver type is inferred from the
arguments and never needs to be re-spelled):

| rule | generated expression |
| --- | --- |
| `Default` | `::collomatique_state::partial_order::ContentOrd::content_cmp(&self.x, &other.x)` |
| `Atom` | `::collomatique_state::partial_order::discrete(&self.x, &other.x)` |
| `PartialOrd` | `::core::cmp::PartialOrd::partial_cmp(&self.x, &other.x)` |
| `With(expr)` | `(#expr)(&self.x, &other.x)` |

For a **struct**, the impl is the product:

```rust
impl ::collomatique_state::partial_order::ContentOrd for Foo {
    fn content_cmp(
        &self,
        other: &Self,
    ) -> ::core::option::Option<::core::cmp::Ordering> {
        ::collomatique_state::partial_order::combine([
            /* one expression per field, in declaration order */
        ])
    }
}
```

A struct with no fields short-circuits to
`::core::option::Option::Some(::core::cmp::Ordering::Equal)` (an empty array
literal would not infer its item type).

For an **enum**: a match over `(self, other)`; each variant produces one arm
destructuring both sides with distinct bindings and combining its fields'
comparisons (same table, applied to the bindings, which are references thanks
to match ergonomics); a unit variant's arm yields `Some(Equal)`; and — only
when the enum has at least two variants — a trailing `_ => None` arm makes
mixed variants incomparable:

```rust
match (self, other) {
    (
        Self::Prefilled { groups: self_groups },
        Self::Prefilled { groups: other_groups },
    ) => ::collomatique_state::partial_order::combine([
        (vec_prefix)(self_groups, other_groups),
    ]),
    (
        Self::Automatic { excluded_students: self_excluded_students },
        Self::Automatic { excluded_students: other_excluded_students },
    ) => ::collomatique_state::partial_order::combine([
        ::collomatique_state::partial_order::ContentOrd::content_cmp(
            self_excluded_students,
            other_excluded_students,
        ),
    ]),
    _ => ::core::option::Option::None,
}
```

(This example is literally what `GroupListFilling` will expand to.)

### 6.3 Commit-2 integration tests

New file `state/tests/derive_content_ord.rs` (integration test on purpose —
the generated code's absolute `::collomatique_state::` paths only resolve from
outside the crate — mirroring `derive_refs.rs` / `derive_join.rs`). It defines
local toy types and pins every macro behavior:

* a struct whose fields exercise **default dispatch** through `Option`,
  `BTreeSet`, `BTreeMap`, `Vec`, `Table<u64, _>` and `OrderedTable<u64, _>`
  (plain `u64` keys satisfy `Key`/`OrderedKey`, no id type needed): row
  removal → `Less`, reorder → `None`, set shrink → `Less`, option clear →
  `Less`, value renumber → `None`;
* a field of a local type with **no** `ContentOrd` impl under `#[ord(atom)]`:
  equal → `Equal`, changed → `None`;
* a field under `#[ord(with = option_lift_discrete)]` (path form) and another
  under `#[ord(with = |a, b| prefix_pointwise(a, b, option_lift_discrete))]`
  (closure form, the exact shape §7.3 uses for `group_names`);
* a field under `#[ord(partial_ord)]` on a type whose derived `PartialOrd` is
  the intended order, asserting the numeric behavior really is used;
* an enum with two named-field variants and a unit variant: same variant →
  product, unit/unit → `Equal`, mixed → `None`;
* the product mixing rules on a two-field struct: one field down + one field
  up → `None`; one down + one equal → `Less`;
* an empty struct → `Equal`;
* `content_le` / `content_lt` default methods behave.

Compile-failure cases (generics, tuple struct, duplicate attribute, unknown
attribute argument) are asserted the same way the existing derives do it — if
`derive_refs.rs`/`derive_join.rs` have no trybuild harness, a comment records
the spanned-error behavior instead of adding a new dev-dependency (no
`Cargo.lock` churn in this step).

---

## 7. Commit 3 — adoption in `state-colloscopes/`

### 7.1 Enrollment of the ids

The single `impl_content_ord_atom!` invocation of §3.4 in `ids.rs`.

### 7.2 The derives

Add `ContentOrd` to the derive list of every type marked `derive` in §3's
tables. Example — old code (`state-colloscopes/src/lib.rs:147-152`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InnerData {
    pub params: colloscope_params::Parameters,
    pub colloscope: colloscopes::Colloscope,
    pub export_config: export_config::ExportConfig,
}
```

New code:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default, ContentOrd)]
pub struct InnerData {
    pub params: colloscope_params::Parameters,
    pub colloscope: colloscopes::Colloscope,
    pub export_config: export_config::ExportConfig,
}
```

with `use collomatique_state::ContentOrd;` added to the module's imports (the
name serves as both derive and trait, exactly like `Join`). The full derive
list (36 types): `InnerData`, `Parameters`, `Periods`, `Weeks`, `Week`,
`Subjects`, `Subject`, `SubjectParameters`,
`SubjectInterrogationParameters`, `SubjectPeriodicity`, `Teachers`,
`Teacher`, `PersonWithContact`, `Students`, `Student`, `Assignments`,
`WeekPatterns`, `WeekPattern`, `Slots`, `Slot`, `Incompats`,
`Incompatibility`, `GroupLists`, `GroupList`, `GroupListParameters`,
`GroupListFilling`, `PrefilledGroup`, `Settings`, `Pairings`, `PairingRule`,
`RulePart`, `SlotPairings`, `SlotPairingRule`, `SlotRulePart`, `Balancing`,
`Colloscope`.

### 7.3 The ten field attributes

| file | field | attribute | why |
| --- | --- | --- | --- |
| `lib.rs` (`PersonWithContact`) | `tel` | `#[ord(with = option_lift_discrete)]` | foreign `NonEmptyString` |
| `lib.rs` (`PersonWithContact`) | `email` | `#[ord(with = option_lift_discrete)]` | foreign `NonEmptyString` |
| `weeks.rs` (`Week`) | `annotation` | `#[ord(with = option_lift_discrete)]` | foreign `NonEmptyString` |
| `periods.rs` (`Periods`) | `first_week` | `#[ord(with = option_lift_discrete)]` | foreign `WeekStart` |
| `slots.rs` (`Slot`) | `start_time` | `#[ord(atom)]` | foreign `SlotStart`; a scalar leaf |
| `subjects.rs` (`SubjectInterrogationParameters`) | `duration` | `#[ord(atom)]` | foreign `NonZeroMinutes`; a scalar leaf |
| `subjects.rs` (`SubjectPeriodicity::AmountForEveryArbitraryBlock`) | `blocks` | `#[ord(atom)]` | ★ decision 10: the `WeekBlock` chain is relational — one composite value |
| `incompats.rs` (`Incompatibility`) | `slots` | `#[ord(with = vec_subsequence)]` | value-borne identity, foreign `SlotWithDuration` element |
| `group_lists.rs` (`GroupListParameters`) | `group_names` | `#[ord(with = \|a, b\| prefix_pointwise(a, b, option_lift_discrete))]` | position-borne identity, foreign element — closure form |
| `group_lists.rs` (`GroupListFilling::Prefilled`) | `groups` | `#[ord(with = vec_prefix)]` | position-borne identity (decision 11) |

with the helpers imported where used:

```rust
use collomatique_state::partial_order::{
    option_lift_discrete, prefix_pointwise, vec_prefix, vec_subsequence,
};
```

The `groups` override is the one site where the blanket's subsequence rule
would be *wrong* rather than merely absent, and it deserves its own comment in
the code:

```rust
    /// Groups are filled manually with prefilled students
    Prefilled {
        // Position-borne identity: group i binds to group name i, and group
        // numbers are referenced by index from the colloscope's placement
        // maps and interrogation cells — so the document order reads this
        // Vec as a map from indices (prefix + pointwise). The Vec blanket's
        // subsequence rule would both reject the minus-one-student fix
        // (a modified group matches nothing under ==) and bless middle
        // removals that silently re-aim every later group's references.
        #[ord(with = vec_prefix)]
        groups: Vec<PrefilledGroup>,
    },
```

### 7.4 The five manual impls

`Data` (`lib.rs`, right below its `PartialEq`):

```rust
/// The document order (design doc §8, step 6.5): delegates to [InnerData],
/// ignoring the id issuer exactly as [PartialEq] does. Two [Data] with equal
/// inner data compare `Equal` even when their issuers differ.
impl ContentOrd for Data {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner_data.content_cmp(&other.inner_data)
    }
}
```

`NonEmptyRangeInclusive<T>` (`non_empty_range.rs` — generic, so outside the
macro's reach):

```rust
/// The document order: a range is an atom — its content is the endpoint
/// pair. Reading `[2..=3] ⊆ [1..=4]` as an order would compare the denoted
/// sets, which is exactly the semantic reading D5.1 forbids.
impl<T: Ord + Clone> ContentOrd for NonEmptyRangeInclusive<T> {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        collomatique_state::partial_order::discrete(self, other)
    }
}
```

`Limits` (`settings.rs`) — identical pattern for `BalancingOptions`
(`balancing.rs`) and `ExportConfig` (`export_config.rs`):

```rust
/// The document order: configuration values compare flat — the type's
/// [Default] value is the bottom, everything else is comparable only to
/// itself. Forced by the Default-minimum axiom: this type's default is not
/// structurally minimal, and nothing may sit below `Default::default()`
/// (design doc §8, step 6.5 — the antisymmetry argument).
impl ContentOrd for Limits {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        collomatique_state::partial_order::flat_with_bottom(self, other, &Limits::default())
    }
}
```

### 7.5 Commit-3 unit tests: the semantics, not the machinery

New file `state-colloscopes/src/partial_order_tests.rs`, declared from
`lib.rs` as `#[cfg(test)] mod partial_order_tests;` (the in-crate pattern of
`resolution/innocent_tests.rs` — ids are forged with
`unsafe { XxxId::new(n) }`, and private-field values are built through the
public ops or crate-internal constructors). These tests pin the *order*
against §3, independently of how the impls are produced. Every test builds a
value and a twin, calls `content_cmp`, and asserts the exact
`Option<Ordering>`.

Must-cover list (one test each; names indicative):

* `data_ignores_the_id_issuer` — two `Data` built by different op sequences
  ending on the same `InnerData` compare `Some(Equal)`.
* `default_is_below_a_populated_document` — build a document through the gate
  (the `data_with_assignment` recipe from `lib.rs`'s test modules is a good
  base), assert `InnerData::default().content_lt(data.get_inner_data())`.
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
  middle one removed: `Some(Less)` (value-borne identity: subsequence).
* `excluded_period_drop_is_strictly_below` — a `Subject` twin with one
  `excluded_periods` element removed: `Some(Less)` (the D5.1
  content-not-semantics pin: the semantics widen, the content shrinks).
* `week_pattern_exclusion_drop_is_strictly_below` — same shape on
  `WeekPattern::excluded_weeks`.
* `optional_edge_clear_is_strictly_below` — a `Slot` twin with
  `week_pattern: None`: `Some(Less)`; a `start_time` tweak on another twin:
  `None`.
* `contact_clear_is_strictly_below` — `PersonWithContact` with `tel`
  cleared: `Some(Less)` (pins the uniform Option rule beyond foreign keys).
* `assignment_row_shrink_and_clear` — value minus one student: `Some(Less)`;
  row removed: `Some(Less)`; student swapped for another: `None`.
* `association_retarget_is_incomparable` — `subjects_associations` value
  changed to another live `GroupListId`: `None`; entry removed: `Some(Less)`.
* `incompat_slot_window_removal_is_strictly_below` — an `Incompatibility`
  twin with one time window removed from `slots`: `Some(Less)` (the
  decision-8 pin: the order pre-exists the map, so this holds although no
  fix touches the field today); a modified window: `None`.
* `periodicity_blocks_are_one_atom` — two subjects differing only inside the
  `WeekBlock` list: `None`; and — the pin that distinguishes atom from
  prefix/subsequence — a twin whose block list is a strict *truncation* of
  the other's is **also** `None` (★ decision 10).
* `group_list_prefilled_minus_student_is_strictly_below` — rebuilt via
  `GroupList::new` with one student removed from one group: `Some(Less)`.
  **This is the regression test for the `vec_prefix` override** — under the
  blanket subsequence rule the modified group would match nothing and the
  comparison would report `None`.
* `group_list_trailing_group_removal_is_strictly_below` — a twin with the
  *last* group and its name removed (both `group_names` and `groups`
  truncated by one): `Some(Less)` — adding a group is adding content, zero
  groups is the minimum (decision 11).
* `group_list_middle_group_removal_is_incomparable` — a twin with a *middle*
  group and its name removed: `None` (the identity shift: later groups'
  externally referenced indices re-aim).
* `group_name_unset_is_strictly_below` — one `group_names` entry moved from
  `Some(name)` to `None`, groups untouched: `Some(Less)`; a renamed entry:
  `None`.
* `group_list_variant_change_is_incomparable` — `Prefilled` vs `Automatic`:
  `None`.
* `colloscope_cell_trim_is_strictly_below` — interrogation cell minus one
  group: `Some(Less)`; placement map minus one student: `Some(Less)`; a
  placement renumbered: `None`.
* `flat_config_bottom` — for each of `Limits`, `BalancingOptions`,
  `ExportConfig`: default below a modified value (`Some(Less)`), two distinct
  non-default values `None`.
* `settings_override_row_removal_is_strictly_below`.
* `pairing_rule_excluded_period_drop_is_strictly_below` and
  `pairing_rule_part_change_is_incomparable`.
* `mixed_directions_are_incomparable` — one document gains a student and
  loses a teacher relative to the other: `None`.

---

## 8. Commit 4 — the document order on the toy types

`state/src/test_utils.rs`, hand-written on purpose (decision 5 of §0.2): these
impls test the combinators and the engine with zero dependence on the derive,
and double as documentation of what the derive expands to. Old code (context —
the current derives):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteData {
    pub students: BTreeSet<u64>,
    /// quote id -> author student id
    pub quotes: BTreeMap<u64, u64>,
}
```

New code, added below the `InMemoryData` impl (derives unchanged):

```rust
/// The document order on the toy: students by set inclusion, quotes by map
/// inclusion with atomic authors (re-authoring a quote is incomparable,
/// removing one is strictly below).
impl ContentOrd for QuoteData {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
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
the mode), so its order must account for both to stay `Eq`-consistent; the
mode never changes during a run, so this costs nothing:

```rust
impl ContentOrd for EvilQuoteData {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let EvilQuoteData(data, mode) = self;
        crate::partial_order::combine([
            data.content_cmp(&other.0),
            crate::partial_order::discrete(mode, &other.1),
        ])
    }
}
```

(with `use crate::partial_order::ContentOrd;` added to the module imports.)

Commit-4 unit tests, beside the toys:

* removing a quote → `Some(Less)`;
* adding a student → `Some(Greater)`;
* re-authoring an existing quote to another author → `None`;
* removing a student while adding a quote → `None`;
* equal → `Some(Equal)`.

---

## 9. Commit 5 — the `Fixable` bound and the in-flight assertion

### 9.1 The trait bound (`state/src/cascade.rs`)

Old code:

```rust
/// Implemented by data whose broken invariants can be repaired by ops: the
/// resolution map. (`PartialEq` backs the engine's no-op-fix panic.)
pub trait Fixable: InMemoryData + PartialEq {
```

New code:

```rust
/// Implemented by data whose broken invariants can be repaired by ops: the
/// resolution map. (`ContentOrd` materializes the document order of the
/// monotonicity contract; the engine checks every fix against it in-flight,
/// and its `PartialEq` supertrait backs the no-op-fix panic.)
pub trait Fixable: InMemoryData + ContentOrd {
```

with `use crate::partial_order::ContentOrd;` and `use std::cmp::Ordering;`
added to the imports. (`ContentOrd: PartialEq`, so implementors keep exactly
the obligations they had, plus the order; after commits 3 and 4, `Data`,
`QuoteData` and `EvilQuoteData` all qualify.)

In the `fix_invariant` doc comment, update the forward reference. Old text:

```rust
    /// well-founded, so this contract is the cascade's termination proof —
    /// a map that *grows* the state makes the cascade loop forever (step 6.5
    /// adds a `PartialOrd`-based in-flight check for exactly that).
```

New text:

```rust
    /// well-founded, so this contract is the cascade's termination proof.
    /// The order is materialized by the [ContentOrd] supertrait bound (the
    /// document order, design doc §8 step 6.5), and [apply_cascade] asserts
    /// after every fix that the state landed strictly below the pre-fix
    /// state — a growing or sideways map panics instead of hanging.
```

Also update the module header's contract sentence the same way (it currently
stops at the no-op panic; add that every fix is additionally checked to land
strictly below the pre-fix state in the document order).

### 9.2 The engine assertion (`state/src/cascade.rs`)

Old code (the success arm of the apply match):

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
no-op, G.2):

```rust
            Ok(backward) => {
                if let Some(before) = before {
                    match (*data).content_cmp(&before) {
                        Some(Ordering::Less) => {}
                        Some(Ordering::Equal) => panic!(
                            "resolution map violated the strict-monotonicity \
                             contract: fix {front:?} applied as a perfect no-op \
                             (return None when no material is present)"
                        ),
                        not_below => panic!(
                            "resolution map violated the strict-monotonicity \
                             contract: fix {front:?} did not land strictly below \
                             the pre-fix state (content_cmp = {not_below:?})"
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

### 9.3 Two new evil modes (`state/src/test_utils.rs`)

Old code (context — the current mode enum and its doc):

```rust
/// The way an [EvilQuoteData] map misbehaves.
///
/// There is no state-*growing* mode: without a round fuse that scenario is a
/// hang, not a test — it belongs to step 6.5's `PartialOrd` in-flight check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvilMode {
```

New code — the doc note is replaced (the growing scenario is now a panic, not
a hang) and two modes are appended after `CreateThenDisown`:

```rust
/// The way an [EvilQuoteData] map misbehaves.
///
/// The two step-6.5 modes ([EvilMode::CreateAuthor] and
/// [EvilMode::ReauthorExisting]) violate the contract while *resolving* the
/// invariant: before the in-flight document-order check they would have led
/// the cascade to a quiet, creative `Ok`. Now they panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvilMode {
    // … the four existing modes, unchanged …

    /// "Fixes" a dangling author by *creating* the missing student — the
    /// state grows, which is exactly what a strictly-decreasing map may
    /// never do, however helpful it looks.
    CreateAuthor { author: u64 },
    /// "Fixes" a dangling author by re-authoring some existing quote to an
    /// existing student — the state moves *sideways* (incomparable),
    /// neither shrinking nor growing.
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

(The mode carries the author id because the map cannot read it from state:
at fix time the dangling quote of the invariant was rolled back and does not
exist in `self`.)

### 9.4 Two new engine tests (`state/src/cascade.rs`, tests module)

Numbered 10 and 11, following the file's comment style. Also tighten test 6's
expected string from `"strict-monotonicity"` to `"applied as a perfect no-op"`
— both panic messages now share the `strict-monotonicity` prefix, and each
panic test should pin its own message.

```rust
    // 10. A fix that grows the state: the map "fixes" the dangling author by
    //     creating the missing student. Before step 6.5 this landed a quiet
    //     creative Ok; the document-order check panics instead.
    #[test]
    #[should_panic(expected = "did not land strictly below")]
    fn a_growing_fix_panics() {
        let mut data =
            EvilQuoteData(quote_data(&[1], &[]), EvilMode::CreateAuthor { author: 2 });
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 99,
            author: 2,
        });

        let _ = apply_cascade(&mut data, target);
    }

    // 11. A fix that moves the state sideways: an existing quote is
    //     re-authored to another existing student — nothing removed, nothing
    //     added, the result incomparable with the pre-fix state.
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

(In test 10 the fix `AddStudent(2)` applies cleanly and `{students: {1, 2}}`
compares `Greater` to `{students: {1}}`. In test 11 the fix re-authors quote
10 to the live student 2, and `{10 → 2}` is incomparable with `{10 → 1}`.)

### 9.5 What validates this commit

The full workspace suite. In particular the 11 `Ok` cascade fixtures
(`state-colloscopes/tests/cascade.rs`) and the two 50-seed × 500-op cascade
fuzz walks (`tests/property_cascade.rs`) now run every legitimate fix through
the strictly-below assertion — the end-to-end demonstration that §3's order
accepts every conforming arm. A failure here means the *order* is wrong (too
strict somewhere), not the map; fix the order against §3 and §4.3.

---

## 10. Commit 6 — the two fuzz properties

New file `state-colloscopes/tests/property_partial_order.rs`, the fourth
property harness, built on the same testgen machinery as
`tests/property_cascade.rs` (same imports and `RunConfig` shape, same
`for_each_seed` / `bootstrap` / `generator::gen_op` / `OpLog` plumbing — copy
that file's `next_op` and `maybe_snapshot` helpers verbatim, and keep the
snapshot bookkeeping on the success path exactly as there; moving it perturbs
the RNG trajectory). Additional imports:
`use collomatique_state::ContentOrd;`.

Configuration, matching the house ruling for step-6-family harnesses (one
hardcoded const, no environment variables, no `#[ignore]` tiers):

```rust
const CONFIG: RunConfig = RunConfig {
    seeds: 50,
    ops_per_run: 500,
    invalid_fraction: 0.15,
};
```

### 10.1 Property (a): the empty document really is the universal minimum

A plain gated walk (`data.annotate` + `data.apply`, as in `property_ops.rs` —
no cascade). After every op that lands, assert the design-doc claim directly:

```rust
/// Design doc §8 step 6.5, fuzz (a): `Default::default()` is the universal
/// minimal element of the document order — below every state the gate can
/// reach.
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
                        bottom.content_le(data.get_inner_data()),
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

(The bootstrap document itself is covered too: the first landing's assertion
compares against a state that includes everything the bootstrap built.)

### 10.2 Property (b): every map answer is `None` or lands strictly below

The same gated walk, but the interesting event is a **rejection over broken
invariants**: at that point `data` is unchanged and valid (the gate rolled
back), which is precisely the state the cascade would consult the map on. For
*every* invariant in the reported set — not only the canonical first pick the
engine would take — ask the map, and when it answers with a fix, land that fix
through the force door on a clone and compare:

```rust
/// Design doc §8 step 6.5, fuzz (b): over generated broken states, every
/// `fix_invariant` answer is `None` or an op whose applied result sits
/// strictly below the pre-fix state — never above, never equivalent. This
/// is also the only systematic exercise of the map's `Some` branches (the
/// innocent-state tests of step 6's §9bis systematically cover `None`).
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
                                fixed
                                    .get_inner_data()
                                    .content_cmp(data.get_inner_data()),
                                Some(std::cmp::Ordering::Less),
                                "fix {fix:?} for {invariant:?} must land \
                                 strictly below the pre-fix state",
                            );
                            probed_fixes.set(probed_fixes.get() + 1);
                        }
                    }
                    Err(Error::InvalidOp(_)) => stats.record(category, false),
                }
            }
        },
    );

    // Coverage guards (step-6 commit-8 lesson: count the specific outcome
    // the test is about, not a proxy). Without them the walk could go green
    // with the map never once answering `Some`.
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

* `fix_invariant` comes from `collomatique_state::Fixable` (crate-root
  re-export); `force_apply` is the public force door on `Data`.
* Using `force_apply` (not `apply`) is deliberate and mirrors the engine: a
  fix is allowed to land a state that still breaks *other* invariants
  (mid-cascade states); the gate would bounce those and hide exactly the
  comparison this property is about. Prechecks still run, and a precheck
  failure is a map bug (the engine panics on it), hence the `expect`.
* Probing every member of the set — not just `set.first()` — is strictly
  wider than what the in-loop assertion of commit 5 sees, and is the point of
  having this property in addition to `property_cascade.rs`.
* Per-seed run time: the extra work is one `Data` clone per probed fix (a few
  thousand across the whole run, on documents of a few dozen entities) — well
  within the existing harness budget (`property_cascade.rs` runs in 7.7 s).

---

## 11. Gate and close-out

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
  **Appendix I** — the `ContentOrd` trait and why it is not `PartialOrd`; the
  intrinsic-order methodology (the order pre-exists the map; fix behavior is
  audit material, never definition material); the §2/§3 order definition with
  the identity criterion for sequences (value-borne → subsequence,
  position-borne → prefix, relational chain → atom) and its instances
  (★ `WeekBlock` atom, prefix-ordered groups/names with the external-index
  argument); the flat rule for the three configuration types derived from the
  `Default`-minimum axiom via antisymmetry; the derive and its three
  attributes; the engine assertion; the two new evil modes; and the two fuzz
  properties with their measured numbers;
* retire this plan: delete `docs/plans/plan_step_6_5.md` in the close-out
  commit and pin **both versions** in the topic memory — the first draft at
  `git show ec0dd2a2:docs/plans/plan_step_6_5.md`, the final version at
  `git show <close-out parent>:docs/plans/plan_step_6_5.md`;
* update the topic memory: step 6.5 closed, next = step 7 (the `ops/`
  remaster).
