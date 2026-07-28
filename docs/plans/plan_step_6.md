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
- **Commit 5.97** (landed, `4c36d824`) — enrich five more `Convergence` variants so their
  arms can pin the offending *shape* and not merely the row's existence (frame point 5):
  `SlotTeacherDoesNotTeachSubject` gains the teacher and the subject,
  `SlotForSubjectWithoutInterrogations` gains the subject, `SlotOverflowsDay` becomes a
  struct variant carrying the start time and the duration, `PairedSlotsNotInSameSubject`
  gains the two slot ids, and
  `ColloscopeStudentGroupOutOfBounds` gains the offending group number. Adopted during the
  commit-6 review (July 28 2026); it was the collection point for every payload enrichment
  §8.2 turned up, so it lands **after** the map is fully reviewed. The review is now
  finished and the list is closed at these five.
- **Commit 5.98** (landed, `c60eba70`) — split the settings elementary op:
  `SettingsOp::Update(Settings)`
  (which ships a whole `Table` value through the op surface) becomes
  `SetGlobal(Limits)` + `SetStudent(StudentId, Option<Limits>)`. Adopted during the
  commit-6 review (July 28 2026), on the `SettingsStudentKey` arm.
- **Commit 5.99** (landed, `bccfe224`) — split the balancing elementary op:
  `BalancingOp::Update(Balancing)`
  (which ships a whole `Table` value through the op surface) becomes
  `SetGlobal(BalancingOptions)` + `SetSubject(SubjectId, Option<BalancingOptions>)`.
  Adopted during the commit-6 review (July 27 2026), on the `BalancingSubjectKey` arm.

  Both are numbered below 6 because they are prerequisites of map arms, not parts of
  commit 5; they are **two separate commits** (user ruling, July 28 2026) even though
  they are structural twins — `Settings { global, students: Table<..> }` and
  `Balancing { global, subjects: Table<..> }` have the same shape, the same wart and the
  same fix, and each carries its own consumer sweep.
- **Commit 6** — the colloscope resolution map: `impl Fixable for Data` in
  `state-colloscopes/src/resolution.rs`, total over `FixableInvariant`.
- **Commit 7** — colloscope integration tests (`state-colloscopes/tests/cascade.rs`), the
  half that asserts `Ok`: the period-removal family (order / depth / breadth /
  confluence-on-one-op / flagship), teacher / subject-update / student-removal cascades,
  the week-pattern family (the D5.4 divergence pin plus the legacy-agreement update pin)
  and the no-op-target pin.
- **Commit 7.5** — the innocent-state `None` tests
  (`state-colloscopes/src/resolution/innocent_tests.rs`): one test per invariant variant,
  asserting that an arm handed an invariant its own state does *not* cause returns `None`.
  This is the mechanical detector for frame point 5. Adopted during the commit-6 review
  (July 28 2026). It is **46 tests, so it ships as ten commits** — 7.5a … 7.5f for the
  dangling-FK arms, then four for the `Convergence` blocks; see §9bis.
- **Commit 7.6** — the rejection fixtures, back in `state-colloscopes/tests/cascade.rs`: the
  half of commit 7 that asserts `Err`, split out and sequenced **after** 7.5 (★ user ruling,
  July 28 2026) because a rejection fixture only means something once the `None` branch it
  rests on has been tested arm by arm. Two families — the self-caused rejections and the
  collateral-damage identity pins — eight fixtures in all; see §9ter.
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
   substitute teacher, no fabricated group), and nothing lands *equivalent*: the map
   returns `None` or a strictly-decreasing op, never a no-op (D4).

   **The order is over the document's *content*, not over the meaning it denotes** (made
   explicit at the July 28 2026 review, and binding on step 6.5's `PartialOrd`). Several
   arms strictly shrink the data while *widening* the semantics, and that is fine: a
   subject/student/week-pattern/pairing-rule that stops excluding a dead period now applies
   more broadly, and a slot whose optional `week_pattern` is cleared to `None` now runs
   every week. In each case an id was removed and nothing was added, so the document
   strictly decreased. Reading the order semantically instead would make these arms look
   like increases and would break the termination proof.

   Because the order is
   well-founded, strict monotonicity **is** the termination proof of the cascade. This
   contract is engraved: it goes verbatim into the `Fixable` trait's doc-comment (the map
   implementor's contract) and into the `apply_cascade` module docs; the engine's `None`
   conviction and no-op panic (D4) are its cheap in-flight detectors, and step 6.5 will
   add the order itself (`PartialOrd` + a strictly-below assertion per fix).
2. **Where a targeted single-edge op exists, use it**; where none exists, rewrite the whole
   value through the domain's `Update` op with the offending element removed, reading the
   current value from the pre-op state.
3. **Remove the reference; remove the entity only when the reference cannot go alone.**
   Sharpened at the July 28 2026 review, replacing the earlier "remove the entity where it
   cannot survive the loss". The test is purely structural: *is the offending reference
   expressible as absent?* If the field is an `Option`, or the reference lives in a set or
   in the value of a map entry, then clear that one field / drop that one element / drop
   that one entry, and the row stays. Only when the reference is mandatory — a bare id
   field, or half of a row's key — does the row have to go with it. Rows that must die:
   a slot without its teacher (`Slot::teacher_id`, a bare `TeacherId`) or subject; a
   pairing rule without both parts (`SlotRulePart::slot_id`); an incompatibility without
   its (mandatory) subject; a colloscope interrogation row without its slot or its week
   (both are key components). Rows that live on: everything the map narrows instead —
   a subject, a student, a week pattern or a pairing rule that stops excluding a dead
   period/week; a slot or an incompat whose optional `week_pattern` is cleared; an
   association entry that is unassigned.
4. **Aim to match the legacy cleaning semantics** (`ops/src/*.rs get_next_cleaning_op`)
   where they exist — but this is an aspiration, not a gate (softened at review). An exact
   match may not always be achievable, and where the map diverges the divergence is
   recorded at close-out: it more likely captures an edge case the hand-written cleaning
   forgot than a regression. Verified against the legacy code in the planning session:
   a group-list shrink removes the out-of-bounds *student placements*; a week-exclusion
   update clears *whole interrogation cells*.

   **One deliberate divergence** (user ruling, July 28 2026). `DeleteWeekPattern`'s legacy
   cleaning (`ops/src/week_patterns.rs:229-256`) *deletes* every referencing slot and
   incompat; the map instead clears their optional `week_pattern` field to `None` and keeps
   the rows — see §8.1's week-pattern table for the argument. This is the divergence D5.4
   anticipated, and it costs us the differential fuzz that would otherwise have pinned this
   arm against the legacy cleaning in a later step. Accepted knowingly.

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
  list is a function of (state, target op). Commit 7's fixture `1a` freezes the pick order on
  the one minimal case where a choice is genuinely made; `1d` shows the complementary
  property, two breaks whose arms agree on a single op.
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

## 7bis. Commit 5.97 — enrich five more `Convergence` variants

Adopted during the July 28 2026 review of §8.2, rows 1, 3, 4, 10 and 16. Same kind of work
as commit 5 and for the same reason, one level deeper: an arm cannot pin the offending
*shape* (frame point 5) if the invariant does not name it. This commit is the **collection
point** for every payload enrichment the map review turns up, so it is written once §8.2 is
reviewed to the end — if a later row needs another field, it joins this commit rather than
starting a new one. The review is now finished and the collection is closed at five
variants: four from the slot/pairing block and one from the colloscope block (row 16).

**Old** (`state-colloscopes/src/invariants.rs:144-156`, `:180` and `:199`):

```rust
SlotTeacherDoesNotTeachSubject(SlotId),
SlotForSubjectWithoutInterrogations(SlotId),
SlotOverflowsDay(SlotId),
PairedSlotsNotInSameSubject(SlotPairingRuleId),
ColloscopeStudentGroupOutOfBounds(GroupListId, StudentId),
```

**New**:

```rust
/// The slot's teacher's `subjects` set lacks the slot's subject
SlotTeacherDoesNotTeachSubject(SlotId, TeacherId, SubjectId),
/// A slot on a subject whose interrogations are disabled
SlotForSubjectWithoutInterrogations(SlotId, SubjectId),
/// The slot's start time plus its subject's interrogation duration
/// overflows the day
SlotOverflowsDay {
    slot: SlotId,
    start: collomatique_time::SlotStart,
    duration: collomatique_time::NonZeroMinutes,
},
/// A slot pairing rule whose two slots are on different subjects
PairedSlotsNotInSameSubject(SlotPairingRuleId, SlotId, SlotId),
/// A placed student with a group number ≥ the list's group count —
/// the third field is that offending group number
ColloscopeStudentGroupOutOfBounds(GroupListId, StudentId, u32),
```

Why each:

- **`SlotTeacherDoesNotTeachSubject`** — load-bearing, and the case that produced frame
  point 5. `force_apply_slot`'s `Update` keeps only `CannotChangeSubject`
  (`slots.rs:455-483`), so `teacher_id` is freely rewritable; without the teacher in the
  payload the arm deletes an innocent slot instead of rejecting the bad edit.
- **`SlotForSubjectWithoutInterrogations`** — the subject is defensive on today's code (the
  slot's subject cannot be changed, and a *freshly added* slot fails the plain "is the slot
  there" test anyway), and is added for the same reasons as the two defensive identity tests
  of frame point 4: uniformity, and a cheap net against a plain bug. Do not remove it later
  as "unused".
- **`PairedSlotsNotInSameSubject`** — the two slot ids, tested per part. `SlotPairingOp::Update`
  (`ops.rs:259`) rewrites the whole rule and its precheck only checks the id
  (`slot_pairings.rs:245-256`), so the invariant can name a rule whose live content is
  innocent.
- **`SlotOverflowsDay`** — load-bearing, and the only variant where an id cannot do the job.
  The bad route (`SlotOp::Update` moving the start to 18:30) and the legitimate route (the
  subject's interrogation duration grows) leave the *same* live slot with the same subject;
  only the start-time *value* separates them. Struct variant because three positional fields
  would be unreadable. `duration` is not needed by the test — it is there so the error reads
  on its own ("slot X starts at 18:30 and lasts 90 min") — and per frame point 5's corollary
  the arm must **not** test it.
- **`ColloscopeStudentGroupOutOfBounds`** — the group number, exactly as `SlotOverflowsDay`
  takes the start time. The fix removes the entry *student → group* from the placement map,
  so the group number is half of what the fix destroys, and it is the only thing separating
  a legitimate route from a bad one. Legitimate: the list shrinks from 5 groups to 2 and the
  stored `student → 3` must go. Bad: a `SetGroupList` writes `student → 99` over an innocent
  stored `student → 0`. Without the number the arm sees a present student in both cases,
  deletes a valid placement in the second, and only errs one round later when the retry
  fires the same invariant again on a state where the student is gone. The final answer is
  `Err` either way — this is precision, not a bug fix — but it is one field, already in
  scope, and it makes the arm honest.

The producer side is five literal edits: the three slot arms (`invariants.rs:427-450`), the
pairing arm (`:533-546`) and the colloscope placement arm (`:624-629`, where `group_num` is
the loop variable) already have every value in scope.

`collomatique_time::SlotStart` (`time/src/lib.rs:530`) is `Clone` but not `Copy`, so
`Convergence` and `FixableInvariant` **lose their `Copy` derive** and keep `Clone`. That is
free at the engine level — `InMemoryData::Invariant` is bounded
`Send + Sync + Clone + Ord + Debug + Display` (`state/src/traits.rs:45`), never `Copy` — and
the compiler finds every `*invariant` site inside `state-colloscopes`. Adding `Copy` to
`SlotStart` instead would work (both its fields are `Copy`), but `Copy` is deliberately kept
for small things only (user ruling, July 28 2026), and the time crate is not this step's
business.

Sweep (every site naming the five variants; each matches on fewer fields today and needs one
more, `(_, _, _)` or `{ .. }`):

- inside the checker (`state-colloscopes/src/invariants.rs`) — the all-variants `Ord`-pin
  list in `convergence_declaration_order_is_canonical`, the `FixableInvariant` pin in
  `dangling_fk_sorts_before_convergence`, and **seven** unit tests, named rather than
  numbered because the line numbers drift with every edit to the file:
  `slot_teacher_does_not_teach_subject`, `slot_for_subject_without_interrogations`,
  `slot_overflows_day`, `paired_slots_not_in_same_subject`,
  `colloscope_student_group_out_of_bounds`, `slot_teacher_check_runs_when_subject_dangles`
  and `compound_convergence_with_dangling`. (The July 28 2026 draft of this list named only
  five, by line number, and missed `slot_overflows_day` and
  `compound_convergence_with_dangling`; both are mechanical, and the implementation found
  them by compiling.)
- in `ops/` — `slots.rs:348`, `:452`, `:471`, `:481` and `:368`; `teachers.rs:210`;
  `slot_pairings.rs:136` and `:234`; `colloscope.rs:146`.

`ops/src/colloscope.rs:146` is the one site that needs a thought rather than an extra `_`:
it translates the invariant into
`UpdateColloscopeGroupListError::InvalidGroupNumForStudentInGroupList(group_list, student)`,
whose payload is frozen. Bind the new field as `_` there — the translation stays
byte-identical.

**A bonus in `ops/`.** The `AddNewSlot` translation captures the teacher id out
of the op payload before the op is moved, with the comment *"the
SlotTeacherDoesNotTeachSubject convergence carries only the slot id, so the reported
(teacher, subject) pair is synthesized from the op payload in scope"* (`:306-313`). The
enrichment removes that need: the pair can be read straight off the invariant. Do the
simplification, and delete the stale comment.

It is **five arms, not two**. In `slots.rs`: `AddNewSlot`, and *both* of `UpdateSlot`'s
arms — the teacher one at `:452` and the `SlotForSubjectWithoutInterrogations` one, which
reads the same captured `subject_id` local, so leaving it would keep the local alive and
defeat the simplification. In `slot_pairings.rs`: both `PairedSlotsNotInSameSubject` arms
(`:136` and `:234`), whose comment clause *"the same-subject convergence carries only the
rule id, so the two slot ids come from the op payload in scope"* becomes false the moment
the rule's two slot ids join the payload.

## 7ter. Commit 5.98 — split the settings elementary op

Adopted during the July 28 2026 review of the map (§8). The trigger is the
`SettingsStudentKey` arm, and the case is the exact twin of commit 5.99 below — read that
section for the reasoning, which transposes word for word. `Settings`
(`state-colloscopes/src/settings.rs:17-23`) is structurally identical to `Balancing`:

```rust
pub struct Settings {
    pub global: Limits,
    pub students: Table<StudentId, Limits>,
}
```

and `SettingsOp::Update(Settings)` (`ops.rs:230-233`) is the last remaining place besides
balancing where a `Table` value travels through the op surface out of `state/`. The arm
needs "drop this one per-student override" and the whole-value rewrite is all it has.

**New**:

```rust
pub enum SettingsOp {
    /// Replace the global limits
    SetGlobal(settings::Limits),
    /// Set or clear the per-student override. `None` removes the entry.
    SetStudent(StudentId, Option<settings::Limits>),
}
```

Two variants, not three, for the same reason as balancing (the `AssignToSubject`
canonical-absent precedent). Both have the content-carrying inverse: `SetGlobal(new)`
reverses to `SetGlobal(old)`, `SetStudent(s, new)` to `SetStudent(s, old_option)`.

**Prechecks.** `SettingsPrecheckError` is currently the empty enum
(`settings.rs:47-51` — the settings op had no carve-out guards at all). It gains
`InvalidStudentId(StudentId)`, checked by `SetStudent` uniformly for `Some` and `None`;
`SetGlobal` stays infallible. Same uniformity choice as `SetRow` and `SetSubject`.

**New** `force_apply_settings` body (shape), replacing the single `Update` arm at
`settings.rs:66-72`:

```rust
AnnotatedSettingsOp::SetGlobal(new_limits) => {
    let old_limits = std::mem::replace(
        &mut self.inner_data.params.settings.global,
        new_limits.clone(),
    );
    Ok(AnnotatedSettingsOp::SetGlobal(old_limits))
}
AnnotatedSettingsOp::SetStudent(student_id, new_limits) => {
    if !self.inner_data.params.students.student_map.contains(student_id) {
        return Err(SettingsPrecheckError::InvalidStudentId(*student_id));
    }
    let students = &mut self.inner_data.params.settings.students;
    let old_limits = match new_limits {
        Some(limits) => students.insert(*student_id, limits.clone()),
        None => students.remove(student_id),
    };
    Ok(AnnotatedSettingsOp::SetStudent(*student_id, old_limits))
}
```

**Consumer sweep.** The `ops/` vocabulary already has exactly the target shape and fakes it
by cloning the whole `Settings`: `SettingsUpdateOp::{UpdateGlobalLimits, UpdateStudentLimits,
RemoveStudentLimits}` (`ops/src/settings.rs:18-25`). It is **unchanged**, so nothing above
`ops/` moves — same doctrine as commits 4 and 5.99. Inside `ops/src/settings.rs`, the three
`apply_no_cleaning` arms (`:63-145`) each drop their clone-edit-push-back for a single
targeted apply. `UpdateStudentLimits` and `RemoveStudentLimits` keep their existing
`InvalidStudentId` pre-checks (so their `.expect` on the gate still holds), and
`RemoveStudentLimits` keeps reading the entry first to raise `NoLimitsForStudent` before
applying `SetStudent(*student_id, None)`.

The fixture sites are mechanical: `storage/tests/populated_round_trip/builder.rs:604` and
`state-colloscopes/tests/refs_registry.rs:344` (each whole-value `Update` becomes one
`SetGlobal` and/or one `SetStudent`), and `state-colloscopes/tests/found_bugs.rs:66` and
`:93` (the second is the *clearing* apply, which becomes `SetStudent(student, None)`).

**The two testgen sites are not both mechanical**, contrary to the first draft of this
paragraph. `gen_settings` (`generator.rs:849`) is: it draws between `SetGlobal` and
`SetStudent`, both directions of the sparse form, and its invalid arm keeps the dangling
student key — which now bounces at the precheck tier rather than landing as a dangling FK,
the same flavor of invalidity `gen_student`'s `Remove(dangling)` arm already produces.
`gen_force_retarget` (`:1314`) is **not**. That recipe's whole job is to hand `force_apply`
an op that *lands* a dangling FK, and settings could only ever do that because the
whole-value `Update` shipped an arbitrary `students` table past a stripped validator. With
the per-student key a prechecked coordinate, no settings op can land a dangling FK at all,
so the candidate is deleted rather than re-spelled. Balancing still makes the candidate set
non-empty after this commit; §7quater has to deal with losing that guarantee.

`synth::settings` has no callers once both sites are rewritten, so it retires with the
whole-value op it fed. The generator still needs the per-entry half it was built from, so
`synth::limits` becomes `pub`.

**Out of scope**: the read side (`Settings::limits_for` and every snapshot reader), for the
same reason as balancing — reading through the inherent `Table` API inside a snapshot is not
shipping a `Table` through an op.

## 7quater. Commit 5.99 — split the balancing elementary op

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

The fixture sites are mechanical: `storage/tests/populated_round_trip/builder.rs:638` and
`state-colloscopes/tests/refs_registry.rs:353`.

`gen_balancing` (`generator.rs:983`) is mechanical too, and keeps *both* flavors of its
invalid arm — they now land in different tiers. An override on a live subject that has no
interrogations still applies cleanly and is caught by the checker as
`BalancingForSubjectWithoutInterrogations`; a dangling subject key bounces at the
`SetSubject` precheck.

**`gen_force_retarget` (`:1321`) is the one site that is not mechanical**, and this commit
is where the bill comes due. Settings and balancing were its two *unconditional* candidates
— the reason its doc comment could say "the candidate set is never empty" — and both were
unconditional only because a whole-value `Update` could ship an arbitrary `Table` past a
stripped validator. Commit 5.98 removed the first; this one removes the second, and every
remaining candidate is gated on a pool. So:

- `gen_force_retarget` returns `Option<Op>`, `None` on an empty state, exactly like
  `gen_force_remove`;
- a new `retargetable_present(inner, pools)` mirrors its pool gates, the way
  `removable_present` mirrors `gen_force_remove`'s;
- `gen_corruption_op` pushes `CorruptionKind::ForceRetarget` onto `eligible` only when that
  predicate holds, and its doc comment loses "retarget and valid are always available".

Without this, a probe fired at a state with no students, subjects, teachers, incompats,
slots or group lists panics inside `rng.random_range(0..0)`. The cross-seed
`attempted[ForceRetarget] > 0` assertion still holds comfortably: `retargetable_present` is
a strict subset of `removable_present`, which already gates `ForceRemove` today.

`synth::balancing` retires the same way `synth::settings` did in 5.98, and
`synth::balancing_options` becomes `pub`.

**Out of scope**: the read side. gtk4 (`gtk4/src/editor/balancing.rs:245`),
`storage/src/encode/spec2.rs:538` and the constraints test still read
`params.balancing.subjects` / `.global` directly. That is reading through the inherent
`Table` API inside a snapshot, not shipping a `Table` value through an op, so it is left
alone.

## 8. Commit 6 — the colloscope resolution map

> **✅ REVIEW COMPLETE — July 28 2026.** This whole section has now been walked arm by arm
> with the user: the frame, all eight target kinds of §8.1, and all sixteen variants of
> §8.2. What the review produced is recorded in place — the frame's five points
> (arm-locality, no-`expect`-on-a-lookup, `self`-is-always-valid,
> presence-names-the-target with its audit criterion, and pin-the-shape-not-just-the-row);
> the D5.3 sharpening (remove the reference, and remove the row only when the reference
> cannot go alone) with its one deliberate legacy divergence in D5.4; a per-row presence /
> shape test column in both tables; and four new commits, 5.97 (§7bis), 5.98 (§7ter), 5.99
> (§7quater) and 7.5 (§9bis). §8.1 was re-audited end to end against frame point 4 after
> that point was discovered mid-table: five rows were missing their identity test —
> `SlotTeacher`, `IncompatSubject` and both `PairingRule` parts (all reachable), plus
> `SlotSubject` and `WeekPeriodFk` (unreachable, added defensively). Commit 7.6's identity
> fixtures (§9ter.4) pin the reachable ones.

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

Five consequences, each of which the tables below rely on:

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
4. **The presence test names the target, not merely "some value is there"** (July 28 2026
   review). Wherever the offending reference sits in a field or entry that could legally
   hold a *different*, live id, the arm must compare against the target before acting —
   otherwise it destroys a perfectly valid reference. The reachable route is always the
   same: an `Update`-style target rewrites a row's field to a dead id, the gate rolls the
   op back, and the arm is handed the *old* row, whose field names a live id.

   **The audit criterion**: an arm needs an explicit identity test **exactly when the target
   id does not appear in the op it emits**. `SetRow(P, subject, ∅)`, `SetSubject(S, None)`,
   `SetInterrogation(slot, W, ∅)` and `SetGroupList(GL, ∅)` all carry the target inside the
   op, so a wrong target is not even expressible and a plain lookup is the whole test. But
   `Remove(row)` and `Update(row, rebuilt)` name only the row; nothing ties them to the
   target, and the identity test is the only thing that does. This criterion is a *shape* a
   reviewer can check by eye, row by row, without reasoning about reachability — which is
   why it is stated here rather than argued case by case in the tables.

   Applied to §8.1, the rows that carry an identity test are: the four *row-removal* arms
   driven by a scalar field — `WeekPeriodFk` (`week.period_id == P`), `SlotSubject`
   (`slot.subject_id == S`), `IncompatSubject` (`incompat.subject_id == S`) and `SlotTeacher`
   (`slot.teacher_id == T`); the four *rule-part* arms — the two `PairingRule` parts
   (`RulePart::subject_id`) and the two `SlotPairingRule` parts (`SlotRulePart::slot_id`),
   each testing **its own** part, separate arms even though both parts emit the same
   `Remove(rule)` op, since a shared arm testing neither would delete a rule whose two parts
   are both live; the two *cleared-field* arms `SlotWeekPattern` and `IncompatWeekPattern`
   (`week_pattern != Some(WP) => None`, not `is_none()`); and `AssociationEntry` under a
   group-list target (the entry's assigned id vs `GL`). The element-removal rebuilds
   (`… minus P`, `… minus St`, the group-list fillings) satisfy the criterion for free: the
   membership test *is* the identity test.

   Two of the four scalar-field arms are, on today's code, unreachable — `SlotSubject`,
   because `force_apply_slot`'s `Update` keeps `CannotChangeSubject` (`slots.rs:465-471`) so a
   live slot's subject can never be rewritten; and `WeekPeriodFk`, because every path that
   sets `Week::period_id` keeps destination-period existence (`force_add_week`,
   `weeks.rs:563-566`; `force_move_week`, `:674-682`; `WeekOp::Update` carries a `WeekDesc`
   with no period at all). **They get the test anyway** (user ruling, July 28 2026): their
   unreachability rests on guards living in other files that nothing obliges to stay, the test
   is one comparison that cannot be wrong, and uniformity is what makes the criterion above
   checkable by shape instead of by argument. It is also a cheap net against a plain bug.

   **What catches a missing identity test**: the criterion above applied by eye, the
   commit-7.6 identity pins (§9ter.4 — end to end: `Err`, and the innocent row still there),
   and — mechanically, one test per variant — the **commit-7.5 `None` tests** (§9bis), which
   call `fix_invariant` directly on a *valid* document with an invariant derived from a
   corrupted twin. (The `Ok`-route fixtures cannot see it: on a legitimate route the target
   id *equals* the live field, so the exact op list comes out the same with or without the
   test.) The commit-8 property test cannot see it either. Point 5 says the rest: do not spend review time deciding what
   a given missing test would lead to — write it.

   Where the reference is part of the row's identity there is nothing to compare (a
   colloscope `(slot, week)` row cannot be about another slot), and a plain lookup remains
   the whole test.
5. **Pin the shape you are about to change, not merely its existence** (July 28 2026 review;
   this is point 4 generalised, and it governs §8.2 as well as §8.1). An invariant names an
   *offending configuration* — a row together with the field values that make it offending.
   The arm must confirm that this exact configuration is the one living in `self`. Testing
   only "the row is there" is not enough, because the failing op was **rolled back before
   `fix_invariant` runs**: the arm is looking at a state in which the row exists but is
   *innocent*. It then repairs the innocent row instead of rejecting a bad edit.

   The worked example, and the reason `Convergence` gains payload in commit 5.97: the target
   is `SlotOp::Update(S, slot with teacher = T2)` where `T2` does not teach the slot's
   subject. The state before the op is **fine**. The break is
   `SlotTeacherDoesNotTeachSubject`, the op rolls back, and an arm testing only "does `S`
   exist?" answers `Some(Remove(S))` — deleting a slot whose real teacher `T1` is perfectly
   valid. With the teacher in the payload the arm tests `live_slot.teacher_id == T2`, finds
   `T1`, and returns `None`: the bad edit is rejected, which is what should happen.

   **Corollary — the payload rule.** A variant that does not carry enough information to
   write that test must be enriched; commit 5.97 (§7bis) is the collection point. The test
   pins only the fields the *fix* is about to destroy, never the whole predicate: pinning a
   field the legitimate cascade route is expected to have changed would reject that route.
   `SlotOverflowsDay` is the case to remember — the arm tests `start` and deliberately does
   **not** test `duration`, because on the legitimate route (the subject's interrogation is
   lengthened) the live subject still holds the *old* duration while the live slot still
   holds the offending start.

   **Do not reason about what a missing shape test would lead to** (user ruling, July 28
   2026). Depending on the arm, the op vocabulary and prechecks living in other files, the
   downstream outcome can be a correct rejection reached wastefully, a rejection reporting
   the wrong thing, a contract panic, a non-terminating cascade, or a wrong `Ok`. Working out
   which one applies to a given arm is a zoology that rests on guards nothing obliges to keep,
   and it rots. The test costs one comparison: write it in every arm, always, and spend no
   review time deciding whether this one "needs" it. The same ruling covers arms whose `Some`
   branch is unreachable on today's code (§8.2 row 10) — `Op::GlobalUpdate` can carry states
   nobody foresaw, and an arm that cannot fire today may fire tomorrow.

**The engine's contract panic is not a safety net** (user ruling, July 28 2026). A fix op
that lands as a perfect no-op panics, and a fix op that trips a precheck panics too — both
are a crash in front of the user, not a repair. They are instruments for the commit-7/7.5/8
tests only. Correctness lives in the arms; never argue that a mistake is "caught anyway".

The table cells below give the op inside the `Some(...)` in the plain-op spelling for
readability; the presence check is implied (e.g. "the row exists", "the element is in the
set"), and per point 2 so is the `None` on any failed lookup. The full table, every arm
settled (rationale tags refer to D5):

### 8.1 `DanglingFk(Reference)` — by target kind and site

**Target: a period `P`** (`PeriodRefSite`, `refs.rs:90-108`):

| Site | Fix | Rule |
|---|---|---|
| `WeekPeriodFk(week)` | `[Week(WeekOp::Remove(week))]` | entity cannot survive (a week belongs to its period, `Week::period_id` is a bare id — D5.3); cascades further; presence = the week exists **and** `week.period_id == P` (frame point 4, defensive: unreachable today) |
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
| `SlotSubject(slot)` | `[Slot(SlotOp::Remove(slot))]` | `Slot::subject_id` is mandatory and authoritative (`SlotOp::Update` rejects changing it) — the slot cannot survive (D5.3); presence = the slot exists **and** `slot.subject_id == S` (frame point 4, defensive: unreachable today) |
| `IncompatSubject(incompat)` | `[Incompat(IncompatOp::Remove(incompat))]` | `Incompatibility::subject_id` is mandatory (D5.3); presence = the incompat exists **and** `incompat.subject_id == S` — **reachable**, `force_apply_incompat`'s `Update` replaces the whole row with no field guards (`incompats.rs:108-124`) |
| `PairingRuleAntecedent(rule)` / `PairingRuleConsequent(rule)` | `[Pairing(PairingOp::Remove(rule))]` | `RulePart::subject_id` is a bare id, so no half-rule exists (D5.3); **two arms, not one** — each tests its own part's subject against `S` (frame point 4) — **reachable**, `force_apply_pairing`'s `Update` has no field guards (`pairings.rs:237-247`) |
| `BalancingSubjectKey` | `[Balancing(BalancingOp::SetSubject(S, None))]` | drops the per-subject override; the subject falls back to `balancing.global` (needs commit 5.99, §7quater) |
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
| `SlotTeacher(slot)` | `[Slot(SlotOp::Remove(slot))]` | `teacher_id` is mandatory (`slots.rs:56-57`, no `Option`), so there is no teacher-less slot to fall back to (D5.3); naming a substitute teacher would be invented data; presence = the slot exists **and** `slot.teacher_id == T` — **reachable**, see below |

Note the contrast with `SlotSubject`, which removes the slot too but for a different reason:
there `SlotOp::Update` *cannot* express the change (`CannotChangeSubject`), here it can — the
teacher field is freely editable — and the map declines on the invented-data rule.

That same editability is what makes this row's identity test (frame point 4) load-bearing
rather than defensive: `force_apply_slot`'s `Update` keeps only `CannotChangeSubject` and
strips `validate_slot`, with no teacher-existence carve-out (`slots.rs:455-483`). So
`SlotOp::Update(slot, new_slot naming a dead teacher)` lands, the checker reports
`SlotTeacher`, the gate rolls back — and without the test the arm would delete a slot whose
live teacher is perfectly valid. This is also the most explosive fix in the table: one teacher
removal takes every one of their slots, and each slot removal then cascades to that slot's
colloscope cells and to any `SlotPairingRule` naming it (commit-7 fixture 2).

**Target: a student `St`** (`StudentRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `GroupListPrefilledStudent(gl)` | `[GroupList(GroupListOp::Update(gl, rebuilt))]` — `GroupList::new(params, filling with St removed)` | sealed rebuild; presence = `filling().contains_student(St)`; removing a member changes neither the group count nor introduces a duplicate, so `new()` cannot fail — `.expect` with that sentence |
| `GroupListExcludedStudent(gl)` | `[GroupList(GroupListOp::Update(gl, rebuilt))]` — `Automatic { excluded_students minus St }` | sealed rebuild; presence = `filling().excluded_students().contains(&St)`; `new()` validates only the `Prefilled` branch, so an `Automatic` rebuild cannot fail at all |
| `SettingsStudentKey` | `[Settings(SettingsOp::SetStudent(St, None))]` | drops the per-student override; the student falls back to `settings.global` (needs commit 5.98, §7ter) |
| `AssignmentsStudent { period, subject }` | `[Assignment(AssignmentOp::SetRow(period, subject, row minus St))]` — row read live | presence = the row exists and holds `St` |
| `ColloscopeGroupListStudent(gl)` | `[Colloscope(ColloscopeOp::SetGroupList(gl, placements minus St))]` | rewrite of the row read from `self.inner_data.colloscope.group_list(gl)`; presence = the row exists **and** places `St` |

Two notes on the student table.

The two group-list rows need no bespoke variant matching: `GroupListFilling::contains_student`
(`group_lists.rs:210`) returns `false` for an `Automatic` filling, and
`excluded_students()` (`:153`) returns a static empty set for a `Prefilled` one. So each arm's
single presence test already short-circuits to `None` when the live filling is the *other*
variant — which happens when the target was a `GroupListOp::Update` whose new value carried
the dead student while the live value is of the other kind. Each arm then rebuilds through
`into_parts()` (`:312`) and the existing mutators (`remove_student` at `:195`, or a plain
`BTreeSet::remove` on the excluded set).

The `ColloscopeGroupListStudent` row corrects the first draft, which said an absent row
"degrades to the empty map — a no-op clear". It must not: `SetGroupList(gl, ∅)` against a
state with no row for `gl` is a **perfect no-op**, which the engine answers with an
unconditional panic (D4). An absent row, and a row that does not place `St`, are both `None`.
Where the row *is* there, it is non-empty (canonical-absent), so removing `St` — even as the
last placement, which clears the row — is always a real change.

**Target: a week pattern `WP`** (`WeekPatternRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `SlotWeekPattern(slot)` | `[Slot(SlotOp::Update(slot, rebuilt))]` — the live slot cloned with `week_pattern = None` | presence = `slot.week_pattern == Some(WP)`, per frame point 4 (D5.4, deliberate divergence) |
| `IncompatWeekPattern(incompat)` | `[Incompat(IncompatOp::Update(incompat, rebuilt))]` — the live incompat cloned with `week_pattern_id = None` | presence = `incompat.week_pattern_id == Some(WP)` (idem) |

**These two rows were reversed at the July 28 2026 review**, and they are the map's one
deliberate departure from the legacy cleaning (D5.4). The first draft *removed* the slot and
the incompat, matching `DeleteWeekPattern` (`ops/src/week_patterns.rs:229-256`), on the
argument that clearing the optional field to `None` would silently widen the row to "every
week". The user overruled it, and on review the rule the map actually follows is D5.3 as now
stated: `Slot::week_pattern` (`slots.rs:65-67`) and `Incompatibility::week_pattern_id`
(`incompats.rs:48-50`) are both `Option`, with `None` a legal documented value meaning "every
week", so the reference *can* go alone and the row stays.

The reasons, in the order they were argued:

- **Repairability.** With the cascade wired to the UI, "the Math slot with M. Smith at 8:00 is
  now every week" is a message a user can act on; "all your slots disappeared" is not. And the
  common real case is a week pattern that had *become* every-week before being deleted, where
  clearing to `None` is not a widening at all — it is the identity.
- **Consistency.** Widening is not in fact disqualifying: every excluded-set arm in this map
  widens. A subject, a student, a week pattern or a pairing rule that stops excluding a dead
  period/week all end up applying more broadly than before. Deleting the referencing row here
  was the outlier, not the rule.
- **No invariant can break either way.** For slots, `InterrogationOnInactiveWeek(slot, week)`
  fires when an interrogation sits on a week the pattern deactivates; `None` deactivates
  nothing, so clearing can only ever *remove* instances of it. For incompats, no `Convergence`
  variant mentions an incompatibility at all — the checker never relates one to a colloscope.
  (An incompat can of course make a colloscope infeasible for the *solver*; that is not this
  layer's contract.)

Cost, accepted knowingly: the divergence forecloses a later differential fuzz of these two
arms against the legacy cleaning. Prechecks checked: `force_apply_slot`'s `Update` keeps only
`CannotChangeSubject` (`slots.rs:455-483`) — the rebuild clones the slot and touches one
field, so the subject is unchanged — and strips the colloscope pattern-compat guard.

**Target: a slot `Sl`** (`SlotRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `SlotPairingRuleAntecedent(rule)` / `SlotPairingRuleConsequent(rule)` | `[SlotPairing(SlotPairingOp::Remove(rule))]` | `SlotRulePart::slot_id` is a bare `SlotId` (`slot_pairings.rs:58-70`), so the reference cannot go alone (D5.3); **two arms, not one** — each tests its own part against the target (frame point 4) |
| `ColloscopeInterrogation { week }` | `[Colloscope(ColloscopeOp::SetInterrogation(Sl, week, BTreeSet::new()))]` | the slot is half the row key (`colloscopes.rs:24`), so clearing is forced (D5.3); presence = the row exists — canonical-absent, so a present row is non-empty and clearing is always a real change |

**Target: a group list `GL`** (`GroupListRefSite`):

| Site | Fix | Rule |
|---|---|---|
| `AssociationEntry { period, subject }` | `[GroupList(GroupListOp::AssignToSubject(period, subject, None))]` | targeted op; the reference is the entry's *value* (`subjects_associations: Table<(PeriodId, SubjectId), GroupListId>`, `group_lists.rs:31`), so it goes alone (D5.3); presence = the entry exists **and** names `GL` (frame point 4) |
| `ColloscopeGroupListKey` | `[Colloscope(ColloscopeOp::SetGroupList(GL, BTreeMap::new()))]` | the group list *is* the row key, so clearing is forced (D5.3); presence = the row exists — canonical-absent, so clearing is always a real change |

`AssignToSubject(.., None)` removes the association entry and **nothing else**: the
`GroupList` value itself survives (user ruling — an unreferenced group list is a legal state,
as already noted under the subject table). Prechecks checked: the `None` payload needs only
subject-exists and period-exists (`group_lists.rs:450-480` checks the group list only when the
payload is `Some`), and both hold because `self` is valid.

### 8.2 `Convergence` — all 16 variants

The checker semantics quoted per variant are `invariants.rs:417-630`; the fixes clear the
now-invalid data (design doc §3, tier 3 — lossy by nature). Unlike §8.1, every row here
spells out its **presence/shape test** — the test is the arm's real content, the op is only
its output (frame point 5).

**Rows 1-4 — the slot and teacher block** (reviewed July 28 2026; the payloads are the
post-commit-5.97 ones):

| Variant | Fix | Presence/shape test |
|---|---|---|
| `SlotTeacherDoesNotTeachSubject(slot, teacher, subject)` | `[Slot(SlotOp::Remove(slot))]` | the slot exists **and** `slot.teacher_id == teacher` **and** `slot.subject_id == subject` (the teacher comparison is the load-bearing one, the subject one defensive) |
| `TeacherSubjectWithoutInterrogations(teacher, subject)` | `[Teacher(TeacherOp::Update(teacher, teacher minus subject))]` | the teacher exists **and** `teacher.subjects.contains(subject)` |
| `SlotForSubjectWithoutInterrogations(slot, subject)` | `[Slot(SlotOp::Remove(slot))]` | the slot exists **and** `slot.subject_id == subject` (defensive) |
| `SlotOverflowsDay { slot, start, duration }` | `[Slot(SlotOp::Remove(slot))]` | the slot exists **and** `slot.start_time == start` — **never** `duration` (frame point 5's corollary) |

**Why the row dies in rows 1, 3 and 4, and survives in row 2.** The D5.3 structural test
decides all four. A slot's `teacher_id`, `subject_id` and `start_time` are bare mandatory
fields (`slots.rs:46-74`): the offending value cannot leave on its own, so the row goes.
`Teacher.subjects` is a `BTreeSet<SubjectId>` (`teachers.rs:33-34`): one element can leave
and the teacher stays valid, so only the element goes. `Teacher` is not sealed, so that
rebuild is a plain clone-and-edit — no `into_parts`.

**Row 3's `Some` branch is structurally shadowed** (found July 28 2026 while tracing commit
7's fixture 3 by hand). Any state where `SlotForSubjectWithoutInterrogations` fires holds a
slot whose subject has interrogations disabled. That slot's teacher either teaches the subject
— and then `TeacherSubjectWithoutInterrogations` also fires, declared *earlier* — or does not,
and then `SlotTeacherDoesNotTeachSubject` fires, declared earlier still. The third case — the
slot's teacher id itself dangles — makes the teacher-teaches check *skip* (`invariants.rs:428`
gates on the teacher lookup), but then the `SlotTeacher` dangle fires instead, and
`DanglingFk` is declared before every `Convergence` (`invariants.rs:207-209`). In every case
something declared earlier is in the set, and the engine picks only `set.first()` with no
fallback, so row 3 can never be the pick.
This holds through `Op::GlobalUpdate` too, since the argument is about the state, not the op.
Exactly as for row 10 below, that is **not** a reason to weaken the arm or to skip its shape
test — see frame point 5's closing ruling. It is recorded here so nobody spends an afternoon
trying to write a fixture that reaches it.

The repairs going the other way are all excluded by D5.1: granting the teacher the subject,
enabling interrogations on the subject, shortening the interrogation duration or moving the
slot's start time each *invent* data, and none of them shrinks the document.

**Legacy comparison.** Rows 1-3 match the old cleaning behaviour exactly:
`ops/src/teachers.rs:85-116` (`UpdateTeacher` deletes the slots of a teacher who lost the
subject), `ops/src/subjects.rs:338-362` (disabling interrogations unregisters the teachers)
and `:390-402` (…and deletes the slots). Row 4 has **no legacy cleaning at all**. The
overflow was only ever a *rejection*, and only on the slot side: `SlotWithDuration` was used
by the retired `validate_slot` (added in `b0c2c479`, deleted by step-5 R2 `56510199`), and
today only `ops/src/slots.rs` translates the invariant (`:368`, `:481` → `AddNewSlotError` /
`UpdateSlotError::SlotOverlapsWithNextDay`). On the subject-duration route `ops/src/subjects.rs`
has no arm for it, so the ops layer reaches its catch-all `panic!("Unexpected invariant
breaks …")` — the bug the user recalled. This is the second place, after the week patterns
of D5.4, where the map has nothing to compare against.

The new behaviour is not a *silent* deletion, either: the cascade collects the ops it applied
and step 7 shows that list to the user. "Your interrogation now lasts 90 min, so the Friday
18:30 slot was removed" is a preview, not a surprise.

**Rows 5-12 — the coordinate block, plus the pairing rule** (reviewed July 28 2026):

| Variant | Fix | Presence/shape test |
|---|---|---|
| `AssignmentForSubjectNotRunningOnPeriod(period, subject)` | `SetRow(period, subject, ∅)` | a row exists at `(period, subject)` |
| `AssignedStudentNotPresentForPeriod { period, subject, student }` | `SetRow(period, subject, row minus student)` | a row exists **and** contains `student` |
| `AssociationForSubjectWithoutInterrogations(period, subject)` | `[GroupList(GroupListOp::AssignToSubject(period, subject, None))]` | an entry exists at `(period, subject)` |
| `AssociationForSubjectNotRunningOnPeriod(period, subject)` | `[GroupList(GroupListOp::AssignToSubject(period, subject, None))]` | an entry exists at `(period, subject)` |
| `BalancingForSubjectWithoutInterrogations(subject)` | `[Balancing(BalancingOp::SetSubject(subject, None))]` | an override entry exists for `subject` (needs commit 5.99, §7quater) |
| `PairedSlotsNotInSameSubject(rule, ant_slot, con_slot)` | `[SlotPairing(SlotPairingOp::Remove(rule))]` | the rule exists **and** `rule.antecedent().slot_id == ant_slot` **and** `rule.consequent().slot_id == con_slot` (needs commit 5.97, §7bis) |
| `InterrogationSlotNotRunningOnPeriod(slot, week)` | `[Colloscope(SetInterrogation(slot, week, ∅))]` | a cell exists at `(slot, week)` |
| `InterrogationOnInactiveWeek(slot, week)` | `[Colloscope(SetInterrogation(slot, week, ∅))]` | a cell exists at `(slot, week)` |

**Seven of these eight are coordinate-shaped, and there the lookup is the whole test.** The
invariant names a coordinate — `(period, subject)`, `subject`, `(slot, week)` — the fix op
carries that same coordinate, and the fix removes the *whole* thing at it. So there is no
field left to compare: the offending shape simply *is* the presence of the row, the entry or
the cell. Row 6 is the one variation, and the membership test in the live row plays exactly
the same part (frame point 4: element-removal rebuilds carry their identity test for free).
An emptied `SetRow` / `SetInterrogation` removes the row outright, per the canonical-absent
contract (`assignments.rs:126-133`).

Rows 7 and 8 emit the same op. When both fire, the canonical pick takes row 7, the fix clears
the entry, and row 8 goes with it. `AssignToSubject(.., None)` removes the association only —
the `GroupList` value survives, as ruled in §8.1. Rows 5 and 6 interact the same way: a row
that is wholly invalid *and* holds excluded students clears in one step, because row 5 is
declared first.

**Row 10 needs the enrichment** and is otherwise settled: which of the two slots is "wrong"
is undecidable, and a `SlotPairingRule` is sealed with two mandatory parts, so a part cannot
leave alone (D5.3) — the rule goes. Note that on today's code the arm's `Some` branch cannot
fire (a slot's subject can never change, so a valid `self` holding that rule has both slots on
one subject). That is **not** a reason to weaken it: see frame point 5's closing ruling.

**Legacy agrees on all eight.** `SubjectsUpdateWarning::UpdatePeriodStatus`
(`ops/src/subjects.rs:426-530`) unassigns the students, clears the colloscope cells and drops
the group-list association when a subject stops running on a period; the interrogation-disable
path clears the association (`:363-388`) and the balancing override (`:405-418`). For rows 11
and 12 the reference is `ops/src/week_patterns.rs`: `UpdateWeekPattern` clears the newly
excluded cells one by one, exactly as the map does, and `ops/src/slots.rs:163-217` does the
same when a slot's own pattern changes. The only difference anywhere in this block is
granularity — legacy unassigns one student per cleaning op where commit 4's `SetRow` clears
the row in one.

**Rows 13-16 — the colloscope block** (reviewed July 28 2026; row 16's payload is the
post-commit-5.97 one). Three facts hold for all four rows, and every arm below rests on
them.

The colloscope is sparse and canonical (`colloscopes.rs:20-28` and `:84-100`): a row exists
**iff** it is non-empty. So `colloscope.interrogation(slot, week)` and
`colloscope.group_list(gl)` return `Some` only for a non-empty row, and a write of an empty
set or map removes the row. Presence therefore already means "non-empty", and an emptying
write can never be a no-op — no arm here can trip the contract panic that way.

Both writers keep a coordinate precheck (`colloscopes.rs:131-140`): `SetGroupList` needs a
live group-list id, `SetInterrogation` a live slot and week. That is safe, because colloscope
rows are walked by the refs registry (`refs.rs:497-515`) — a valid `self` cannot hold a row
keyed by a dead id, so a passing presence test guarantees a passing precheck.

The shape test is the presence test in all four rows, and never touches the params side. The
reasoning is worth stating once, because at first sight frame point 5 seems to ask for more.
Take row 14: the offending configuration is "the list is prefilled **and** a row exists", and
two different edits can create it. If the op writes a row onto an already-prefilled list, the
pre-op state has no row, the test fails, and the engine convicts the op — the right answer.
If the op flips the list to prefilled while a row exists, the pre-op row is a real, innocent
row, the arm clears it, and the retry succeeds — which is exactly what legacy does
(`ops/src/group_lists.rs:651-675`). So testing prefilled-ness on `self` would be *wrong*: it
would reject an edit legacy accepts. Rows 15 and 16 have the same two routes. The rule that
keeps all of them honest is the one already engraved in the frame — pin what the fix
destroys, never the predicate that makes it offending (D2).

| Variant | Fix | Presence / shape test on `self` |
|---|---|---|
| `InterrogationGroupOutOfBounds(slot, week, group)` | `Colloscope(SetInterrogation(slot, week, cell minus group))` | `interrogation(slot, week)` is `Some(cell)` **and** `cell.contains(&group)` |
| `ColloscopeGroupListPrefilled(gl)` | `Colloscope(SetGroupList(gl, ∅))` | `group_list(gl).is_some()` — presence is the whole test |
| `ColloscopeStudentExcluded(gl, student)` | `Colloscope(SetGroupList(gl, placements minus student))` | `group_list(gl)` is `Some(p)` **and** `p.contains_key(&student)` |
| `ColloscopeStudentGroupOutOfBounds(gl, student, group)` | `Colloscope(SetGroupList(gl, placements minus student))` | `group_list(gl)` is `Some(p)` **and** `p.get(&student) == Some(&group)` |

Row by row:

- **Row 13** needs no enrichment — commit 5 already put the `u32` there. The arm must **not**
  re-check the bound: a group-list shrink legitimately needs this trim even though the group
  was in bounds a moment ago (D2 again). If the trim empties the set the row disappears,
  which is what we want.
- **Row 14** clears the row in a single op where legacy removes the students one at a time
  (`group_lists.rs:651-675`). Deliberate divergence, same fixpoint, fewer rounds, and a
  shorter op list to show the user. There is no single element to blame here — for a
  prefilled list the offending thing *is* the whole row.
- **Row 15** must not look at the filling's excluded set. Adding a student to that set has to
  clean the placement (legacy: `group_lists.rs:618-648`); placing an already-excluded student
  has to be rejected. The presence test gives both.
- **Row 16** matches the legacy shrink cleaning (`group_lists.rs:374-389`) and is the fourth
  variant of commit 5.97 — see §7bis for why the group number is needed and what it buys.

Implementation notes:

- Every "minus" value and presence/shape test is computed from `self` (the live pre-fix
  state); an arm finds its removable material absent only in self-caused routes, and then
  returns `None` — the engine convicts the failing op (D4). An arm that misjudges presence
  and emits a no-op fix trips the engine's contract panic, which the commit-8 fuzz is meant
  to hit before production — but that panic is a test instrument, not a safety net (see the
  frame), and the systematic check on presence and shape is commit 7.5.
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
`Data::annotate`, and drive `apply_cascade` on it directly.

**Status (July 28 2026).** Landed: `1a` (`32b64bb8`), `1b`–`1e` (`df9357a2`), `2`
(`a9201341`), `3` (`ba82ac5b`), `4` (`ea73e700`), `5a`/`5b` (`cd2eb958`), `6` (`62816871`).
**§9 is complete.** Every fixture passed on its first run, so no map bug surfaced anywhere in
the scenario suite. What remains of commit 7 is §9bis (commit 7.5) and §9ter (commit 7.6).

Three descriptions below needed correcting, all marked **★ CORRECTION** in place. Two were
found at implementation: `1b`'s document is not constructible as described, and scenario 2
needs two slot pairing rules rather than one. The third was found *before* implementation, at
the discussion that opened scenario 5 — the plan's account of what `None` means was plain
wrong, and it would have mis-shaped target A's semantic assertion. See the note under target A.

Scenarios 3 and 4 held up as written. Scenario 3 carries two **★ ADDITION**s (things the
fixture needed in order to *see* what it asserts, not changes to its trace); scenario 4 needed
nothing at all.

The `1b` correction states a constraint that binds every fixture wanting a colloscope cell:
the cell needs a group list associated to its `(period, subject)`, and that association is
itself a live period reference. It duly bound fixture 2 and scenario 5. It was written as
binding scenario 3 too; it does not — scenario 3 has no cell, and its association is there
because the scenario asks for one.

Three rules apply to the whole section, settled at the July 28 2026 review.

**Expected op lists are written by hand first.** Every fixture below asserts something about
the ops that landed. That expected list must be derived on paper from the §8.1 / §8.2 tables
*before* the test is run, and only then compared with what the engine actually produced. If
the two differ, the difference is a finding to explain — possibly a map bug — never a value to
paste. Running the test first and pasting its output turns every one of these fixtures from a
correctness pin into a regression pin that freezes whatever happens to be there, right or
wrong.

**Sequence versus content.** An assertion on the *order* of the landed ops is only meaningful
where a choice was actually made, i.e. where a single failing apply reported more than one
broken invariant and the engine picked `set.first()` out of the `BTreeSet`. Where every round
reports exactly one break, the sequence is forced by the data and asserting it pins depth, not
order. So: fixtures 1a and 1b assert the literal sequence, for those two different reasons;
every other fixture asserts **content** — the exact length of `applied.inner()` plus a
`contains` for each expected op — and is deliberately blind to order. `AnnotatedOp` derives
only `Debug, Clone, PartialEq, Eq` (`ops.rs:311`), so a sorted-vector comparison is not
available and `Ord` is not worth adding for a test; length plus `contains` catches an extra,
a missing and a wrong op, and the one case it misses (a duplicate paired with an omission)
cannot occur — a fix landing twice would be a perfect no-op and the engine would panic first.

(★ Amended July 28 2026. The second of "those two different reasons" no longer holds as
written: `1b`'s round 1 turned out to report two breaks, not one — see the correction under
`1b` below. `1b` still asserts its literal sequence, but that sequence is now forced by the
canonical pick order as well as by the stack, so the two reasons overlap instead of being
disjoint. `1a` remains the fixture whose *job* is the pick order, on a two-op diff.)

This is a reversal of the first draft, which had the flagship assert its full sequence
literally and called that "the design-doc §8 confluence pin". It is not one. Confluence means
*the final state does not depend on which break is picked first*, and a frozen path does not
test that; the engine hardcodes `set.first()`, so confluence cannot be varied and tested
directly at all. What an ordered list really buys is a tripwire on the derived `Ord` of
`FixableInvariant` — a reorder, or a new variant inserted in the middle during step 7, changes
the picks. Fixture 1a provides that tripwire on two ops, where the failure is readable in five
seconds. The flagship's version of the same signal is a twelve-line diff whose only practical
response is to paste the new list, which is precisely the failure mode the first rule forbids.

**A known coverage limit, stated as a decision.** Commit 7.5 (§9bis) tests the `None` branch
of *every* arm, systematically, one test per arm. Nothing tests the `Some` branch
systematically. The `Some` branches are covered by whichever fixtures below happen to walk
through them, plus whatever commit 8's random walk happens to hit. A second forty-six-test
series mirroring 7.5 was considered and rejected as more weight than it buys. The asymmetry is
recorded here so that it is a decision rather than an oversight.

Scenarios:

1. **The period-removal family.** The first draft had a single "flagship deep cascade" fixture
   carrying order, depth and breadth at once. It is split into five, so that a failure names
   its own cause; the flagship remains as the integration test that catches interactions the
   other four isolate away.

   1. **Order** (`1a`). The minimal fixture in which the engine genuinely *chooses*: a period
      `P` excluded by one subject and by one student, and referenced by nothing else. One
      round, two simultaneous breaks — `SubjectExcludedPeriods` and `StudentExcludedPeriods` —
      whose fixes are two *different* ops. Assert `Ok` and the **literal sequence**
      `[Subject(Update(.., minus P)), Student(Update(.., minus P)), Period(Remove(P))]` or its
      transposition, whichever the enum's declaration order dictates. This fixture, and only
      this one, pins the canonical pick order.
   2. **Depth** (`1b`). The minimal chain: a period with exactly **one** week (two would give
      two simultaneous breaks and drag order back in), one slot with one colloscope cell on
      that week, and no week pattern excluding it. Round 1: `PeriodOp::Remove` breaks
      `WeekPeriodFk(w)`, fix `Week(Remove(w))`. Round 2: that fix breaks
      `ColloscopeInterrogation { slot }`, one break, fix `SetInterrogation(slot, w, ∅)`. Round
      3: the clear lands, the week removal is retried and lands, the period is retried and
      lands. This is a fix of a fix of the target — depth three, which no engine test reaches
      (the toy tests stop at depth two).

      **★ CORRECTION, found at implementation (commit `df9357a2`, July 28 2026). The document
      as described above is not constructible, and the fixture lands four ops, not three.** The
      paragraph asked for a document where *every* round reports exactly one break, and closed
      with "because each round has a single break, the sequence is forced". Round 1 has two.

      A colloscope cell must be non-empty — an empty one is canonical-absent, so writing it
      removes the row — and `InterrogationGroupOutOfBounds` bounds every group number in a cell
      by the group count of the group list associated to `(the week's period, the slot's
      subject)`. **With no association the bound is 0**, so *any* group number is out of bounds
      and the cell cannot be filled at all. The cell therefore forces an association on
      `(P, subject)`, and that association is itself a seventh period reference site
      (`AssociationEntry`). Round 1 breaks `WeekPeriodFk(w)` *and* `AssociationEntry`.

      The literal sequence to assert is therefore

      ```
      [Colloscope(SetInterrogation(slot, w, ∅)), Week(Remove(w)),
       GroupList(AssignToSubject(P, subject, None)), Period(Remove(P))]
      ```

      and the fixture's reason for existing is untouched: `PeriodRefSite` declares
      `WeekPeriodFk` before `AssociationEntry`, so the week is still picked first and the
      depth-three chain runs to completion before the association is cleared. What is lost is
      only the "one break per round" property, so the sequence is now forced by the canonical
      pick order as well as by the stack — which is `1a`'s subject, and is why `1a` remains the
      fixture that pins it.

      **This constraint is general and binds every later fixture that wants a colloscope
      cell**: the cell needs a group list associated to its `(period, subject)`, and that
      association is a live period reference. It duly bit fixture 2 (commit `a9201341`), where
      the association is present purely to make the cell fillable. Scenarios 3 and 5 need it
      too.
   3. **Breadth** (`1c`). A period referenced from **all seven** of its sites at once
      (`WeekPeriodFk`, `SubjectExcludedPeriods`, `StudentExcludedPeriods`,
      `PairingRuleExcludedPeriods`, `SlotPairingRuleExcludedPeriods`, `AssignmentsKey`,
      `AssociationEntry`). Assert `Ok`, content (not sequence), a clean final state, and that
      every referencing row is gone or updated. Note what this does *not* catch: an arm missing
      from the map is a compile error already, since the match is total with no wildcard. What
      it catches is an arm that wrongly answers `None`, or emits the wrong op — including the
      two sealed rebuilds (`PairingRule` / `SlotPairingRule`), whose `.expect` is exercised here
      for the first time.
   4. **Confluence on one op** (`1d`). Two different broken invariants whose arms emit the
      *same* fix. `invariants.rs:617-628` runs two independent `if`s over the same placement,
      with no `else`, so one `GroupListOp::Update(gl, new_list)` that both shrinks
      `params().group_names` **and** adds a student to `excluded_students` makes both
      `ColloscopeStudentExcluded(gl, st)` and `ColloscopeStudentGroupOutOfBounds(gl, st, g)`
      fire against a live colloscope row placing `st` at group `g`. Per §8.2 both arms emit
      `SetGroupList(gl, placements minus st)`. Assert `Ok` and exactly two landed ops,
      `[SetGroupList, target]` — whichever break is picked, one fix kills both, the retry
      succeeds, and no second fix is ever requested. That last point is the reason the fixture
      earns its place: a redundant second fix would apply as a perfect no-op and the engine
      would panic.
   5. **The flagship** (`1e`). `1c`'s document, plus depth: the weeks carry colloscope cells,
      the subject has a teacher and a slot on a week pattern, and there are assignments and a
      group-list association. Target `PeriodOp::Remove`. Assert `Ok`; assert **content** —
      length plus `contains`, order-insensitively, per the rule above; assert the final state
      is clean (`broken_invariants() == Ok(∅)`, direct call) and that the period, its weeks and
      every referencing row are gone.
   *(An undo round-trip fixture stood here in the first two drafts and was dropped at the
   July 28 2026 review, deliberately and with nothing kept in its place. It tested only that
   the collected reverses replay in reverse order, and every part of that is already pinned
   elsewhere: `property_ops.rs`'s Property 4 (`apply_then_apply_rev_is_identity`, `:208`) and
   Property 2 (`undo_all_and_redo_all_round_trip`, `:119`) cover per-op reversal and whole-
   history composed undo at 100 seeds × 1000 ops against real colloscope data; `history.rs:494`
   pins `AggregatedOp::rev`'s ordering; fixtures `1a`/`1b` pin the order of `applied` itself;
   and the `forward`/`backward` pairing in `cascade.rs:97-100` is generic engine code already
   covered by the `QuoteData` toy test. The two residues we looked for do not exist either: a
   mispaired reverse is caught by that toy test, and the undo path revisits exactly the forward
   run's intermediate states in reverse, every one of which the gate already validated.)*
2. **Teacher removal.** A teacher with two slots, one carrying colloscope cells and one paired
   by a `SlotPairingRule`. Target: `TeacherOp::Remove`. Removal cascades to two slot removals,
   each of which opens its own sub-cascade — cell clears for the first, the pairing-rule
   removal for the second.

   **★ CORRECTION, found at implementation (commit `a9201341`, July 28 2026): "a
   `SlotPairingRule`", singular, is wrong — the scenario needs two.** The sentence above and
   the coverage paragraph below cannot both hold with one rule. Full `SlotRefSite` coverage
   wants the removed teacher's slots to appear once as an antecedent and once as a consequent;
   the construction note below forbids putting them on the two sides of the *same* rule; so
   each of them needs its own rule, both pairing with the second teacher's slot. The fixture
   lands **six** ops: two rule removals, one cell clear, two slot removals, the target.

   This is not a duplicate of `1c`, and the difference is worth stating because it is not
   obvious. `1c` is breadth **at the root**: seven fixes hanging directly off the target, all
   flat. This is breadth **below** the root: one break round yields two `SlotTeacher` fixes,
   and each of those slot removals then fans out again. The stack gets wider at depth two,
   which neither `1b` (one break per round by construction) nor `1c` (all fixes flat) ever
   does.

   It also completes the site coverage of two more target kinds, checked against `refs.rs`
   rather than against the §8.1 tables. `TeacherRefSite` has exactly one variant,
   `SlotTeacher(SlotId)` (`refs.rs:147-150`), and `SlotRefSite` exactly three —
   `SlotPairingRuleAntecedent`, `SlotPairingRuleConsequent`, `ColloscopeInterrogation { week }`
   (`refs.rs:182-191`). So this fixture covers every teacher site and every slot site, provided
   the removed teacher's slots appear once as an antecedent and once as a consequent. Together
   with `1c` (all seven period sites) that gives the suite full site coverage for three target
   kinds.

   Assertions, the same four as elsewhere: `Ok`; **content**, not sequence (two simultaneous
   `SlotTeacher` breaks mean the engine genuinely picks, and pinning that pick is `1a`'s job);
   a clean final state; and the specific rows gone — both slots, their cells, the rule.

   Two construction details, both load-bearing.

   Give the fixture a **second teacher with their own slot**, and assert that slot is still
   there at the end. §8.1 calls `SlotTeacher` the most explosive fix in the map — one op
   deleting a whole teacher's timetable — and when a fix is that destructive the thing worth
   pinning is what it leaves alone as much as what it removes. This comes almost free: let the
   `SlotPairingRule` pair *our* teacher's slot with the *second* teacher's slot. One entity then
   buys both the pairing-rule cascade and the innocent-bystander assertion.

   Do **not** pair the removed teacher's two slots with each other. The first slot removal
   would take the rule with it, and by the time the second slot is removed there would be no
   rule left to break — the two-arm coverage would collapse to one arm, silently and with the
   test still green.

   That is a coverage argument and nothing more. **Pairing two slots of the same teacher is
   perfectly legal**; `SlotPairingRule::new` (`slot_pairings.rs`) refuses only a rule whose two
   parts name the *same slot*, and its doc calls that "the only value-internal invariant". The
   note is recorded here because the implementation write-up first stated the restriction as if
   it were a validity limit, which it is not.
3. **Subject update turning interrogations off.** A subject with interrogations enabled, a
   teacher who teaches it, a slot on it, a group-list association and a balancing override.
   Target: `SubjectOp::Update` turning interrogations off.

   Every fixture before this one walks `DanglingFk` sites. This one walks a `Convergence`
   family, which is a different axis of the map: nothing dangles, every reference is live, and
   what is wrong is a *relation* between two live rows.

   **The round-by-round trace, done by hand as the section's first rule demands — and it does
   not match the first draft's one-line description.** `Convergence`'s declaration order is the
   canonical pick order (`invariants.rs:141-142` states this) and runs
   `SlotTeacherDoesNotTeachSubject`, `TeacherSubjectWithoutInterrogations`,
   `SlotForSubjectWithoutInterrogations`, `SlotOverflowsDay`, …,
   `AssociationForSubjectWithoutInterrogations`, …, `BalancingForSubjectWithoutInterrogations`.

   - **Round 1.** The target breaks four invariants. `SlotTeacherDoesNotTeachSubject` does *not*
     fire — the teacher still teaches the subject. So the pick is
     `TeacherSubjectWithoutInterrogations`, and the fix is `Teacher(Update(t, minus S))`.
   - **Round 2.** That fix is applied and **fails the gate itself**: with the subject gone from
     the teacher, the slot's teacher no longer teaches the slot's subject, so
     `SlotTeacherDoesNotTeachSubject` breaks. Its arm removes the slot.
   - **Round 3 onward.** The slot removal lands; the teacher trim is retried and lands; the
     target is retried, and the association and balancing breaks clear one per round in
     declaration order; then the target lands.

   Two consequences, and the first is the reason to keep this fixture.

   It is the suite's only **fix op that is itself rejected and cascades further**, and the chain
   is `Convergence → Convergence`. `1b` and fixture 2 both cascade through dangling references.
   The intermediate state here — a teacher who has dropped a subject while a slot of theirs
   still runs on it — is one no user action can produce directly, which is exactly where a map
   bug would hide.

   And `SlotForSubjectWithoutInterrogations` is **never picked**: the slot is already gone,
   removed by a different arm. That is not an artefact of this fixture but structural, and
   §8.2's row 3 now records the argument.

   Assertions: `Ok`; **content**, not sequence (round 1 has four simultaneous breaks, so the
   engine genuinely picks, and pinning that pick is `1a`'s job); a clean final state; and the
   landed set is **five** ops, not four — the extra one being the slot removal attributed to
   `SlotTeacherDoesNotTeachSubject`. This is the first fixture that requires **commit 5.99**
   (§7quater): `BalancingOp::SetSubject(subject, None)` does not exist before it.

   **★ ADDITION, made at implementation (commit `ba82ac5b`, July 28 2026).** The trace above
   was confirmed on the first run, against the op list derived by hand from the §8.2 table
   beforehand, so nothing in the description needed correcting. The document grew by two rows
   all the same, and neither of them touches the trace: they are both about what the
   assertions can *see*.

   - **A second subject `S2`, also taught by the teacher `t`.** It keeps its interrogations, so
     it fires nothing and takes no part in the chain. What it buys is the shape of the teacher
     fix. §8.2's row 2 claims the offending *element* leaves and the teacher survives —
     `Teacher::subjects` is a set — but with a single-subject teacher the fix produces the
     empty set, which is indistinguishable from an arm that cleared the whole thing. The
     expected `AnnotatedTeacherOp::Update` is compared whole, so `S2` still sitting in it is
     the assertion that separates the two readings. This is the same move as scenario 2's
     second teacher and scenario 5's `WP2`, applied to a *field* rather than to a row.
   - **A balancing override that is not `BalancingOptions::default()`.** With the default
     value the override is byte-equal to the global options, so clearing it changes nothing
     observable and `options_for(subject)` returns the same thing before and after. A distinct
     override makes the removal a real change, lets the global options be asserted untouched
     (the fix is `SetSubject`, not `SetGlobal`), and lets the *semantics* be asserted rather
     than the table entry: the subject falls back to the global options at the end.

   One more thing worth recording, since the paragraph above predicted it and the fixture is
   where it shows: `SlotForSubjectWithoutInterrogations` does fire in round 1 and is never
   picked, because by the time the engine could reach it the slot is already gone. The fixture
   asserts a landed set of exactly five, so an arm that started emitting a second slot removal
   would fail here — which is the closest thing §8.2 row 3's shadowing argument has to a test.
4. **Student removal** (added July 28 2026). Target: `StudentOp::Remove`. `StudentRefSite`
   (`refs.rs:152-…`) has five variants and this fixture covers all five at once, which needs
   three group lists — a filling is either `Prefilled` or `Automatic`, so one list cannot play
   two of the roles:

   - `gl1`, **prefilled**, one of whose groups contains the student → `GroupListPrefilledStudent`;
   - `gl2`, **automatic**, excluding the student → `GroupListExcludedStudent`;
   - `gl3`, **automatic**, *not* excluding the student, carrying a colloscope row that places
     them → `ColloscopeGroupListStudent`. It must be a third list: placing the student in `gl2`
     would break `ColloscopeStudentExcluded`, and in `gl1` would break
     `ColloscopeGroupListPrefilled`, so either would test something else by accident.
   - a per-student settings override → `SettingsStudentKey`;
   - an assignments row holding the student → `AssignmentsStudent { period, subject }`.

   The reason this fixture is worth its weight is `gl1` and `gl2`. Their arms are the two
   **sealed `GroupList::new` rebuilds** of §8.1, each carrying an `.expect`. Commit 7.5 tests
   only their `None` branch, and no other fixture reaches a group list at all — so without this
   one, those two `.expect`s are never executed by any test in the suite. (`1c` covers the other
   two sealed rebuilds, `PairingRule` and `SlotPairingRule`.) It is also the only fixture
   exercising `SettingsOp::SetStudent` from **commit 5.98**, so it requires that commit as well.

   Assertions: `Ok`; **content** (five simultaneous breaks); a clean final state; the student
   gone from all five places; and a **second student**, present in the same prefilled group, the
   same assignments row and the same colloscope row, still there and untouched at the end.
5. **The week-pattern family** (added July 28 2026, detailed at the review of the same day).
   Two targets on one document: `WeekPatternOp::Remove`, which is the map's one deliberate
   divergence from the legacy cleaning (D5.4), and `WeekPatternOp::Update`, which is the
   legacy-agreement case. They are kept in one scenario precisely so the two sit side by side:
   *when the pattern narrows the map does what legacy does; when the pattern disappears it
   deliberately does not.* A reader who wonders why the removal case looks strange gets the
   answer in the next paragraph.

   **The document.** A week pattern `WP` excluding some weeks, used by one slot and one
   incompatibility. The slot carries a colloscope cell on a week `WP` allows. And — the part
   the first draft was missing — a **second pattern `WP2`**, with its own slot and its own
   incompatibility.

   `WP2` is the innocent bystander, and without it the fixture is much weaker than it looks.
   Both arms test `slot.week_pattern == Some(WP)` (resp. `incompat.week_pattern_id`) before
   clearing, per frame point 4. If every pattern-bearing row in the document points at `WP`,
   that comparison passes trivially and the fixture cannot see it at all: a map that ignored
   the test and cleared *every* row's pattern would pass. This is the same move as scenario 2's
   second teacher and scenario 4's second student.

   **Target A — `WeekPatternOp::Remove(WP)`.** Two breaks in one round, `SlotWeekPattern(slot)`
   and `IncompatWeekPattern(incompat)`, whose fixes are independent. **Content, not sequence** —
   the order here would teach nothing that `1a` does not already pin. Assertions:

   - `Ok`, and `applied.inner()` of length **exactly three**. Depth one, breadth two, nothing
     deeper. That length is the concrete form of §8.1's argument that clearing to `None` can
     only ever *remove* instances of `InterrogationOnInactiveWeek`, never create one; if a
     future change made widening break something, this is where it surfaces.
   - the two fix ops, compared **whole**. `SlotOp::Update` carries an entire `Slot` value, so
     asserting the exact op pins that *only* `week_pattern` moved — an arm that rebuilt the
     slot from something else, or reset another field on the way, is caught here. Same for the
     incompat. "The row survives intact" is the whole claim of the divergence, so the test has
     to check the whole row, not one field.
   - the slot and the incompat both **still present**, with the field `None`. The legacy
     cleaning would have deleted both (`ops/src/week_patterns.rs:229-256`).
   - `WP2`, its slot and its incompatibility **byte-identical** at the end.
   - the colloscope cell **still there**: widening destroys nothing.
   - the semantics, not merely the field — and as a **before/after flip**, not as a final
     value. Take `w`, a week `WP` excluded: `is_interrogation_possible(slot, w)` is `false`
     before the cascade and `true` after (`colloscope_params.rs:59`; the underlying
     `is_week_active(week, None)` is `:45`). Asserting `slot.week_pattern == None` would say a
     field moved; it would not say the slot got *wider*, which is the whole claim of the
     divergence. And a bare "true at the end" could pass for a reason unrelated to the map,
     where the flip cannot.

     **★ CORRECTION, July 28 2026 (user ruling, before implementation).** The sentence that
     stood here — "if `None` ever stopped meaning *every week*, this assertion is the one that
     should scream" — was wrong, and wrong in a way that would have mis-shaped the assertion.
     `None` does not mean "every week". It means "no pattern excludes this slot".
     `is_week_active` is a **conjunction**: the week must run interrogations *and* not be
     excluded by the slot's pattern. Clearing the pattern drops the second conjunct only, so
     `None` means "every week that runs interrogations".

     The consequence for the fixture is a **non-goal**: it does not check that a week with
     `interrogations: false` stays impossible once the pattern is gone. That would be testing
     `is_week_active`, which is `colloscope_params`' business and has its own tests. A cascade
     fixture stops at: the map cleared the field, and clearing it widened the slot.

   **Target B — `WeekPatternOp::Update(WP, excluded_weeks + w)`**, with the slot's colloscope
   cell sitting on `w`. One break, `InterrogationOnInactiveWeek(slot, w)` (§8.2 row 12), fix
   `SetInterrogation(slot, w, ∅)`, then the update lands. Assert `Ok`, two ops, the cell gone
   and the pattern updated.

   Target B is here because **no other commit-7 fixture reaches §8.2 row 12** — checked scenario
   by scenario at the review. `1b` and `1e` do clear colloscope cells, but through the
   `ColloscopeInterrogation` dangling-FK arm on week removal, which is a different arm
   entirely. Without target B the row is covered only by commit 7.5's `None` branch and by
   whatever commit 8's random walk happens to hit. It is also the legacy-agreement pin:
   `UpdateWeekPattern` (`ops/src/week_patterns.rs:200-226`) clears exactly the newly excluded
   cells, one at a time, which is what the map does.

   **Why this scenario carries more weight than its size suggests.** Everywhere else in the map
   a future differential fuzz against `ops/` could catch a drift. Target A's behaviour disagrees
   with legacy on purpose, so that check is foreclosed forever. This fixture is the only thing
   standing between the D5.4 decision and a quiet regression back to deletion.
6. **A no-op target lands, and does not panic** (rewritten July 28 2026). Target:
   `SlotOp::Update(slot, the identical slot)`. Assert `Ok`, `applied.inner()` of length 1, and
   the document unchanged.

   This replaces a one-line "clean target lands alone" fixture — a benign op cascading to
   exactly `[itself]` — which was dropped at the review as testing nothing new. When the target
   breaks nothing the map is **never consulted**, so that fixture never touched the code commit
   6 adds; the engine's fast path is already toy test 3 (§5), and "an ordinary edit does not
   trip the checker" is what `property_apply_gate.rs` and the rest of the suite do all day.

   What is left is worth keeping, because it guards a deliberate carve-out that nothing else
   touches. `cascade.rs` computes the no-op snapshot only for fixes:

   ```rust
   // Snapshot for the no-op-fix panic; only fix ops are held to it (a
   // no-op *target* is a legitimate perfect no-op, G.2).
   let before = (!is_target).then(|| data.clone());
   ```

   The strict-monotonicity panic is skipped for the target on purpose, because the gate accepts
   perfect no-ops (the G.2 widening). Turn that line into an unconditional `data.clone()` and
   every no-op target starts panicking — and today no test would notice. The toy tests do not
   cover it, and `property_apply_gate.rs` exercises the gate, not the cascade.

   **★ VERIFIED at implementation (commit `62816871`, July 28 2026).** The paragraph above is
   an argument about a mutation, so it was run rather than trusted. Replacing line 83 with
   `Some(data.clone())` makes this fixture — and only this fixture — fail, with the
   strict-monotonicity panic quoted verbatim; the other ten fixtures in the file and the 56
   unit tests of `collomatique-state` all still pass. The reason no other test can notice is
   stronger than "they exercise something else": `apply_cascade` is called from exactly two
   places in the repository, `state/src/cascade.rs`'s own toy tests and
   `state-colloscopes/tests/cascade.rs`. Every other suite, `property_apply_gate.rs` included,
   is structurally incapable of reaching the carve-out. The mutation was reverted before the
   fixture was committed.

   An identical `Slot` is the right op for it: unambiguously a no-op, and clear of the
   canonical-absent rules that make an emptying colloscope or assignments write a real change.

Two scenarios stood here through the earlier drafts and were moved out at the July 28 2026
review, into their own commit 7.6 (§9ter): the **self-caused rejections** and the
**collateral-damage identity pins**. Both convict the target and land nothing, both read the
`None` branch of an arm end to end, and that branch is only trustworthy once commit 7.5 has
tested it systematically. What is left in §9 is exactly the fixtures that assert `Ok`.

## 9bis. Commit 7.5 — the innocent-state `None` tests

Adopted during the July 28 2026 review, as the mechanical detector for frame point 5. One
test per **arm** — every `Convergence` variant and every `Reference` site, counting a
two-part site as the two arms it really is — all of the same shape (the split into ten
commits is §9bis.1):

```rust
// 1. A valid fixture, built through the public surface.
let valid: Data = /* ops through `Manager::apply`, then `get_data().clone()` */;

// 2. Its corrupted twin, carrying exactly the offending shape. An `InnerData`,
//    never a `Data`: `from_inner_data` validates and would reject it (`lib.rs:453`).
let mut corrupt = valid.get_inner_data().clone();
/* move the slot's start to 18:30, in place */

// 3. The invariant, *derived* from the twin — never hand-written.
let set = corrupt
    .broken_invariants()
    .expect("the corruption is fixable, not a logic error");
assert_eq!(set, BTreeSet::from([FixableInvariant::Convergence(expected)]));
let invariant = set.into_iter().next().unwrap();

// 4. The point: the *valid* document holds nothing that causes it.
assert_eq!(valid.fix_invariant(&invariant), None);
```

Why this shape:

- **It tests the arm, not the engine.** No `apply_cascade`, no rejection semantics, no
  rollback reasoning. Step 4 is literally the question frame point 5 asks.
- **Step 3 derives the invariant instead of hand-writing it.** That keeps the test from
  drifting away from the checker, and it survives commit 5.97's payload changes — the
  expected literal has to be updated with them, instead of the test quietly asserting `None`
  about an invariant nobody meant.
- **`assert_eq!` on the whole set, not `contains`.** It pins that the corruption is
  *surgical*: one edit, one broken shape. A corruption that breaks two things at once makes
  a muddy test.

  **Two arms cannot meet the one-element form, and get a stated exception** (found at the
  July 28 2026 second review). For `SlotSubject`, *any* state where `slot.subject_id`
  dangles co-breaks something: the teacher-teaches check gates only on the *teacher*
  lookup — an id used as a compared value deliberately does not gate
  (`invariants.rs:410-414`) — so a live teacher never `contains` the dead subject and
  `SlotTeacherDoesNotTeachSubject` fires beside the dangle; making the teacher dead
  instead merely swaps that companion for the `SlotTeacher` dangle. And for
  `SlotForSubjectWithoutInterrogations`, §8.2 row 3's shadowing argument applies to the
  corrupted twin exactly as it does to live states: row 2 (or row 1, or a dangle) always
  fires with it. For these two tests the expected literal is a **two-element set, still
  hand-derived and still `assert_eq!`'d whole**. Keep the slot's teacher live and teaching
  so the companion is the deterministic one: for `SlotSubject` the set is the dangle plus
  `SlotTeacherDoesNotTeachSubject(slot, teacher, dead_subject)`; for row 3 (corrupt by
  turning the subject's `interrogation_parameters` to `None`) it is row 3 plus row 2,
  `TeacherSubjectWithoutInterrogations(teacher, subject)` — and that fixture's subject
  must carry no association and no balancing override, or rows 7 and 9 join the set.
  Step 4 then runs on the element the test is about, selected from the set — not on
  `set.first()`. Every other arm keeps the one-element form; a future arm that cannot is a
  finding to record here, never a licence to fall back to `contains`.
- **The twin is built by direct field surgery, not by an op.** `force_apply` cannot reach
  several of these shapes — it keeps the coordinate carve-out prechecks, so
  `CannotChangeSubject` blocks a slot's subject (`slots.rs:465-471`), `force_add_week` and
  `force_move_week` both block a dead period on a week (`weeks.rs:576-585`, `:674-682`), and
  `AssignToSubject` checks the group list whenever the payload is `Some`
  (`group_lists.rs:450-480`). A recipe that works for some variants and not others drags a
  per-variant argument into every test; surgery works for all of them uniformly. Nothing is
  *applied* here, so no `GlobalUpdate`, no gate and no id issuer are involved either.

**Placement: in-crate.** The surgery reaches containers whose mutators are crate-internal
(the sparse `Slots` and `Assignments` sidecars in particular), so the file is
`state-colloscopes/src/resolution/innocent_tests.rs`, declared `#[cfg(test)] mod
innocent_tests;` from `resolution.rs` — beside the map it audits, and following the house
`foo.rs` + `foo/` layout.

**Two surgeries must go through the sidecar helpers, not raw fields** (settled at the
July 28 2026 second review). `Slots` and `Weeks` both carry a type-level ordering mirror,
and the mirror LogicErrors short-circuit `broken_invariants()` (`invariants.rs:234-246`):
a twin whose mirror is desynced dies at step 3 with `SlotOrderingWrongSubject` /
`WeekOrderingWrongPeriod` instead of yielding the invariant. Their fields are
module-private, but the `pub(crate)` compound mutators are reachable from this module
(`invariants.rs`'s own tests already use them) and keep the mirror consistent — and the
ordering sidecar's *row keys* are deliberately not liveness-checked, which is exactly the
hole these twins need:

- `SlotSubject`: `remove_slot(id)`, then `insert_slot_at(dead_subject, modified_slot, 0)`
  — the ordering row for the dead subject is created on demand.
- `WeekPeriodFk`: `move_week_entry(week_id, dead_period, 0)` (`weeks.rs:452-491`) — it
  does not check that the destination period exists, and rewrites the sidecar and
  `week_map[week].period_id` together in one call.
- Do **not** reach for `replace_slot` with a changed subject: it is a bare `mem::replace`
  on `slot_map` ("subject unchanged" is a doc promise, not a check) and desyncs the
  mirror.

**The negative half only.** These tests say the arm does *not* fire on an innocent document.
The positive half — the arm *does* fire when the offending shape really is in the live state
— belongs to the commit-7 scenarios, because that is the legitimate route: the target
disables interrogations on a subject, the live (valid) document still lists the teacher, the
fix lands.

**One end-to-end policy pin, not one per variant.** A `GlobalUpdate` target carrying a
corrupt document is always rejected in the end whatever the arms do — its payload is corrupt,
and no repair to the *live* document can make it valid — so a sloppy arm merely churns
(repair the innocent row, retry, same break, material now gone, `None`, convict) and the
caller sees the same `Err`. Such a test cannot see a missing shape test, so one or two are
enough, kept for what they *do* pin: **a corrupt `GlobalUpdate` is rejected whole, never
cleaned**. That is D4 applied to a whole-document target, and it is user-visible, since an
import takes exactly that shape. They reuse the idiom of the existing corrupt-`GlobalUpdate`
tests at `lib.rs:620` and `:780`.

### 9bis.1 The split — ten commits

One test per arm means **46 tests**: 30 for §8.1 (a two-part site such as
`PairingRuleAntecedent` / `Consequent` is *two* arms and therefore two tests) and 16 for
§8.2. That is far too much for one commit, so it ships as ten (★ user ruling, July 28 2026).

The arms are spread very unevenly across the eight target kinds — period 7, subject 8,
student 5, slot 3, week 2, week pattern 2, group list 2, teacher 1 — so "one commit per
target kind" is kept as the rule, but the four smallest kinds are merged in pairs rather than
producing commits whose whole content is two tests.

**The dangling-FK half — six commits:**

| Commit | Content | Tests |
|---|---|---|
| **7.5a** | scaffolding + **target: a teacher** | 1 |
| **7.5b** | **target: a period** | 7 |
| **7.5c** | **target: a subject** | 8 |
| **7.5d** | **target: a student** | 5 |
| **7.5e** | **targets: a week and a week pattern** | 4 |
| **7.5f** | **targets: a slot and a group list** | 5 |

7.5a is the commit that introduces the file, its `#[cfg(test)] mod innocent_tests;` line, the
shared valid-fixture builder and the surgery helpers. Its payload is a *single* test on
purpose: the review there is about the shape of the idiom, not about volume. `SlotTeacher`
earns that slot because it is the arm that produced frame point 5, and one of only three
whose identity test is reachable on today's code. 7.5c comes early for the same reason — it
holds the other two (`IncompatSubject` and the two `PairingRule` parts). The two merges pair
arms of a common shape: 7.5e is four arms that clear a reference out of a pattern or a
pattern out of a row; 7.5f is five arms that clear colloscope rows or remove pairing rules.

**The `Convergence` half — four commits, one per review block**: rows 1-4 (the slot and
teacher block), rows 5-8, rows 9-12, and rows 13-16 (the colloscope block). Four tests each,
and each commit lines up exactly with one chunk of the §8.2 table, so a reviewer can read the
commit beside the paragraph that justifies it.

The two `GlobalUpdate` policy pins fold into the last of those four rather than getting a
commit of their own: one or two tests, a different idiom (`lib.rs:620`, `:780`), and nothing
depends on them.

Three rules for the series:

- **Ordering.** The whole series lands after commit 5.97, so the enriched payloads are final
  and no test has to be rewritten. Within the series only 7.5a is constrained (it carries the
  scaffolding); the other nine are independent, and each is green on its own — they are pure
  tests against a map that already landed in commit 6.
- **A failing test is a map bug, not a test bug.** That is the entire point of the series. The
  house rule then applies: commit the failing test alone, then the fix in a separate commit.
- **One shared valid fixture serves most tests.** What differs between tests is the
  corruption, not the document. The builder belongs in 7.5a and is written to be reused —
  do not copy a fixture per test.

**Status: 7.5a** (`b04bdcaf`)**, 7.5b** (`362dde77`) **and 7.5c** (`acaab2fb`) **have landed**, so
`src/resolution/innocent_tests.rs` exists with its `#[cfg(test)] mod innocent_tests;` line in
`resolution.rs`, the shared builder, and sixteen tests — the `SlotTeacher` arm, all seven
`PeriodRefSite` arms and all eight `SubjectRefSite` ones. Every one passed on its first run, with
its hand-derived set; no map bug has surfaced. Seven commits remain (7.5d–7.5f and the four
`Convergence` ones). Four things settled while writing 7.5a, all of them things the rest of the
series inherits:

- ★ **The in-crate equivalent of "through `Manager::apply`" is `Data::annotate` + `Data::apply`.**
  §9bis's step-1 sketch says "ops through `Manager::apply`, then `get_data().clone()`", which is
  the *integration*-test idiom of `tests/cascade.rs`. In-crate there is no reason to wrap the
  document in an `AppState` at all: `AppState::apply` delegates to `Data::apply`, which *is* the
  apply/check/rollback gate, and `Data::annotate` hands back the `Option<NewId>` the fixture needs
  to read fresh ids. The builder is therefore a plain `Data::default()` plus a local `apply`
  helper that panics on rejection, and the neighbouring `apply_tests` module (`lib.rs:691`) was
  already doing exactly this. Validity is still by construction, which is the only property the
  sketch was really asking for.
- ★ **The dead-id recipe is create-then-remove here too, *not* `unsafe { Id::new(n) }`.** The
  forging idiom is used right next door (`invariants.rs`'s own tests, which build their states
  by hand and have no live entities to collide with), so an implementer will be tempted. Do not:
  this module's fixture is a *populated* document, and a forged number can name a live row, which
  would silently turn an innocent-state test into something else. Adding an entity and removing
  it again is three lines, cannot collide, and leaves the document otherwise untouched. 7.5a
  builds one dead id of every kind (period, week, subject, teacher, student, week pattern, slot,
  group list) at the very end of the builder, so the later commits have theirs ready.
- ★ **`ValidDocument` carries `#[allow(dead_code)]` for the duration of the series.** The struct
  is built whole in 7.5a but each commit reads only the fields its own arms need, so without the
  attribute 7.5a alone would emit two dozen unread-field warnings. Recorded as a decision rather
  than left to be rediscovered: the attribute comes off once the tenth commit has landed, and
  taking it off is the cheap check that the fixture has no field nobody ever needed.
- ★ **No general "surgery helpers" turned out to be needed for 7.5a.** §9bis.1 describes 7.5a as
  carrying "the shared valid-fixture builder and the surgery helpers", but the `SlotTeacher`
  surgery is two lines — read the live slot, `replace_slot` it with one field changed — and
  factoring that would obscure it. The surgeries §9bis names explicitly (`remove_slot` +
  `insert_slot_at` for `SlotSubject`, `move_week_entry` for `WeekPeriodFk`) belong to 7.5c and
  7.5e and can be written there, inline, in the test that needs them. A helper is worth extracting
  only once two tests want the same one.

And two more from 7.5b, which are about what makes a test in this series *strong* rather than
merely green:

- ★ **A key-half arm has no identity test, so the corruption must be an addition, not a move.**
  §8.1 splits the sites two ways: those that hold the target inside a row (an FK field, a member
  of an excluded set), where the fix names only the row and the arm therefore carries an explicit
  identity test; and those that hold it in a **row key** (`AssignmentsKey`, `AssociationEntry`,
  `ColloscopeInterrogation`, `ColloscopeGroupListKey`), where the fix carries the target and no
  identity test is expressible. For the second group the arm's whole content is one lookup, and a
  test that merely corrupts by *moving* the live row onto the dead id proves nothing: the valid
  document is then innocent for the trivial reason that it has no such row at all, and an arm that
  keyed on the other half of the pair alone would pass. So the corruption **adds** a row on the
  dead id for an entity that *already* has a live row of the same kind. The valid document then
  really does hold a row for that subject/slot/group list — just not on the dead coordinate — and
  an arm that dropped half the key would find it, answer `Some`, and clear something nothing
  complained about. 7.5b does this for both of its key-half arms; 7.5c, 7.5d and 7.5f each have
  more, and should do the same.
- ★ **Steps 3 and 4 are factored into `assert_arm_finds_nothing(valid, corrupt, expected, why)`.**
  Seven identical tails in one commit was the point at which the helper earned itself; before
  that (7.5a, one test) it would have been premature. Each test now reads: build, corrupt, assert
  — with the interesting content in the corruption and in the `why` sentence. The two arms §9bis
  singles out as needing a **two-element** set cannot use it, since they select the element under
  test out of the set rather than taking the only one; its doc comment says so, so the next
  implementer does not try to generalise it.

And two from 7.5c, the commit that carried the first of the two two-element exceptions:

- ★ **The `SlotSubject` exception is confirmed exactly as §9bis predicted, and the second element
  is selected by *shape*, not by `set.first()`.** The checker's own comment
  (`invariants.rs:415-421`) states the rule the exception follows: a predicate skips when a lookup
  it needs to *read data* fails, but an id used only as a *compared value* does not gate. The
  teacher-teaches check reads the teacher and compares the subject id, so a dead subject leaves it
  firing — `SlotTeacherDoesNotTeachSubject(slot, teacher, dead_subject)` sits beside the dangle,
  and the expected literal is a two-element set. Step 4 then picks the dangle with
  `.find(|i| matches!(i, FixableInvariant::DanglingFk(_)))`. The derived `Ord` happens to put it
  first, so `set.first()` would work today — which is exactly why it must not be used: it would
  make this test quietly depend on pick order, which is fixture `1a`'s job and nothing else's.
  That same gating rule is what keeps the other seven subject arms at one break, and it is worth
  reading before hand-deriving any later set: `find_subject` behind `let … else { continue }`
  (teachers, associations, balancing) or `if let` (assignments, slots) means a dead subject makes
  the predicate skip.
- ★ **The shared fixture gained a `lone_slot`.** It is a third slot on the running subject that is
  referenced by *nothing* — no week pattern, no colloscope cell, no pairing rule. `SlotSubject`'s
  corruption changes what a slot *is*, and doing that to either of the two existing slots would
  also break `PairedSlotsNotInSameSubject` (both slots resolve, and their subjects now differ) and
  the cell's group bound (a dead subject has no association, so every group number is
  out of bounds). The rule generalises: **a corruption that moves a row between owners needs a row
  that owns nothing else.** 7.5f's `ColloscopeInterrogation` arms are the next place it will bite.

## 9ter. Commit 7.6 — the self-caused rejection fixtures (`tests/cascade.rs`)

Split out of commit 7 at the July 28 2026 review (★ user ruling). Commit 7's fixtures all
assert `Ok`: the cascade repairs the document and the target lands. These assert `Err`: the
target is convicted and *nothing* lands. The reason for the split is sequencing, not subject
matter — an end-to-end rejection test is only convincing once the `None` branch it rests on
has been tested arm by arm, and that is commit 7.5. So the order is **7 → 7.5 → 7.6**.

The file is the same `tests/cascade.rs`, the fixture style is the one at the head of §9, and
the three shared rules there apply here too.

The commit holds **two families**, both of which convict the target through the same engine
path: the **self-caused rejections** (§9ter.3), where the op's payload is bad in itself, and
the **collateral-damage identity pins** (§9ter.4), where the op points an otherwise fine row at
a dead id. They were reviewed a day apart and moved here for the same reason.

### 9ter.1 What is being tested

Every fixture here sends a single op that is bad **on its own terms**. The document is valid
before the op, and the op alone carries the fault. The engine path is `cascade.rs:112-119`:

```rust
None if is_target => {
    *data = snapshot;
    return Err(ApplyError::BrokenInvariants(last_target_break.expect(..)));
}
```

The checker reports a break, the gate rolls the op back, and `fix_invariant` then runs on the
**restored** state — which is the valid pre-op document. The arm looks for the material it
would remove, does not find it, and answers `None`. Since the failing op is the target, the
engine restores the snapshot and reports the break.

This is the production-visible half of frame point 5. If any of these arms answered `Some`
instead, the cascade would quietly repair the state, `apply` would return `Ok`, and the user
would be told an edit succeeded that was in fact refused. Two of the §9ter.3 fixtures also keep
a live `ops/`-layer translation alive: `UpdateColloscopeInterrogationError::InvalidGroupNumInInterrogation`
(`ops/src/colloscope.rs:216-223`) and `UpdateSlotError::SlotOverlapsWithNextDay`
(`ops/src/slots.rs:481`) both read a `BrokenInvariants` error that would never arrive.

Commit 7.5 does not cover this. 7.5 calls the map directly, on a state built by `InnerData`
field surgery. These fixtures go through the public surface, with ops a user can actually
issue, and check the whole route: `None` → snapshot restore → `Err` at the API.

### 9ter.2 The construction rule: fail on the *last* conjunct

Every shape test in §8.1 and §8.2 is a conjunction — "the row exists **and** it holds `x`".
A fixture must be built so that every conjunct passes except the last one. Otherwise a map
that dropped the last conjunct entirely would still return `None`, and the test would go green
for the wrong reason.

Concretely, for §9ter.3's fixture 3: do **not** write the out-of-bounds group into a cell that
did not exist before. Then the *presence* half already fails and the membership half is never
reached. Start from a cell that already holds a valid group, and add the bad one to it. Same
for its fixture 2: the assignments row must already exist and already hold a different,
legitimate student. Fixture 1a needs no such care — the slot obviously exists, so only the `start_time`
comparison can fail.

The rule applies to §9ter.4 as well, and there it is satisfied for free: those fixtures always
keep the row alive and only make its *reference* wrong, so the presence conjunct passes by
construction and the identity conjunct is the one that decides.

### 9ter.3 The self-caused rejection fixtures

Each asserts four things: `Err(Error::BrokenInvariants(set))` with `set` compared against a
**hand-derived exact set** (not a `contains` — the first shared rule of §9); the document
unchanged; and, for the fixtures that have one, the innocent neighbour still in place.

On "unchanged": the assertion is `assert_eq!(&data, &before)` on a `Data` cloned before the
call. `impl PartialEq for Data` compares `inner_data` only (`lib.rs:145-194`), so this pins
the document and not the id issuer. That is the right coverage here — none of these three
targets issues an id — and the id issuer needs no fixture of its own in any case: the engine
restores with `*data = snapshot` on the whole value, so recycling cannot regress separately
from `Clone`.

**1a — `SlotOverflowsDay`, rejected.** A subject with a 60-minute interrogation and a slot
starting at 23:00 — a valid document, because a slot ending *exactly* at midnight does not
overflow (`SlotWithDuration::new` accepts it; its doctest pins `22:00 + 2h = 00:00` as
`Some`, `time/src/lib.rs:639-656`). Target: `SlotOp::Update(slot, same slot at 23:30)`.
Post-op, `SlotWithDuration::new(23:30, 60)` is `None` and the checker reports
`SlotOverflowsDay { slot, start: 23:30, duration: 60 }` (`invariants.rs:438-446`). The gate
rolls back. The arm compares the invariant's `start` with the live `slot.start_time`, which is
still 23:00 — mismatch, `None`, target convicted.

The 23:00 start is load-bearing for the *pair*, not just for 1a (fixed at the July 28 2026
second review — an earlier draft had the slot at 22:00). 1b below reuses this document and
grows the interrogation to 90 minutes; from 22:00 that ends at 23:30 and does not overflow,
so 1b would silently fire nothing. From 23:00 both halves overflow: 23:30 + 60 for 1a,
23:00 + 90 for 1b.

This fixture is unwritable before **commit 5.97**. On today's payload the variant carries only
a `SlotId`, so the arm has no second conjunct at all: the slot exists, and the only possible
answer is `Some(Slot::Remove(slot))` — the user asks to move a slot and the application
deletes it. 1a is the end-to-end reason 5.97 exists.

**1b — `SlotOverflowsDay`, accepted.** The mirror of 1a, and the pair is worth more than
either half. Same document, but the target is `SubjectOp::Update(S, interrogation duration
90)` with the slot left at 23:00 (90 minutes from 23:00 ends at 00:30 — overflow). The same
invariant fires on the same slot — but this time
the slot's `start_time` is exactly what the invariant names, the shape test passes, the arm
answers `Some(Slot::Remove(slot))`, the fix lands and the target lands after it. Assert `Ok`,
`applied.inner()` of length 2, and the slot gone.

Alone, 1a only shows that the arm says `None` somewhere. Together the two show that the
`start` field *discriminates*: same invariant, same arm, opposite verdict, and the only
difference is which of the two operands the op moved.

1b asserts `Ok` and so does not strictly depend on commit 7.5. It stays here anyway, next to
its twin, because the contrast is the point and splitting the pair across two commits would
lose it.

1b is also §8.2 row 4's only pin. That row has **no legacy behaviour to compare against**:
`ops/src/subjects.rs` has no arm for the overflow, so the ops layer reaches its catch-all
`panic!("Unexpected invariant breaks …")` today. This fixture is what states the new answer.

**2 — `AssignedStudentNotPresentForPeriod`.** A subject that genuinely runs on period `P`, an
assignments row at `(P, S)` already holding student `A`, and a student `B` who excludes `P`.
Target: `AssignmentOp::SetRow(P, S, {A, B})`. The arm finds the row and finds `A` in it, but
not `B` — `None`, target convicted.

Two traps in the construction. The subject must really run on `P`: otherwise
`AssignmentForSubjectNotRunningOnPeriod` fires, and it is declared **before**
`AssignedStudentNotPresentForPeriod` (`invariants.rs:158-167`), so the engine picks it
instead — its shape test (a row exists at `(P, S)`) passes on the pre-op state, a fix lands,
and the trace becomes a different one, with the test still green. And the row must pre-exist
with `A` in it, per §9ter.2.

**3 — `InterrogationGroupOutOfBounds`.** A group list with 3 groups, associated to the
subject, and a colloscope cell at `(slot, week)` already holding group `0`. Target:
`ColloscopeOp::SetInterrogation(slot, week, {0, 7})`. The arm finds the cell and finds it
does not contain `7` — `None`, target convicted. The enrichment this needs is **commit 5**,
already landed.

### 9ter.4 The collateral-damage identity pins

Four fixtures, moved here from commit 7 at the July 28 2026 review (they assert `Err` and read
the same `None` branch, so the criterion that created 7.6 applies to them unchanged).

**Where they come from.** These are not a general idea, they are the residue of one audit. Frame
point 4 — *the presence test names the target* — was discovered mid-way through the §8.1 review,
and §8.1 was then re-read from the top against it. Five rows were missing their identity test.
Two (`SlotSubject`, `WeekPeriodFk`) proved unreachable and got the test defensively; the other
three are reachable, and these fixtures are their end-to-end pins.

**Reachable is meant precisely**, and §8.1 established it row by row: `force_apply_slot`'s
`Update` has no teacher-existence guard (`slots.rs:455-483`), `force_apply_incompat`'s `Update`
replaces the row with no field guards (`incompats.rs:108-124`), and `force_apply_pairing`'s
`Update` likewise (`pairings.rs:237-247`). So the bad op really lands, the checker really
reports the dangle, and the gate really rolls it back. The arm is then asked, on the restored
state, and that is the moment being tested.

**What the failure would look like.** Suppose the `SlotTeacher` arm skipped its identity test and
answered `Some(Slot::Remove(slot))` merely because the slot exists. A user pointing a slot at a
teacher who no longer exists — typically a stale UI view, or a script racing another edit —
would have a slot deleted whose real, live teacher was perfectly fine, and the target would
land afterwards, so the operation would report success. That is why the assertion that carries
the weight is not the `Err`; it is **the innocent row is still there**.

**The dead-id recipe, shared by all four.** Every fixture needs a `TeacherId` or a `SubjectId`
that is *not* live, and an integration test cannot fabricate one: the id types are opaque and
carry no public constructor. The route is **create-then-remove** — add a teacher (or subject)
that nothing references, remove it, keep the id. The removal cascades to nothing and lands
alone. Write this helper once and share it; it is three lines, but it is the step someone
implementing from this plan will otherwise stall on.

**The expected set is exactly one break in all four**, which matters because §9's first rule
requires the expected set to be derived by hand. For the slot fixture the reason is in the
checker: the teacher-teaches predicate sits behind `if let Some(teacher) = …`
(`invariants.rs:428`), so a dead teacher makes it *skip* rather than fire —
`SlotTeacherDoesNotTeachSubject` does **not** accompany the dangle. For the other three it is
simpler still: no `Convergence` variant mentions an incompatibility or a `PairingRule` at all.

The fixtures:

1. **`SlotOp::Update`** giving a slot a dead `teacher_id`, the slot's real teacher alive.
   Expected: `DanglingFk(Teacher { target: dead, site: SlotTeacher(slot) })`. The `SlotTeacher`
   arm must return `None`, not delete the slot.
2. **`IncompatOp::Update`** giving an incompatibility a dead `subject_id`, its real subject
   alive. Expected: `DanglingFk(Subject { target: dead, site: IncompatSubject(incompat) })`.
3. **`PairingOp::Update`** giving the rule's **antecedent** a dead `subject_id`.
4. **`PairingOp::Update`** giving the rule's **consequent** a dead `subject_id`.

Fixtures 3 and 4 are a pair, and the first draft had only the first of them, described as "the
antecedent arm returns `None`, and the consequent arm must not fire at all". The second half of
that sentence is not an assertion a test can make: with only the antecedent's subject dead the
checker reports only the antecedent site, so the consequent arm is never called and that
fixture tests it in no way whatsoever. §8.1 insists these are **two arms, not one**; the mirror
fixture is what makes that true of the test suite as well, and it costs three lines.

Both build their rule through `PairingRule::new(...).expect(..)` — the sealed constructor is the
only door, and it accepts these payloads because its single failure is the two parts *sharing*
a subject, which a dead id on one side cannot cause. Same pattern the gtk4 dialogs use.

Assertions, per fixture: `Err(Error::BrokenInvariants(set))` with `set` the exact one-element
set above; the document unchanged, with the §9ter.3 reading of "unchanged" (`inner_data` only —
none of these targets issues an id either); and, spelled out separately because it is the
actual point, the referencing row **still present and unmodified**.

### 9ter.5 One engine branch these fixtures do not reach

`cascade.rs:124-131` holds a second conviction route: the target fails on broken invariants, a
fix lands, the target is retried, and *that* attempt fails its **precheck** because a fix
removed something the target names. The engine then reports the remembered `BrokenInvariants`
rather than the precheck error.

Nothing exercises it. Toy test 7 (§5) reaches "target convicted after fixes landed", but
through `None`, not through `InvalidOp`. Toy test 4 reaches `InvalidOp`, but at round one,
where `last_target_break` is still `None`. The `Some(set)` arm of that `match` is untouched.

The first two drafts of this plan gave fixture 1a as the example of this trace. That was
written before the arm had its shape test and is simply wrong: 1a rejects at round one and
nothing lands. A search for a target that does reach the branch was made at the July 28 2026
review and came up empty, but the argument was not judged solid enough to record as a
determination. So the position is: **the branch is untested, and no colloscope target for it
is known.** If it is to be pinned, the natural home is a tenth engine test in §5, where an
evil mode can delete the target's referent directly — the branch is generic engine code, and
the reasons it looks hard to reach are all facts about the colloscope map, which the engine
knows nothing about.

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

**Configuration — start wide, shrink later** (★ user ruling, July 28 2026). The first draft
proposed a modest 20 seeds × 300 ops because the cascade multiplies gate calls. That is the
wrong way round: while the migration is in flight we would rather catch a bug and wait a bit.
So start at **50 seeds × 500 ops**, in the house style — a single hardcoded `RunConfig`
const, no env variables, no `#[ignore]` tiers (the standing decision recorded at
`property_ops.rs:30-34`), and `invalid_fraction` at the usual `0.15`. For scale, the two
existing harnesses run 100 seeds × 1000 ops each (`property_ops.rs:35-39`,
`property_apply_gate.rs:46`), so this sits below them, not above.

Shrinking is a **later** decision, taken once step 7's migration is finished and the map has
stopped moving — not a tuning knob to reach for the first time the suite feels slow. When
that day comes, justify the new size the way `property_ops.rs:32-34` justifies its own ("100
seeds was verified to still catch every bug found by the original 500-seed configuration"),
and keep the wide configuration around as the slow reference for milestone checks. The main
`property_ops` / `property_apply_gate` harnesses remain the deep oracles throughout. Run the
suite once, captured to a scratchpad file, per the house testing rules.

## 11. Non-goals, gates, close-out

**Non-goals**: no `ops/` migration onto the cascade, no dry-run/preview UX, no `Warning`
retirement, no history/`Manager` integration of the cascade (step 7 decides whether
`apply_cascade` gets a `Manager`-level wrapper), no storage **format** change, no gtk4
change, no variant reorder, and no `PartialOrd` / monotonicity checking — that is step 6.5
(design doc §8), recorded, not implemented here.

Three of those need their edges drawn, because several commits *do* touch `ops/` and the
storage tests without crossing into step 7 — and because the gtk4 line turned out to be
stricter than the first draft dared claim (all three audited July 28 2026).

**`ops/` changes in four places, and none of them is the remaster.** `Warning`,
`get_next_cleaning_op` and the whole `UpdateError` vocabulary stay exactly as they are; what
moves is spelling forced from below.

- commit 4 — the `SetRow` translation rewrite, an op-surface adaptation;
- commit 5 (landed) — `InterrogationGroupOutOfBounds` gained a field, matched at
  `ops/src/colloscope.rs:218`;
- commit 5.97 (landed) — every one of the five enriched variants is matched in `ops/`:
  `SlotTeacherDoesNotTeachSubject` (three sites), `SlotOverflowsDay` (two),
  `PairedSlotsNotInSameSubject` (two), `SlotForSubjectWithoutInterrogations` (one),
  `ColloscopeStudentGroupOutOfBounds` (one, `colloscope.rs:146`, bound as `_`);
- commits 5.98 and 5.99 (both landed) — `ops/src/settings.rs` and `ops/src/balancing.rs` each
  construct the op being split at three sites.

**"No storage change" means the format.** `storage/tests/populated_round_trip/builder.rs:604`
and `:638` build `SettingsOp::Update` and `BalancingOp::Update`, so commits 5.98 and 5.99
re-spell that test builder. Nothing in this step goes near serialization, and elementary ops
are never persisted.

**"No gtk4 change" is now verified rather than predicted.** The first draft said "no gtk4
change beyond the mechanical commit-1 re-spelling"; commit 1 landed touching twenty-one files
and **not one of them under `gtk4/`**. Nor is any later commit expected to: gtk4 never names
`SettingsOp`, `BalancingOp`, `Convergence`, `FixableInvariant` or the `Error` variants. What it
does use — `BalancingOptions`, `DecodeError::LogicError`, `Op::GlobalUpdate` — none of this
step moves.

**Gates**: `cargo test --workspace` (background, output captured once) green after every
commit; `Cargo.lock` unchanged (no new dependencies); commit 8's harness run at its full
50 × 500 configuration.

The property harnesses **keep their configuration and their oracles** — that is the invariant
to hold, not "untouched", which is already false three times over: commit 1 re-spelled
`property_apply_gate.rs` onto the new error surface, commits 5.98/5.99 change the op
vocabulary in `testgen-colloscopes/src/generator.rs` (`:849`, `:985`, `:1314`, `:1321`) which
feeds *both* existing harnesses, and commit 8 adds a third. What must not change is seeds,
op counts, `invalid_fraction`, or what any of them assert.

Two clarifications the 5.98/5.99 implementation forced, since "keep the oracles" could be
read as forbidding both. First, **the generated op sequences drift and that is unavoidable**:
`gen_settings` and `gen_balancing` draw different numbers of RNG values than the whole-value
builders did, so a given seed now walks a different path. The seed *numbers*, the op counts
and `invalid_fraction` are what stay fixed. Second, **`gen_corruption_op`'s eligibility rule
is not an oracle**: 5.99 makes `ForceRetarget` conditional on material being present. The
oracles are what `property_apply_gate.rs` asserts — atomicity on rejection, full validity on
`Ok`, the reverse restoring the pre-state, and every `CorruptionKind` being both attempted
and (if corrupting) rejected at least once across seeds. None of those moved.

One per-commit obligation lives outside this section and is repeated here because it is easy
to lose: **within the 7.5 series, a failing test is a map bug**, and the house rule applies —
commit the failing test alone, then the fix in a separate commit (§9bis.1). The reflex
mid-series is to fix and commit together; do not.

The user runs the acceptance scripts / gtk4 smoke at their own cadence; no step in this plan
blocks on it.

**Close-out ritual** (after the user's gate): record the delivered state as **Appendix H**
of the design doc — the `ApplyError` reshaping of the G.2 error surface (G stays as the
historical step-5 record), the `SetRow` op swap and the invariant-payload enrichments
(`InterrogationGroupOutOfBounds`, then commit 5.97's four), the engine contract and its
deviations from the §5 pseudocode (the
annotated-op surface; one-step `Option` fixes recomputed per round; the engraved
strict-monotonicity contract with `Default::default()` as the minimum; the `None`
conviction and the unconditional no-op-fix panic replacing the §5 repetition ledger;
target-fallible/fix-infallible; the remembered-error rule; snapshot-restore failure; no
fuse — hang accepted until 6.5), the resolution table's policy rules
(the frame's five points — in particular pin-the-shape-not-just-the-row and its payload
corollary — the D5.3 remove-the-reference-first rule, the content-not-semantics reading of
the order, orthogonality, legacy semantics as aspiration with the week-pattern and
`SlotOverflowsDay` divergences recorded), the two op splits (commits 5.98 and 5.99) and
their shared motivation (no `Table` through the op surface), and the test inventory. Then
retire this plan with a pin, per the house pattern.

The test inventory is the part that grew most during the July 28 2026 review, so it is spelled
out rather than left to "the tests we wrote":

- **the three tiers and why they are three.** Commit 7 = the fixtures that assert `Ok`;
  commit 7.5 = the innocent-state `None` tests, one per arm, forty-six of them across ten
  commits; commit 7.6 = the fixtures that assert `Err`, sequenced *after* 7.5 because a
  rejection fixture only means something once the `None` branch it rests on has been tested
  arm by arm (★ user ruling). Plus commit 8's property harness.
- **the fixture-writing rules**, which are the reusable part: expected op lists derived by
  hand from the §8 tables *before* the test runs; sequence versus content, and why an ordered
  literal is a tripwire on a derived `Ord` and **not** a confluence pin; fail on the *last*
  conjunct; the create-then-remove recipe for a dead id.
- **the accepted asymmetry**: 7.5 covers every arm's `None` branch systematically, nothing
  covers the `Some` branches systematically, and a second forty-six-test series was
  considered and rejected. Record it as a decision, since that is what it is.
- **the two deliberate deletions and their reasons** — the undo round-trip fixture (every
  component already pinned by `property_ops.rs` Properties 2 and 4, `history.rs:494`, the
  order fixtures and the toy test) and "clean target lands alone" (when nothing breaks the map
  is never consulted, so it never touched this step's code); what replaced the latter is the
  no-op-target pin, which guards the `(!is_target).then(..)` carve-out.
- **the two structural findings**: §8.2 row 3's `Some` branch is shadowed by declaration order
  and can never be the pick; and the engine's `InvalidOp`-with-remembered-break conviction
  route (`cascade.rs:124-131`) is reached by no test, with no colloscope target known for it
  (§9ter.5). Both recorded as facts, neither used as a licence to weaken an arm.
- and why the contract panic is **not** counted as a safety net.
