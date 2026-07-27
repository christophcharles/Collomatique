# Step 6 session plan — the cascade: a generic engine plus the colloscope resolution map

Opened July 26 2026, immediately after the step-5 close-out (`3a105246`); revised July 27
2026 after the user's review (the `SetRow` op swap is adopted rather than deferred, the
invariant vocabulary gains one payload, the cascade surface works in annotated ops, the
round fuse is gone — a contract-violating map hangs until step 6.5 — failure restores a
snapshot instead of replaying inverses, and a no-op fix is a panic). The
architectural background is `docs/plans/invariant_cascade_design.md` (referred to below as "the
design doc"), in particular §5 (the cascade as settled July 15 2026) and Appendix G (the step-5
delivered state this step builds on). Where this plan deviates from the §5 pseudocode, the
deviation is stated explicitly and justified — the design doc will record the deltas in a new
appendix at close-out.

## 1. Context and goal

Since step 5, every elementary operation goes through one gate,
`state-colloscopes/src/lib.rs` (`impl InMemoryData for Data`, `fn apply`): snapshot →
`force_apply` → `broken_invariants` → rollback. A failed op leaves the state bit-identical and
reports precisely *why* it failed. In particular, when an op would leave dangling references or
broken convergence facts, the caller receives the exact set of broken invariants
(`BTreeSet<FixableInvariant>`), in a canonical order.

Step 6 builds the machinery that *consumes* those sets: the **cascade**. Instead of rejecting
an op that breaks invariants, the cascade asks a **resolution map** for one repair step per
broken invariant (invariant → the elementary op that repairs it, one step at a time), applies
repairs first (depth-first, discovering further breakage through further failures), then
retries the original op. Every
committed intermediate state is valid, because every op — target and fix alike — goes through
the same gate. The result of a successful cascade is the exact list of `(forward, backward)`
op pairs that landed, ready to be stored as a single history slot.

Vocabulary, used consistently below: the **target** is the op the caller passed to the
cascade — the thing the user actually asked for (what earlier discussions called "the
original op"; it enters the cascade already annotated, see D6). A **fix** is an op the
resolution map emitted to repair one broken invariant. The target sits at the bottom of the
retry stack for its whole life; fixes are pushed above it and run first.

**Step 6 adds no consumers.** The `ops/` crate keeps its hand-written pre-cleaning
(`get_next_cleaning_op`) and `Warning` machinery untouched; migrating it onto the cascade (and
building the dry-run preview UX) is step 7. This step delivers the engine, the map, and tests
— nothing in production calls the cascade yet.

Deliverables, as commits (commit 0 is this plan itself plus the step-6.5 paragraph added to
the design doc's §8):

- **Commit 1** — restructure the error surface: a generic two-tier
  `ApplyError<InvalidOp, Invariant>` defined in `state/`, adopted by `state-colloscopes`,
  with the mechanical consumer sweep (ops/, gtk4, storage, testgen, tests).
- **Commit 2** — the generic cascade engine in `state/src/cascade.rs`: the `Fixable` trait
  and the `apply_cascade` function.
- **Commit 3** — a small dedicated test implementor (`QuoteData`: students + quotes) in
  `state/src/test_utils.rs`, and the engine's unit tests against it.
- **Commit 4** — the elementary op swap `AssignmentOp::Assign` →
  `AssignmentOp::SetRow` (row-valued, the `SetInterrogation`/`SetGroupList` pattern), with
  its consumer sweep. Adopted at review (it was a deferred optimization in the first
  draft): it makes every assignment fix in the map a single minimal op.
- **Commit 5** — enrich `Convergence::InterrogationGroupOutOfBounds` with the offending
  group number (the one information-poor variant found by the review's error survey), so
  its fix can trim minimally instead of clearing the whole cell.
- **Commit 5.99** — split the balancing elementary op: `BalancingOp::Update(Balancing)`
  (which ships a whole `Table` value through the op surface) becomes
  `SetGlobal(BalancingOptions)` + `SetSubject(SubjectId, Option<BalancingOptions>)`.
  Adopted during the commit-6 review (July 27 2026); numbered 5.99 because it is a
  prerequisite of the map's two balancing arms, not a part of commit 5.
- **Commit 6** — the colloscope resolution map: `impl Fixable for Data` in
  `state-colloscopes/src/resolution.rs`, total over `FixableInvariant`.
- **Commit 7** — colloscope integration tests (`state-colloscopes/tests/cascade.rs`):
  deep-cascade fixtures, self-caused-rejection fixtures, the confluence pin test, undo
  round-trips.
- **Commit 8** — the cascade property test (`state-colloscopes/tests/property_cascade.rs`):
  random valid walks driven through `apply_cascade`; no panic, `Ok` ⇒ clean, `Err` ⇒
  bit-identical state.

Every commit compiles and passes the suite on its own. The on-disk format is untouched
(nothing in this step goes near storage, and elementary ops are never persisted), so no
byte-stability concern arises.

## 2. The decision ledger

These decisions were settled in the planning sessions (July 26–27 2026) and are not to be
reopened during implementation.

**D1 — The fixable/unfixable split is lifted to the trait level.** The generic `state` crate
today treats `InMemoryData::Error` as an opaque `std::error::Error`. Every attempt to write a
generic cascade against an opaque error needs a classification hook, and every hook shape is
a workaround. The root fix: the trait itself distinguishes the two kinds of failure. The
associated type `Error` is replaced by two associated types and a shared generic enum:

- `InvalidOp` — "we cannot make sense of this op against this state". This covers both the
  step-5 `Precheck` tier (no-clobber, dangling op target, bad anchor/position) *and* the
  `Logic` tier (the op would land logically impossible rows — and no op is valid against a
  state we could not make sense of). Not resolvable; the cascade never tries.
- `BrokenInvariants(BTreeSet<Invariant>)` — the op is well-formed, but the state does not yet
  satisfy what the op needs; the set is what the cascade resolves.

The naming stays in the established register (`FixableInvariant`, `broken_invariants`, "the
map is total over `FixableInvariant`"). The word "precondition" was considered for the second
arm and rejected: design-doc §4 already uses "the precondition carve-out" for the *other* arm
(the precheck family), and flipping the word's meaning would trip every future reader.

**D2 — The resolution map is a trait method returning one repair step, or `None`.**
`fn fix_invariant(&self, invariant: &Self::Invariant) -> Option<Self::AnnotatedOperation>`.
Totality is enforced by an exhaustive match with no wildcard arm. The engine — not the map —
picks *one* invariant per round (`BTreeSet::first()`, the canonical minimum), and the map
returns *one op* per call: one repair step, computed from the live current state. The
one-step protocol is general — an invariant whose repair takes N ops is fixed over N rounds
(the engine re-runs the failing op, the same invariant is re-picked, and the arm sees the
shrunken state and removes the next piece) — though after the commit-4 `SetRow` swap no
colloscope arm actually needs more than one round per invariant instance. `None` means "the
current state holds nothing that causes this invariant" — it can then only come from the
failing op's own payload, and the engine convicts accordingly (D4).

The return type is the **annotated** op, deliberately (settled at review): the map receives
only `&self`, so it physically cannot reach the id issuer — a fix cannot carry a fresh id,
which is exactly the D5 contract's "never creates" leaning expressed in the signature. It
also removes all `NewInfo` bookkeeping from the engine: fixes arrive annotated, and the
target arrives annotated from the caller (D6). For the deletive ops the map emits, the
annotated form is payload-identical to the plain form (e.g. `AnnotatedAssignmentOp` /
`ops.rs:956-960` annotates by identity), so the arms simply construct `AnnotatedOp`
variants directly.

The arm doctrine that makes both routes work: an arm checks **presence of the material it
would remove** (is the element in the set, is the student in the row, is the group in the
cell) — never the truth of the invariant's *predicate*, which may depend on the failing
op's payload and is unknowable from the state. Example: the (commit-5 enriched)
`InterrogationGroupOutOfBounds(slot, week, group)` arm asks "is `group` in the current
cell" — never "is `group` out of bounds against the current group count" (a group-list
shrink legitimately needs the trim even though the group is in-bounds against the *current*
count) — and returns `None` only when the group is absent from the cell.

**D3 — Rejected alternative shapes (settled in the planning session; do not reopen).**

- *`Vec<Op>` fixes* (resolve the whole invariant in one call) — **rejected as a time bomb**:
  it is easy to produce one op that moves toward resolution, and easy to get a multi-op list
  subtly wrong when a middle op does not produce the intermediate state the author imagined.
  `Vec` was only an optimization (fewer failed retries); the sanctioned alternative was to
  **replace** `AssignmentOp::Assign` with a row-valued
  `AssignmentOp::SetRow(period, subject, BTreeSet<StudentId>)` (the `SetInterrogation` /
  `SetGroupList` pattern) — an orthogonality-preserving swap, not an addition. At review
  (July 27) that swap was **adopted** as commit 4 rather than deferred: it is the right op
  shape on its own merits, and it makes every assignment fix a single minimal op.
- *"The fix must be buildable from the error alone (no state reads)"* — rejected: for the
  minus-rebuild sites it forces either fix material into invariant payloads (the checker
  would be computing repairs — wrong layering; and whole values in payloads kill the
  `Copy + Ord` vocabulary) or new single-edge elementary ops next to the whole-value
  `Update`s — which violates the standing principle that **elementary ops are orthogonal:
  no two ops express the same state change** (`GlobalUpdate`, the external-data door, is
  the accepted exception). Reading the row you are about to clear is not a sin; inventing a
  second op to avoid the read would be.
- *Single-op rounds with a state-progress guard* (an earlier draft) — subsumed: the
  single-op part is adopted (D2), the state-progress ledger is not; D4's detectors replace
  it.

**D4 — Conviction rules: the target op is fallible at every retry; map-emitted ops are
infallible-or-panic.** Ops whose own payload breaks an invariant make the *target* fail
mid-cascade through no fault of the map. Two canonical traces:

1. `SlotOp::Update(S, start 23:00)` on a subject with a 2-hour duration fails with
   `SlotOverflowsDay(S)`; the only deletive fix is `SlotOp::Remove(S)`; it applies (the
   pre-op slot has its old, valid start); the retried update then hits the `InvalidSlotId`
   precheck — the resolution consumed the op's own target. Real fixes land before the
   conviction, so the engine must genuinely restore.
2. `ColloscopeOp::SetInterrogation(slot, week, {5})` against a 4-group bound fails with
   the (commit-5 enriched) `InterrogationGroupOutOfBounds(slot, week, 5)`; the arm asks
   whether group 5 is in the *current* (pre-op) cell — it is not — and returns `None`:
   nothing in the state causes the invariant, the op does. Round-one rejection, no fix
   ever applied. (Before the enrichment this trace needed a wasteful clear-then-`None`
   round trip; the group payload is what makes the arm precise.)

Such ops are bad *input*, not map bugs. They are programming-error-adjacent — gtk4 never
offers them (the UI pre-filters, e.g. the assign control is absent when the student is not
in the period) — but the same op surface is driven by Python/RPC scripting, where op
sequences are user-authored programs, and by UI code racing a stale view; the public op
surface must stay panic-free on data-dependent input. So they surface as `Err`. The engine
rules:

- **`None` from the map** convicts the *failing op*: failing op is the target → restore
  the snapshot, return `Err`; failing op is a fix → panic (the map declared unfixable an
  invariant that a fix op of its own produced). This is the sole `Err` route for
  self-caused targets — both traces above end here.
- **`InvalidOp` from the target** (any retry) → restore + `Err`; from a fix op → panic.
- **A fix that applies as a perfect no-op → panic, unconditionally** (settled at review).
  The D5 contract is: return `None`, or an op that lands *strictly below* —
  `Some(equivalent)` is a map bug, full stop. A conforming arm cannot produce a no-op fix:
  it checks presence of the material, and if the material is present its op removes it (a
  real change); the no-material case is exactly what `None` is for. So a no-op fix never
  encodes bad user input — only a broken map — and map bugs panic, to be caught fast (by
  the commit-8 fuzz, ideally never in production). A no-op **target**, by contrast, is a
  legitimate success (G.2's widened acceptance). Detection: state equality around each fix
  apply (hence the `PartialEq` bound on `Fixable`).
- When the target is convicted mid-cascade, return its **last `BrokenInvariants` error**,
  not the immediate error — trace 1 must report "would break SlotOverflowsDay", not a
  baffling "invalid slot id" for a slot the user can see.
- **There is no round fuse** (settled at review; the first draft had a 10 000-round cap).
  No meaningful bound exists — real cascades are bounded by the document, and any constant
  loose enough to be safe detects nothing in useful time. Termination rests entirely on
  the D5 monotonicity contract: `None` and the no-op panic catch every removal-shaped
  violation in-flight, and a map that keeps *growing* the state makes the cascade **loop
  forever** — accepted for now. Step 6.5 (recorded in the design doc §8) closes this by
  requiring `PartialOrd` on `Fixable` implementors and asserting strictly-below after
  every fix, in the loop. The §5 (op, picked-invariant) repetition ledger is likewise
  **retired**: under one-step fixes, re-picking the same pair is the legitimate path, not
  a bug signature.
- There is no `Logic` special case anywhere: `Logic` is inside `InvalidOp` (D1) and follows
  the positional rule like everything else. Through the cascade it is unreachable anyway
  (`ops` never issues `GlobalUpdate`); the non-cascade `apply` path keeps returning it as an
  ordinary error, which decode relies on.
- Since no rule attributes fault to a fix's *requester* anymore (the first draft's no-op
  rule did), the engine needs only "is the failing op the target", which is structural:
  the target is the front of the stack exactly when it is alone (`stack.len() == 1`). No
  origin tags.

**D5 — Resolution policy for the colloscope map.** Four rules, applied uniformly in §8:

1. **Fixes are strictly monotonically decreasing.** States form a partial order with a
   universal minimal element — **`Default::default()`, the empty document** (`Data`
   implements `Default`, `state-colloscopes/src/lib.rs:421`); every fix op must land
   strictly *below* the current state — remove a row/entity, clear an optional edge, or
   rewrite a whole value *minus* the offending element. Nothing is ever invented (no
   substitute teacher, no widened week pattern), and nothing lands *equivalent*: the map
   returns `None` or a strictly-decreasing op, never a no-op (D4). Because the order is
   well-founded, strict monotonicity **is** the termination proof of the cascade. This
   contract is engraved: it goes verbatim into the `Fixable` trait's doc-comment (the map
   implementor's contract) and into the `apply_cascade` module docs; the engine's `None`
   conviction and no-op panic (D4) are its cheap in-flight detectors, and step 6.5 will
   add the order itself (`PartialOrd` + a strictly-below assertion per fix).
2. **Where a targeted single-edge op exists, use it**; where none exists, rewrite the whole
   value through the domain's `Update` op with the offending element removed, reading the
   current value from the pre-op state.
3. **Where the referencing entity cannot survive the loss, remove the entity**: a slot cannot
   exist without its teacher or subject; a pairing rule cannot exist without both parts; an
   incompatibility cannot exist without its (mandatory) subject.
4. **Aim to match the legacy cleaning semantics** (`ops/src/*.rs get_next_cleaning_op`)
   where they exist — but this is an aspiration, not a gate (softened at review). An exact
   match may not always be achievable, and where the map diverges the divergence is
   recorded at close-out: it more likely captures an edge case the hand-written cleaning
   forgot than a regression. Verified against the legacy code in the planning session:
   week-pattern deletion *deletes* referencing slots and incompats (it does not clear
   their optional pattern field to `None` — that would silently widen the slot to every
   week); a group-list shrink removes the out-of-bounds *student placements*; a
   week-exclusion update clears *whole interrogation cells*.

The review also asked whether any invariant payload is too poor to fix precisely — the
error cases are not fixed in stone, and the checker often holds the precise culprit in
hand at emission time. Survey verdict (all 16 `Convergence` variants, every `DanglingFk`
site, checked against their fix arms): exactly **one** variant is information-poor —
`InterrogationGroupOutOfBounds(SlotId, WeekId)` drops the offending group number the
checker just computed. Commit 5 enriches it (one instance per offending group), after
which every arm can fix minimally from payload + live state. Everything else already
carries enough.

**D6 — Names and the annotated-op surface.** Generic error:
`ApplyError<InvalidOp, Invariant>` with variants `InvalidOp(InvalidOp)` and
`BrokenInvariants(BTreeSet<Invariant>)`, defined in `state/src/traits.rs`. Subtrait:
`Fixable: InMemoryData + PartialEq` with `fn fix_invariant`, defined in
`state/src/cascade.rs` next to `pub fn apply_cascade`. The cascade works entirely in
**annotated** ops (settled at review): the caller annotates the target itself
(`Manager`-style: annotate → cascade), keeps the `NewInfo`, and passes the
`AnnotatedOperation`; `apply_cascade` returns a bare
`AggregatedOp<T::AnnotatedOperation>` on success — there is no `CascadeSuccess` struct and
no `NewInfo` threading anywhere in the engine. The colloscope crate keeps exporting a type
named `Error` (now an alias) so most consumer code keeps reading naturally.

## 3. Commit 1 — the generic error surface (`ApplyError`)

### 3.1 The trait, before and after

Today (`state/src/traits.rs:22-67`):

```rust
pub trait InMemoryData: Clone + Send + Sync + std::fmt::Debug {
    type OriginalOperation: Operation;
    type AnnotatedOperation: Operation;
    type NewInfo;

    /// Error type for when [Self::apply] fails.
    type Error: std::error::Error + Send + Sync + Clone;

    fn annotate(&self, op: Self::OriginalOperation) -> (Self::AnnotatedOperation, Self::NewInfo);

    fn apply(
        &mut self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::Error>;
}
```

After this commit:

```rust
/// Error surface of the apply/check/rollback gate, shared by every
/// [InMemoryData] implementor.
///
/// Two tiers. [ApplyError::InvalidOp] means the op cannot be made sense of
/// against the current state (bad input: no-clobber, dangling op target, bad
/// anchor — or an op payload that would land logically impossible data). It is
/// never resolvable. [ApplyError::BrokenInvariants] means the op is
/// well-formed but the state does not satisfy what it needs: the payload is
/// the exact set of broken invariants, in the canonical `Ord`. At step 6 this
/// is what the cascade resolves; outside the cascade it is simply an error.
///
/// Either way the failed `apply` left the data strictly unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ApplyError<InvalidOp, Invariant>
where
    InvalidOp: std::error::Error,
    Invariant: std::fmt::Debug + std::fmt::Display + Ord,
{
    /// Bad op input; never resolvable.
    #[error(transparent)]
    InvalidOp(InvalidOp),
    /// The op needs these invariants fixed first; resolvable by the cascade.
    #[error("the operation would break data invariants: {}", format_error_set(.0))]
    BrokenInvariants(std::collections::BTreeSet<Invariant>),
}

/// Itemises a set of errors through each entry's own [std::fmt::Display] so a
/// UI dialog can surface a meaningful message without learning the vocabulary.
/// (Moved up from `state-colloscopes`, which keeps its own private copy for
/// its remaining local enums.)
fn format_error_set<T: std::fmt::Display>(set: &std::collections::BTreeSet<T>) -> String {
    set.iter().map(T::to_string).collect::<Vec<_>>().join("; ")
}

pub trait InMemoryData: Clone + Send + Sync + std::fmt::Debug {
    type OriginalOperation: Operation;
    type AnnotatedOperation: Operation;
    type NewInfo;

    /// The unresolvable tier of [ApplyError]: bad op input (including op
    /// payloads that would land logically impossible data).
    type InvalidOp: std::error::Error + Send + Sync + Clone;

    /// The resolvable tier of [ApplyError]: one broken invariant. `Ord` is the
    /// canonical order the cascade's deterministic pick relies on.
    type Invariant: Send + Sync + Clone + Ord + std::fmt::Debug + std::fmt::Display;

    fn annotate(&self, op: Self::OriginalOperation) -> (Self::AnnotatedOperation, Self::NewInfo);

    fn apply(
        &mut self,
        op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, ApplyError<Self::InvalidOp, Self::Invariant>>;
}
```

Notes for the implementer:

- The derives on `ApplyError` (`PartialEq, Eq`) generate correctly-bounded impls; the
  colloscope instantiation satisfies them (all its payloads are `PartialEq + Eq`).
- Every `<... as InMemoryData>::Error` mention in `traits.rs` (the `Manager::apply` return
  type at line 105-126, `update_internal_state_with_aggregated` at 237-284) becomes
  `ApplyError<<... as InMemoryData>::InvalidOp, <... as InMemoryData>::Invariant>`. The
  grep confirms `traits.rs` is the only file in the workspace naming the associated `Error`
  at the type level; everything else matches concrete variants (swept in §3.3).
- `state/src/lib.rs:30` re-exports grow `ApplyError`:
  `pub use traits::{ApplyError, Description, InMemoryData, Operation};`

### 3.2 The colloscope adoption

Today (`state-colloscopes/src/lib.rs:247-264`):

```rust
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error(transparent)]
    Precheck(#[from] PrecheckError),
    #[error("the operation would leave logically impossible data: {}", format_error_set(.0))]
    Logic(BTreeSet<LogicError>),
    #[error("the operation would break data invariants: {}", format_error_set(.0))]
    Invariants(BTreeSet<FixableInvariant>),
}
```

After:

```rust
/// The unresolvable tier of the gate's error surface: bad op input. Both the
/// carve-out prechecks (design doc §4) and the logic tier live here — an op
/// whose payload would land logically impossible rows is an invalid op
/// (`Logic` stays reachable only from data built outside this crate:
/// `GlobalUpdate` payloads, decode).
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum InvalidOp {
    #[error(transparent)]
    Precheck(#[from] PrecheckError),
    #[error("the operation would leave logically impossible data: {}", format_error_set(.0))]
    Logic(BTreeSet<LogicError>),
}

/// Error surface of the apply/check/rollback gate ([Data::apply]): the shared
/// two-tier [ApplyError] instantiated with this crate's vocabulary.
pub type Error = collomatique_state::ApplyError<InvalidOp, FixableInvariant>;

/// Lets the gate keep writing `self.force_apply(op)?`.
impl From<PrecheckError> for Error {
    fn from(e: PrecheckError) -> Self {
        collomatique_state::ApplyError::InvalidOp(InvalidOp::Precheck(e))
    }
}
```

The gate body (`lib.rs:311-344`) changes only its constructors: `Err(Error::Logic(logic))` →
`Err(ApplyError::InvalidOp(InvalidOp::Logic(logic)))`, `Err(Error::Invariants(fixable))` →
`Err(ApplyError::BrokenInvariants(fixable))`. The `impl InMemoryData for Data` block swaps
`type Error = Error;` for `type InvalidOp = InvalidOp; type Invariant = FixableInvariant;`.
`FixableInvariant` already satisfies every bound (it is `Copy + Clone + Ord + Debug` and
thiserror gives `Display`). `FromInnerDataError` and the `format_error_set` helper stay in
the crate untouched (decode does not go through `ApplyError`).

`FakeData` (`state/src/test_utils.rs:47-78`) has no invariants, which the new shape can now
state precisely instead of merely commenting:

```rust
impl InMemoryData for FakeData {
    type OriginalOperation = FakeOp;
    type AnnotatedOperation = FakeOp;
    type NewInfo = ();
    // FakeData has no invariants: the resolvable tier is uninhabited, so
    // `ApplyError::BrokenInvariants` is unrepresentable for it.
    type InvalidOp = FakeError;
    type Invariant = std::convert::Infallible;

    fn apply(&mut self, op: &FakeOp) -> Result<FakeOp, ApplyError<FakeError, std::convert::Infallible>> {
        // ... same bodies, errors wrapped: Err(ApplyError::InvalidOp(FakeError::...))
    }
}
```

(`Infallible` implements `Error`, `Display`, `Ord`, `Clone`, `Send`, `Sync` — every bound —
and makes the second arm a compile-time impossibility. Tests comparing `FakeError` values
gain the `ApplyError::InvalidOp` wrapper.)

### 3.3 The consumer sweep

Everything that pattern-matches the old three-tier enum. From the workspace grep (the counts
are match-arm occurrences, all mechanical):

- **`ops/src/`** — `students.rs` (5), `teachers.rs` (5), `week_patterns.rs` (5), `slots.rs`
  (4), `slot_pairings.rs` (4), `pairings.rs` (4), `incompatibilities.rs` (4),
  `colloscope.rs` (4), `assignments.rs` (2), `subjects.rs` (1), `group_lists.rs` (1). The
  translation doctrine of step 5 (design doc G.3) is untouched; only the patterns re-nest.
  Representative site (`ops/src/students.rs:441-455`), before:

  ```rust
  Error::Precheck(PrecheckError::Student(
      StudentPrecheckError::InvalidStudentId(id),
  )) => UpdateStudentError::InvalidStudentId(id),
  Error::Invariants(set) => {
      for inv in &set {
          if let FixableInvariant::DanglingFk(Reference::Period {
              target,
              site: PeriodRefSite::StudentExcludedPeriods(_),
          }) = inv
          { return UpdateStudentError::InvalidPeriodId(*target); }
      }
      ...
  ```

  after:

  ```rust
  Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Student(
      StudentPrecheckError::InvalidStudentId(id),
  ))) => UpdateStudentError::InvalidStudentId(id),
  Error::BrokenInvariants(set) => {
      // body unchanged
  ```

  (`Error` being an alias, `Error::InvalidOp` resolves to the `ApplyError` variant; imports
  gain `InvalidOp`.) Catch-all panic arms that today name `Error::Logic` fold into the
  `InvalidOp` arm or stay wildcard panics — preserve each site's existing precedence
  ordering exactly. (`assignments.rs`'s sites get touched again by commit 4's `SetRow`
  swap; do the mechanical re-nesting here anyway — each commit stays self-contained.)
- **`state-colloscopes/tests/`** — `property_apply_gate.rs` (5), `found_bugs.rs` (5),
  `week_ops.rs` (3), `period_consistency_in_subjects.rs` (1): exact-set asserts re-spell
  `Error::Invariants(set)` as `Error::BrokenInvariants(set)` and precheck pins gain the
  `InvalidOp::Precheck` layer. `property_apply_gate.rs`'s `ForceLogic` arm now asserts
  `Error::InvalidOp(InvalidOp::Logic(_))`.
- **`state-colloscopes/src/ops.rs`** (1) — a doc-comment mention; re-word.
- **`testgen-colloscopes/src/generator.rs`** (3) — corruption-probe expectations, same
  re-spelling.
- **`storage/src/decode.rs`** (1) and **`gtk4/src/loading/file_loader.rs`** (1) — single
  mentions (comment / error display path); gtk4's `e.to_string()` dialogs keep working
  because thiserror `Display` on `ApplyError` itemizes exactly as before.

Done-check for the commit: `cargo test --workspace` green; a workspace grep for
`Error::Precheck`, `Error::Logic(`, `Error::Invariants(` returns nothing.

## 4. Commit 2 — the generic engine (`state/src/cascade.rs`)

New file, plus `pub mod cascade;` in `state/src/lib.rs` and re-exports
`pub use cascade::{apply_cascade, Fixable};`. (Plain `cascade.rs`, no
directory — house rule.)

```rust
//! The cascade: apply an op; when it fails on broken invariants, fix the
//! smallest one and retry — depth-first, discovering breakage through failure
//! (design doc §5). Every landed op passed the full apply/check/rollback gate,
//! so no invalid state ever escapes a single elementary `apply`.

use std::collections::BTreeSet;

use crate::history::{AggregatedOp, ReversibleOp};
use crate::traits::{ApplyError, InMemoryData};

/// Implemented by data whose broken invariants can be repaired by ops: the
/// resolution map. (`PartialEq` backs the engine's no-op-fix panic.)
pub trait Fixable: InMemoryData + PartialEq {
    /// One repair step for `invariant` on the current state, or `None` when
    /// the current state holds nothing that causes it (the invariant can then
    /// only come from the failing op's own payload — [apply_cascade] rejects
    /// the target op, or panics if a fix op produced the invariant).
    ///
    /// # Contract (the engraved cascade contract — design doc §5)
    ///
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
    ///
    /// The return type is the *annotated* op on purpose: with only `&self`,
    /// an implementation cannot reach the id issuer, so a fix physically
    /// cannot carry a fresh id — the signature leans the same way the
    /// contract does.
    ///
    /// Total: every representable invariant has an arm; no wildcard match.
    /// One step per call: the engine retries the failing op and asks again,
    /// so an invariant needing N removals is repaired over N rounds, each arm
    /// call seeing the then-current state. An arm decides by checking the
    /// **presence of the material it would remove**, never by re-evaluating
    /// the invariant's predicate (which may depend on the failing op's
    /// payload).
    fn fix_invariant(&self, invariant: &Self::Invariant) -> Option<Self::AnnotatedOperation>;
}
```

The engine. The queue is a stack (`Vec`, top = next to try) so a fix runs before the op
that needed it retries; the target sits at the bottom for its whole life, so *the front is
the target exactly when it is alone* (`stack.len() == 1`) — no origin tags needed (D4).
The caller passes the target already annotated and keeps its `NewInfo` (D6); on success
the return value is the history-ready `AggregatedOp` (the target is always its last
entry, `.rev()` is the compound undo). Failure restores an entry snapshot — no backward
replay, no unwind panic path.

```rust
pub fn apply_cascade<T: Fixable>(
    data: &mut T,
    target: T::AnnotatedOperation,
) -> Result<AggregatedOp<T::AnnotatedOperation>, ApplyError<T::InvalidOp, T::Invariant>> {
    // Failure = "*data = snapshot": bit-identical restore, id issuer included.
    let snapshot = data.clone();
    let mut stack: Vec<T::AnnotatedOperation> = vec![target];
    let mut applied: Vec<ReversibleOp<T::AnnotatedOperation>> = Vec::new();
    // The target's most recent BrokenInvariants set: the informative error
    // when the target is convicted mid-cascade (D4 — the SlotOverflowsDay trace).
    let mut last_target_break: Option<BTreeSet<T::Invariant>> = None;

    loop {
        let Some(front) = stack.last().cloned() else {
            return Ok(AggregatedOp::new(applied));
        };
        let is_target = stack.len() == 1;

        // Snapshot for the no-op-fix panic; only fix ops are held to it (a
        // no-op *target* is a legitimate perfect no-op, G.2).
        let before = (!is_target).then(|| data.clone());

        match data.apply(&front) {
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
                applied.push(ReversibleOp { forward: front, backward });
            }
            Err(ApplyError::BrokenInvariants(set)) => {
                let pick = set
                    .first()
                    .expect("a BrokenInvariants error carries a non-empty set")
                    .clone();
                if is_target {
                    last_target_break = Some(set);
                }
                match data.fix_invariant(&pick) {
                    Some(fix) => stack.push(fix),
                    // None: nothing in the state causes `pick` — the failing
                    // op's own payload does.
                    None if is_target => {
                        *data = snapshot;
                        return Err(ApplyError::BrokenInvariants(
                            last_target_break.expect("just stored for the target"),
                        ));
                    }
                    None => panic!(
                        "resolution map declared {pick:?} unfixable, yet a cascade \
                         fix op produced it: {front:?}"
                    ),
                }
            }
            Err(ApplyError::InvalidOp(e)) => {
                if is_target {
                    *data = snapshot;
                    // Mid-cascade, a fix consumed the target's own target; the
                    // informative error is what the target kept breaking.
                    return Err(match last_target_break {
                        Some(set) => ApplyError::BrokenInvariants(set),
                        None => ApplyError::InvalidOp(e),
                    });
                }
                panic!("cascade fix op {front:?} was rejected as invalid: {e}");
            }
        }
    }
}
```

Points worth restating in the module docs (and holding during review):

- **The whole queue is annotated ops, annotated exactly once.** The target arrives
  annotated from the caller (who keeps its `NewInfo`); fixes arrive annotated from the map
  itself, which — holding only `&self` — cannot issue ids, so there is no fix `NewInfo` to
  discard. A retry re-applies the identical annotated value, and the recorded `applied`
  list replays deterministically.
- **Failure restores the entry snapshot** (`*data = snapshot`), which makes
  `Err ⇒ bit-identical` literally true — id issuer included: ids the target's annotation
  or mid-cascade applies consumed are recycled on failure. That is safe because nothing
  observes them (the caller's `NewInfo` is only meaningful when the cascade succeeds, and
  fixes never issue ids). The backward ops are still collected — they are the success
  payload, the history slot — but the failure path never replays them, so the old
  unwind-failure panic arm does not exist.
- **Termination = the engraved contract** (D5 rule 1): fixes are strictly monotonically
  decreasing on a well-founded state order whose minimum is `Default::default()`. The
  engine's detectors are cheap proxies, not the proof: `None` catches an invariant with no
  material, the no-op panic catches a fix that removed nothing, and a state-*growing* map
  — undetectable without the order itself — loops forever until step 6.5 adds the
  `PartialOrd` in-flight assertion (on `Fixable` only; generic `InMemoryData` stays
  unbounded by it). The §5 (op, picked-invariant) repetition ledger from earlier drafts is
  deliberately absent — under one-step fixes, re-picking the same pair across rounds is a
  normal path, not a bug signature.
- **Determinism / confluence**: the checker's set is canonically ordered, the pick is
  `first()`, and `fix_invariant` is a pure function of `(&self, invariant)` — the emitted op
  list is a function of (state, target op). The commit-7 pin test freezes one instance.
- **Cost note**: one `data.clone()` at entry (the failure snapshot) plus one per fix round
  (the no-op check), on top of the gate's own snapshot, and one failed target apply per
  repair round. Linear overhead, fine at document scale; the property test watches the
  wall-clock.
- `ReversibleOp`'s fields and `AggregatedOp::new` are `pub(crate)`; `cascade.rs` lives in
  the same crate, so no visibility change is needed.

## 5. Commit 3 — the toy implementor and the engine tests

`QuoteData` joins `FakeData` in `state/src/test_utils.rs` (still `#[cfg(test)]`): the
smallest state with a real invariant. Students are a set of ids; quotes are rows attributed
to a student; removing a student strands their quotes.

```rust
/// A minimal state *with* an invariant: every quote's author must exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteData {
    pub students: BTreeSet<u64>,
    /// quote id -> author student id
    pub quotes: BTreeMap<u64, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuoteOp {
    AddStudent(u64),
    RemoveStudent(u64),
    /// Sets (or overwrites) a quote row. The author is *not* prechecked:
    /// a dangling author is an invariant break, which is the point.
    SetQuote { quote: u64, author: u64 },
    /// Removing an absent quote is a perfect no-op (G.2 precedent), with a
    /// no-op inverse (itself).
    RemoveQuote(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum QuoteInvalidOp {
    #[error("unknown student {0}")]
    UnknownStudent(u64),
    #[error("student {0} already exists")]
    StudentExists(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum QuoteInvariant {
    #[error("quote {0} has a dangling author")]
    DanglingQuoteAuthor(u64),
}
```

`impl InMemoryData for QuoteData`: `OriginalOperation = AnnotatedOperation = QuoteOp`
(identity annotate, caller-chosen ids, like `FakeOp`), `NewInfo = ()`,
`InvalidOp = QuoteInvalidOp`, `Invariant = QuoteInvariant`. The `apply` body mimics the real
gate: precheck (`RemoveStudent` on an unknown id, `AddStudent` no-clobber), then mutate a
clone, sweep `quotes` for dangling authors, and either commit the clone and return the
inverse (`RemoveStudent(s)` ⇄ re-adding the student — since the toy's `RemoveStudent`
returns an inverse that re-adds the student *and nothing else*, its quotes must already be
gone, which is exactly what the cascade guarantees) or return
`ApplyError::BrokenInvariants(set)` leaving `self` untouched. The canonical map:

```rust
impl Fixable for QuoteData {
    fn fix_invariant(&self, invariant: &QuoteInvariant) -> Option<QuoteOp> {
        match invariant {
            // Presence of the removable material (D2): Some only if the quote
            // row actually exists in the current state.
            QuoteInvariant::DanglingQuoteAuthor(quote) => self
                .quotes
                .contains_key(quote)
                .then(|| QuoteOp::RemoveQuote(*quote)),
        }
    }
}
```

For the panic/rejection paths the tests use a wrapper `EvilQuoteData(QuoteData, EvilMode)`
that delegates `InMemoryData` and overrides `fix_invariant` per mode (a blind fix, a
wrong-target-else-`None` fix, an invalid fix, a fix that creates a fresh invariant then
disowns it). There is no state-growing mode: without the fuse that scenario is a hang, not
a test — it belongs to step 6.5's `PartialOrd` check.

Engine unit tests (`#[cfg(test)] mod tests` in `cascade.rs`, the `traits.rs` pattern; each
test annotates its target itself — identity for the toy — before calling `apply_cascade`):

1. **Happy cascade + exact order**: two quotes by one student; `RemoveStudent` cascades over
   two repair rounds (one dangling-quote invariant fixed per round, canonical pick order) to
   exactly `[RemoveQuote(min), RemoveQuote(other), RemoveStudent]` — asserted as the exact
   `applied.inner()` sequence (determinism); final state has no quotes and no student.
2. **Undo round-trip**: replay the `applied` backwards in reverse order through
   `data.apply` and assert the exact original state returns (the compound reverse works
   stepwise, design doc §5).
3. **Single-op fast path**: a target that breaks nothing lands alone
   (`applied.inner().len() == 1`).
4. **Initial `InvalidOp`**: `RemoveStudent(unknown)` → `Err(ApplyError::InvalidOp(..))`,
   state untouched, nothing applied.
5. **Self-caused target → `None` → `Err`, round one**: `SetQuote { quote, author: missing }`
   — the canonical arm sees no such quote row in the pre-op state, returns `None` →
   `Err(BrokenInvariants {DanglingQuoteAuthor})`, state bit-identical, no fix ever applied.
6. **A no-op fix panics** (`#[should_panic(expected = "strict-monotonicity")]`): evil
   *blind* mode returns `Some(RemoveQuote(quote))` unconditionally; against
   `SetQuote { quote, author: missing }` the fix applies as a perfect no-op (the quote does
   not exist) — a map-contract violation regardless of who requested the fix (D4).
7. **Mid-cascade restore is real**: evil *wrong-target-else-`None`* mode fixes
   `DanglingQuoteAuthor(q)` by removing some *other*, existing quote when one exists, and
   returns `None` once none are left — fixes apply (innocent quotes destroyed, state
   changes each round) until the exhausted map says `None` with the target as the failing
   op → `Err` carrying the remembered `DanglingQuoteAuthor` set, and every innocent quote
   is back (state bit-identical despite a non-empty applied prefix — the snapshot restore
   actually ran).
8. **Fix-op `InvalidOp` panics** (`#[should_panic]`): evil map returns
   `Some(RemoveStudent(unknown))`.
9. **`None` for a fix-created invariant panics**: evil map fixes the target's invariant
   with `Some(SetQuote { quote: q2, author: missing2 })` (a fix that breaks a fresh
   invariant), then answers `None` for `DanglingQuoteAuthor(q2)` — the failing op is a fix,
   not the target → panic.

## 6. Commit 4 — `AssignmentOp::Assign` → `AssignmentOp::SetRow`

Adopted at review (D3): the assignment domain's elementary op becomes row-valued, matching
`SetInterrogation`/`SetGroupList`. One op now expresses any change to a `(period, subject)`
assignments row, so every assignment fix in the map is a single minimal op — and the
`ops/`-level translations get simpler, not just different. This is an elementary-op surface
change only: the `Assignments` state shape (`assignments.rs:23-27`, sparse
canonical-absent) is untouched, elementary ops are never persisted, and the `ops/` crate's
public `AssignmentsUpdateOp`/`UpdateError` vocabulary stays frozen (G.3).

### 6.1 The op and its annotated mirror

Today (`state-colloscopes/src/ops.rs:154-157`, and the identical
`AnnotatedAssignmentOp` at 554-557, annotated by identity at 956-960):

```rust
pub enum AssignmentOp {
    /// Assign (or deassign) a student to a subject on a given period
    Assign(PeriodId, StudentId, SubjectId, bool),
}
```

After:

```rust
pub enum AssignmentOp {
    /// Sets the whole assignments row for a `(period, subject)` pair.
    /// An empty set removes the row (rows are canonical-absent: a row exists
    /// iff at least one student is assigned).
    SetRow(PeriodId, SubjectId, BTreeSet<StudentId>),
}
```

`AnnotatedAssignmentOp` mirrors the swap; annotation stays the identity (no ids are
issued). The inverse of a `SetRow` is `SetRow` with the *previous* row (the empty set when
the row was absent) — same content-carrying inverse pattern as `SetInterrogation`.

### 6.2 `force_apply_assignment` (`state-colloscopes/src/assignments.rs:81-149`)

The three coordinate-existence prechecks survive unchanged in kind (the carve-out subset,
Appendix E.3): `InvalidPeriodId`/`InvalidSubjectId` on the key, and `InvalidStudentId` now
checked for **every** id in the incoming set. The body then reads the previous row
(`map.get(&key).cloned().unwrap_or_default()`), writes the new one preserving canonical
absence by construction (empty set → `map.remove(&key)`, non-empty → insert), and returns
`AnnotatedAssignmentOp::SetRow(period, subject, previous_row)` as the inverse. No semantic
guards, per the force_apply "fixes nothing" doctrine — unchanged.

### 6.3 The consumer sweep

Only sites that build the **elementary** op change; sites emitting the `ops/`-level
`AssignmentsUpdateOp::Assign` (e.g. the `general_planning.rs` cleaning emissions around
lines 634/840) are untouched, because that vocabulary stays.

- **`ops/src/assignments.rs`** — the three translations in `apply_no_cleaning`:
  - `Assign(period, student, subject, status)` (line 118): read the current row from
    `data`, insert or remove the student, emit one
    `AssignmentOp::SetRow(period, subject, new_row)`. The error translation keeps its
    exact shape: any `InvalidStudentId` precheck can only name the op's own student (the
    rest of the row came from a valid state), and the two `BrokenInvariants` extractions
    (`AssignmentForSubjectNotRunningOnPeriod`, `AssignedStudentNotPresentForPeriod`)
    already carry the ids they report.
  - `DuplicatePreviousPeriod` (line 190): today a per-(student, subject) `Assign` loop;
    becomes one `SetRow` per subject whose row changes, computing the target row by the
    same per-student rules the loop applies today (students excluded from either period
    keep their current status; subjects with no previous row keep their current row).
    Same observable result, fewer history entries.
  - `AssignAll(period, subject, status)` (line 275): today one `Assign` per non-excluded
    student; becomes exactly one `SetRow` — the full non-excluded student set for
    `status == true`, the empty set for `status == false` (every assigned student in a
    valid state is non-excluded, so clearing them all *is* the row's removal).
- **`ops/src/general_planning.rs:1237`** — the period-duplication path that directly
  applies elementary `Assign(new_id, student, subject, true)` per student: becomes one
  `SetRow(new_id, subject, row.clone())` per copied subject row.
- **`testgen-colloscopes/src/generator.rs`** — `gen_assignment` (~line 493): the invalid
  arm puts a dangling `StudentId` *inside* the set; the valid arm computes
  `current row ± picked student` (preserving today's assign/deassign distribution). The
  `ExcludedAssign` semantic recipe (~line 1491) becomes
  `SetRow(period, subject, current row ∪ {excluded student})`.
- **`storage/tests/populated_round_trip/builder.rs:374/380`** and
  **`state-colloscopes/tests/refs_registry.rs:366`** — fixture-building calls, mechanical
  (each `Assign(p, s, subj, true)` becomes `SetRow(p, subj, row-so-far)` or one
  consolidated `SetRow`).
- **`state-colloscopes/tests/`** — any exact-op or exact-error pins naming
  `AssignmentOp::Assign` re-spell mechanically.

Done-check: `cargo test --workspace` green; a workspace grep for `AssignmentOp::Assign`
and `AnnotatedAssignmentOp::Assign` returns nothing.

## 7. Commit 5 — enrich `InterrogationGroupOutOfBounds` with the offending group

The one payload upgrade the D5 survey called for. Today
(`state-colloscopes/src/invariants.rs:187-190`):

```rust
    /// An interrogation assigning a group number ≥ the associated group list's
    /// group count
    #[error("interrogation ({0:?}, {1:?}) assigns an out-of-bounds group number")]
    InterrogationGroupOutOfBounds(SlotId, WeekId),
```

After:

```rust
    /// An interrogation assigning a group number ≥ the associated group list's
    /// group count — one entry per offending group number
    #[error("interrogation ({0:?}, {1:?}) assigns out-of-bounds group number {2}")]
    InterrogationGroupOutOfBounds(SlotId, WeekId, u32),
```

A `u32` third field keeps the whole `FixableInvariant` vocabulary `Copy + Ord` (a
`BTreeSet<u32>` payload would not), and one-instance-per-group fits the one-step cascade:
each instance's fix trims exactly one group. The checker's emission
(`invariants.rs:592-596`) changes from an `any()` to a per-group sweep:

```rust
if let Some(bound) = bound {
    for &group_num in groups {
        if group_num >= bound {
            out.insert(Convergence::InterrogationGroupOutOfBounds(
                slot_id, week_id, group_num,
            ));
        }
    }
}
```

The derived `Ord` now orders instances by `(slot, week, group)`; the canonical pick trims
the smallest offending group first — fine, no reorder needed.

Sweep (every site naming the variant): the all-variants `Ord`-pin list
(`invariants.rs:1091`), the checker unit test (`invariants.rs:2104-2114` — the two-group
fixture's expected set gains the `2` payload), `tests/week_ops.rs:536`,
`tests/found_bugs.rs:338`, and `ops/src/colloscope.rs:218` (the translation arm gains a
third binding; note that a multi-group bad op can now put *several* instances in the set —
the existing first-match loop shape still translates correctly).

## 7bis. Commit 5.99 — split the balancing elementary op

Adopted during the July 27 2026 review of the map (§8). The trigger is the
`BalancingSubjectKey` arm: it needs "drop this one per-subject override", and the only
elementary op available is `BalancingOp::Update(Balancing)`, a whole-value rewrite. That op
is also the last place a `Table` value travels through the op surface out of `state/` — the
house rule (`feedback_no_table_outside_state`) says a `Table` stays inside `state/`, and
consumers see `BTreeMap`-shaped or targeted values instead. Splitting the op fixes both at
once, and the split is not invented: the `ops/`-level vocabulary already has exactly this
shape and fakes it by cloning the whole `Balancing`.

**Old** (`state-colloscopes/src/ops.rs:268-271`, and the same in `AnnotatedBalancingOp`):

```rust
pub enum BalancingOp {
    /// Update the balancing configuration
    Update(balancing::Balancing),
}
```

**New**:

```rust
pub enum BalancingOp {
    /// Replace the global balancing options
    SetGlobal(balancing::BalancingOptions),
    /// Set or clear the per-subject override. `None` removes the entry.
    SetSubject(SubjectId, Option<balancing::BalancingOptions>),
}
```

Two variants rather than three (no separate `RemoveSubject`): the `Option` payload is the
same canonical-absent row shape as the neighbouring
`GroupListOp::AssignToSubject(period, subject, Option<GroupListId>)`, so it introduces no new
idiom. The two variants are orthogonal — neither can touch the other's material — and both
have a content-carrying inverse, the `SetRow` pattern: `SetGlobal(new)` reverses to
`SetGlobal(old)`, and `SetSubject(s, new)` to `SetSubject(s, old_option)` where the old
option is the entry read before the write (`None` when there was none).

**Prechecks** (`force_apply_balancing`, `state-colloscopes/src/balancing.rs:88-99`, currently
infallible): `SetSubject` checks that the subject exists, uniformly for `Some` and `None` —
the same choice `force_apply_assignment`'s `SetRow` already makes (it prechecks the period and
the subject even when the incoming set is empty and the op only removes a row). This keeps the
coordinate carve-out uniform per op variant rather than conditional on the payload.
`BalancingPrecheckError` gains `InvalidSubjectId(SubjectId)`; `SetGlobal` stays infallible.

**Old** force_apply body:

```rust
AnnotatedBalancingOp::Update(new_balancing) => {
    let old_balancing = std::mem::replace(&mut self.inner_data.params.balancing, new_balancing.clone());
    Ok(AnnotatedBalancingOp::Update(old_balancing))
}
```

**New** (shape):

```rust
AnnotatedBalancingOp::SetGlobal(new_options) => {
    let old_options = std::mem::replace(
        &mut self.inner_data.params.balancing.global,
        new_options.clone(),
    );
    Ok(AnnotatedBalancingOp::SetGlobal(old_options))
}
AnnotatedBalancingOp::SetSubject(subject_id, new_options) => {
    if self.inner_data.params.subjects.find_subject(*subject_id).is_none() {
        return Err(BalancingPrecheckError::InvalidSubjectId(*subject_id));
    }
    let subjects = &mut self.inner_data.params.balancing.subjects;
    let old_options = match new_options {
        Some(options) => subjects.insert(*subject_id, options.clone()),
        None => subjects.remove(subject_id),
    };
    Ok(AnnotatedBalancingOp::SetSubject(*subject_id, old_options))
}
```

**Consumer sweep.** The `ops/` public vocabulary (`BalancingUpdateOp::UpdateGlobalOptions` /
`UpdateSubjectOptions` / `RemoveSubjectOptions`, and their error enums) is **unchanged**, so
nothing above `ops/` moves — this is an internal re-plumbing, the same doctrine as commit 4.
Inside `ops/src/balancing.rs:69-140`, each of the three arms currently clones the whole
`Balancing`, edits one field and pushes it back; each becomes a single targeted apply. For
example `UpdateGlobalOptions`:

```rust
// old
let mut new_balancing = data.get_data().get_inner_data().params.balancing.clone();
new_balancing.global = options.clone();
let result = data.apply(Op::Balancing(BalancingOp::Update(new_balancing)), self.get_desc())
    .expect("BalancingOp::Update should never fail");

// new
let result = data.apply(Op::Balancing(BalancingOp::SetGlobal(options.clone())), self.get_desc())
    .expect("BalancingOp::SetGlobal should never fail");
```

`RemoveSubjectOptions` keeps its own `NoOptionsForSubject` error: it reads the current
override first (`params.balancing.subjects.get(subject_id)`), returns that error when absent,
and otherwise applies `SetSubject(*subject_id, None)`. `UpdateSubjectOptions` keeps its
`InvalidSubjectId` pre-check as today (so its `.expect` on the gate still holds) and applies
`SetSubject(*subject_id, Some(options.clone()))`.

The remaining op-construction sites, all mechanical: `testgen-colloscopes/src/generator.rs:983`
and `:1321` (the generated balancing op — build a `SetSubject` / `SetGlobal` instead of
synthesising a whole `Balancing`), `storage/tests/populated_round_trip/builder.rs:638`, and
`state-colloscopes/tests/refs_registry.rs:353`.

**Out of scope**: the read side. gtk4 (`gtk4/src/editor/balancing.rs:245`),
`storage/src/encode/spec2.rs:538` and the constraints test still read
`params.balancing.subjects` / `.global` directly. That is reading through the inherent
`Table` API inside a snapshot, not shipping a `Table` value through an op, so it is left
alone.

## 8. Commit 6 — the colloscope resolution map

New file `state-colloscopes/src/resolution.rs` (`mod resolution;` in `lib.rs`; nothing new
is exported — the map surfaces through the `Fixable` impl). The impl reads the pre-op state
directly (`self.inner_data`; private fields are visible crate-wide since `Data` is declared
at the crate root, the same access `force_apply` uses).

```rust
impl collomatique_state::Fixable for Data {
    fn fix_invariant(&self, invariant: &FixableInvariant) -> Option<AnnotatedOp> {
        match invariant {
            FixableInvariant::DanglingFk(reference) => self.fix_dangling(reference),
            FixableInvariant::Convergence(convergence) => self.fix_convergence(convergence),
        }
    }
}
```

with one private helper per family, each an exhaustive match (no wildcard arm — totality is
the compiler's business). The arms construct `AnnotatedOp` variants directly (D2/D6): every
op the map emits is deletive, and the deletive ops' annotated forms are payload-identical
to their plain forms, so this is plain construction, no annotate call and no issued id.

**The whole job of an arm** (settled at the July 27 2026 review): *can I remove, from the
current state, the thing the invariant complains about?* If yes, `Some(op)`; if no, `None`.
No more, no less. The arm is entirely **local** — what the engine then does with `None`
(convict the target, or panic when the invariant came from a fix) is the engine's business
and no arm needs to reason about it.

Three consequences, each of which the tables below rely on:

1. **Presence, never predicate** (the D2 doctrine). An arm asks whether the material it would
   remove is *there*; it never re-evaluates the invariant's own condition, which may depend
   on the failing op's payload. Worked example — `InterrogationGroupOutOfBounds(slot, week, 3)`:
   the arm asks "is group `3` still in the cell `(slot, week)`?", **not** "is `3 >= the group
   count?". The predicate form would be wrong: after a group-list shrink is itself repaired,
   the count read from the state can be back above `3` while group `3` still has to go.
2. **No `expect` on a state lookup — a miss is `None`.** The invariant set the engine hands
   the map was computed on `self` *plus the op that just failed*, and that op was rolled back.
   So a row named by a site may simply not exist in `self`. Concretely: `PairingOp::Add(rule)`
   with an `excluded_periods` entry naming a non-existent period lands (the `Add` precheck only
   checks that the new id is free — `pairings.rs:210-228`, the semantic validation is stripped),
   the checker reports `DanglingFk(Period, PairingRuleExcludedPeriods(new_id))`, the gate rolls
   back, and the map is asked to fix it — with the rule row absent from `self`. Every arm is
   therefore a chain of lookups where any miss short-circuits to `None`:
   `let rule = self.…pairing_rule_map.get(rule_id)?;`. The only `expect` allowed anywhere in
   the map is on a **sealed constructor** rebuild, where the failure is provably impossible
   from the value alone (see the `PairingRule::new` case below). *(This corrects the first
   draft of this section, which wrote the lookup as
   `.expect("a dangling excluded-period entry implies the rule row exists")` — a false premise.)*
3. **`self` is always a valid state at fix time**, so the ids a fix op names are alive. Every
   op that ever landed — target or fix — passed the full apply/check/rollback gate, and the
   entry document was validated on decode; a valid state has no dangling reference. This is
   what makes the row-clearing fixes legal even though the target of the dangling reference is
   "gone": it is *not* gone in `self`. When `PeriodOp::Remove(P)` fails, the data was rolled
   back, `P` is still in the table, and `SetRow(P, subject, ∅)` — whose precheck demands that
   the period exist (`assignments.rs:87-95`) — applies cleanly. The hole only appears once the
   retried target finally lands, by which time every row that would have dangled is already
   gone. The same argument covers `AssignToSubject` (period + subject prechecks,
   `group_lists.rs:450-468`), `SetInterrogation` (week + slot, `colloscopes.rs:196-217`) and
   `SetGroupList` (group list, `colloscopes.rs:166-176`).

The table cells below give the op inside the `Some(...)` in the plain-op spelling for
readability; the presence check is implied (e.g. "the row exists", "the element is in the
set"), and per point 2 so is the `None` on any failed lookup. The full table, every arm
settled (rationale tags refer to D5):

### 8.1 `DanglingFk(Reference)` — by target kind and site

**Target: a period `P`** (`PeriodRefSite`, `refs.rs:90-108`):

| Site | Fix | Rule |
|---|---|---|
| `WeekPeriodFk(week)` | `[Week(WeekOp::Remove(week))]` | entity cannot survive (a week belongs to its period); cascades further |
| `SubjectExcludedPeriods(subject)` | `[Subject(SubjectOp::Update(subject, subject minus P))]` | whole-value minus element |
| `StudentExcludedPeriods(student)` | `[Student(StudentOp::Update(student, student minus P))]` | idem |
| `PairingRuleExcludedPeriods(rule)` | `[Pairing(PairingOp::Update(rule, rebuilt))]` | sealed rebuild, below |
| `SlotPairingRuleExcludedPeriods(rule)` | `[SlotPairing(SlotPairingOp::Update(rule, rebuilt))]` | idem |
| `AssignmentsKey { subject }` | `[Assignment(AssignmentOp::SetRow(P, subject, BTreeSet::new()))]` | single row-clearing op (commit-4 `SetRow`); presence = the row exists |
| `AssociationEntry { subject }` | `[GroupList(GroupListOp::AssignToSubject(P, subject, None))]` | targeted single-edge op |

The sealed rebuild (representative snippet — `PairingRule` fields are private, the validating
constructor is the only door, and removing a period cannot trip its only build error, which
is about the two parts sharing a subject):

```rust
// Any miss is `None` (frame point 2): the row may not exist in `self`…
let rule = self.inner_data.params.pairings.pairing_rule_map.get(rule_id)?;
// …and the rule may exist without excluding this period.
if !rule.excluded_periods().contains(&period) {
    return None;
}
let (antecedent, consequent, mut excluded, soft) = rule.clone().into_parts();
excluded.remove(&period);
let rebuilt = PairingRule::new(antecedent, consequent, excluded, soft)
    .expect("removing an excluded period cannot make the parts share a subject");
Some(Op::Pairing(PairingOp::Update(*rule_id, rebuilt)))
```

`PairingRule::into_parts` (`pairings.rs:159`) exists precisely for callers that rebuild, so
the rebuild goes through it rather than cloning four accessors. The lone `.expect` is the
sealed-constructor exception of frame point 2 and is honest: `PairingRule::new`'s only
failure is `SameSubjectInBothParts` (`pairings.rs:107-131`), and the two parts are moved
across untouched. `SlotPairingRule` is the exact twin.

**Target: a week `W`** (`WeekRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `WeekPatternExcludedWeek(pattern)` | `[WeekPattern(WeekPatternOp::Update(pattern, pattern minus W))]` | whole-value minus element (`WeekPattern { name, excluded_weeks }` is an open struct) |
| `ColloscopeInterrogation { slot }` | `[Colloscope(ColloscopeOp::SetInterrogation(slot, W, BTreeSet::new()))]` | clearing op; empty set = row removal (canonical-absent) |

**Target: a subject `S`** (`SubjectRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `TeacherSubjects(teacher)` | `[Teacher(TeacherOp::Update(teacher, teacher minus S))]` | whole-value minus element |
| `SlotSubject(slot)` | `[Slot(SlotOp::Remove(slot))]` | `Slot::subject_id` is mandatory and authoritative (`SlotOp::Update` rejects changing it) — the slot cannot survive |
| `IncompatSubject(incompat)` | `[Incompat(IncompatOp::Remove(incompat))]` | `Incompatibility::subject_id` is mandatory |
| `PairingRuleAntecedent(rule)` / `PairingRuleConsequent(rule)` | `[Pairing(PairingOp::Remove(rule))]` | a rule needs both parts; no half-rule exists |
| `BalancingSubjectKey` | `[Balancing(BalancingOp::SetSubject(S, None))]` | drops the per-subject override; the subject falls back to `balancing.global` (needs commit 5.99, §7bis) |
| `AssignmentsKey { period }` | `[Assignment(AssignmentOp::SetRow(period, S, BTreeSet::new()))]` | single row-clearing op; presence = the row exists |
| `AssociationEntry { period }` | `[GroupList(GroupListOp::AssignToSubject(period, S, None))]` | targeted op |

Settled at the review, and it applies to every `AssociationEntry` row in this section:
unassigning leaves the `GroupList` row it pointed at in place, possibly referenced by nothing.
That is a legal state — no invariant demands that a group list be assigned — and group lists
are edited independently of their associations, so the map deliberately does **not** delete
it. Removing it would be destruction the invariant never asked for.

**Target: a teacher `T`** (`TeacherRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `SlotTeacher(slot)` | `[Slot(SlotOp::Remove(slot))]` | `teacher_id` is mandatory (`slots.rs:56-57`, no `Option`), so there is no teacher-less slot to fall back to; naming a substitute teacher would be invented data |

Note the contrast with `SlotSubject`, which removes the slot too but for a different reason:
there `SlotOp::Update` *cannot* express the change (`CannotChangeSubject`), here it can — the
teacher field is freely editable — and the map declines on the invented-data rule. Presence
check: the slot row exists. This is also the most explosive fix in the table: one teacher
removal takes every one of their slots, and each slot removal then cascades to that slot's
colloscope cells and to any `SlotPairingRule` naming it (commit-7 fixture 3).

> ## ⛔ REVIEW STOPPED HERE — July 27 2026
>
> Everything **above** this banner has been walked arm by arm with the user and is
> confirmed: the §8 frame, and the period, week, subject and teacher targets of §8.1. The
> findings of that review are the frame's three points (arm-locality, no-`expect`-on-a-lookup,
> `self`-is-always-valid) and commit 5.99 (§7bis).
>
> Everything **below** it — the student, week-pattern, slot and group-list targets of §8.1,
> and the whole of §8.2 (`Convergence`) — is still **unreviewed first-draft material**.
> Resume the review at the `StudentRefSite` table immediately below.

**Target: a student `St`** (`StudentRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `GroupListPrefilledStudent(gl)` | `[GroupList(GroupListOp::Update(gl, rebuilt))]` — `GroupList::new(params.clone(), Prefilled with St removed from its group)` | sealed rebuild; removing a member changes neither the group count nor introduces duplicates, so `new()` cannot fail — `.expect` with that sentence |
| `GroupListExcludedStudent(gl)` | `[GroupList(GroupListOp::Update(gl, rebuilt))]` — `Automatic { excluded_students minus St }` | idem |
| `SettingsStudentKey` | `[Settings(SettingsOp::Update(settings minus St))]` | whole-value minus keyed entry |
| `AssignmentsStudent { period, subject }` | `[Assignment(AssignmentOp::SetRow(period, subject, row minus St))]` — row read live | presence = `St` is in the row |
| `ColloscopeGroupListStudent(gl)` | `[Colloscope(ColloscopeOp::SetGroupList(gl, placements minus St))]` | rewrite of the row read from `self.inner_data.colloscope.group_list(gl)` (an absent row degrades to the empty map — a no-op clear) |

**Target: a week pattern `WP`** (`WeekPatternRefSite`) — both cells verified against the
legacy `DeleteWeekPattern` cleaning (`ops/src/week_patterns.rs:230-256`), which *deletes* the
referencing entities; clearing the optional field to `None` would silently widen the
slot/incompat to "every week" and was rejected:

| Site | Fix | Rule |
|---|---|---|
| `SlotWeekPattern(slot)` | `[Slot(SlotOp::Remove(slot))]` | legacy match (D5.4) |
| `IncompatWeekPattern(incompat)` | `[Incompat(IncompatOp::Remove(incompat))]` | legacy match |

**Target: a slot `Sl`** (`SlotRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `SlotPairingRuleAntecedent(rule)` / `SlotPairingRuleConsequent(rule)` | `[SlotPairing(SlotPairingOp::Remove(rule))]` | a rule needs both parts |
| `ColloscopeInterrogation { week }` | `[Colloscope(ColloscopeOp::SetInterrogation(Sl, week, BTreeSet::new()))]` | clearing op |

**Target: a group list `GL`** (`GroupListRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `AssociationEntry { period, subject }` | `[GroupList(GroupListOp::AssignToSubject(period, subject, None))]` | targeted op |
| `ColloscopeGroupListKey` | `[Colloscope(ColloscopeOp::SetGroupList(GL, BTreeMap::new()))]` | clearing op |

### 8.2 `Convergence` — all 16 variants

The checker semantics quoted per variant are `invariants.rs:417-630`; the fixes clear the
now-invalid data (design doc §3, tier 3 — lossy by nature).

| Variant | Fix | Notes |
|---|---|---|
| `SlotTeacherDoesNotTeachSubject(slot)` | `[Slot(SlotOp::Remove(slot))]` | teaching the subject cannot be granted (creative); the slot goes |
| `TeacherSubjectWithoutInterrogations(teacher, subject)` | `[Teacher(TeacherOp::Update(teacher, teacher minus subject))]` | minimal: only the stale subject entry goes |
| `SlotForSubjectWithoutInterrogations(slot)` | `[Slot(SlotOp::Remove(slot))]` | a slot for a colle-less subject is meaningless |
| `SlotOverflowsDay(slot)` | `[Slot(SlotOp::Remove(slot))]` | shortening the duration or moving the start would be invented data |
| `AssignmentForSubjectNotRunningOnPeriod(period, subject)` | `SetRow(period, subject, ∅)`; `None` if the row is absent (purely op-caused, D4) | single row clear |
| `AssignedStudentNotPresentForPeriod { period, subject, student }` | `SetRow(period, subject, row minus student)`; `None` if the student is not in the current row | minimal; the `None` arm is the round-one self-caused rejection (D4) |
| `AssociationForSubjectWithoutInterrogations(period, subject)` | `[GroupList(GroupListOp::AssignToSubject(period, subject, None))]` | |
| `AssociationForSubjectNotRunningOnPeriod(period, subject)` | `[GroupList(GroupListOp::AssignToSubject(period, subject, None))]` | |
| `BalancingForSubjectWithoutInterrogations(subject)` | `[Balancing(BalancingOp::SetSubject(subject, None))]` | needs commit 5.99 (§7bis) |
| `PairedSlotsNotInSameSubject(rule)` | `[SlotPairing(SlotPairingOp::Remove(rule))]` | which slot is "wrong" is undecidable; the rule goes |
| `InterrogationSlotNotRunningOnPeriod(slot, week)` | `[Colloscope(SetInterrogation(slot, week, ∅))]` | whole cell |
| `InterrogationOnInactiveWeek(slot, week)` | `[Colloscope(SetInterrogation(slot, week, ∅))]` | whole cell; matches the legacy week-exclusion cleaning |
| `InterrogationGroupOutOfBounds(slot, week, group)` | `Colloscope(SetInterrogation(slot, week, cell minus group))` if `group` is in the current cell; `None` otherwise (an emptied result = canonical-absent row removal) | minimal trim, enabled by the commit-5 payload; the arm must **not** re-check the bound (a group-list shrink legitimately needs the trim even though the group is in-bounds against the current count — the D2 presence-not-predicate doctrine); when self-caused, the group is absent from the pre-op cell → round-one `None` rejection (D4 trace 2) |
| `ColloscopeGroupListPrefilled(gl)` | `[Colloscope(SetGroupList(gl, ∅))]` | prefilled lists carry their filling in params; the colloscope row is wholly invalid |
| `ColloscopeStudentExcluded(gl, student)` | `[Colloscope(SetGroupList(gl, placements minus student))]` | minimal, matches legacy |
| `ColloscopeStudentGroupOutOfBounds(gl, student)` | `[Colloscope(SetGroupList(gl, placements minus student))]` | minimal, matches the legacy group-list-shrink cleaning (`ops/src/group_lists.rs`) |

Implementation notes:

- Every "minus" value and presence check is computed from `self` (the live pre-fix state);
  an arm finds its removable material absent only in self-caused routes, and then returns
  `None` — the engine convicts the failing op (D4). An arm that misjudges presence and
  emits a no-op fix trips the engine's contract panic — the commit-8 fuzz exists to catch
  that before production.
- Deep chains come free from the engine: e.g. removing a period fails on `WeekPeriodFk` →
  `WeekOp::Remove` → itself fails on `WeekPatternExcludedWeek` and
  `ColloscopeInterrogation` → pattern updates and cell clears land first — depth-first, one
  invariant at a time, every intermediate state valid.
- Design-doc §8 allows reordering `FixableInvariant`/`Convergence` variants at step 6 if the
  canonical pick proves awkward (a variant-order edit plus its ordering-pin tests, not a
  mechanism change). No reorder is currently expected; do not reorder pre-emptively.

## 9. Commit 7 — colloscope cascade fixtures (`tests/cascade.rs`)

Fixture style: build a document through the public surface
(`AppState::<Data, String>::new(Data::new())` plus `Manager::apply`, the `read_api.rs` /
`week_ops.rs` idiom), then take `app.get_data().clone()`, annotate the target through
`Data::annotate`, and drive `apply_cascade` on it directly. Scenarios:

1. **The flagship deep cascade (and the confluence pin).** A document with one period, two
   weeks, an interrogation subject with a teacher and a slot on a week pattern, assignments,
   a group-list association, and colloscope cells. Target: `PeriodOp::Remove`. Assert
   `Ok`; assert the **exact** `applied.inner()` op list, literally, in order (this is the
   design-doc §8 confluence pin — it freezes the canonical pick order and the map against
   refactor drift); assert the final state is clean (`broken_invariants() == Ok(∅)` via a fresh
   `from_inner_data` round or direct call) and the period, its weeks, and every referencing
   row are gone.
2. **Undo round-trip.** Replay scenario 1's reverses in reverse order through `apply`;
   assert the exact original `InnerData` returns.
3. **Teacher removal.** A teacher with two slots (one carrying colloscope cells, one paired
   by a `SlotPairingRule`): removal cascades slot removals, which cascade cell clears and
   the pairing-rule removal.
4. **Subject update turning interrogations off.** Assert the cascade trims the teacher's
   subject list, removes the subject's slots, clears balancing override and associations —
   the `*WithoutInterrogations` family end-to-end.
5. **Self-caused rejections** (each asserts `Err(Error::BrokenInvariants(..))` with the
   expected variant *and* a bit-identical state):
   - `SlotOp::Update` moving a slot's start so it overflows the day — this is the
     target-consumed-mid-cascade trace: real fixes land (the slot and its cells go), the
     retry prechecks out, the snapshot restore brings everything back, and the reported
     error is the remembered `SlotOverflowsDay` set, **not** the precheck;
   - `AssignmentOp::SetRow` whose set includes a student excluded from the period —
     round-one `None` from the `AssignedStudentNotPresentForPeriod` arm;
   - `ColloscopeOp::SetInterrogation` with an out-of-bounds group number — round-one
     `None` from the enriched `InterrogationGroupOutOfBounds` arm (the offending group is
     absent from the pre-op cell).
6. **Clean target lands alone**: a benign op cascades to exactly `[itself]`.

## 10. Commit 8 — the cascade property test (`tests/property_cascade.rs`)

The "fuzz that there are no panics". Reuse `collomatique_testgen_colloscopes`
(`harness::bootstrap(rng)` + `generator::gen_op`), the `property_ops.rs` idiom, but route
every generated op through `annotate` + `apply_cascade` instead of `apply`:

- **No panic** over the whole run (implicit in the test passing) — the canonical map never
  trips the engine's fix-op panics (invalid fix, disowned fix-created invariant, no-op
  fix). With the round fuse gone, this fuzz — plus the audit of every arm against the D5
  contract — is what stands between a map bug and a production hang, until step 6.5 adds
  the `PartialOrd` in-flight check.
- **`Ok` ⇒ honesty**: `broken_invariants() == Ok(∅)`, the target op is the last entry of
  `applied`, and replaying the reverses in reverse restores the pre-call state exactly.
- **`Err` ⇒ atomicity**: the state is bit-identical to before the call.

Modest configuration (the cascade multiplies gate calls): start at 20 seeds × 300 ops and
tune to keep the suite's wall-clock reasonable; the main `property_ops`/`property_apply_gate`
harnesses remain the deep oracles. Run the suite once, captured to a scratchpad file, per the
house testing rules.

## 11. Non-goals, gates, close-out

**Non-goals**: no `ops/` migration onto the cascade (the commit-4 `SetRow` translation
rewrite inside `ops/` is an op-surface adaptation, not the step-7 remaster — `Warning`,
`get_next_cleaning_op` and the `UpdateError` vocabulary all stay), no dry-run/preview UX,
no `Warning` retirement, no history/`Manager` integration of the cascade (step 7 decides
whether `apply_cascade` gets a `Manager`-level wrapper), no storage change, no gtk4 change
beyond the mechanical commit-1 re-spelling, no variant reorder, and no `PartialOrd` /
monotonicity checking — that is step 6.5 (design doc §8), recorded, not implemented here.

**Gates**: `cargo test --workspace` (background, output captured once) green after every
commit; the property harnesses untouched and green; `Cargo.lock` unchanged (no new
dependencies). The user runs the acceptance scripts / gtk4 smoke at their own cadence; no
step in this plan blocks on it.

**Close-out ritual** (after the user's gate): record the delivered state as **Appendix H**
of the design doc — the `ApplyError` reshaping of the G.2 error surface (G stays as the
historical step-5 record), the `SetRow` op swap and the `InterrogationGroupOutOfBounds`
enrichment, the engine contract and its deviations from the §5 pseudocode (the
annotated-op surface; one-step `Option` fixes recomputed per round; the engraved
strict-monotonicity contract with `Default::default()` as the minimum; the `None`
conviction and the unconditional no-op-fix panic replacing the §5 repetition ledger;
target-fallible/fix-infallible; the remembered-error rule; snapshot-restore failure; no
fuse — hang accepted until 6.5), the resolution table's policy rules
(presence-not-predicate arms, orthogonality principle, legacy semantics as aspiration with
divergences recorded), and the test inventory. Then retire this plan from the tree with a
pin, per the house pattern.
