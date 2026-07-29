# Step 6.5 session plan — monotonicity checking (`ContentOrd` + derive)

**Status: DRAFT — awaiting user sign-off. No implementation commit has landed.**

This is the fourth version of the plan. Version history: v1 (`ec0dd2a2`)
built the order out of hand-written `PartialOrd` implementations; v2
(`85f44889`) reworked it into a dedicated **`ContentOrd`** trait implemented
by a derive macro and re-founded the methodology (the order is intrinsic and
pre-exists the resolution map); v3 (`74a80456`) folded in the first proofread
round (equivalence tolerance, configuration records as atoms, the
`Default`-universal-minimum axiom retired, `#[ord(ignore)]`, end-to-end
derive tests). This v4 folds in the second proofread round (§0.2, decisions
16–20): the trait laws become fully self-contained (no law mentions
`PartialEq`), the container scope law is enforced in the type system by the
**`ContentIdentity`** marker trait, the marker gets an explicit checked
derive (never an automatic one), `#[ord(partial_ord)]` is renamed
`#[ord(total)]` and generates `Ord::cmp`, and every edge-case ruling must
land as a comment at its code site.

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
entirely on the engraved map contract, whose step-6 wording was:

> States form a partial order with a universal minimal element:
> `Default::default()`, the empty document. Every returned op must land
> **strictly below** the current state in that order.

(The "universal minimal element" half of that wording is retired by this
step — see §0.1; the strictly-below half is what this step enforces.)

Step 6 enforced the contract only partially in-flight. The `None` convictions
and the no-op-fix panic catch every *removal-shaped* violation, but the order
itself was never materialized, so a map bug that keeps **growing** the state is
undetectable and makes the cascade loop forever. Step 6.5 closes that hole:

1. define the partial order as a real trait implementation on the document
   (`InnerData`, and `Data` above it),
2. require that trait on `Fixable` implementors (only there — the generic
   `InMemoryData` trait is untouched),
3. assert in the cascade loop, after every fix that lands, that the new state
   is **strictly below** the pre-fix state — turning a growing or sideways map
   into a loud panic instead of a hang,
4. fuzz the contract: every `fix_invariant` answer is `None` or an op whose
   applied result lands strictly below. (An earlier sketch had a companion
   property, "`Default::default()` ≤ every reachable state"; it died with the
   universal-minimum axiom — decision 13.)

### 0.1 Binding constraints (★ user rulings, recorded in the design doc)

* **D5.1 — the order is over the document's *content*, not the meaning it
  denotes.** Several conforming map arms strictly shrink the data while
  *widening* the semantics: a subject that stops excluding a dead period now
  applies more broadly; a slot whose `week_pattern` is cleared now runs every
  week. In each case an id was removed and nothing was added, so the document
  strictly decreased. An order that compared meanings would reject these arms
  and break the termination proof. Every rule in §3 below is a rule about
  *content*. The trait's name, `ContentOrd`, records this ruling.
* **The trait is self-contained; equivalence classes are tolerated**
  (decisions 12 and 16). `ContentOrd`'s laws never mention `PartialEq`:
  `Some(Equal)` is an equivalence relation ("content equivalence"), possibly
  coarser than `==` — `Data` ignoring its id issuer is the canonical case.
  The engine never consults `==`: a fix must land `Some(Ordering::Less)`,
  which means "below and not equivalent" (the user's "S2 ≤ S1 and
  not(S1 ≤ S2)"); `S1 != S2` appears nowhere. The relation to `==` exists
  only as a *fact about the building blocks*: `discrete` — and hence every
  atom and blanket — **defines** content equivalence as the leaf's `==`, so a
  type assembled from them inherits whatever `==` means on its leaves. For
  `InnerData` and everything below it, content equivalence *is* structural
  equality — an implementation property of this crate (pinned by §7.5's twin
  tests), not a trait law.
* **Container matching is by `==`/`Ord`, and the type system enforces where
  that is sound** (decision 17). Inside a container, "the same element/row"
  can only mean `==` (for `Ord`-backed storage, Rust's own contract already
  ties `Ord`'s `Equal` to `==`). That is correct exactly when `==` coincides
  with content equivalence *for the element/key type* — a positional
  requirement, never a global one (a global law would outlaw the `Data`
  quotient). The `ContentIdentity` marker trait states it, and the container
  blanket impls **require** it at every matching position, so putting a
  quotiented type in a container is a compile error, not a prose violation.
* **Well-foundedness is the termination requirement — the universal-minimum
  axiom is retired** (decision 13, superseding the step-6 ruling "minimal
  element = `Default::default()`"). Termination needs finite descent, nothing
  more; the order may have many minimal elements. The empty document remains
  *a* minimal element (§4.4), just not the unique bottom.

### 0.2 Decisions from the design reviews (July 29 2026)

Settled with the user across the review rounds:

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
   type anywhere gains a new `PartialOrd`.**
3. **Blanket `Vec` impl = subsequence.** The value-borne-identity reading,
   which is the common case reached through trait dispatch (id lists inside
   table values, where field attributes cannot reach). Since decision 17 it is
   gated on `T: ContentOrd + ContentIdentity`, so it can only ever fire on
   id/scalar vectors — a structured element type without the marker simply
   does not dispatch, and the field demands an explicit attribute.
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
6. **The derive gets its own direct integration test** in
   `state/tests/derive_content_ord.rs`, in the style of `derive_refs.rs` /
   `derive_join.rs`.
7. **A `Vec` analogue of `option_lift_discrete` is provided**:
   `vec_subsequence` (elements matched by plain `==`, i.e. discretely), for
   `#[ord(with = …)]` on a `Vec` whose element type is foreign and cannot be
   enrolled in the trait.
8. **★ The content order *pre-exists* the resolution map.** The order is an
   intrinsic property of the document type, derived from first principles —
   content inclusion, D5.1, well-foundedness, scalar leaves as atoms. The map
   is then *obligated* to move strictly downward in it. "No fix touches this
   field" is **never** a reason for choosing a rule; such statements are
   theorems about the map and live in the §4.3 audit, not in the definition.
   (This corrected one rule outright: `Incompatibility::slots` is
   subsequence-ordered, not atomic.)
9. **The identity criterion for `Vec` fields.** Ask where an element's
   identity lives:
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
    of an earlier same-length rule). Adding a group is adding content, and
    zero groups is the minimum element; removing a *middle* group is an
    identity shift, not a removal — group numbers are referenced *by index*
    from the colloscope's placement maps and interrogation cells — and stays
    incomparable. The `same_length_pointwise` combinator of the earlier draft
    was replaced by `prefix_pointwise`.
12. **★ Equivalence classes are tolerated structurally.** `ContentOrd` has no
    `PartialEq` supertrait; its laws are stated up to the equivalence
    `Some(Equal)`; a `content_eq` default method names that relation so
    nobody reaches for `==`. The engine is `==`-free.
13. **★ Configuration values are atoms; the flat order and the
    `Default`-universal-minimum axiom are retired** (superseding both plan
    v2's flat rule and the step-6 minimal-element phrasing). The principled
    ground: `Limits` and `BalancingOptions` are *whole-entry override
    records* — per the long-standing override semantics, a `None` field there
    means **"disabled", an active choice, not absent content**, so the
    Option-lift reading is semantically false inside them and the record is
    one composite value. `ExportConfig` is one composite presentation
    preference. Termination needs only well-foundedness, which atoms give
    trivially; the order simply has several minimal elements.
14. **`#[ord(ignore)]`: a field the order does not see** (the user proposed
    the spelling `none`, changed to `ignore` so it cannot be read as "compare
    with `None`"). Generates a constant `Some(Equal)` and puts no bounds on
    the field's type. This is the structural source of equivalence classes —
    and it lets `Data` itself be **derived**, ignoring its `id_issuer`. The
    toy mirror: `EvilQuoteData` delegates to its inner data only, ignoring
    the mode, by hand.
15. **The derive gets a second, end-to-end test class**: a toy
    `InMemoryData + Fixable` whose `ContentOrd` is *derived*, driven through
    `apply_cascade` — including a small deterministic op-walk with no new
    dependencies (§6.6).
16. **★ No law relates `ContentOrd` to `PartialEq`** (second proofread
    round; the earlier "`==` must imply equivalence" law is dropped —
    `PartialEq` never promises structural equality, so no law may lean on
    it). The trait laws are internal: `Some(Equal)` is an equivalence;
    reflexivity, transitivity, antisymmetry up to it; well-foundedness.
    Reflexivity through `discrete` needs the leaf's `==` to be *reflexive*,
    so the discrete-matching helpers are bounded `Eq`, not `PartialEq`
    (`discrete<T: Eq>`, `option_lift_discrete<T: Eq>`,
    `vec_subsequence<T: Eq>`, `subsequence<T: Eq>`) — the obligation lives in
    the type system, and every leaf in use derives `Eq` (verified:
    `time/src/lib.rs:106,530,593,838`; `non_empty_string` likewise).
17. **★ The `ContentIdentity` marker trait** (second proofread round —
    answering "should `Ord` and `ContentOrd` share equivalence classes?").
    The orthogonality between `ContentOrd` and `Ord` is in the *ordering*,
    not the equality: Rust ties `Ord`'s `Equal` to `==` (`Ord: Eq` plus the
    consistency contract), so container matching is matching by `==`, and
    the only question is whether `==` coincides with content equivalence
    *for the element/key type*. `ContentIdentity: Eq` is the marker that
    asserts exactly that, emitted alongside every atom enrollment
    (`impl_atoms`, `impl_content_ord_atom`, tuples of markers), and
    **required by all five container blankets at matching positions**.
    Consequences: a quotiented type in a container is a compile error; and
    the one remaining footgun dies — `PrefilledGroup` has no marker, so the
    `Vec` blanket cannot apply to `Vec<PrefilledGroup>` and *omitting*
    `#[ord(with = vec_prefix)]` fails to compile instead of silently
    landing a too-strict order. Entity structs deliberately do not get the
    marker (they never sit at matching positions; opting in stays an
    explicit, auditable assertion).
18. **★ `ContentIdentity` is derivable, but never automatic** (second
    proofread round). Auto-emitting the marker from `#[derive(ContentOrd)]`
    is unsound: the compiler strips `#[derive(...)]` lists from macro input,
    so the macro *cannot know* whether `PartialEq` is derived or
    hand-written, and must never sign a claim it cannot check. Instead, an
    explicit `#[derive(ContentIdentity)]` checks everything checkable —
    rejects `#[ord(ignore)]` (a quotient by definition) and
    `#[ord(with = …)]` (unanalyzable; hand-write the marker impl if the rule
    preserves identity), statically asserts `ContentIdentity` on default
    fields and `Eq` on atom fields, accepts `total` fields for free (`Ord`'s
    own contract) — leaving exactly **one** human-signed premise: the type's
    `==` is the structural field-wise equality, auditable at a glance
    because `PartialEq` sits in the same derive list.
19. **★ `#[ord(partial_ord)]` is renamed `#[ord(total)]`** and generates
    `Some(Ord::cmp(…))` instead of `partial_cmp` (second proofread round).
    Strictly the laws need `PartialOrd + Eq`, but that is a two-trait side
    condition prose would have to carry; calling `Ord::cmp` makes the
    requirement self-enforcing — the call demands `Ord`, `Ord: Eq` gives
    reflexivity, and `Ord`'s contract ties `Equal` to `==`. The exotic case
    (a genuinely partial but `Eq`-honest order) goes through
    `#[ord(with = …)]` like any other custom rule.
20. **★ Every edge-case ruling lands as a comment at its code site** (user
    request). The snippets in this plan carry the comment text; the
    implementer copies them. The mandated sites: the two trait docs; the
    `Eq` bounds on the discrete-matching helpers; every container blanket's
    marker bound; the `Vec` blanket's non-dispatch example; the atom macros
    (emit both traits, and why); the tuple marker impls; the `total` and
    `ignore` codegen arms; the `ContentIdentity` derive's unverifiable
    premise; `Data`'s ignored issuer; `EvilQuoteData`'s ignored mode; the
    `groups` field's `vec_prefix` attribute; the three configuration-record
    enrollments; the `NonEmptyRangeInclusive` impls; the engine's two panic
    arms.

### 0.3 What this step does *not* do

No storage format change, no change to the op surface, no change to `ops/`
behavior, no gtk4 change, no new dependencies (`syn`/`quote`/`proc-macro2` are
already dependencies of `collomatique-state-derive`, so no Nix `cargoHash`
refresh). Nothing in production calls the cascade yet; that is still step 7's
decision.

### 0.4 Commit map

| commit | contents |
| --- | --- |
| 0 | this plan (v1 `ec0dd2a2`, v2 `85f44889`, v3 `74a80456`; this revision lands as its own commit) |
| 1 | `state/`: the `ContentOrd` and `ContentIdentity` traits, the combinators, the leaf/container/tuple impls, the `impl_content_ord_atom!` macro, the `with =` helpers + unit tests |
| 2 | `collomatique-state-derive`: `#[derive(ContentOrd)]` (four field attributes) and `#[derive(ContentIdentity)]` + the direct integration tests in `state/tests/derive_content_ord.rs` + the end-to-end cascade tests on a derived toy in `state/tests/cascade_on_derived_order.rs` |
| 3 | `state-colloscopes/`: adoption — the atom enrollments, the derives with their attributes (including `Data` with `#[ord(ignore)]`), the two manual impls + the semantic unit tests |
| 4 | the toy types: manual `ContentOrd` for `QuoteData` / `EvilQuoteData` + unit tests |
| 5 | the engine: `Fixable: InMemoryData + ContentOrd`, the in-loop strictly-below check, the contract rewording, two new evil modes, two new engine tests |
| 6 | the contract fuzz property (`state-colloscopes/tests/property_content_ord.rs`) |

Each commit builds and passes the whole workspace suite on its own. Commit 4
only needs commit 1; commits 3 and 5 need everything before them.

---

## 1. The two traits

```rust
/// The document order: a partial order over *content* (design doc §8,
/// step 6.5, and ruling D5.1 — content, not the meaning it denotes).
///
/// This is the order of the cascade's monotonicity contract: the order is
/// well-founded, and every resolution-map fix must land strictly below the
/// pre-fix state. [crate::apply_cascade] checks that in-flight. The order
/// is intrinsic to the data type — it is defined from the structure alone,
/// and the resolution map is *held to it*, never the other way around.
///
/// # Laws (self-contained: no law mentions `PartialEq`)
///
/// * `Some(Ordering::Equal)` is an equivalence relation — **content
///   equivalence** (see [ContentOrd::content_eq]). It may be coarser than a
///   type's `==`: a type may quotient away non-content fields (an id
///   issuer, a test-harness mode). No law relates it to `PartialEq` —
///   `PartialEq` never promises structural equality, so nothing here may
///   lean on it. The provided building blocks ([discrete] and the blankets)
///   *define* content equivalence from `==` for the types they cover; a
///   type assembled from them inherits whatever `==` means on its leaves.
/// * `content_cmp` is a partial order up to that equivalence: reflexive
///   (`x.content_cmp(&x) == Some(Equal)`), transitive, and antisymmetric up
///   to equivalence.
/// * **Well-foundedness on document data**: every strict decrease removes
///   an element from a finite container or moves an `Option` from `Some` to
///   `None` — so there is no infinite strictly-decreasing chain, and strict
///   monotonicity of fixes is a termination proof.
///
/// This is deliberately *not* `PartialOrd`: the standard library implements
/// `PartialOrd` lexicographically on containers (under which removing an
/// element can make a set sort *later*), and the typed ids must keep their
/// numeric `Ord` for use as map keys. A distinct trait keeps both worlds
/// intact and unambiguous.
pub trait ContentOrd {
    /// Compares two values in the document order.
    fn content_cmp(&self, other: &Self) -> Option<Ordering>;

    /// `self` and `other` are content-equivalent. Use this, never `==`,
    /// when the question is about the document order.
    fn content_eq(&self, other: &Self) -> bool {
        self.content_cmp(other) == Some(Ordering::Equal)
    }

    /// `self` is below or equal to `other` in the document order.
    fn content_le(&self, other: &Self) -> bool {
        matches!(
            self.content_cmp(other),
            Some(Ordering::Less | Ordering::Equal)
        )
    }

    /// `self` is strictly below `other` in the document order: below and
    /// not equivalent. This is the fix obligation.
    fn content_lt(&self, other: &Self) -> bool {
        self.content_cmp(other) == Some(Ordering::Less)
    }
}

/// Marker: `==` coincides with content equivalence — this type carries no
/// content quotient, so containers may match it by `==`/`Ord`.
///
/// Inside a container, "the same element/row" can only mean `==` (for
/// `Ord`-backed storage, `Ord`'s own contract ties its `Equal` to `==`), and
/// that is sound exactly when `==` is content identity for the element/key
/// type. This requirement is *positional* — it must hold at container
/// matching positions and nowhere else (a global law would outlaw quotients
/// like [`ContentOrd` on `Data`], which ignores the id issuer). The
/// container blanket impls require this marker at every matching position,
/// so a quotiented type inside a container is a compile error.
///
/// Deliberately opt-in: entity structs whose equivalence happens to equal
/// `==` today still do not get the marker unless they need it — "safe to
/// match by `==`" stays an explicit, auditable assertion. Enrollment paths:
/// the atom macros emit it together with `ContentOrd` (an atom's
/// equivalence *is* `==` by construction), tuples of markers are markers,
/// and composite types use `#[derive(ContentIdentity)]` (§6.4) or a
/// hand-written impl.
pub trait ContentIdentity: Eq {}
```

The comparison rule for a compound value is determined by its type's impl;
struct and enum impls are generated by `#[derive(ContentOrd)]` (commit 2) as
the product of their fields. Four field attributes override the default
dispatch:

* `#[ord(atom)]` — compare this field discretely, inline (`==` or
  incomparable); no trait impl needed on the field's type. The escape hatch
  for foreign types (orphan rule). The field's type must be `Eq` (reflexive
  `==`), enforced by the generated call's bound.
* `#[ord(ignore)]` — the order does not see this field: the generated
  comparison is a constant `Some(Equal)`, and no bound is placed on the
  field's type. **Using it introduces equivalence classes** (two values
  differing only in ignored fields are content-equivalent); that is its
  purpose, and it is legal under the laws above.
* `#[ord(with = <expr>)]` — compare this field with the given expression,
  which must be callable as `fn(&T, &T) -> Option<Ordering>`; a path or an
  inline closure.
* `#[ord(total)]` — this field's native *total* order is its content order:
  the generated code is `Some(Ord::cmp(…))`. Calling `Ord::cmp` makes the
  soundness requirement self-enforcing: the call demands `Ord`, `Ord: Eq`
  gives reflexivity, and `Ord`'s contract ties `Equal` to `==`. (A genuinely
  partial but `Eq`-honest order goes through `with` instead.) Never a
  default, because of the numeric-order traps of decision 2.

---

## 2. The order, block by block

The order is defined from first principles (decision 8): structure decomposes
until it bottoms out in scalar leaves, and each structural layer is read *as
content*. Seven named building blocks; §3 assigns every type a composition of
them.

1. **Atom** (discrete order). Comparable only when equal: `Some(Equal)` iff
   `==`, otherwise `None`. In order-theory vocabulary this is the discrete
   partial order — two different values are neither above nor below one
   another. Atoms must be `Eq` (a reflexive `==` is what makes the block
   reflexive — a `PartialEq`-only type like `f64` would break the law
   through `NaN != NaN`; the helper bounds enforce this). Atoms are, first,
   the scalar leaves of the document: strings, numbers, booleans, times,
   ids, ranges. A range deserves the explicit argument: its *content* is the
   endpoint pair, and reading `[2..=3] ⊆ [1..=4]` as an order would compare
   the denoted sets — exactly the semantic reading D5.1 forbids. Text is
   likewise opaque: a string is a scalar value, not a container of
   characters. Second, atoms are the composite values that read as *one*
   value: relational chains (the `WeekBlock` list, decision 10) and the
   three **configuration records** (decision 13) — `Limits`,
   `BalancingOptions`, `ExportConfig`. In the two override records a `None`
   field means "disabled", an active choice, so the Option lift's reading
   ("`None` is less content") is false inside them; `ExportConfig` is one
   composite presentation preference.
2. **Option lift.** `None` is strictly below `Some(_)`; two `Some` values
   compare by the inner rule. This applies to every `Option` field *whose
   `None` means absent content* — which is every `Option` field the order
   descends into (the override records, where `None` means "disabled", are
   atoms and never descended into). Clearing optional content is removing
   content; the rule is load-bearing for the two optional foreign keys a fix
   actually clears (`Slot::week_pattern`, `Incompatibility::week_pattern_id`).
3. **Set inclusion.** `Equal` iff equal, `Less` iff strict subset, `Greater`
   iff strict superset, `None` otherwise. Elements are matched by `==`/`Ord`
   and must be `ContentIdentity` (§0.1, enforced by the blanket bound).
4. **Map inclusion with a value rule.** Below iff the key set is included and
   every shared key's value is below or equal. Rows are matched **by id**:
   the same teacher under a different id is incomparable (the user's defining
   example); the same id with different content compares by the value rule.
   Keys must be `ContentIdentity`.
5. **Sequence embedding (subsequence).** `Less` iff the left is a strict
   subsequence of the right — obtainable by *deleting elements only*, the
   survivors keeping their relative order; contiguity is **not** required:
   `[1,3] < [1,2,3]`. Reordering is incomparable: ordering is user-visible
   data. Elements are matched by `==` and must be `ContentIdentity` on the
   blanket path.
6. **Prefix pointwise.** A `Vec` whose positions carry the identity is
   content-wise a **map from indices to values**; map inclusion (block 4)
   specialized to index sets that are initial segments gives: below iff the
   left is at most as long *and* every shared index compares below or equal.
   Appending is adding content (the empty vector is the minimum); shrinking a
   value in place is a decrease; removing a *middle* element shifts the
   identity of every later element and is incomparable. (No element
   *matching* happens here — positions match by construction — so no
   `ContentIdentity` is involved.)
7. **Product** — for structs *and enum variants*. For a struct: the
   field-wise combination. `Equal` iff every field `Equal`; `Less` iff at
   least one `Less` and none `Greater` or incomparable; `None` as soon as one
   field is incomparable or two fields disagree in direction. A product of
   atoms degenerates to an atom. For an **enum**: two values of the *same*
   variant compare as the product of that variant's fields (a unit variant is
   the empty product, `Equal`); two values of *different* variants are
   incomparable.

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

**Where equivalence classes may appear.** Only above the containers: through
`#[ord(ignore)]` or a hand-written quotient (`Data`'s ignored issuer,
`EvilQuoteData`'s ignored mode). Inside containers, matching is `==`/`Ord`,
and the `ContentIdentity` bounds make a quotiented element or key a compile
error rather than a prose violation.

Two implementation rules apply everywhere: **never `#[derive(PartialOrd)]`,
never the standard library's container orders** (both lexicographic, wrong for
a removal order); and no impl may skip a field *silently* — the derive walks
every field by construction, and ignoring one is exactly what
`#[ord(ignore)]` states out loud.

---

## 3. The complete specification, type by type

Notation: "atom", "lift(X)", "set-inclusion", "map-inclusion(V)",
"subsequence", "prefix(V)", "product" refer to §2. The **how** column says
what produces the impl: `derive` (with any attributes), `blanket` (a
container impl from commit 1), `macro` (`impl_content_ord_atom!`, which also
emits `ContentIdentity`), `manual`, or `—` (no impl; the type is only ever
compared inside an atom boundary).

### 3.1 The two roots

| type | rule | how |
| --- | --- | --- |
| `Data` (`lib.rs`) | product with `id_issuer` **ignored** — the content equivalence is "same inner data", which coincides with `Data`'s hand-written `PartialEq` | derive, `#[ord(ignore)]` on `id_issuer` |
| `InnerData` (`lib.rs`) | product: `params` × `colloscope` × `export_config` | derive |

### 3.2 `Parameters` and everything under it

| type (module) | rule | how |
| --- | --- | --- |
| `Parameters` (`colloscope_params.rs`) | product of its fourteen fields | derive |
| `Periods` (`periods.rs`) | product: `first_week` lift(atom `WeekStart`) × `ordered_period_list` embedding (unit values always `Equal`) | derive; `first_week` gets `#[ord(with = option_lift_discrete)]` (foreign inner type) |
| `Weeks` (`weeks.rs`) | product: `week_map` map-inclusion(`Week`) × `ordering` map-inclusion(subsequence of `Vec<WeekId>`) | derive — the `Table` and `Vec` blankets compose (ids carry `ContentIdentity`) |
| `Week` (`weeks.rs`) | product: `period_id` atom × `interrogations` atom × `annotation` lift(atom) | derive; `annotation` gets `#[ord(with = option_lift_discrete)]` |
| `Subjects` (`subjects.rs`) | embedding of `OrderedTable<SubjectId, Subject>` with pointwise `Subject` values | derive |
| `Subject` (`subjects.rs`) | product: `parameters` × `excluded_periods` set-inclusion | derive |
| `SubjectParameters` (`subjects.rs`) | product: `name` atom × `interrogation_parameters` lift(product) | derive — the `Option` blanket dispatches into the structural `SubjectInterrogationParameters` |
| `SubjectInterrogationParameters` (`subjects.rs`) | product: `students_per_group` atom (range) × `groups_per_interrogation` atom (range) × `duration` atom × `take_duration_into_account` atom × `periodicity` | derive; `duration` gets `#[ord(atom)]` (foreign `NonZeroMinutes`) |
| `SubjectPeriodicity` (`subjects.rs`) | enum: same variant → product of its fields (all atoms), different variants → incomparable | derive; the `blocks` field of `AmountForEveryArbitraryBlock` gets `#[ord(atom)]` — ★ decision 10: the `WeekBlock` chain is one composite value |
| `WeekBlock` (`subjects.rs`) | — (inside the `blocks` atom) | — |
| `NonEmptyRangeInclusive<T>` (`non_empty_range.rs`) | atom — content is the endpoint pair; interval inclusion would be the semantic reading D5.1 forbids | manual (generic, so outside the macro's reach; also gets `ContentIdentity`) |
| `Teachers` (`teachers.rs`) | map-inclusion(`Teacher`) | derive |
| `Teacher` (`teachers.rs`) | product: `desc` × `subjects` set-inclusion | derive |
| `PersonWithContact` (`lib.rs`) | product: `surname` atom × `firstname` atom × `tel` lift(atom) × `email` lift(atom) | derive; `tel` and `email` get `#[ord(with = option_lift_discrete)]` (foreign `NonEmptyString`) |
| `Students` / `Student` (`students.rs`) | as `Teachers` / `Teacher`, with `excluded_periods` set-inclusion | derive |
| `Assignments` (`assignments.rs`) | `map` map-inclusion(set-inclusion) — the `(PeriodId, SubjectId)` key is `ContentIdentity` via the tuple impl | derive |
| `WeekPatterns` / `WeekPattern` (`week_patterns.rs`) | map-inclusion(product: `name` atom × `excluded_weeks` set-inclusion) | derive |
| `Slots` (`slots.rs`) | product: `slot_map` map-inclusion(`Slot`) × `ordering` map-inclusion(subsequence of `Vec<SlotId>`) | derive |
| `Slot` (`slots.rs`) | product: `subject_id` atom × `teacher_id` atom × `start_time` atom × `extra_info` atom × `week_pattern` lift(atom) × `cost` atom | derive; `start_time` gets `#[ord(atom)]` (foreign `SlotStart`); `week_pattern` needs no attribute (`Option` blanket + id atom) |
| `Incompats` (`incompats.rs`) | map-inclusion(`Incompatibility`) | derive |
| `Incompatibility` (`incompats.rs`) | product: `subject_id` atom × `name` atom × `slots` **subsequence** × `minimum_free_slots` atom × `week_pattern_id` lift(atom) | derive; `slots` gets `#[ord(with = vec_subsequence)]` — a time window's identity is its value, so removing one is removing content (decision 8); the helper is needed because `SlotWithDuration` is foreign |
| `GroupLists` (`group_lists.rs`) | product: `group_list_map` map-inclusion(`GroupList`) × `subjects_associations` map-inclusion(atom `GroupListId`) | derive |
| `GroupList` (`group_lists.rs`, sealed) | product: `params` × `filling` | derive (in-module; private fields are fine) |
| `GroupListParameters` (`group_lists.rs`) | product: `name` atom × `students_per_group` atom (range) × `group_names` **prefix**(lift(atom)) — un-naming a group is below, renaming is incomparable, truncating is below, a middle removal shifts bindings and is incomparable | derive; `group_names` gets `#[ord(with = \|a, b\| prefix_pointwise(a, b, option_lift_discrete))]` (closure form; the element type is foreign) |
| `GroupListFilling` (`group_lists.rs`) | `Prefilled`/`Prefilled`: **prefix**(`PrefilledGroup`); `Automatic`/`Automatic`: `excluded_students` set-inclusion; mixed variants: incomparable | derive; the `groups` field gets `#[ord(with = vec_prefix)]` — position-borne identity (decision 11). `PrefilledGroup` has no `ContentIdentity`, so the `Vec` blanket cannot apply here: omitting the attribute is a compile error (decision 17) |
| `PrefilledGroup` (`group_lists.rs`) | `students` set-inclusion | derive (deliberately **no** `ContentIdentity`) |
| `Settings` (`settings.rs`) | product: `global` atom × `students` map-inclusion(atom) | derive (`Limits` carries the atom impl) |
| `Limits` (`settings.rs`) | atom — a whole-entry override record: a `None` field means "disabled", an active choice, not absent content (decision 13) | macro |
| `Pairings` (`pairings.rs`) | map-inclusion(`PairingRule`) | derive |
| `PairingRule` (`pairings.rs`, sealed) | product: `antecedent` × `consequent` × `excluded_periods` set-inclusion × `soft` atom | derive (in-module) |
| `RulePart` (`pairings.rs`) | product: `subject_id` atom × `should_have` atom — a product of atoms, i.e. effectively an atom, but derived so any future field joins the order | derive |
| `SlotPairings` / `SlotPairingRule` / `SlotRulePart` (`slot_pairings.rs`) | as pairings | derive |
| `Balancing` (`balancing.rs`) | product: `global` atom × `subjects` map-inclusion(atom) | derive |
| `BalancingOptions` (`balancing.rs`) | atom — same whole-entry override argument as `Limits` | macro |

### 3.3 The colloscope and the export configuration

| type (module) | rule | how |
| --- | --- | --- |
| `Colloscope` (`colloscopes.rs`) | product: `interrogations` map-inclusion(set-inclusion of `BTreeSet<u32>`) × `group_lists` map-inclusion(map-inclusion(atom `u32`)) — the `(SlotId, WeekId)` key via the tuple `ContentIdentity` | derive — the blankets compose all the way down |
| `ExportConfig` (`export_config.rs`) | atom — one composite presentation preference (decision 13) | macro |

### 3.4 Enrollment of the atoms, and the unenrolled types

In `state-colloscopes/src/ids.rs`, one invocation — ids are scalar reference
tokens with no internal content, so they are atoms wherever they appear as
field *values*, and their `==` is content identity (the macro emits both
traits):

```rust
collomatique_state::impl_content_ord_atom!(
    PeriodId, WeekId, SubjectId, TeacherId, StudentId, WeekPatternId,
    SlotId, IncompatId, GroupListId, PairingRuleId, SlotPairingRuleId,
);
```

Next to their definitions, the three configuration records, each with a short
comment carrying the decision-13 argument:

```rust
// A whole-entry override record: a `None` field means "disabled" — an
// active choice, not absent content — so the document order treats the
// whole record as one atom (plan step 6.5, decision 13).
collomatique_state::impl_content_ord_atom!(Limits);
```

(and likewise `BalancingOptions` in `balancing.rs`, `ExportConfig` in
`export_config.rs` — the latter's comment says "one composite presentation
preference" instead of the override sentence).

`NonEmptyRangeInclusive<T>` is generic, which the macro cannot express, so it
gets the one hand-written pair of impls (§7.4).

Types with **no** impl at all (only ever compared inside an atom boundary):
`WeekBlock`, `SoftParam<T>`, `Color`, `PageOrientation`, `GlobalConfig`,
`ColloscopeConfig`, `PerStudentGroupsConfig`, `PerGroupListConfig`,
`WeekDesc`, and every `collomatique-time` / `non_empty_string` type.

---

## 4. Why this order is correct

### 4.1 Consistency of equality and equivalence

Every block generated by the derive, the blankets and the macro reports
`Some(Equal)` exactly when the two values are `==`: atoms by definition (with
`Eq` guaranteeing reflexivity); the lift because both sides must be `None` or
both inner values equal; inclusion/embedding because mutual inclusion of
finite structures forces equality; prefix pointwise because mutual prefix
forces equal lengths and pointwise equality; and a product of such fields is
again exact, provided every field participates — which the derive guarantees
by construction. Equivalence classes therefore enter **only** where they are
declared: `#[ord(ignore)]` (the derived `Data`, whose equivalence "same inner
data" coincides with its hand-written `PartialEq`) and the hand-written toy
quotient (`EvilQuoteData`). No trait law relates `ContentOrd` to `PartialEq`
(decision 16); the sharper fact that content equivalence *is* structural
equality for `InnerData` and everything below it is a crate-level property,
pinned by §7.5's twin tests.

### 4.2 Well-foundedness (the termination proof survives)

Every strict decrease under any block removes an element from a finite
container (including a trailing prefix element) or moves an `Option` from
`Some` to `None`. Each block is therefore well-founded on the finite values
the document holds, and a product of well-founded orders is well-founded (an
infinite strictly-decreasing chain would have to decrease some field
infinitely often). Strict monotonicity of fixes plus a well-founded order is
the termination proof, unchanged from step 6 — this step only *materializes*
the order it was already stated in. Note that termination needs nothing else:
in particular it does not need a universal minimum (decision 13), and
well-foundedness itself is argued here, not checked by a test — a fuzz for
finite descent would be hairy and slow for no payoff.

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

No arm creates anything, renames anything, reorders anything, or touches a
configuration record — so the comparability the atoms withhold is never
needed by today's map, and the in-loop assertion holds every *future* arm to
the same discipline. Shapes 4 and 5 include the D5.1 arms that widen
semantics while shrinking content; under §3 they are plain decreases, as
required.

### 4.4 Minimal elements

The empty document `InnerData::default()` is *a* minimal element: nothing
sits strictly below it (its tables are empty and cannot lose a row,
`first_week` is already `None`, and its configuration records are atoms —
comparable only to themselves). It is **not** a universal minimum: a document
that differs from the default only in a configuration value is incomparable
to it, and is itself minimal. This plurality is deliberate (decision 13,
retiring the step-6 "universal minimal element" phrasing) and harmless: the
termination proof (§4.2) rests on well-foundedness alone. §7.5 keeps a unit
pin of the sanity half — the default *is* strictly below a populated document
whose configuration was never touched.

### 4.5 What the order rejects, on purpose

Same entity content under a different id; same id with different scalar
content; any reordering; a retargeted association or renumbered placement; a
*middle* group removed from a prefilled list (the surviving groups' externally
referenced indices silently re-aim — an identity shift, not a removal); one
table shrinking while another grows; any change to a configuration record.
Each would be a contract violation if a fix produced it, and the in-loop
assertion turns each into a panic.

---

## 5. Commit 1 — the `ContentOrd` layer in `state/`

Everything lands in a new module `state/src/partial_order.rs`; `tables.rs` is
**not** touched (the container impls read `Table`/`OrderedTable` through their
public API).

### 5.1 The module

Contents, in order: the module doc, the two traits of §1 (verbatim, with
their full doc comments), then the combinators — the shared vocabulary the
blanket impls, the derive expansion, the manual impls and the attributes all
use:

```rust
//! The document order: building blocks, the [ContentOrd] trait and the
//! [ContentIdentity] marker (design doc §8, step 6.5).
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

pub trait ContentOrd { /* §1, verbatim */ }
pub trait ContentIdentity: Eq {} /* §1, verbatim */
```

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
///
/// `Eq`, not `PartialEq`: the document order's reflexivity law rests on the
/// leaf's `==` being reflexive — a `PartialEq`-only type like `f64` would
/// break it through `NaN != NaN`. The bound puts that obligation in the
/// type system.
pub fn discrete<T: Eq + ?Sized>(a: &T, b: &T) -> Option<Ordering> {
    (a == b).then_some(Ordering::Equal)
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

/// Sequence embedding: a strict subsequence — obtainable by *deleting*
/// elements, the survivors keeping their relative order; contiguity is NOT
/// required, so `[1,3]` is a subsequence of `[1,2,3]` and compares `Less`.
/// A reordering is incomparable. Elements are matched by `==` (`Eq` for the
/// same reflexivity reason as [discrete]).
pub fn subsequence<T: Eq>(a: &[T], b: &[T]) -> Option<Ordering> {
    fn embeds<T: Eq>(small: &[T], big: &[T]) -> bool {
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
element types the orphan rule keeps out of the traits; the third is the
positional-identity rule for enrolled types:

```rust
/// `Option` lift with a discrete inner comparison — for
/// `#[ord(with = option_lift_discrete)]` on an `Option` whose inner type is
/// foreign (`Option<NonEmptyString>`, `Option<WeekStart>`, …).
pub fn option_lift_discrete<T: Eq>(a: &Option<T>, b: &Option<T>) -> Option<Ordering> {
    option_lift(a, b, |x, y| discrete(x, y))
}

/// Sequence embedding with elements matched discretely (by `==`) — the `Vec`
/// analogue of [option_lift_discrete], for `#[ord(with = vec_subsequence)]`
/// on a `Vec` whose element type is foreign. (For an *enrolled*
/// [ContentIdentity] element type the blanket `Vec` impl already gives
/// exactly this behavior.)
pub fn vec_subsequence<T: Eq>(a: &Vec<T>, b: &Vec<T>) -> Option<Ordering> {
    subsequence(a, b)
}

/// Prefix-pointwise comparison through [ContentOrd] — for
/// `#[ord(with = vec_prefix)]` where element identity is positional
/// (prefilled groups). Positional elements are never matched by `==`, so no
/// [ContentIdentity] is required of them — deliberately: a positional
/// element type *without* the marker also keeps the `Vec` blanket from
/// applying, which turns a forgotten attribute into a compile error.
pub fn vec_prefix<T: ContentOrd>(a: &Vec<T>, b: &Vec<T>) -> Option<Ordering> {
    prefix_pointwise(a, b, ContentOrd::content_cmp)
}
```

### 5.3 The leaf, container and tuple impls

Scalar atoms, via a private macro — note it emits **both** traits (an atom's
content equivalence is `==` by construction, so the marker is always
truthful for it):

```rust
macro_rules! impl_atoms {
    ($($t:ty),* $(,)?) => { $(
        impl ContentOrd for $t {
            fn content_cmp(&self, other: &Self) -> Option<Ordering> {
                discrete(self, other)
            }
        }
        // An atom's content equivalence IS `==`, so `==` is content
        // identity and the type may be matched inside containers.
        impl ContentIdentity for $t {}
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
foreign trait on a local type; `state-colloscopes` enrolls its ids and its
configuration records with this):

```rust
/// Enrolls local types into the document order as atoms (discretely
/// compared: equal or incomparable), together with [ContentIdentity] —
/// an atom's content equivalence is `==` by construction, so the marker is
/// always truthful for it. For foreign types use `#[ord(atom)]` on the
/// field instead; for generic types write the impls by hand.
#[macro_export]
macro_rules! impl_content_ord_atom {
    ($($t:ty),* $(,)?) => { $(
        impl $crate::partial_order::ContentOrd for $t {
            fn content_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                $crate::partial_order::discrete(self, other)
            }
        }
        impl $crate::partial_order::ContentIdentity for $t {}
    )* };
}
```

Tuples of markers are markers (composite table keys — `(PeriodId,
SubjectId)`, `(SlotId, WeekId)` — are matched by `Ord`, which compares
component-wise `==`):

```rust
// A tuple of content identities is a content identity: tuple `==` is
// component-wise `==`, which coincides with component-wise content
// equivalence by the components' own markers.
impl<A: ContentIdentity, B: ContentIdentity> ContentIdentity for (A, B) {}
impl<A: ContentIdentity, B: ContentIdentity, C: ContentIdentity> ContentIdentity for (A, B, C) {}
```

The container blankets — the heart of the dispatch design. Every matching
position requires `ContentIdentity` (decision 17):

```rust
impl<T: ContentOrd> ContentOrd for Option<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        option_lift(self, other, ContentOrd::content_cmp)
    }
}

// Set elements are matched by `Ord`, whose `Equal` is `==` by Rust's own
// contract — sound exactly when `==` is content identity for the element
// type, hence the marker bound. A quotiented element type does not compile.
impl<T: Ord + ContentIdentity> ContentOrd for BTreeSet<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        set_inclusion(self, other)
    }
}

// Keys are row identity; the marker bound is the same argument as for sets.
impl<K: Ord + ContentIdentity, V: ContentOrd> ContentOrd for BTreeMap<K, V> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        map_inclusion(self, other, ContentOrd::content_cmp)
    }
}

/// Sequence embedding — the value-borne-identity reading, which is the
/// common case reached through trait dispatch (id lists inside table
/// values). Elements are matched by `==`, hence the [ContentIdentity]
/// bound; it doubles as a safety net: a structured element type without
/// the marker (e.g. `PrefilledGroup`) does not dispatch at all, so the
/// field demands an explicit `#[ord(...)]` attribute instead of silently
/// getting a wrong rule. Where identity is positional, use
/// `#[ord(with = vec_prefix)]`; where the list is a relational chain,
/// `#[ord(atom)]` (§2, the identity criterion).
impl<T: ContentOrd + ContentIdentity> ContentOrd for Vec<T> {
    fn content_cmp(&self, other: &Self) -> Option<Ordering> {
        subsequence(self, other)
    }
}

// Table keys are row identity — same marker argument as `BTreeMap`.
impl<I: Key + ContentIdentity, T: ContentOrd> ContentOrd for Table<I, T> {
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

impl<I: OrderedKey + ContentIdentity, T: ContentOrd> ContentOrd for OrderedTable<I, T> {
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
`None` from the key comparison before any value is consulted. The
`ContentIdentity: Eq` supertrait also supplies the `Eq` that `subsequence`
demands of the key vectors.)

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
pub use partial_order::{ContentIdentity, ContentOrd};
```

(The combinators and helpers stay behind the module path —
`collomatique_state::partial_order::…` — only the traits get root
re-exports, mirroring `Fixable`.)

### 5.5 Commit-1 unit tests

In `partial_order.rs` `#[cfg(test)]`, on plain `u64` / `String` fixtures:

* `combine`: all-equal → `Equal`; a `Less` among equals → `Less`; `Less` and
  `Greater` mixed → `None`; any `None` → `None`; empty iterator → `Equal`.
* `discrete`, `option_lift` (all four arms).
* `set_inclusion`: equal / strict subset / strict superset / crossed →
  `None`; **the lexicographic trap pinned**: `set_inclusion` of `{1,3}` vs
  `{1,2,3}` is `Some(Less)` while std's `Ord` sorts `{1,3}` *after*
  `{1,2,3}` — assert both facts side by side, as documentation.
* `map_inclusion`: key-subset with equal values → `Less`; same keys, one
  value `Less` → `Less`; key-subset but a shared value `Greater` → `None`.
* `subsequence`: `[1,3]` vs `[1,2,3]` → `Less` (the non-contiguity pin);
  reorder `[2,1]` vs `[1,2]` → `None`; equal; empty below anything.
* `prefix_pointwise`: truncation `[1]` vs `[1,2]` → `Less`; equal length
  with one pointwise decrease → `Less`; **the middle-removal pin**: `[1,3]`
  vs `[1,2,3]` with a discrete element rule → `None` (contrast with
  `subsequence`, which says `Less` on the same input — the two rules differ
  exactly on identity); mixed directions → `None`; empty below anything.
* the default methods: `content_eq` / `content_le` / `content_lt` on a small
  enrolled type, including `content_lt == false` on equivalent values, and a
  reflexivity check `x.content_cmp(&x.clone()) == Some(Equal)`.
* Blanket dispatch smoke tests: `Option<u32>` (`None < Some(3)`,
  `Some(3)` vs `Some(4)` → `None`), `Vec<u32>` subsequence,
  `BTreeMap<u64, u32>` (value renumber → `None`), `Table` / `OrderedTable`
  with a local test id type (row removal → `Less`, reorder → `None`, value
  change dispatching into the value's impl), and a
  `Table<(u64, u64), BTreeSet<u64>>` exercising the tuple `ContentIdentity`
  (composite keys compile and compare — this is the assignments/colloscope
  shape).

---

## 6. Commit 2 — the derives in `collomatique-state-derive`

### 6.1 Registration

New file `state-derive/src/content_ord.rs` holding both entry points; in
`state-derive/src/lib.rs`, next to the existing entries:

```rust
#[proc_macro_derive(ContentOrd, attributes(ord))]
pub fn derive_content_ord(input: TokenStream) -> TokenStream {
    content_ord::derive(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

// Registers `attributes(ord)` too: the identity derive must *read* the
// `#[ord(...)]` attributes to reject quotients, and registration keeps the
// attribute inert even when `ContentIdentity` is derived alone.
#[proc_macro_derive(ContentIdentity, attributes(ord))]
pub fn derive_content_identity(input: TokenStream) -> TokenStream {
    content_ord::derive_identity(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
```

And in `state/src/lib.rs`, the derives join the existing re-export (derive
macros and traits live in different namespaces, so each name can be both —
the `Join` trait/derive pair is the in-house precedent). Old code:

```rust
pub use collomatique_state_derive::{EntityId, Join, References};
```

New code:

```rust
pub use collomatique_state_derive::{ContentIdentity, ContentOrd, EntityId, Join, References};
```

### 6.2 Shape gate and attribute parsing

Accepted inputs, for both derives: **non-generic structs with named fields**
(including empty ones) and **non-generic enums whose variants have named
fields or are unit variants**. Everything else — generics (the `Join`
precedent), tuple structs, tuple variants, unions — is rejected with a
spanned `syn::Error` naming the restriction. All our targets are non-generic
named shapes.

The attribute grammar, shared by both derives:

```rust
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Expr, Field, Fields, FieldsNamed, Ident, Token};

/// The comparison rule of one field, from its optional `#[ord(...)]`
/// attribute.
enum FieldRule {
    /// No attribute: dispatch through `ContentOrd` on the field's type.
    Default,
    /// `#[ord(atom)]`: inline discrete comparison (`discrete`, whose `Eq`
    /// bound enforces the reflexivity obligation on the field's type).
    Atom,
    /// `#[ord(ignore)]`: the order does not see this field — the
    /// structural source of equivalence classes.
    Ignore,
    /// `#[ord(total)]`: the field's native total order is its content
    /// order (`Ord::cmp`, self-enforcing — see the codegen table).
    Total,
    /// `#[ord(with = <expr>)]`: call the expression (path or closure).
    With(Expr),
}

impl Parse for FieldRule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "atom" => Ok(FieldRule::Atom),
            "ignore" => Ok(FieldRule::Ignore),
            "total" => Ok(FieldRule::Total),
            // A plain expression, not a string: `with = option_lift_discrete`
            // and `with = |a, b| …` both parse (house style, like
            // `#[join(error = NewId)]`).
            "with" => {
                input.parse::<Token![=]>()?;
                Ok(FieldRule::With(input.parse()?))
            }
            _ => Err(syn::Error::new(
                ident.span(),
                "expected `atom`, `ignore`, `total` or `with = <expression>`",
            )),
        }
    }
}

/// Extracts the rule of one field; at most one `#[ord(...)]` per field.
fn field_rule(attrs: &[Attribute]) -> syn::Result<FieldRule> {
    let mut found: Option<FieldRule> = None;
    for attr in attrs {
        if !attr.path().is_ident("ord") {
            continue;
        }
        if found.is_some() {
            return Err(syn::Error::new(
                attr.span(),
                "at most one `#[ord(...)]` attribute per field",
            ));
        }
        found = Some(attr.parse_args()?);
    }
    Ok(found.unwrap_or(FieldRule::Default))
}
```

### 6.3 `ContentOrd` codegen

Per-field generated comparison (fully-qualified paths throughout, per the
house lesson on derive hygiene; trait methods are called in function position
so the receiver type is inferred from the arguments and never re-spelled).
`lhs`/`rhs` are the borrowed access expressions — `&self.x` / `&other.x` for
structs, the match bindings for enums:

```rust
fn cmp_expr(rule: &FieldRule, lhs: TokenStream, rhs: TokenStream) -> TokenStream {
    match rule {
        FieldRule::Default => quote! {
            ::collomatique_state::partial_order::ContentOrd::content_cmp(#lhs, #rhs)
        },
        FieldRule::Atom => quote! {
            ::collomatique_state::partial_order::discrete(#lhs, #rhs)
        },
        // Constant Equal: the order does not see the field. This is what
        // makes the containing type's content equivalence coarser than its
        // `==` — the declared quotient.
        FieldRule::Ignore => quote! {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        },
        // `Ord::cmp`, not `partial_cmp`: the call itself demands `Ord`,
        // whose contract (`Ord: Eq`, `cmp(x, x) == Equal`, `Equal` iff
        // `==`) is exactly what makes the field's native order a valid,
        // reflexive content order. A genuinely partial order goes through
        // `with` instead.
        FieldRule::Total => quote! {
            ::core::option::Option::Some(::core::cmp::Ord::cmp(#lhs, #rhs))
        },
        FieldRule::With(expr) => quote! { (#expr)(#lhs, #rhs) },
    }
}
```

The struct body — the product over all fields, in declaration order; an
empty struct short-circuits (an empty array literal would not infer its item
type):

```rust
fn struct_body(fields: &FieldsNamed) -> syn::Result<TokenStream> {
    if fields.named.is_empty() {
        return Ok(quote! {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        });
    }
    let cmps = fields
        .named
        .iter()
        .map(|field| {
            let rule = field_rule(&field.attrs)?;
            let name = field.ident.as_ref().expect("named field");
            Ok(cmp_expr(&rule, quote! { &self.#name }, quote! { &other.#name }))
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {
        ::collomatique_state::partial_order::combine([#(#cmps),*])
    })
}
```

The enum body — one arm per variant pair of the *same* variant, destructuring
both sides with distinct bindings (which are references thanks to match
ergonomics, so they feed `cmp_expr` directly); a unit variant is the empty
product; a trailing `_ => None` arm is emitted only when the enum has at
least two variants (on a single-variant enum it would be unreachable and
warn):

```rust
fn enum_body(data: &syn::DataEnum) -> syn::Result<TokenStream> {
    let mut arms = Vec::new();
    for variant in &data.variants {
        let v = &variant.ident;
        match &variant.fields {
            Fields::Unit => arms.push(quote! {
                (Self::#v, Self::#v) =>
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal),
            }),
            Fields::Named(fields) => {
                let names: Vec<&Ident> =
                    fields.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                let self_bind: Vec<Ident> =
                    names.iter().map(|n| format_ident!("self_{}", n)).collect();
                let other_bind: Vec<Ident> =
                    names.iter().map(|n| format_ident!("other_{}", n)).collect();
                let cmps = fields
                    .named
                    .iter()
                    .zip(self_bind.iter().zip(&other_bind))
                    .map(|(field, (s, o))| {
                        let rule = field_rule(&field.attrs)?;
                        Ok(cmp_expr(&rule, quote! { #s }, quote! { #o }))
                    })
                    .collect::<syn::Result<Vec<_>>>()?;
                arms.push(quote! {
                    (
                        Self::#v { #(#names: #self_bind),* },
                        Self::#v { #(#names: #other_bind),* },
                    ) => ::collomatique_state::partial_order::combine([#(#cmps),*]),
                });
            }
            Fields::Unnamed(f) => {
                return Err(syn::Error::new(
                    f.span(),
                    "#[derive(ContentOrd)] does not support tuple variants",
                ));
            }
        }
    }
    let fallback = (data.variants.len() > 1)
        .then(|| quote! { _ => ::core::option::Option::None, });
    Ok(quote! {
        match (self, other) {
            #(#arms)*
            #fallback
        }
    })
}
```

And the entry point:

```rust
pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    reject_generics(&input)?;
    let ident = &input.ident;
    let body = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => struct_body(fields)?,
            Fields::Unit => quote! {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal)
            },
            Fields::Unnamed(f) => {
                return Err(syn::Error::new(
                    f.span(),
                    "#[derive(ContentOrd)] does not support tuple structs",
                ));
            }
        },
        Data::Enum(e) => enum_body(e)?,
        Data::Union(u) => {
            return Err(syn::Error::new(
                u.union_token.span(),
                "#[derive(ContentOrd)] does not support unions",
            ));
        }
    };
    Ok(quote! {
        impl ::collomatique_state::partial_order::ContentOrd for #ident {
            fn content_cmp(
                &self,
                other: &Self,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                #body
            }
        }
    })
}
```

(This example expansion is literally what `GroupListFilling` will produce:
one `Prefilled`/`Prefilled` arm calling `(vec_prefix)(self_groups,
other_groups)`, one `Automatic`/`Automatic` arm dispatching the student set
through the trait, and `_ => None`.)

### 6.4 `ContentIdentity` codegen

**Never automatic** (decision 18): the compiler strips `#[derive(...)]`
lists from macro input, so no macro can know whether `PartialEq` is derived
or hand-written — and a macro must never sign a claim it cannot check. The
explicit derive checks everything checkable and documents the one premise it
cannot:

```rust
/// `#[derive(ContentIdentity)]`: asserts that `==` coincides with content
/// equivalence for this type, so containers may match it by `==`/`Ord`.
///
/// The macro verifies what it can see: every field's rule must preserve
/// identity — `ignore` is rejected outright (an ignored field IS a content
/// quotient), `with` is rejected as unanalyzable (hand-write the marker
/// impl if the custom rule preserves identity), default fields must
/// themselves be `ContentIdentity`, atom fields must be `Eq`, and `total`
/// fields are safe by `Ord`'s own contract (`Equal` iff `==`).
///
/// ONE premise remains that no macro can verify: this type's `==` must be
/// the structural, field-wise equality — in practice, `PartialEq` must be
/// *derived*, in the same derive list where this marker is requested. That
/// co-location is the audit trail: replacing the derived `PartialEq` with a
/// hand-written one obligates re-justifying the `ContentIdentity` right
/// next to it.
pub fn derive_identity(input: DeriveInput) -> syn::Result<TokenStream> {
    reject_generics(&input)?;
    let mut asserts = Vec::new();
    for field in all_named_fields(&input)? /* struct fields, or every variant's */ {
        let ty = &field.ty;
        match field_rule(&field.attrs)? {
            FieldRule::Default => asserts.push(quote! {
                assert_content_identity::<#ty>();
            }),
            FieldRule::Atom => asserts.push(quote! {
                assert_eq_impl::<#ty>();
            }),
            // Safe by Ord's contract: Ord: Eq, and cmp == Equal iff ==.
            FieldRule::Total => {}
            FieldRule::Ignore => {
                return Err(syn::Error::new(
                    field.span(),
                    "an `#[ord(ignore)]`d field is a content quotient: \
                     this type cannot be ContentIdentity",
                ));
            }
            FieldRule::With(_) => {
                return Err(syn::Error::new(
                    field.span(),
                    "`#[ord(with = ...)]` cannot be analyzed by the derive; \
                     write the ContentIdentity impl by hand if the custom \
                     rule preserves identity",
                ));
            }
        }
    }
    let ident = &input.ident;
    // The static-assert pattern, not `where`-clauses on the impl: failures
    // are deterministic compile errors with a clear span, and the
    // `ContentIdentity: Eq` supertrait already forces `#ident: Eq` through
    // the emitted impl itself.
    Ok(quote! {
        impl ::collomatique_state::partial_order::ContentIdentity for #ident {}
        const _: () = {
            fn assert_content_identity<T: ::collomatique_state::partial_order::ContentIdentity>() {}
            fn assert_eq_impl<T: ::core::cmp::Eq>() {}
            #[allow(dead_code)]
            fn asserts() {
                #(#asserts)*
            }
        };
    })
}
```

(`reject_generics` and `all_named_fields` are the small shared helpers the
snippets above imply: the first errors on any generic parameter, the second
flattens struct fields or all variants' named fields and errors on tuple
shapes.)

### 6.5 Direct integration tests

New file `state/tests/derive_content_ord.rs` (integration test on purpose —
the generated code's absolute `::collomatique_state::` paths only resolve from
outside the crate — mirroring `derive_refs.rs` / `derive_join.rs`). It defines
local toy types and pins every macro behavior:

* a struct whose fields exercise **default dispatch** through `Option`,
  `BTreeSet`, `BTreeMap`, `Vec`, `Table<u64, _>` and `OrderedTable<u64, _>`
  (plain `u64` keys satisfy `Key`/`OrderedKey` and carry the marker): row
  removal → `Less`, reorder → `None`, set shrink → `Less`, option clear →
  `Less`, value renumber → `None`;
* a field of a local type with **no** impls under `#[ord(atom)]`: equal →
  `Equal`, changed → `None`;
* a field under `#[ord(ignore)]` — of a type with no impls at all: two
  values differing only there compare `Some(Equal)` (and `content_eq` is
  true) while being `!=` — the equivalence-class pin;
* a field under `#[ord(with = option_lift_discrete)]` (path form) and another
  under `#[ord(with = |a, b| prefix_pointwise(a, b, option_lift_discrete))]`
  (closure form, the exact shape §7.3 uses for `group_names`);
* a field under `#[ord(total)]` on a numeric-like type, asserting the native
  total order really is used (`3` vs `5` → `Some(Less)` — the one rule where
  two different values are comparable);
* an enum with two named-field variants and a unit variant: same variant →
  product, unit/unit → `Equal`, mixed → `None`;
* the product mixing rules on a two-field struct: one field down + one field
  up → `None`; one down + one equal → `Less`;
* an empty struct → `Equal`;
* `content_eq` / `content_le` / `content_lt` default methods behave;
* **the identity derive**: `#[derive(PartialEq, Eq, ContentOrd,
  ContentIdentity)]` on a struct of default/atom/total fields, then that
  type used as a `BTreeSet` element inside another derived struct — the
  dispatch compiling *is* the test, plus one behavioral assertion through
  the outer set.

Compile-failure cases (generics, tuple struct, duplicate attribute, unknown
attribute argument, `ContentIdentity` over an `ignore`d or `with` field) are
asserted the same way the existing derives do it — if
`derive_refs.rs`/`derive_join.rs` have no trybuild harness, a comment records
the spanned-error behavior instead of adding a new dev-dependency (no
`Cargo.lock` churn in this step).

### 6.6 End-to-end tests: the cascade running on a derived order

New file `state/tests/cascade_on_derived_order.rs` (decision 15). The direct
tests above check what the macro *generates*; this file checks that a derived
order actually drives `apply_cascade` — the derive, the blankets, the
`Fixable` bound and the in-loop check working together, with no
`state-colloscopes` involvement.

The toy: authors and books, deliberately isomorphic to `QuoteData` so the
expected behavior is already understood, but with the order **derived**:

```rust
#[derive(Clone, Debug, PartialEq, Eq, ContentOrd)]
struct LibraryData {
    authors: BTreeSet<u64>,
    /// book id -> author id
    books: BTreeMap<u64, u64>,
}
```

(the two blankets give set inclusion and map inclusion over atomic author
ids — no attribute needed), plus a hand-written `InMemoryData` and `Fixable`
in the test file, following `QuoteData`'s shape in `state/src/test_utils.rs`:
ops `AddAuthor` / `RemoveAuthor` / `SetBook` / `RemoveBook`, the invariant
"every book's author exists", and the honest map "remove the dangling book if
it is present, else `None`". A `mode` field on a wrapper (or a second
implementor) provides one evil variant whose "fix" is `AddAuthor` — the
growing map.

Tests (note commit 5 must land for the panic test to pass; this file is
therefore written in commit 2 with the happy-path tests, and the panic test
is added by commit 5 — each commit stays green):

* `a_cascade_repairs_through_a_derived_order` — one author, two books,
  remove the author: three ops land in canonical order, final state empty;
  the in-loop strictly-below check (once commit 5 lands) silently passes on
  every fix.
* `a_growing_fix_through_a_derived_order_panics` (added in commit 5) — the
  evil variant: `#[should_panic(expected = "did not land strictly below")]`.
* `a_deterministic_walk_never_panics_and_errors_are_atomic` — a small
  op-walk with a hand-rolled deterministic generator (a linear congruential
  step over a `u64` seed, e.g. `x = x.wrapping_mul(6364136223846793005)
  .wrapping_add(1442695040888963407)`, selecting op kind and ids from its
  bits — **no new dependency**, so no `Cargo.lock`/cargoHash churn): a few
  hundred ops through `apply_cascade`; asserts `Ok` ⇒ target last in the
  aggregated op, `Err` ⇒ state `==` before (the toy has honest `PartialEq`);
  a coverage counter asserts at least one landing needed a fix — the
  commit-8 lesson, a walk that never cascades proves nothing.

The full-scale end-to-end remains the domain fixtures and harnesses of
commits 3, 5 and 6.

---

## 7. Commit 3 — adoption in `state-colloscopes/`

### 7.1 Enrollment of the atoms

The `impl_content_ord_atom!` invocations of §3.4: the eleven ids in `ids.rs`,
and the three configuration records next to their definitions (`Limits` in
`settings.rs`, `BalancingOptions` in `balancing.rs`, `ExportConfig` in
`export_config.rs`), each with its decision-13 comment. (The macro emits
`ContentIdentity` alongside — truthful for atoms by construction, and
harmless where unused.)

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
name serves as both derive and trait, exactly like `Join`).

`Data` itself is derived, ignoring the issuer (decision 14) — old code:

```rust
#[derive(Debug)]
pub struct Data {
    id_issuer: std::sync::Mutex<IdIssuer>,
    inner_data: InnerData,
}
```

New code:

```rust
#[derive(Debug, ContentOrd)]
pub struct Data {
    // The document order does not see the issuer: two `Data` with equal
    // inner data are content-equivalent even when their issuers differ —
    // the same quotient the hand-written `PartialEq` below takes.
    #[ord(ignore)]
    id_issuer: std::sync::Mutex<IdIssuer>,
    inner_data: InnerData,
}
```

The full derive list (37 types): `Data`, `InnerData`, `Parameters`,
`Periods`, `Weeks`, `Week`, `Subjects`, `Subject`, `SubjectParameters`,
`SubjectInterrogationParameters`, `SubjectPeriodicity`, `Teachers`,
`Teacher`, `PersonWithContact`, `Students`, `Student`, `Assignments`,
`WeekPatterns`, `WeekPattern`, `Slots`, `Slot`, `Incompats`,
`Incompatibility`, `GroupLists`, `GroupList`, `GroupListParameters`,
`GroupListFilling`, `PrefilledGroup`, `Settings`, `Pairings`, `PairingRule`,
`RulePart`, `SlotPairings`, `SlotPairingRule`, `SlotRulePart`, `Balancing`,
`Colloscope`. **None of them derives `ContentIdentity`** — no entity type
sits at a container matching position today, and the marker stays an
explicit assertion (decision 17).

### 7.3 The eleven field attributes

| file | field | attribute | why |
| --- | --- | --- | --- |
| `lib.rs` (`Data`) | `id_issuer` | `#[ord(ignore)]` | not content; the `Data` quotient (decision 14) |
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

The `groups` field carries the fullest comment — the identity argument plus
the compile-time safety net (decision 17):

```rust
    /// Groups are filled manually with prefilled students
    Prefilled {
        // Position-borne identity: group i binds to group name i, and group
        // numbers are referenced by index from the colloscope's placement
        // maps and interrogation cells — so the document order reads this
        // Vec as a map from indices (prefix + pointwise). The blanket Vec
        // rule (subsequence) does not even apply here: `PrefilledGroup` is
        // deliberately not `ContentIdentity`, so omitting this attribute is
        // a compile error, not a silently wrong order.
        #[ord(with = vec_prefix)]
        groups: Vec<PrefilledGroup>,
    },
```

### 7.4 The two manual impls

`NonEmptyRangeInclusive<T>` (`non_empty_range.rs` — generic, so outside the
macro's reach), with both traits and both comments:

```rust
/// The document order: a range is an atom — its content is the endpoint
/// pair. Reading `[2..=3] ⊆ [1..=4]` as an order would compare the denoted
/// sets, which is exactly the semantic reading D5.1 forbids.
impl<T: Ord + Clone> ContentOrd for NonEmptyRangeInclusive<T> {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        collomatique_state::partial_order::discrete(self, other)
    }
}

/// An atom's content equivalence is `==` by construction, so a range may
/// be matched by equality inside containers.
impl<T: Ord + Clone> ContentIdentity for NonEmptyRangeInclusive<T> {}
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
  ending on the same `InnerData`: `content_eq` is true (the equivalence-class
  pin at the root; for `Data` this coincides with its hand-written `==`).
* `default_is_below_a_populated_document` — build a document through the gate
  **without touching any configuration op** (the `data_with_assignment`
  recipe from `lib.rs`'s test modules qualifies), assert
  `InnerData::default().content_lt(data.get_inner_data())`. A sanity pin,
  deliberately *not* a universal claim (§4.4).
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
  `GroupList::new` with one student removed from one group: `Some(Less)` —
  the semantic pin of the `vec_prefix` rule (the wrong-blanket route is
  already dead at compile time, decision 17).
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
* `config_values_are_atoms` — for each of `Limits`, `BalancingOptions`,
  `ExportConfig`: equal → `Some(Equal)`, two distinct values → `None` —
  **including the default against a modified value** (the decision-13 pin:
  no bottom, `None` fields in the override records are choices, not absent
  content).
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

`EvilQuoteData` ignores its mode — the hand-written miniature of `Data`'s
ignored issuer, and a live example of the self-contained laws (its content
equivalence is coarser than its derived `==`, which compares the mode too):

```rust
/// The document order sees only the inner data: the mode is test-harness
/// configuration, not content. Two [EvilQuoteData] with equal data but
/// different modes are content-equivalent while `!=` — the toy mirror of
/// [`Data`]'s ignored id issuer, and legal because no law relates
/// `ContentOrd` to `PartialEq`.
impl ContentOrd for EvilQuoteData {
    fn content_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.content_cmp(&other.0)
    }
}
```

(with `use crate::partial_order::ContentOrd;` added to the module imports.)

Commit-4 unit tests, beside the toys:

* removing a quote → `Some(Less)`;
* adding a student → `Some(Greater)`;
* re-authoring an existing quote to another author → `None`;
* removing a student while adding a quote → `None`;
* equal → `Some(Equal)`;
* two `EvilQuoteData` with the same inner data and different modes:
  `content_eq` true while `!=` (the equivalence pin).

---

## 9. Commit 5 — the `Fixable` bound and the in-flight assertion

### 9.1 The trait bound and the contract rewording (`state/src/cascade.rs`)

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
/// and its content equivalence backs the no-op-fix panic — the engine never
/// compares with `==`.)
pub trait Fixable: InMemoryData + ContentOrd {
```

with `use crate::partial_order::ContentOrd;` and `use std::cmp::Ordering;`
added to the imports. (After commits 3 and 4, `Data`, `QuoteData` and
`EvilQuoteData` all qualify.)

The engraved contract text in the `fix_invariant` doc comment is reworded —
both for the materialized order and for the retired axiom (decision 13). Old
text:

```rust
    /// States form a partial order with a universal minimal element:
    /// `Default::default()`, the empty document. Every returned op must land
    /// **strictly below** the current state in that order: it removes a row
    /// or entity, clears an edge, or rewrites a value minus an element —
    /// never creates, and never lands equivalent. Return `None`, or a
    /// strictly-decreasing op; an op that applies as a perfect no-op is a
    /// contract violation, and the engine panics on it. The order is
    /// well-founded, so this contract is the cascade's termination proof —
    /// a map that *grows* the state makes the cascade loop forever (step 6.5
    /// adds a `PartialOrd`-based in-flight check for exactly that).
```

New text:

```rust
    /// States form a **well-founded** partial order — the document order,
    /// materialized by the [ContentOrd] supertrait bound (design doc §8,
    /// step 6.5); the empty document `Default::default()` is a minimal
    /// element. Every returned op must land **strictly below** the current
    /// state in that order: it removes a row or entity, clears an edge, or
    /// rewrites a value minus an element — never creates, and never lands
    /// equivalent. Return `None`, or a strictly-decreasing op. Because the
    /// order is well-founded, this contract is the cascade's termination
    /// proof, and [apply_cascade] asserts it after every fix: a fix landing
    /// equivalent, above, or incomparable panics instead of hanging.
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

New code — the equality check becomes a three-way check on the document order
(no `==` anywhere: `Some(Equal)` means *equivalent*, which is what the no-op
panic is really about); only fix ops are held to it, exactly as before (the
`before` snapshot is still taken only when `!is_target`, so a no-op *target*
remains legitimate, G.2):

```rust
            Ok(backward) => {
                if let Some(before) = before {
                    // The in-flight monotonicity check (step 6.5): a fix
                    // must land strictly below the pre-fix state in the
                    // document order. Equivalent = the old no-op panic;
                    // above or incomparable = a growing/sideways map, which
                    // without this check would hang the cascade instead.
                    match (*data).content_cmp(&before) {
                        Some(Ordering::Less) => {}
                        Some(Ordering::Equal) => panic!(
                            "resolution map violated the strict-monotonicity \
                             contract: fix {front:?} landed equivalent to the \
                             pre-fix state (a perfect no-op — return None when \
                             no material is present)"
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

Numbered 10 and 11, following the file's comment style. Also update test 6's
expected string to `"landed equivalent"` (its message changed in §9.2, and
each panic test should pin its own message rather than the shared
`strict-monotonicity` prefix). This commit also adds the
`a_growing_fix_through_a_derived_order_panics` test to
`state/tests/cascade_on_derived_order.rs` (§6.6).

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

## 10. Commit 6 — the contract fuzz property

New file `state-colloscopes/tests/property_content_ord.rs`, the fourth
property harness, built on the same testgen machinery as
`tests/property_cascade.rs` (same imports and `RunConfig` shape, same
`for_each_seed` / `bootstrap` / `generator::gen_op` / `OpLog` plumbing — copy
that file's `next_op` and `maybe_snapshot` helpers verbatim, and keep the
snapshot bookkeeping on the success path exactly as there; moving it perturbs
the RNG trajectory). Additional imports:
`use collomatique_state::ContentOrd;`.

One property. (Plan v2 had a second — "`Default::default()` ≤ every reachable
state" — retired with the universal-minimum axiom, decision 13: a walk that
touches a configuration op reaches states incomparable to the default, and
that is now correct behavior. The sanity half survives as the §7.5 unit pin.
The design doc §8 paragraph still prescribes the retired property; the
close-out amends it.)

Configuration, matching the house ruling for step-6-family harnesses (one
hardcoded const, no environment variables, no `#[ignore]` tiers):

```rust
const CONFIG: RunConfig = RunConfig {
    seeds: 50,
    ops_per_run: 500,
    invalid_fraction: 0.15,
};
```

### 10.1 The property: every map answer is `None` or lands strictly below

A plain gated walk (`data.annotate` + `data.apply`, as in `property_ops.rs` —
no cascade), where the interesting event is a **rejection over broken
invariants**: at that point `data` is unchanged and valid (the gate rolled
back), which is precisely the state the cascade would consult the map on. For
*every* invariant in the reported set — not only the canonical first pick the
engine would take — ask the map, and when it answers with a fix, land that fix
through the force door on a clone and compare:

```rust
/// Design doc §8 step 6.5: over generated broken states, every
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
                            assert!(
                                fixed.get_inner_data().content_lt(data.get_inner_data()),
                                "fix {fix:?} for {invariant:?} must land \
                                 strictly below the pre-fix state (content_cmp \
                                 = {:?})",
                                fixed.get_inner_data().content_cmp(data.get_inner_data()),
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
`property_content_ord.rs`. This step touches no storage bytes, no op
vocabulary, no `ops/` behavior and no gtk4 code, so no contract-script or GUI
surface changed; the standing user-run checks stay at the user's discretion.
No `Cargo.toml`/`Cargo.lock` change is expected (no new dependencies — no Nix
`cargoHash` refresh).

**Close-out (after the user's sign-off that the step is done):**

* update `docs/plans/invariant_cascade_design.md`: mark the §8 step-6.5
  paragraph completed with commit anchors — including amending its retired
  prescriptions (the fuzz-(a) companion, the "equivalence classes modulo the
  id issuer" phrasing, and every "universal minimal element" wording, notably
  Appendix H's policy rule D5.1's opening line) — and record the delivered
  state as **Appendix I**: the `ContentOrd` trait with self-contained laws
  and why it is not `PartialOrd`; the `ContentIdentity` marker (positional
  coincidence of `==` and content equivalence, enforced at container
  matching positions; opt-in, derivable but never automatic); the
  intrinsic-order methodology (the order pre-exists the map; fix behavior is
  audit material, never definition material); the §2/§3 order definition
  with the identity criterion for sequences and its instances (★ `WeekBlock`
  atom, prefix-ordered groups/names with the external-index argument); the
  configuration records as atoms and the retirement of the universal-minimum
  axiom (★ superseding the step-6 ruling); the derive with its four
  attributes (`atom`/`ignore`/`total`/`with`); the engine assertion; the two
  new evil modes; and the contract fuzz with its measured numbers;
* retire this plan: delete `docs/plans/plan_step_6_5.md` in the close-out
  commit and pin the versions in the topic memory — v1 at
  `git show ec0dd2a2:…`, v2 at `git show 85f44889:…`, v3 at
  `git show 74a80456:…`, the final version at
  `git show <close-out parent>:docs/plans/plan_step_6_5.md`;
* update the topic memory: step 6.5 closed, next = step 7 (the `ops/`
  remaster).
