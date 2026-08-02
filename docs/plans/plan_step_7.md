# Step 7 session plan — the `ops/` remaster

Status: **IN FLIGHT.** Session plan for step 7 of
`docs/plans/invariant_cascade_design.md` (§8, "migrate `ops/`"), designed with the user on
July 30 2026 and proof-read with them on July 31. Read Appendices H and I of the design
doc first: the cascade engine, the resolution map and the `ContentOrd` termination
mechanism are delivered and tested, and **nothing in production calls them yet**. This
step is the consumer.

**Position (August 1 2026): commits 0 through 4 have landed** — the fix vocabulary,
the engine wrapper, the session struct, **all fifteen families**, the dispatch above
them and the property walk over that, plus the riders 0bis/0ter, 3.3bis, 3.12+ and
3.13bis. The `landed` column of §3's table is the authoritative record. **Commit 3 is
closed and the new path is fuzzed; commit 5 is closed — 5.0, 5.1 and 5.2 have
all landed (this section was reviewed and re-planned with the user August 2),
and commit 6.0 has landed on top of it**, so the next commit is 6.1, then
6.2–6.4 and 7. Commit 6.0 was not in the original plan: surveying gtk4 for 6.1
showed the rendering helpers could not be called there at all, and §5-C6's 6.0
records why and what changed. `cascade_dry_apply` / `cascade_apply` now exist,
but nothing in production calls them: no consumer moves before 6.2.

The migration pattern is the step-5 one: build the new world in parallel under
transitional names, move consumers over, delete the old world, rename at the very end so
the lasting API carries no migration scars.

Every decision in §0 is settled, including the three survey-surfaced ones (D12–D14),
each ★-ruled by the user at sign-off on July 30 2026, and the `Fix` vocabulary
redesign (★ D15, user-driven design round of July 30 2026, after the first draft) —
D15 revises D2 and D6 in place; their entries below are already rewritten to the
final ruling.

**One decision has since become void: D12.** The global-group-list work of July 31
2026 (branch `global_grouplist_update`, landed before this step starts) removed its
premise — see its entry in §0. The consequences are already folded in below: §2.3's
`CascadeWarning` has one shape instead of two, and §3.14, §5-C5, §6 and §7 follow.

---

## 0. Decision ledger

Rationale is recorded so the executing session does not re-litigate.

- **D1 — Sequential session struct, not a pre-generated op list.** An earlier sketch
  (`UpdateOp::generate_op_list(&self, data) -> Vec<Op>`) was **rejected**: (a) a plain
  `Vec<Op>` cannot express "the id op 1 will create" — `CutPeriod` creates a period and
  then moves weeks *into it*, threading the returned `PeriodId` through every later step
  (`general_planning.rs:1150-1305`); (b) a list generated against the pre-state has no
  guarantee of surviving the cascades of its own prefix. The old imperative
  one-op-at-a-time shape was right; we keep it behind a session struct that owns the
  manager, applies one elementary op at a time through the cascade, hands ids back
  inline, and accumulates the fixes.
- **D2 — The engine tags every fix with its `Fix` value; the invariant stays
  engine-internal** (revised by ★ D15, July 30 — the original ruling exported
  `(op, invariant)` pairs). Attribution is still exact inside the engine: each cascade
  round picks one invariant (`BTreeSet::first()`) and the map answers one `Fix`; the
  pick keeps feeding the no-progress ledger. But nothing downstream needs the cause
  anymore — rendering keys on `Fix` (D15), and invariant→fix attribution is pinned by
  direct `fix_invariant` unit tests in `state-colloscopes` — so the receipt exports the
  meaning, not the cause. Return shape: fixes as `Vec<(ReversibleOp, Fix)>` with the
  target held separately — **not** an `Option<Fix>` in one Vec (encodes impossible
  states) and **not** a parallel Vec (index re-alignment at every reader). The same
  `Fix` shape may appear on several fixes (the N-round path); that is honest. If a
  future consumer genuinely wants the cause, adding a tag back is additive.
- ★ **D3 — `Manager` gets the cascade wrapper; the allocator is outside rollback**
  (settles H.6's open question; re-argued and confirmed July 31 2026). A defaulted
  trait method mirroring `Manager::apply` — original op in, `(NewInfo, Vec<Fix>)`
  out, annotation inside — bounded `where Self::Data: Fixable`. An annotated-in
  variant (caller annotates through a new public `Manager::annotate`; `apply_cascade`
  then strictly `Err ⇒ bit-identical`) was worked out and **rejected**: annotated ops
  are deliberately never publicly *injectable* — no `Manager` method accepts one
  (they flow out through `get_last_op`/`undo`/`redo` as read-only history views,
  never in), and opening an input door invites spurious/transplanted ids. The unease
  the variant answered is resolved by doctrine instead: the manager's
  rollback-managed state is **document + history**; the id allocator is monotone
  across the manager's whole life. `Manager::apply` burns annotation ids on failure
  (the gate's snapshot is post-annotation, `lib.rs:336-337`, "history ids are never
  reused"), and `undo` *must not* rewind the allocator — the undone entities' ids
  stay live because `redo` can restore them. `apply_cascade`'s contract is therefore
  *uniform* with `apply`: `Err` ⇒ data and history unchanged (engine entry-snapshot
  restore, `cascade.rs:89-90`, `:163`, `:176`; `store` only on `Ok`), allocator
  possibly advanced. Harmless: burned ids never appear anywhere (~2^63 deep), and
  the `ops/` session is dropped on error anyway. `ops/` never reaches around the
  manager into the raw `Data`.
- **D4 — No list comparison anywhere.** With D2 the warnings *are* the cascade's own
  `Fix` answers; the earlier "diff applied vs intended" idea is redundant and fragile.
  Dropped.
- **D5 — `UpdateError` runs as today.** The per-family translation of state-layer errors
  stays at the call sites, copied from the old bodies **including the scan order**, which
  is documented in-code as reproducing the old validator's first-error order (e.g.
  `slots.rs:324-330`, `assignments.rs:163-181`) and pinned by
  `ops/tests/assignments_error_surface.rs`. What changes: the "should be cleaned before"
  panic arms become dead and are removed.

  ★ **The growth rule** (restated by the user, August 1 2026, after commit 3.12 — the
  original wording said "the vocabulary may grow **additively only**" and then named two
  concrete candidates, which read as a closed list it was never meant to be). Two
  different things were being conflated, and they pull in opposite directions:
  - **Prechecks must not grow.** An ops-level guard that refuses input the cascade
    would happily repair is exactly what this step is deleting; re-adding one under a
    new error name puts the cleaning phase back by the side door. None of those is ever
    added.
  - **A panic on reachable input must be dealt with.** Where a state-layer break can
    reach a residual catch-all `panic!` — the target is convicted, the map has no repair
    to offer, and no scan of the arm names the break — the family's vocabulary **gains a
    variant**. A crash is not a contract. This is not a judgement call to weigh case by
    case: whenever a family's fixtures or a reading of its arms turn one up, it gets a
    typed error, in that family's own commit or in a follow-up.

  The panics that *stay* are the ones for genuinely unreachable states — the arm the
  fixtures establish no input can produce, an instrument per H.2. The test is
  reachability, not taste.

  The cases known so far, in the order they were found: balancing options on a subject
  whose interrogations are disabled (§3.3, fixed by commit **3.3bis**), the
  teacher/non-interrogation-subject case (§3.4), `DeletePeriodAndWeeks`'s dead `InvalidPeriodId`
  variant coming alive (D13), and — found while writing commit 3.12's fixtures — a
  colloscope group-list row aimed at a *prefilled* list (§3.12). The first two are the
  same shape, a `Convergence::…WithoutInterrogations` on a subject the op's own payload
  names, and both are genuinely new vocabulary (pre-step-4 the balancing body crashed
  too, on `.expect("BalancingOp::Update should not fail")`). The fourth is the odd one
  out: `ops/` answered it cleanly until step 4 dropped the guard (see §3.12), so its
  follow-up **restores a lost error** rather than inventing one. Removals of dead
  variants are deferred to commit 7 (D14). Python's ~80 exception-matching sites in
  `python/src/glue.rs` stay intact.

  **The audit rule, and its result for 3.1–3.11.** Finding these is mechanical, not a
  matter of inspiration. The engine rolls the failing target back *before* asking the
  map, and every map arm is a presence test — so an invariant a target breaks with its
  **own written content** always finds its material gone, always answers `None`, and
  always lands in that family's `BrokenInvariants` arm. An invariant broken by
  invalidating **pre-existing** material survives the rollback and is repaired. So per
  family the question is only: enumerate the reference sites originating in the row the
  family writes, plus the convergence predicates whose offending fields the op writes,
  and check each is scanned or excluded by an ops-level address check. Commits 3.1–3.11
  were swept this way on August 1 2026 and hold no further case: every own-content break
  is named, and the remaining panics (the `*IdAlreadyExists` arms, which only
  `force_apply`'s `Add` branches emit; `SlotPrecheckError::CannotChangeSubject`, which
  sits *after* `InvalidSlotId` and so cannot fire on the pinned subject; and
  `InvalidOp::Logic`, unreachable from any elementary op) are instruments.
- **D6 — Warning texts: keyed on the `Fix` variant, phrased as the effect, in `ops/`**
  (keying revised by ★ D15, July 30 — the original ruling keyed on `FixableInvariant`).
  Keyed on `Fix` so the `match` is exhaustive with no wildcard: a new fix shape is a
  compile error in the renderer, and — because D15's variants are one-per-rendered-
  meaning — the renderer never inspects the invariant at all. Effect only
  ("L'interrogation de X sera supprimée", never "… car son colleur a été supprimé") —
  the user just performed the action; §5's own design-doc example is effect-only.
  Located in `ops/` because `state-colloscopes/` carries no French, no presentation, no
  serde, and rendering needs entity *names* — a data-snapshot read, exactly what today's
  `build_desc_from_data` does. The desync risk the original D6 had to accept (a
  resolution arm silently changing its fix shape under an unchanged invariant) is now
  closed structurally by D15; the backstop is commit 5.2's fuzz totality check (the
  property walk renders every warning it collects against the true pre-state), plus a
  pointer line in `resolution.rs`'s module doc ("every `Fix` variant has a French
  description in `ops/src/warning_text.rs`"). The renderer sits on
  `ops/src/rendering.rs` (commit 5.0) — the shared, noun-less id-rendering vocabulary
  that gtk4 itself adopts in commit 6.1, so a warning and the UI describe the same
  entity with the same words (revised August 2, review of this section with the user).
- **D7 — Rendering is lazy, against the composite's pre-state** (revised July 30,
  superseding an earlier per-op-pre-state rule). In principle a later op's cascade could
  touch material *created* by an earlier op of the same composite, which the pre-state
  never heard of — but a warning about such material would be incomprehensible ("cette
  colle n'a jamais existé dans mon document"), so the case is **outlawed as a composite
  bug** (the frame rule's rendering corollary, §2.5), not rendered correctly. With that
  rule the composite pre-state is sound, and it is the *right* state: it is the document
  the user is looking at when the dialog appears (gtk4 stashes `new_state`; the UI still
  shows the old one). Mechanically: `CascadeWarning` stores no text — it exposes
  `text(&self, data: &Data) -> Result<String, MissingId>`, computed only where needed
  (gtk4). The python/scripting route never renders. A failed lookup inside `text`
  returns `Err(MissingId)` naming the missing material (revised August 2 — a panic is
  a bad API for code that merely *tries* to render); the callers that must not fail
  panic on the `Err` themselves — gtk4 `.expect`s it, the 5.2 property walk panics
  printing both the error and the fix — which keeps the old `.expect("Warning should
  have a desc when applied on same state")` (`lib.rs:425`) role as an instrument for
  the tests per H.2's ruling.
- ★ **D8 — Composites keep their structure, drop their cleaning; the names are split
  by layer: the elementary op is `PeriodOp::Remove`, the composite is
  `DeletePeriodAndWeeks`** (re-revised August 1 2026 — the July 31 revision renamed
  the *elementary* op `RemoveWithWeeks`, which put the user-facing semantics on the
  wrong layer; commit 0bis reverses commit 0's rename, commit 0ter renames the
  composite instead). What every composite body loses is its
  *reconciliation/cleaning* steps (divergences are §6); the structural bodies stay.
  The layering doctrine: **elementary ops are elementary** — `PeriodOp::Remove`
  removes the period and nothing else, and its name must say no more than it does.
  The state layer has no week-empty guard (`PeriodPrecheckError` is target-existence
  + no-clobber only, `periods.rs:112`); a dangling `Week::period_id` is deliberately
  a *fixable* `DanglingFk` (`weeks.rs:28-31`, design doc Appendix F.4) repaired by
  `WeekOp::Remove` (`resolution.rs:145`), and `cascade.rs`'s fixture 1b pins a bare
  period removal landing through exactly that chain. The *semantic* fact — a user
  deleting a period expects its weeks to go with it — belongs to the user-facing
  layer, and to its **name**: `GeneralPlanningUpdateOp::DeletePeriodAndWeeks` says
  explicitly that the weeks go, and authors their removal itself (weeks first, then
  the period) by the authored-loss doctrine that voided D12. Authoring the week
  removals keeps the warning list down to the genuinely surprising effects — each
  week removal's own cascade on colloscope cells and week-pattern bits — instead of
  one « la semaine X sera supprimée » line per week restating the user's request.
  Both renames are mechanical; the elementary enums carry no serde, so no wire
  format moves (the `ops/` enums do derive serde, but nothing in-tree serializes
  them).
- **D9 — Testing: fixtures on known documents, no differential fuzz, one
  non-differential walk.** Behaviour diverges from legacy on purpose (§6), so there is no
  reference to diff against. Every user-facing op gets fixtures asserting the exact
  resulting state and the exact warning list (default base document: the frozen
  hogwarts copy, §7). Behaviour-divergence fixtures must be
  **mutation-checked** (green-on-first-run proves nothing until seen red). The walk
  (commit 4) fires random `UpdateOp`s through the new path asserting no-panic + valid
  result; `ops/` had zero fuzz before it.
- **D10 — Consumers.** gtk4 keeps its existing `Vec<String>` warning dialog fed by the
  new texts; a richer warning window is **out of scope**. Python keeps discarding
  warnings (the Python API revamp is separate work). `rpc-engine`'s `collomatique-ops`
  dependency is dead (declared, never referenced) and is dropped.
- **D11 — Final names.** Transitional `cascade_dry_apply` / `cascade_apply` become plain
  `dry_apply` / `apply` in commit 7. The final `apply` keeps today's exact signature
  (`&mut T` → `Result<Option<NewId>, UpdateError>`), so `python/src/glue.rs` ends the
  step textually unchanged.
- ★ **D12 — VOID since July 31 2026; its premise was removed.** The original ruling
  (user, July 30, rejecting an earlier "accept the silence" proposal) said a prefilled
  shrink must keep warning: the GUI edited a list's group *count* and its *filling* in
  two separate dialogs, so a user shrinking the count might never see that a dropped
  group held students, and `LooseStudentsInPrefilledGroupList` therefore had to
  survive as a composite-emitted `DroppedPrefilledStudents` warning pushed through a
  `pub(crate)` channel on `CascadeSession`.

  The global-group-list work (branch `global_grouplist_update`, landed before this
  step starts) removed **both** halves of that premise. The two dialogs are now one
  window showing count and filling side by side, and
  `GroupListsUpdateOp::UpdateGroupList` carries a whole sealed `GroupList` —
  parameters *and* filling in one payload. The loss is therefore **authored by the
  caller**, not discovered by this layer: a group they deleted and a student they took
  out of a group are their own edits, and warning about them tells the user what they
  just said. `LooseStudentsInPrefilledGroupList` and the shrink pre-empt that emitted
  it are deleted (commit `93f83345`); the ops-level group-list op family also lost
  `SetFilling` and the parameters-only `UpdateGroupList` there.

  **Consequences for this plan.** `CascadeWarning` has exactly **one** shape, the
  cascade fix (§2.3); `CascadeWarningView`, `DroppedPrefilledStudents` and
  `push_dropped_prefilled_students` are never built; §3.14's composite has nothing to
  detect and nothing to emit. The hand-written-warning door stays **shut** — every
  warning in the new world is a cascade `Fix` — and re-opening it needs a fresh ruling
  here. Unchanged: colloscope placements referencing dropped groups still warn through
  the cascade (`ColloscopeStudentGroupOutOfBounds`); that half of D12 was never about
  the payload.
- ★ **D13 — `DeletePeriodAndWeeks` on a dead id stops crashing.** Today the variant
  `DeletePeriodAndWeeksError::InvalidPeriodId` (né `DeletePeriodError`, renamed in
  commit 0ter; `general_planning.rs:277`) is **never
  constructed** — the arm has no precheck at all and a dead id dies on
  `.expect("All data should be valid at this point")` (`general_planning.rs:1117`),
  reachable from Python. Ruling (confirmed July 30): the new arm translates the state
  layer's `PeriodPrecheckError::InvalidPeriodId` into the existing variant — the dead
  variant comes alive, additive-safe, and a crash on data-dependent input disappears.
- ★ **D14 — Dead vocabulary is removed, and the `MoveSlotDown` wart is fixed, in
  commit 7** (user ruling, July 30, rejecting an earlier "replicate forever" proposal:
  no consumer code matches on these errors today — no script checks them — so the
  frozen-vocabulary caution does not apply). At commit 7:
  `UpdateSlotError::InvalidSubjectId` (`slots.rs:119`, dead — the subject is pinned by
  the ops layer) and `UpdatePeriodWeekCountError::SubjectImpliesMinimumWeekCount`
  (`general_planning.rs:271`, dead) are **deleted**; `MoveSlotDown` on a dead id stops
  returning `MoveSlotUpError::InvalidSlotId` cross-enum (`slots.rs:560`) and returns
  its own `MoveSlotDownError::InvalidSlotId`, pinned by a fixture in the same commit.
  Until commit 7 the old surface is replicated verbatim — commit 3 stays mechanical.
- ★ **D15 — The resolution map answers a `Fix` enum, not a raw op** (user-driven design
  round, July 30, after the first draft; revises D2 and D6 above). Motivation: with the
  map returning bare `AnnotatedOp`s, the fix *shape* is implicit in each arm, and the
  UI text lives two crates away keyed by cause — a resolution arm could change its fix
  and the text would silently lie (the risk the original D6 had to accept). The ruling:
  - **The vocabulary.** `state-colloscopes` gains a public `Fix` enum — the closed set
    of repair shapes the map can answer (`DeleteSlot`, `ClearAssignmentRow`, …; full
    catalogue in §5-C5). It is structurally deletive: creation is unrepresentable. A
    small trait in `state/` (`FixOp`, one method `to_annotated_op(&self) -> Op`) lets
    the engine translate generically; `Fixable` gains `type Fix: FixOp<…>` and
    `fix_invariant` returns `Option<Self::Fix>`.
  - **Single lookup; payload-carrying variants; pure translation.** An earlier split
    ("`fix_invariant` presence-tests, `to_annotated_op(&data)` re-looks-up and
    materializes") was **rejected**: it does every lookup twice and its second half
    needs `.expect`s justified only by call-timing discipline — a new panic surface.
    Instead `fix_invariant` keeps its single lookup (presence test + build, exactly
    today's arm bodies) and the variant carries *everything the op needs*: ids for the
    pure cases, the rebuilt payload for the whole-value ops (`Update(id, rebuilt)`,
    `SetRow`, `SetGroupList` — the elementary vocabulary is whole-value and frozen, so
    the payload must travel). `to_annotated_op` is then **total, pure and testable in
    isolation**, and the engine contract sentence "the map is a pure function of
    (state, invariant)" stays literally true.
  - **Granularity: one variant per rendered meaning** — not per invariant, and not per
    op shape. Several invariants share a variant when the user-facing sentence is the
    same (dead subject and dead teacher both yield `DeleteSlot`); the same elementary
    op splits into two variants when the meaning differs (`DeleteOverflowingSlot`
    carries the « il déborderait sur le jour suivant » nuance; `ClearSlotWeekPattern`
    means "the slot now runs every week", not just an update). Consequence: the
    renderer is a function of `Fix` alone. A future cause-dependent wording need is
    answered by a new variant, never by re-exporting the invariant.
  - **The invariant is demoted to engine-internal** (D2 as rewritten): receipt, manager
    wrapper and warnings all carry `Fix`, nothing carries `FixableInvariant`.
    Attribution pinning relocates to direct `fix_invariant` unit tests in
    `state-colloscopes` (state + invariant in, expected `Fix` value out — the map is
    now unit-testable at exactly that seam).
  - **Accepted residuals**, named consciously: (a) payload-carrying variants hold both
    the semantic delta and the rebuilt value three lines apart in one arm, and nothing
    *forces* e.g. `student ∉ rebuilt_row` — but the fixtures' exact-post-state asserts
    pin the payload, and the drift class shrinks from "cross-crate, invisible" to
    "same screen, pinned"; (b) fixture literals for the rebuild shapes contain the
    rebuilt payload — tolerable: the attribution unit tests build their documents
    in-process so the expected value comes from the same builders, the ops-level
    fixtures derive it from the decoded hogwarts base (read the entity, remove the
    element — §7), and the majority of variants (deletes, clears, unassigns) stay
    id-only.

---

## 1. The world as it stands

### 1.1 The old machinery (what this step replaces)

`UpdateOp::dry_apply` (`ops/src/lib.rs:459`) clones the manager into an `AppSession` and
runs the recursive cleaning loop; `apply` (`lib.rs:473`) is three lines that overwrite
and **discard the warnings**. The heart is `rec_apply_no_session` (`lib.rs:415`):

```rust
fn rec_apply_no_session<T: …Manager<Data = Data, Desc = Desc>>(
    &self,
    data: &mut T,
) -> Result<RecApplyResult, UpdateError> {
    let mut warnings = BTreeSet::new();

    while let Some(cleaning_op) = self.get_next_cleaning_op(data) {
        let warning_desc = cleaning_op
            .warning
            .build_desc_from_data(data)
            .expect("Warning should have a desc when applied on same state");
        warnings.insert((cleaning_op.warning, warning_desc));

        let result = cleaning_op.op.rec_apply_no_session(data)?;   // recursion
        warnings.extend(result.warnings);
    }

    let new_id = self.apply_no_cleaning(data)?;

    Ok(RecApplyResult { warnings, new_id })
}
```

Sixteen `get_next_cleaning_op` scans re-implement the reference graph by hand, one fix
per call, driven to fixpoint by the `while let`. Cleaning ops are themselves user-facing
`UpdateOp`s, so cleaning recurses across families (`CleaningOp`, `lib.rs:262`).
Termination rests on nothing but each scan eventually returning `None`. The per-family
`apply_no_cleaning` bodies feed plain `Op`s to `Manager::apply` and translate errors by
scanning the `BrokenInvariants` set in the old validator's order, with
`panic!("… should be cleaned before …")` coupling translation to cleaning.

The survey's totals, for scale: **34 warning variants** across 7 non-empty families (8
families have empty warning enums); **9 explicit "should be cleaned before" panics**
(`students.rs:469,511,517,523`, `week_patterns.rs:375,378`, `teachers.rs:213,252`,
`group_lists.rs:880`) plus implicit ones —
`DeleteSlot` has **no** `BrokenInvariants` arm at all (`slots.rs:513-516`), and
`general_planning.rs` does **zero** invariant translation (every apply is `.expect`ed).

### 1.2 The new primitives (what this step consumes)

- `apply_cascade` (`state/src/cascade.rs:85`): takes an **annotated** target, returns
  the history-ready `AggregatedOp` (target last) on success, restores a bit-identical
  snapshot on failure. Conviction rules, the no-progress ledger and the `ContentOrd`
  strictly-below assertion are inside (H.2, I.5, the review's termination rider).
- The resolution map (`state-colloscopes/src/resolution.rs`), total over
  `FixableInvariant`, every fix deletive. The full fix catalogue (with the
  invariant → variant mapping) is §5-C5.
- `Manager::apply` (`state/src/traits.rs:150`): annotate → gate → store a single-entry
  `AggregatedOp`. `annotate` takes `&mut self` since the review.
- `AppSession` (`state/src/state.rs:100`): blank history, `commit(desc)` collapses into
  one parent history slot, `cancel()` unwinds.

---

## 2. Target architecture

Six new pieces, bottom-up.

### 2.1a `Fix` / `FixOp` — the fix vocabulary (commit 1a)

The trait side, in `state/src/cascade.rs`:

```rust
// state/src/cascade.rs
/// A repair the resolution map can answer: one value of a closed, deletive
/// vocabulary. The variant carries everything its op needs (ids; the rebuilt
/// payload for whole-value ops), so translation is total and pure — it reads
/// no state and can be tested in isolation. Neither the map (`&self`) nor
/// this translation can reach the id issuer: a fix physically cannot carry a
/// fresh id.
pub trait FixOp: Clone + std::fmt::Debug {
    type Op;
    fn to_annotated_op(&self) -> Self::Op;
}

pub trait Fixable: InMemoryData + ContentOrd {
    /// The repair vocabulary (D15): one variant per *rendered meaning*.
    type Fix: FixOp<Op = Self::AnnotatedOperation>;
    fn fix_invariant(&self, invariant: &Self::Invariant) -> Option<Self::Fix>;
}
```

`fix_invariant`'s contract prose (`cascade.rs:47-76`) moves onto the new signature
essentially unchanged — presence test, single lookup, strict monotonicity, totality,
one step per call. What changes in each arm's *body* is only the last line: instead of
constructing the `AnnotatedOp` in place, it constructs the `Fix` variant, with the
payload built exactly where it is built today (single lookup — an earlier
two-function split was rejected, D15). `to_annotated_op` gets one arm per variant,
each a pure translation reproducing today's op construction verbatim.

The enum side, in `state-colloscopes/src/resolution.rs` (deliberately the same file
as the map — the vocabulary, the arms and the translation stay on one screen),
re-exported from the crate root beside `FixableInvariant` (`lib.rs:69`): `pub enum
Fix` with the ~25 variants of the §5-C5 catalogue, `derive(Clone, Debug, PartialEq,
Eq)`, plus `impl FixOp for Fix`. Variant *names* are polishable at implementation
(like the French templates); the partition — which invariants share a variant — is
the settled part.

The engine's push site (`cascade.rs:158-159`) materializes at push time:

```rust
match data.fix_invariant(&pick) {
    Some(fix) => stack.push((fix.to_annotated_op(), Some(fix))),
    …
}
```

Push-time materialization is deliberate: when a fix op itself fails and gets its own
sub-fix, the engine retries the original fix with its already-materialized payload —
that is today's semantics, byte-identical; materializing lazily at retry time would
be a silent behaviour change.

The toy `QuoteData` (`state/src/test_utils.rs`) gains a `QuoteFix` so the engine
tests keep compiling; the fifteen engine tests themselves are untouched by 1a (the
return type does not change yet).

### 2.1b `CascadeReceipt` — the engine return, re-shaped (commit 1b)

```rust
// state/src/cascade.rs
/// Everything a successful cascade landed: the fixes in application order,
/// each carrying the `Fix` it materialized from, and the target last.
pub struct CascadeReceipt<T: Fixable> {
    fixes: Vec<(ReversibleOp<T::AnnotatedOperation>, T::Fix)>,
    target: ReversibleOp<T::AnnotatedOperation>,
}

impl<T: Fixable> CascadeReceipt<T> {
    /// The fixes in application order, with their meanings.
    pub fn fixes(&self) -> &[(ReversibleOp<T::AnnotatedOperation>, T::Fix)];
    /// Rebuild the history-ready aggregated op (fixes in order, target last).
    pub fn into_aggregated_op(self) -> AggregatedOp<T::AnnotatedOperation>;
}

pub fn apply_cascade<T: Fixable>(
    data: &mut T,
    target: T::AnnotatedOperation,
) -> Result<CascadeReceipt<T>, ApplyError<T::InvalidOp, T::Invariant>>
```

Engine internals: today `stack: Vec<T::AnnotatedOperation>` and
`applied: Vec<ReversibleOp<…>>` (`cascade.rs:91-92`). With 1a's push site the stack is
already `Vec<(T::AnnotatedOperation, Option<T::Fix>)>` — the target pushed with `None`
(`cascade.rs:91`), each fix with `Some(fix)`; 1b makes `applied` collect the `Fix`
alongside each `ReversibleOp` at the single success site (`cascade.rs:138-141`). On
loop exit the last `applied` entry is the target by construction (assert it; its tag is
`None`), the rest split off as the fixes. The `Option` never leaves the engine — the
public type is exact (D2). The invariant pick never leaves the engine at all (D15): it
feeds the no-progress ledger and nothing else. Error behaviour, the monotonicity check
and the ledger are untouched.

### 2.2 `Manager::apply_cascade` (commit 2a)

```rust
// state/src/traits.rs — inside trait Manager, beside apply()
/// Apply `op` through the cascade (see [crate::cascade::apply_cascade]) and
/// keep the modification history consistent: the whole cascade lands as one
/// history slot. Returns the annotation's NewInfo and the fixes the cascade
/// had to apply, as their `Fix` meanings (D15).
///
/// On `Err`, data and history are unchanged; the id allocator may have
/// advanced (the annotation's ids burn, never to be reused) — the same
/// contract as [Manager::apply], and the same relationship `undo` has to
/// the allocator (undone ids stay live for `redo`). The allocator is
/// monotone across the manager's whole life; it is not part of rollback.
fn apply_cascade(
    &mut self,
    op: <Self::Data as InMemoryData>::OriginalOperation,
    desc: Self::Desc,
) -> Result<
    (
        <Self::Data as InMemoryData>::NewInfo,
        Vec<<Self::Data as crate::cascade::Fixable>::Fix>,
    ),
    ApplyError<…>,
>
where
    Self::Data: crate::cascade::Fixable,
{
    let (annotated_op, new_info) = self.get_in_memory_data_mut().annotate(op);
    let receipt = crate::cascade::apply_cascade(self.get_in_memory_data_mut(), annotated_op)?;
    let fixes = receipt
        .fixes()
        .iter()
        .map(|(_rev_op, fix)| fix.clone())
        .collect();
    self.get_modification_history_mut()
        .store(receipt.into_aggregated_op(), desc);
    Ok((new_info, fixes))
}
```

The `Err` contract holds with no manager-level snapshot: the engine restores its
entry snapshot bit-identically on every `Err` (`cascade.rs:89-90`, `:163`, `:176` —
pinned by engine tests 4, 5, 7 and 13), nothing inside a cascade can issue ids (the
map holds `&self`), and `store` runs only on `Ok`. Only the annotation's issuer bump
survives a failure — deliberately outside rollback, per D3 (an annotated-in variant
with a strictly-intact contract was rejected there: annotated ops are never publicly
injectable).

### 2.3 `CascadeSession`, `CascadeWarning`, `CascadeResult` (commit 2b, `ops/src/cascade.rs`)

```rust
// ops/src/cascade.rs (new module; `pub mod cascade; pub use cascade::*;` in lib.rs)
/// One warning attached to an update: a fix the cascade had to apply beyond the
/// user's own ops. That is the *only* kind there is (D12 void) — no composite
/// ever hand-writes one. Content is private (crate-private construction,
/// borrowed read-only accessor), so a warning can never desynchronize from what
/// actually happened. No text is stored: rendering is a method, computed on
/// demand against the composite's pre-state (D7).
pub struct CascadeWarning {
    /// The fix's `Fix` meaning (D15) — the invariant that caused it never
    /// leaves the engine.
    fix: collomatique_state_colloscopes::Fix,
}

impl CascadeWarning {
    /// Borrowed read-only view of the warning's content.
    pub fn fix(&self) -> &collomatique_state_colloscopes::Fix;
    // commit 5.1 adds:
    /// French, effect-phrased description, rendered against the composite's
    /// PRE-state (D7). `Err(MissingId)` if `data` does not hold the material
    /// the warning names (the frame rule's rendering corollary was violated);
    /// callers that must not fail (gtk4, the property walk) panic on it.
    pub fn text(&self, data: &Data) -> Result<String, MissingId>;
}

/// A modification session applying elementary ops through the cascade,
/// accumulating every fix as a [CascadeWarning]. Owns the manager; always
/// finish with [commit] or [cancel].
pub struct CascadeSession<T: Manager<Data = Data, Desc = Desc>> {
    session: AppSession<T, Desc>,
    warnings: Vec<CascadeWarning>,
}

impl<T: Manager<Data = Data, Desc = Desc>> CascadeSession<T> {
    pub fn new(manager: T) -> Self;
    /// Apply one elementary op through the cascade; fixes land in the warning
    /// log, the new id (if any) comes back inline.
    pub fn apply(
        &mut self,
        op: collomatique_state_colloscopes::Op,
        desc: Desc,
    ) -> Result<Option<collomatique_state_colloscopes::NewId>, collomatique_state_colloscopes::Error>;
    /// Collapse into one history slot on the owned manager.
    pub fn commit(self, desc: Desc) -> (T, Vec<CascadeWarning>);
    pub fn cancel(self) -> T;
}
```

With one shape left, `CascadeWarning` is a newtype over `Fix` — and it stays a distinct
type rather than collapsing to `Vec<Fix>`, because `text` has to live somewhere: `Fix`
belongs to `state-colloscopes`, which carries no French and no presentation (D6), so the
orphan rule puts the rendering method on an `ops/`-owned wrapper. Crate-private
construction rides along for free.

`apply` calls `self.session.apply_cascade(op, desc)` (2a), extends the log with the
returned fixes, and hands the `NewInfo` back — nothing more. Rendering never happens here (D7). There is **no** hand-written-warning channel:
the composites can only produce warnings by making the cascade fix something, so the
warning set cannot drift from what actually happened. Re-opening that door (the void
D12 wanted a `pub(crate)` push) needs a ruling in §0.

`CascadeResult<T>` mirrors today's `DryResult<T>` (`lib.rs:290`):

```rust
pub struct CascadeResult<T: Manager<Data = Data, Desc = Desc>> {
    pub warnings: Vec<CascadeWarning>,
    pub new_id: Option<collomatique_state_colloscopes::NewId>,
    pub new_state: T,
}
```

### 2.4 The per-family bodies (commits 3.1–3.15) — the translation doctrine

Each family gains `apply_to_session` **beside** its untouched `apply_no_cleaning`:

```rust
pub(crate) fn apply_to_session<T: …Manager<Data = Data, Desc = Desc>>(
    &self,
    session: &mut CascadeSession<T>,
) -> Result<Option<XxxId>, XxxUpdateError>
```

The body is the old `apply_no_cleaning` body with exactly four mechanical changes and
no other rewriting — the new code should read like the old code:

1. `data.apply(op, self.get_desc())` → `session.apply(op, self.get_desc())`;
2. the "should be cleaned before" panic arms are deleted — the invariant sets that
   reached them are now repaired by the cascade, never returned;
3. composite-internal recursion (`UpdateOp::…(…).rec_apply_no_session(data)`) becomes a
   direct call of the sibling's `apply_to_session`;
4. each precheck translation is restructured into a two-level match: peel to the
   family's own precheck enum, then match *that* exhaustively — no wildcard at the
   inner level (see the exhaustivity rider below).

Three doctrine riders:

- **Scan order is copied verbatim** (D5). Where a set can carry several breaks, which
  one wins is public API.
- **Exhaustivity where the type allows it; the residual catch-all panics stay
  outside.** The old bodies collapse each precheck translation to one deep pattern
  plus the wildcard, so a new precheck variant falls into the catch-all silently — at
  runtime. The new bodies peel to the family's own precheck enum and match it
  **exhaustively, no wildcard**: variants the arm cannot produce get explicit panic
  arms. A new variant in the family's vocabulary is then a compile error at every
  translation site — the discipline D6/D15 apply to `Fix`, applied to prechecks. The
  wildcard survives only on the outer `Error`, where per-arm exhaustivity is not
  meaningful (it would enumerate every other family's vocabulary); it and the
  remaining panics (`panic!("Unexpected invariant breaks during …")`,
  `panic!("Unexpected error during …")`) mean "the state layer produced an error this
  op cannot produce" — a bug; the fixtures establish unreachability. H.2's ruling
  applies: instruments, not safety nets. Mechanically: match on `&e`, never `e` —
  binding the inner enum by value partially moves the error, and the panic arms lose
  `{e:?}`.
- **Ops-level prechecks stay ops-level.** The `find_period_position(...).ok_or(...)`
  style checks (category (a) in the survey) are address checks the composites need
  *before* deciding what elementary ops to emit; they are not part of the cleaning
  machinery and do not move.

Worked example, `TeachersUpdateOp::DeleteTeacher` (old body `teachers.rs:228-264`):

```rust
Self::DeleteTeacher(teacher_id) => {
    let result = session
        .apply(
            collomatique_state_colloscopes::Op::Teacher(
                collomatique_state_colloscopes::TeacherOp::Remove(*teacher_id),
            ),
            self.get_desc(),
        )
        .map_err(|e| {
            use collomatique_state_colloscopes::{
                Error, InvalidOp, PrecheckError, TeacherPrecheckError,
            };
            match &e {
                Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Teacher(te))) => match te {
                    TeacherPrecheckError::InvalidTeacherId(id) => {
                        DeleteTeacherError::InvalidTeacherId(*id)
                    }
                    TeacherPrecheckError::TeacherIdAlreadyExists(_) => {
                        panic!("Unexpected TeacherPrecheckError during DeleteTeacher: {e:?}")
                    }
                },
                _ => panic!("Unexpected error during DeleteTeacher: {e:?}"),
            }
        })?;

    assert!(result.is_none());

    Ok(None)
}
```

The whole `Error::BrokenInvariants` arm is gone: a teacher referenced by slots no longer
errs — the cascade removes the slots (and whatever dangles from *them*), each removal
landing in the warning log. Content-write cases keep their translation verbatim
(`AddNewTeacher`'s scan for `SubjectRefSite::TeacherSubjects` dangles →
`InvalidSubjectId`, `teachers.rs:156-166`): the map answers `None` for invariants caused
by the failing op's own payload, the engine convicts the target, and the set surfaces
exactly as before.

### 2.5 The top-level dispatch (commit 3.16)

```rust
impl UpdateOp {
    fn apply_to_session<T: …>(&self, session: &mut CascadeSession<T>)
        -> Result<Option<NewId>, UpdateError>;   // 15-arm dispatch, like apply_no_cleaning

    pub fn cascade_dry_apply<T: …>(&self, data: &T) -> Result<CascadeResult<T>, UpdateError> {
        let mut session = CascadeSession::new(data.clone());
        let new_id = self.apply_to_session(&mut session)?;   // Err ⇒ session dropped (a clone)
        let (new_state, warnings) = session.commit(self.get_desc());
        Ok(CascadeResult { warnings, new_id, new_state })
    }

    pub fn cascade_apply<T: …>(&self, data: &mut T) -> Result<Option<NewId>, UpdateError> {
        let result = self.cascade_dry_apply(data)?;
        *data = result.new_state;
        Ok(result.new_id)     // warnings deliberately discarded, as today (D10)
    }
}
```

**The prefix-survival frame rule** (engraved in `CascadeSession`'s doc): a composite must
be written so that each of its ops is valid against the state produced by its own earlier
ops *and their cascades*. A composite whose later op is convicted because an earlier op's
cascade consumed its target is a bug in the composite, not user input. **Rendering
corollary (D7)**: a composite's cascades must only ever touch material present in the
composite's pre-state — a fix on material a previous op of the same composite created is
the same kind of bug, and would produce a warning the user cannot comprehend. The
per-composite fixtures establish both halves; the residual catch-all panics and the
renderer's lookup panic are where a violation would surface.

---

## 3. Commit plan overview

Every commit builds green and runs the full workspace suite (background, captured once
to the scratchpad and grepped — never run twice).

The `landed` column is the authoritative position record — fill it in as each commit
lands.

| commit | content | crates | landed |
| --- | --- | --- | --- |
| 0 | `PeriodOp::Remove` → `RemoveWithWeeks` rename (D8, first revision) — **reversed by 0bis** | state-colloscopes, testgen-colloscopes, ops | `8e53118b` |
| 0bis | reverse commit 0: the elementary op is plain `Remove` again (D8, final) | state-colloscopes, testgen-colloscopes, ops | `8e81a16b` |
| 0ter | `DeletePeriod` → `DeletePeriodAndWeeks` user-facing rename (D8, final) | ops, gtk4, python | `04f90645` |
| 1a | `FixOp` trait + `Fix` enum + map refactor + attribution pins | state, state-colloscopes | `ec14c9b2` |
| 1b | `CascadeReceipt` engine re-shape + test adaptation | state, state-colloscopes (tests) | `51f04459` |
| 2a | `Manager::apply_cascade` + toy tests | state | `0040e4e4` |
| 2b | `CascadeSession`/`CascadeWarning`/`CascadeResult` + struct tests; frozen hogwarts fixture copy + storage dev-dep (⇒ **cargoHash**) | ops | `8ebea0cd` |
| 3.1–3.15 | one family per commit, `apply_to_session` + family fixtures | ops | 3.1 `0b948537` … 3.14 `f837ff0a`, 3.15 `4d77427c` — **all fifteen landed** |
| 3.3bis | typed rejection for balancing options on a no-interrogation subject (D5's growth rule, §3.3), built test-first: the crash pinned, then the variant, then the guard | ops | `9b8a7875` |
| 3.12+ | restore the prefilled-group-list error (D5's growth rule, §3.12): a) this plan, b) the variant with no emitter, c) the pin, **committed red**, d) the emitter on the new path only | docs, ops | `77fb1948`…`e30abb92` |
| 3.13bis | move the interrogation-row convergences ahead of the association ones (§3.13) | state-colloscopes, ops | `9ef4299b` |
| 3.16 | `UpdateOp` dispatch + `cascade_dry_apply`/`cascade_apply` | ops | `e00c62ff` |
| 4 | the `UpdateOp` property walk (testgen dev-dep ⇒ **cargoHash**) | ops | `6ca28b95` |
| 5.0 | `rendering.rs` — shared noun-less id renderers + `MissingId` + `join_french` | ops | `509fb83c` |
| 5.1 | `warning_text.rs` renderer + `CascadeWarning::text` | ops | `883657bb` |
| 5.2 | all the tests: walk renders every warning; rendering unit tests | ops | `b043b8a2` |
| 6.0 | the rendering helpers take the document parts they read, not `&Data` — the precondition for 6.1 | ops | `d21b61e3` |
| 6.1 | gtk4 helper dedup — local renderers replaced by `ops::rendering` | gtk4 | — |
| 6.2 | gtk4 warning-dialog switch (gtk4 smoke here covers 6.1 too) | gtk4 | — |
| 6.3 | python switch (contract scripts run here) | python | — |
| 6.4 | drop dead rpc-engine dep (⇒ **cargoHash**) | rpc-engine | — |
| 7 | delete the old world + final rename + test re-cuts | ops, gtk4, python | — |
| close-out | design doc Appendix J, §8, retire this plan, memory | docs | — |

Interstitial work landed between step-7 commits, recorded here so the history reads
straight: `16c3c0b5` + `c2fc945b` (the D8 re-ruling and its design-doc note, between 3.14
and 0bis) and `7b7da087` (the `InterrogationGroupsOutOfBounds` pluralization, §5-C5's
last catalogue row).

Family order for commit 3, simple → complex (grouping trivial ones is fine if they stay
reviewable): 3.1 export_config, 3.2 settings, 3.3 balancing, 3.4 teachers, 3.5
incompatibilities, 3.6 pairings, 3.7 slot_pairings, 3.8 assignments, 3.9 students, 3.10
week_patterns, 3.11 slots, 3.12 colloscope, 3.13 subjects, 3.14 group_lists, 3.15
general_planning.

---

## 4. Commits 0–2 in detail

### Commits 0, 0bis, 0ter — the naming of period removal

Commit 0 renamed `PeriodOp::Remove` / `AnnotatedPeriodOp::Remove` to
`RemoveWithWeeks`, reasoning that the name should advertise the cascade contract
(no week-empty guard; leftover weeks are cascade-deleted). That reasoning put the
user-facing semantics on the wrong layer, and commit 0bis reverses it: an
elementary op's name must say what the op *does* — remove the period — not what
the surrounding system does about the aftermath. The cascade contract lives on in
the variant's doc comment, where commit 0 correctly moved it: period removal has
**no** week-empty guard, and any weeks still on the period dangle at
`Week::period_id` for the cascade to delete (`weeks.rs:28-31`, design doc
Appendix F.4). The two reverse-annotation arms in `force_apply_period`
(`AddFront`/`AddAfter` answer `Remove(id)` as their reverse) rename back with it,
and are plainly honest: the reverse of adding an empty period is removing it. No
elementary op enum carries serde, so no stored format moves; all sites are
compiler-found (~20 across `state-colloscopes` src+tests,
`testgen-colloscopes/src/generator.rs`, `ops/src/general_planning.rs:1113`).

Commit 0ter puts the semantics where they belong: the user-facing
`GeneralPlanningUpdateOp::DeletePeriod` becomes **`DeletePeriodAndWeeks`**,
because that is what the user means and what the composite does — it removes the
weeks first (authored, so the cascade never emits a per-week fix or warning for
them), then the period. `DeletePeriodError` and the
`GeneralPlanningUpdateError::DeletePeriod` variant rename to match
(`DeletePeriodAndWeeksError`, `DeletePeriodAndWeeks`). The `ops/` enums derive
serde (the variant name is the JSON tag), but nothing in-tree serializes them;
the French UI label (« Supprimer une période ») is untouched.

### Commit 1a — the fix vocabulary

Sites: `state/src/cascade.rs` (§2.1a — the `FixOp` trait, the `Fixable::Fix`
associated type, the retyped `fix_invariant`, the materializing push site);
`state/src/test_utils.rs` (`QuoteFix` for the toy types); `state-colloscopes/src/resolution.rs`
(the `Fix` enum per the §5-C5 catalogue, `impl FixOp`, and the arm-by-arm refactor:
each arm keeps its presence test and its single lookup, and its last line builds the
variant instead of the op); `state-colloscopes/src/lib.rs:69` (re-export `Fix` beside
`FixableInvariant`).

The refactor is where the map *dedupes*: materializations that today repeat across
arms (the `AssignToSubject(period, subject, None)` unassign appears four times, the
group-list placement rebuild three times, the assignment row rebuild twice) become one
`to_annotated_op` arm per variant; where several `fix_invariant` arms build the same
variant they may share a private helper.

Behaviour is byte-identical: `apply_cascade` still returns `AggregatedOp`, so the
engine's 15 tests, the 20 cascade fixtures and `property_cascade.rs` run **unchanged**
in this commit — they are the proof the refactor moved code without changing it.

New unit tests — the attribution pins (D15 relocated them here): direct
`fix_invariant` tests in `resolution.rs`'s in-file test module, one per `Fix`
variant — state + invariant in, expected full `Fix` value out (payload included for
the rebuild shapes), plus `to_annotated_op` translation pins on the payload-carrying
variants (pure function, trivially testable).

### Commit 1b — `state/`: `CascadeReceipt`

Sites: `state/src/cascade.rs` (§2.1b — the struct, `applied` collecting the tag, the
split on exit). Consumers to adapt, all mechanical:

- the engine's own tests (`cascade.rs`, 15 tests): `forward_ops` helper reads
  `receipt.into_aggregated_op()` or `fixes()` + target directly;
- `state-colloscopes/tests/cascade.rs` (20 fixtures): op-list asserts move to the
  receipt's accessors — expected fix lists become `Vec<Fix>` literals (compact for the
  id-only variants; the rebuild shapes carry their expected payload, derivable from
  the same in-process builders);
- `state-colloscopes/tests/property_cascade.rs`: `cascade_step` reads
  `applied.inner().len() - 1` for the fix count (`property_cascade.rs:255`) and replays
  `applied.rev()` (`:264-274`) — both re-expressed on the receipt
  (`fixes().len()`; `into_aggregated_op().rev()`).

New unit tests: the tagging itself on the toy `QuoteData` — the two-round repair of
`happy_cascade_repairs_in_canonical_order` (`cascade.rs:223-240`) asserts `fixes()`
carries the expected `QuoteFix` values in order and the target is last, untagged.

### Commit 2a — `state/`: `Manager::apply_cascade`

Site: `state/src/traits.rs`, beside `Manager::apply` (§2.2). Note the per-method
`where Self::Data: Fixable` bound — the rest of `Manager` stays available for
non-`Fixable` implementors (`FakeData` in the trait tests).

Tests (toy types, `traits.rs` test module): a cascading op stores **one** history slot
whose aggregated op holds fixes + target and undoes in one step; a convicted op
leaves **data and history** unchanged (asserted against pre-call copies — the
allocator is explicitly *outside* this assertion, per D3's contract); the returned
fixes are the expected `QuoteFix` values; `NewInfo` comes back from annotation.

### Commit 2b — `ops/`: the session struct

Site: new `ops/src/cascade.rs` (§2.3), `pub mod cascade; pub use cascade::*;` in
`lib.rs`. This commit also lands the frozen fixture base:
`ops/tests/fixtures/hogwarts.collomatique` copied from `examples/`, plus the
`collomatique-storage` dev-dependency (§7; ⇒ **cargoHash**). The struct needs no
`UpdateOp` — tests drive **raw elementary ops** against the hogwarts base (a real
teacher with real slots, no in-process document building):

- delete a teacher who has slots → warnings accumulate as `Fix::DeleteSlot` values
  (plus each slot's own colloscope/pairing fixes), in application order;
- `commit` collapses to one undo slot on the returned manager (undo restores fully);
- `cancel` returns the manager untouched;
- an `Add` op hands its id back inline;
- a convicted op returns `Err` and adds nothing to the log.

---

## 5. Commit 3 — the fifteen families

Format per family: what the survey found → what changes → fixtures. "Warnings via"
names the invariant whose fix the cascade emits; the fixture asserts the resulting
`Fix` values (the invariant itself is engine-internal, D15 — the invariant → variant
mapping is the §5-C5 catalogue). Old-code anchors are in the survey reports; key ones
are repeated here.

### 3.1 `export_config.rs` — trivial

Eleven **ops-level** variants (the user-facing granularity, each with its own French
history description), but only **one** elementary op behind them since the pre-step-7
sidework of July 31 2026 (plan retired, pinned at
`git show 15b59b1c:docs/plans/plan_export_config_op.md`): each variant reads the
current config, patches its one field, and issues the single whole-struct
`ExportConfigOp::Update`. Error enum **empty**, the one apply still
`.expect("… should never fail")`.

Change: the current-config read moves from `data.get_data().get_inner_data()` to the
session's read surface — `apply_to_session` gets no `data` parameter (§2.4), so the
accessor the read-modify-write families need (assignments' snapshot maps, colloscope's
lookups) serves this one too; and `data.apply` → `session.apply`, the expect kept. No
warnings possible (export config references nothing). Fixture unchanged: one op
round-trips, zero warnings.

### 3.2 `settings.rs` / 3.3 `balancing.rs` — precheck-only twins

Three variants each, single elementary, errors are **all ops-level prechecks** including
the deliberate "removing an absent override" detection (`SetStudent(_, None)` is a
state-level no-op, so `ops/` detects `NoLimitsForStudent`/`NoOptionsForSubject` itself —
`settings.rs:124-136`, `balancing.rs:124-136`). Keep all prechecks; swap the apply call.
No warnings possible as *targets*. Fixtures: error surface pins + zero warnings on the
happy path. Also fix in passing (commit 3.2): `settings.rs:55`'s
`get_next_cleaning_op` return type says `CleaningOp<WeekPatternsUpdateWarning>` — a
copy-paste leftover; it dies in commit 7 anyway, so only note it, don't churn it.

**The twins are not symmetric (commit 3.3bis, D5's growth rule).** "Expects kept" held
for settings and *not* for balancing, and the sentence above hid the difference until
commit 3.3 replicated the `.expect`s and found it. For settings a live student is
enough: the settings table's only edge is the student key, and no convergence predicate
mentions it. For balancing a live subject is **not** enough — only an interrogated
subject may carry options (`Convergence::BalancingForSubjectWithoutInterrogations`), so
`UpdateSubjectOptions` on a subject with interrogations disabled reached the state
layer, got convicted (the map answers `None`: the rolled-back entry is not in the state,
so there is nothing to repair) and killed the process on data-dependent input. Commit
3.3bis added `UpdateSubjectOptionsError::SubjectHasNoInterrogation(SubjectId)` and the
guard that emits it, beside the existing address check and reusing its single
`find_subject` lookup. Nothing else needed wiring: the balancing family has no Python
surface, and gtk4 renders update errors through `to_string()`. The old
`apply_no_cleaning` body keeps the hole deliberately — it dies at commit 7, and gtk4
cannot reach it (the balancing panel only lists subjects with interrogations).

### 3.4 `teachers.rs`

| variant | keeps | loses | warnings via |
| --- | --- | --- | --- |
| `AddNewTeacher` | `InvalidSubjectId` content scan (`:156-166`) | — | none (add breaks nothing else) |
| `UpdateTeacher` | `InvalidTeacherId` precheck, `InvalidSubjectId` scan | `panic!("Slots should be cleaned …")` (`:213`) | `SlotTeacherDoesNotTeachSubject` → slot removed (+ transitive colloscope/pairing fixes) |
| `DeleteTeacher` | `InvalidTeacherId` precheck | the whole `BrokenInvariants` arm (`:245-256`) | `DanglingFk(Teacher@SlotTeacher)` → slots removed |

**Additive variant needed (D5's concrete case)**: adding/updating a teacher whose
`subjects` names a subject with interrogations **disabled** breaks
`Convergence::TeacherSubjectWithoutInterrogations`; the map's presence test fails (the
rolled-back op's pair is not in the state) → `Err`. Today that set misses every scan and
**panics** (`teachers.rs:166/216`) — reachable from Python. New:
`AddNewTeacherError::SubjectHasNoInterrogation(SubjectId)` (and the `Update` twin),
scanned after the dangling-subject pass. Fixtures pin both.

Fixtures: delete-teacher-with-slots (exact warning list: one `Fix::DeleteSlot` per
slot, plus that slot's own colloscope/pairing fixes in engine order); update dropping
a subject; the two new error variants; clean paths.

### 3.5 `incompatibilities.rs` / 3.6 `pairings.rs` / 3.7 `slot_pairings.rs`

Mechanical: all single-elementary, translations are content-write dangle scans (kept
verbatim — note the two-pass antecedent/consequent scans sharing one `InvalidSubjectId`/
`InvalidSlotId` variant, `pairings.rs:102-119`, `slot_pairings.rs:114-131`) plus
`PairedSlotsNotInSameSubject` for slot pairings. Deletes have no invariant arms today
(nothing references these entities — no `Reference` variant exists for their ids) and
that stays true. Fixtures: error surfaces + zero-warning deletes.

### 3.8 `assignments.rs`

`Assign` keeps its full five-variant translation **in its documented order**
(`assignments.rs:163-181`: payload-student dangle scanned first — the pre-step-7
review's address/content settlement) — `ops/tests/assignments_error_surface.rs` pins
this and must stay green untouched through this commit. `AssignAll` and
`DuplicatePreviousPeriod` are ops-level-precheck-only with `.expect`ed applies; the
composite's read-modify-write logic is unchanged (snapshot maps, then one `SetRow` per
subject — no ids, no recursion). No warnings possible as targets (assignment rows
reference only prechecked/live material). Fixtures: the composite's merge semantics on a
known document; zero warnings.

### 3.9 `students.rs`

| variant | keeps | loses | warnings via |
| --- | --- | --- | --- |
| `AddNewStudent` | `InvalidPeriodId` content scan | — | none |
| `UpdateStudent` | `InvalidStudentId` precheck, `InvalidPeriodId` scan | `panic!("Assignments should be cleaned …")` (`:469`) | `AssignedStudentNotPresentForPeriod` → row rebuilt minus the student |
| `DeleteStudent` | `InvalidStudentId` precheck | all three cleaned-before panics (`:511,517,523`) | `Student@GroupListPrefilledStudent/GroupListExcludedStudent/AssignmentsStudent/SettingsStudentKey/ColloscopeGroupListStudent` |

Old cleaning order (colloscope → group lists → assignments → settings,
`students.rs:212-346`) is replaced by canonical invariant order; the *set* of effects is
identical (the survey confirmed the old scans and the map's arms cover the same five
sites). Fixtures: delete a fully-connected student (exact list), update excluding a
period with assignments, error pins.

### 3.10 `week_patterns.rs`

The behaviour-divergence family (§6.3). `DeleteWeekPattern` loses its two cleaned-before
panics (`:375,378`); the cascade **keeps** referencing slots and incompats, clearing
their `week_pattern` to `None` (`resolution.rs:411-446`, the map's one recorded
divergence from legacy, ★ ruled July 28). Old cleaning *deleted* them (with their
colloscope data); the new document keeps the slot running **every week**. `UpdateWeekPattern`'s
colloscope cleaning (cells on newly-excluded weeks) is replaced by the convergence
route: excluding a week makes interrogations on it impossible for slots following the
pattern, so existing cells break the `InterrogationOnInactiveWeek`-family convergences
and the cascade clears exactly those, warned. **Derive the fixture's expected set from
the checker and the §5-C5 table, not from this paragraph** — which convergence variant
fires is the checker's business. Fixtures: delete a referenced pattern (rows kept,
`week_pattern = None`, mutation-checked against the old delete-the-rows expectation);
update excluding a week that has cells.

### 3.11 `slots.rs`

`DeleteSlot` today has **no** invariant arm — colloscope rows and pairing rules land in
`panic!("Unexpected error during DeleteSlot")` if the cleaning missed them
(`slots.rs:513-516`). New: the catch-all becomes genuinely unreachable — cascade clears
cells (`Slot@ColloscopeInterrogation`) and removes pairing rules
(`Slot@SlotPairingRuleAntecedent/Consequent`), all warned. `UpdateSlot` keeps its
five-variant invariant scan in order (`slots.rs:433-486`; the sixth surface variant,
`InvalidSlotId`, is the precheck above it, and the in-code comment at `:425-432`
records `InvalidSubjectId` as unreachable — D14's dead variant); its cleaning (cells on weeks excluded by
the *new* pattern) is replaced by the same convergence route as week patterns.
`AddNewSlot` keeps both ops-level prechecks and its four-variant scan. Move ops
unchanged (D14 wart replicated). Fixtures: delete-slot-with-everything (exact list);
update changing the pattern; the `SlotOverflowsDay` target rejection
(`UpdateSlot(S, 23:00)` still errs `SlotOverlapsWithNextDay` — the map's presence test
on the *old* start answers `None`, the engine convicts with the remembered break; this
pins the conviction route end-to-end through `ops/`).

### 3.12 `colloscope.rs`

The richest existing translation (both `InvalidOp` and three-pass `BrokenInvariants`
scans in old-validator order for both update ops, `colloscope.rs:111-230`) — kept
verbatim; these are all target-caused (the map's presence tests answer `None` on
colloscope content writes because the row was rolled back). The two erase composites
keep their collect-then-clear shape and `.expect("No error possible for erasing")` —
clearing only removes, so no cascade ever fires. Fixtures: error-surface pins (the
five-variant interrogation surface), erase round-trips with zero warnings.

**Follow-up (D5's growth rule, four commits after 3.12).** Writing the fixtures turned
up one shape that reaches the group-list arm's residual catch-all on reachable input: a
row aimed at a **prefilled** list. The state layer lets the write through (the
`is_prefilled` guard is stripped from `force_apply_colloscope`,
`colloscopes.rs:186`), the checker reports `Conv:ColloscopeGroupListPrefilled`
(`invariants.rs:632`), and the map's arm for it (`resolution.rs:965`) tests row
*presence* — which a rolled-back write on a prefilled list never has — so it answers
`None` and the target is convicted with a break no scan names.

Git says this is a **regression, not a gap**. Before step 4 the live path was the old
checked apply, whose `SetGroupList` body rejected a prefilled target outright and
`ops/` translated it (`git show 59008052^:ops/src/colloscope.rs` — the guard is at
`git show 56510199^:state-colloscopes/src/colloscopes.rs`, ~`:361`, reusing
`ColloscopeError::InvalidGroupListId` with a comment admitting the op "targets them by
mistake"). Step 4's `force_apply` copies dropped the guard by design (step-3 survey
Table 1: strip the semantic guards, keep coordinate existence), the condition became a
plain invariant, and nothing in `ops/` was ever taught to name it.

So the follow-up restores the error with a name of its own instead of the old overloaded
one: `UpdateColloscopeGroupListError::PrefilledGroupListInColloscope(GroupListId)`. It is
scanned **first**, above the three placement scans, because that is where the guard sat
in both old bodies (before `validate_group_list_placements`). Consequence, stated
plainly: a prefilled list named together with a dead student now answers the prefilled
case where commit 3.12 answered `InvalidStudentId` — the historical answer, restored.
Only `apply_to_session` is taught; the old `apply_no_cleaning` keeps the panic and dies
with the rest of it at commit 7.

### 3.13 `subjects.rs`

The family where old cleaning and the cascade align almost 1:1 — worth pinning as such:

- `UpdateSubject` disabling interrogations: old cleaning ran teachers → associations →
  slots → balancing (`subjects.rs:321-426`); new: the same four effects arrive as
  `TeacherSubjectWithoutInterrogations`, `AssociationForSubjectWithoutInterrogations`,
  `SlotForSubjectWithoutInterrogations`, `BalancingForSubjectWithoutInterrogations`
  convergence fixes, in canonical order.
- `UpdatePeriodStatus(false)`: old assignments/colloscope/association cleaning → new
  `AssignmentForSubjectNotRunningOnPeriod`, `InterrogationSlotNotRunningOnPeriod`,
  `AssociationForSubjectNotRunningOnPeriod` fixes.
- `DeleteSubject`: seven old cleaning phases → the seven `Subject@…` dangle arms.
- **§6.2 lands here**: lengthening an interrogation over a late slot no longer aborts
  the process (`subjects.rs:758` `.expect`) — the cascade removes the overflowing slot
  (`SlotOverflowsDay` fix tests `start`, deliberately not `duration`) and warns.
  Fixture is the ops-level twin of step 6's fixture 1b, mutation-checked.
- `DeleteSubject` keeps the module's sole `PrecheckError` translation
  (`subjects.rs:772-788`); the documented catch-all (`:786`) stays.

**What the fixtures found about ordering** (August 1 2026), two things that must not be
confused. The first needs no action: the engine is depth-first *and* rolls the failing
target back while it hunts for a fix, so a repair that cannot land yet has its own
repairs land before it — striking a subject off a teacher's list is refused while that
teacher still holds its slots, so the slots go first. The warning list is therefore not
in the checker's order, and the removal and interrogation fixtures pin that inversion.
That is what the design does; nothing to change.

The second was a real defect in what the user reads, fixed in commit **3.13bis** (user
ruling, same day). `UpdatePeriodStatus(false)` never reaches
`InterrogationSlotNotRunningOnPeriod` — that break describes the state the rolled-back
target *would* produce, and the association break outranked it — so the association was
cleared first, the group bound at that coordinate fell to zero, and every group of every
cell there became its own `InterrogationGroupOutOfBounds` — the variant's shape at the
time, one break per group number, so the colles died group by group. The fix: the three
interrogation-row predicates move ahead of the two association ones in `invariants.rs`,
as a block, leaving association-before-balancing untouched.

This is a judgement about that one pair and the sentences it produces. No general
ordering principle is claimed, and none of the other placements were revisited. Scope:
only the *pick* moves, and only where those two convergences co-occur — every fix list
that reaches its cell repairs through a dangling reference is untouched, dangles sorting
ahead of every convergence. And unassigning a group list **directly** still collapses
the bound with no interrogation-row break in sight, so those cells are emptied by the
trim fix rather than cleared as colles; that case reads the user's own edit back to
them, and is left alone. (Since then the break has been pluralized into
`InterrogationGroupsOutOfBounds`, one per cell naming all its groups, so such a cell
now empties in a single fix.)

### 3.14 `group_lists.rs`

Five variants since the global-group-list work (`AddNewGroupList`, `UpdateGroupList`,
`DeleteGroupList`, `AssignGroupListToSubject`, `DuplicatePreviousPeriod` — the
parameters-only `UpdateGroupList` and `SetFilling` are gone, merged into one op
carrying a whole sealed `GroupList`). `DuplicatePreviousPeriod` is the composite
(snapshot previous-period associations, one `AssignToSubject` per eligible subject —
no ids, no recursion). The old module's eleven cleaning scans across four arms — four
on `UpdateGroupList` (all colloscope-erasing), five on `DeleteGroupList` (two
colloscope-erasing, three pre-cleaning the doomed list's own filling and association),
one interrogation-trimming scan each on `AssignGroupListToSubject` and
`DuplicatePreviousPeriod` — are replaced by `ColloscopeStudentGroupOutOfBounds` /
`InterrogationGroupsOutOfBounds` / `ColloscopeStudentExcluded` /
`ColloscopeGroupListPrefilled` convergence fixes and the two `GroupList@…` dangle
arms. The old panic-only invariant scan
(`panic!("Associated subjects should be properly cleaned")`, `group_lists.rs:880`)
dies.

Both `AddNewGroupList` and `UpdateGroupList` hand the state layer a payload that is
already whole and already validated, so the bodies neither pad nor truncate anything —
and with D12 void there is nothing for them to detect and emit either. All eleven
ops-level prechecks kept, including **both** student-existence sweeps
(`AddNewGroupList`'s and `UpdateGroupList`'s — the merge moved the old `SetFilling`
sweep onto the update and gave Add one of its own).

### 3.15 `general_planning.rs`

The big one. All nine variants keep their structural bodies (D8); the module keeps its
**zero-translation** style except where D13 adds one. *(Line numbers below re-verified
August 1 2026, after commit 0ter shifted this file by four lines.)*

- `AddNewPeriod` / grow-branch `UpdatePeriodWeekCount`: id-threading loops unchanged
  (`WeekOp::AddFront`/`AddAfter` chains); infallible, expects kept.
- `UpdatePeriodWeekCount` shrink: the loop `WeekOp::Remove(week_id)` per dropped week —
  each removal cascades cells (`Week@ColloscopeInterrogation`) and pattern bits
  (`Week@WeekPatternExcludedWeek`) with warnings; the
  `.expect("Cleaning made the removed weeks trivial")` (`:1072`) becomes
  `.expect("the cascade resolves everything a week removal breaks")`.
- `DeletePeriodAndWeeks`: remove weeks in reverse (cascading as above), then
  `PeriodOp::Remove` — whose landing cascades the period-scoped remnants
  (`Period@SubjectExcludedPeriods/StudentExcludedPeriods/PairingRuleExcludedPeriods/
  SlotPairingRuleExcludedPeriods/AssignmentsKey/AssociationEntry`), all warned. The
  composite authors the week removals itself not out of necessity — the bare op would
  cascade them away (D8) — but because they are what its name promises: the user
  asked for the weeks to go, so no « semaine supprimée » fix or warning may appear,
  only each week removal's own cascade on genuinely surprising content. The old
  eight-phase cleaning dies. **D13**: translate `InvalidPeriodId` precheck instead of
  `.expect`ing (`:1121`).
- `CutPeriod`: unchanged structurally (the five-step id-threading body incl. the
  exclusion/assignment/association copies that must precede the moves,
  `:1167-1172`) — it cleans nothing today (`:413`) and nothing changes.
- `MergeWithPreviousPeriod`: **the fixme fix, §6.1.** Move the weeks (content travels
  with `WeekId` — cells are keyed `(SlotId, WeekId)` and untouched), then call the
  sibling `DeletePeriodAndWeeks` `apply_to_session` directly (doctrine change 3, replacing
  `rec_apply_no_session` at `:1388`). The old reconcile-with-previous cleaning
  (six phases, `:674-897`) dies entirely: the dead period's config is dropped by
  `DeletePeriodAndWeeks`'s cascade instead of being aligned first. Cells survive unless the
  surviving period's context genuinely invalidates them (then
  `InterrogationSlotNotRunningOnPeriod`/`ColloscopeStudentGroupOutOfBounds`-family
  fixes clear exactly those, warned).
- `UpdateWeekStatus(false)`: old colloscope cleaning → `InterrogationOnInactiveWeek`
  fixes on the week's cells.
- Dead variant `UpdatePeriodWeekCountError::SubjectImpliesMinimumWeekCount`
  (`:275`) stays dead **in this commit** — D14 deletes it at commit 7, and until then
  the old surface is replicated verbatim so commit 3 stays mechanical.

Fixtures: **merge preserves colloscope data when group lists are compatible**
(test-first in spirit, mutation-checked — the old path is not being fixed, so the pin is
written against the new composite; delete `docs/todos/fixme_ops.md` only at commit 7);
merge with *incompatible* group lists clears exactly the invalid cells; period shrink
and delete with exact warning lists; `DeletePeriodAndWeeks` on a dead id returns
`InvalidPeriodId` (D13).

**As landed** (commit `4d77427c`, fourteen fixtures, twenty-three mutations). The audit
turned up no reachable panic to name: D13's translation is the module's only one, and
every `.expect` kept got a message saying why nothing can fail. The one argument worth
recording is `CutPeriod`'s: its four copy loops run *before* the weeks move precisely so
that a copied row or association is valid at the new coordinate for the same reason it
is valid at the old one — which is also what makes the cut lose no colle. Hogwarts
excludes nobody from anything, so the two exclusion copy loops needed a fixture of their
own that builds an absent subject and an absent student first.

### 3.16 — dispatch + transitional API

§2.5. `ops/tests/general_planning_content.rs` (the `CutPeriod` content-preservation
contract) gets a twin running through `cascade_dry_apply`, proving the new path
preserves content identically — the old file keeps running against the old path until
commit 7.

**As landed** (commit `e00c62ff`, three fixtures, four mutations). The document setup and
the post-cut assertions became shared helpers, so the two paths are read back through the
same sentences and the old half is a clean deletion at commit 7. The twin adds what only
the new path has: the cut's warning list is **empty** — a cut that had to warn about a
colle would be a cut that lost one — and the merge's is exactly one
`UnassignGroupList` on the emptied period, the copy of the association the cut had given
it. Both were derived by hand before the run and both held. The twin also pins §6.1
end-to-end through the public API (the colle comes back with its week), which the family
fixtures pin from inside. A third fixture covers `cascade_apply`, which has no warnings
to show and must still install the state and hand the created id back. Mutations: the
dispatch dropping the general-planning id, `cascade_dry_apply` emptying its warning list,
`cascade_dry_apply` returning the pre-state, `cascade_apply` not installing.

---

## 5-C4. Commit 4 — the property walk

New `ops/tests/property_update_ops.rs`. Adds `collomatique-testgen-colloscopes` as a
**dev-dependency of `ops/`** (Cargo.lock changes ⇒ user runs the Nix cargoHash refresh
before this commit; the dev-dep cycle pattern is already established —
`state-colloscopes` does the same).

Two deliberate deviations from the `property_cascade.rs` template, discovered at survey:

- **Own seed loop, not `harness::for_each_seed`**: the harness's cross-seed guard
  asserts every entry of `generator::CATEGORIES` (the 17 *elementary* categories) was
  attempted, and its `OpLog::push` is typed to elementary `Op` — both wrong for
  `UpdateOp`s. The loop is small: `ChaCha8Rng::seed_from_u64(seed)` for
  `seed in 0..CONFIG.seeds`, panic context = the seed number.
- **Bootstrap re-homed onto the ops `Desc`**: `harness::bootstrap` returns
  `AppState<Data, String>`; rebuild as
  `AppState::new(bootstrapped.get_data().clone())` with `Desc = (OpCategory, String)`.

Skeleton:

```rust
const CONFIG: RunConfig = RunConfig { seeds: 100, ops_per_run: 500, invalid_fraction: 0.15 };

// In-file generator: gen_update_op(rng, data, invalid_fraction) -> (&'static str, UpdateOp)
// - pools of live ids read from data.get_data().get_inner_data()
// - entity payloads from testgen's synth::* builders
// - invalid draws swap in ids from a DANGLING_BASE-style const
// - covers all 15 families, weighted so destructive ops (deletes, merges, shrinks)
//   are frequent enough to exercise the cascade

#[test]
fn update_ops_never_panic_and_land_valid() {
    let landed = Cell::new(0usize);
    let warned = Cell::new(0usize);       // runs that produced ≥1 warning
    let errored = Cell::new(0usize);
    for seed in 0..CONFIG.seeds {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut state = /* re-homed bootstrap */;
        for _ in 0..CONFIG.ops_per_run {
            let (_family, op) = gen_update_op(&mut rng, &state, CONFIG.invalid_fraction);
            match op.cascade_dry_apply(&state) {
                Ok(result) => {
                    // the oracle: the committed state is valid
                    assert!(result.new_state.get_data().get_inner_data()
                        .broken_invariants() == Ok(BTreeSet::new()));
                    if !result.warnings.is_empty() { warned.set(warned.get() + 1); }
                    landed.set(landed.get() + 1);
                    state = result.new_state;
                }
                Err(_) => errored.set(errored.get() + 1),   // clean rejection is fine
            }
        }
    }
    // coverage guards — a walk that never cascaded proves nothing
    assert!(landed.get() > 0 && warned.get() > 0 && errored.get() > 0);
}
```

House rules apply: committed `CONFIG` const, no env vars, no `#[ignore]` tiers;
shrinking is a later decision. Once commits 5.0–5.1 land, commit 5.2 extends the loop
to call `w.text(pre_state.get_data())` on every warning, panicking on `Err` with both
the `MissingId` and the `Fix` — rendering totality (every reachable fix resolves all
its ids against the true pre-state) gets fuzzed for free.

**As landed** (this commit, `ops/tests/property_update_ops.rs`). Three things moved
from the sketch above.

*The width.* `CONFIG` landed at **100 × 500**, not 50 × 300 — the same width as the
two elementary walks (`property_ops.rs` and `property_apply_gate.rs` run 100 × 1000)
and wider than the cascade harness's 50 × 500. It costs 21s of a ~6min suite, which
is a fair share for the only fuzz `ops/` has. The measured run: **37223 targets
landed, 3483 of them needing a repair, 9214 repairs in all, 12777 rejected.**

*The coverage guard grew a second half.* The three cross-seed counters are as
sketched (landed / warned / errored, all > 0). On top of them the run keeps a
per-family tally and asserts, for each of the fifteen, both that it was **drawn** and
that it **landed at least once** — a family drawn and rejected every time would have
had only its address checks fuzzed and its body never entered. Every family clears
it; the thinnest is `slot_pairings` at 387/1983, which needs two slots that can
legally be paired and keeps losing them to the walk's own deletions. The tallies are
printed, so a future reader can see at a glance whether the walk has degenerated.

*Every family needs a dead-address fallback.* A family whose pools have all run dry
has no op left to express, and handing `weighted` an all-zero weight array is a
test-side panic, not a finding. Two families were found this way on the first run —
`slots` and `incompatibilities`, both after the walk deleted the last subject with
interrogations — and both now answer with a `Delete` on a dangling id, which
exercises the family's address check instead. The pairing families already had it.

**The mutation check, and the false negative that came first.** A bare `panic!` at
the head of `apply_to_session`'s `AddNewSlot` `BrokenInvariants` arm turns the walk
**red on seed 0, inside 0.01s** — so the walk really does reach the new bodies'
translation arms, which is what the no-panic claim rests on. The first attempt was a
false negative, recorded because both of its causes are traps for the next session:
turning that arm's dangling-teacher scan into a `break` stays **green**, because a
dangling teacher also breaks `SlotTeacherDoesNotTeachSubject` and the very next scan
catches it — the scans overlap, so sabotaging one proves nothing. And the edit had
landed in `apply_no_cleaning`'s copy of the translation (`slots.rs:338`) rather than
`apply_to_session`'s (`slots.rs:674`): the two bodies stand side by side until commit
7, they read almost identically, and this walk drives only the second.

## 5-C5. Commits 5.0–5.2 — the rendering vocabulary, the renderer, and the `Fix` catalogue

*(Section rewritten August 2 2026 after a point-by-point review with the user; the
partition and both id columns of the catalogue are unchanged.)*

### 5.0 — `ops/src/rendering.rs`, the shared id-rendering vocabulary

One function per id kind, each **noun-less** (it says « 5 (du 28/09/2026 au
04/10/2026) », never « Semaine 5 … » — the caller owns the sentence and the noun), each
taking `&Data` plus the id and doing its own lookups:

```rust
/// What a failed lookup names. One variant per id kind, plus the two
/// structural lookups that are not a bare id.
pub enum MissingId {
    Period(PeriodId), Week(WeekId), Subject(SubjectId), Teacher(TeacherId),
    Student(StudentId), WeekPattern(WeekPatternId), Slot(SlotId),
    GroupList(GroupListId), Incompat(IncompatId), PairingRule(PairingRuleId),
    SlotPairingRule(SlotPairingRuleId),
    /// The association entry at (period, subject) — read by `UnassignGroupList`'s
    /// template to name the list; neither id alone is the missing thing.
    Association { period: PeriodId, subject: SubjectId },
    /// A group index out of bounds is not a missing list.
    Group { group_list: GroupListId, index: u32 },
}
// From<each id type> ×11, From<(PeriodId, SubjectId)> for Association.

pub fn render_week(data: &Data, id: WeekId) -> Result<String, MissingId>;
pub fn render_period(data: &Data, id: PeriodId) -> Result<String, MissingId>;
// … render_subject / render_teacher / render_student / render_group_list /
//   render_incompat / render_week_pattern / render_slot / render_pairing_rule /
//   render_slot_pairing_rule, same shape.
pub fn render_group_name(data: &Data, list: GroupListId, index: u32)
    -> Result<Option<String>, MissingId>;  // Ok(None) = exists but unnamed
pub fn render_group(data: &Data, list: GroupListId, index: u32)
    -> Result<String, MissingId>;          // name if named, else 1-based number
pub fn join_french(items: &[String]) -> String; // "a" / "a et b" / "a, b et c"
```

`Result` over `Option` is deliberate: lookups chain (a slot resolves its subject and
teacher; a rule resolves its two endpoints), and a fuzz panic must name the *true*
missing thing, not the outer id the caller happened to hold. The two structural
variants exist because the group pair must distinguish the error path from the
legitimate no-name path.

The bodies are **adapted copies of the gtk4 helpers** (which stay in place until
commit 6.1): `generate_week_title` / `generate_period_title` /
`generate_week_succession_title` (`gtk4/src/editor.rs`),
`GroupEntry::generate_group_name` (`editor/colloscope/interrogation_dialog.rs`), the
group/week bind logic (`editor/colloscope/colloscope_display.rs`), and both rule
`generate_summary`s (`editor/pairings/pairings_display.rs`,
`editor/slot_pairings/slot_pairings_display.rs`). The gtk4 versions take precomputed
numbers; the ops versions compute them (`weeks.find_period_position_and_first_week`
gives a period's global first-week number). The formats, all settled:

- **Week**: global 1-based number; « 5 (du 28/09/2026 au 04/10/2026) » when
  `periods.first_week` is `Some` (dates moved into parens for clarity), bare « 5 »
  otherwise.
- **Period**: « 1 (du 31/08/2026 au 27/09/2026 - semaines 1 à 4) »; the singular
  « semaine {n} » and empty « (vide) » branches of
  `generate_week_succession_title` are kept verbatim.
- **Student / teacher**: `{firstname} {surname}`. **Subject / group list / incompat /
  week pattern**: the bare name.
- **Slot**: « Séverus Rogue - lundi 14h00 (Physique) » — teacher first (it is how a
  user *finds* a slot), then lowercase weekday + time; the subject trails in parens
  because it is context, not the slot itself. All three are needed to identify one.
- **Group**: name *or* 1-based number, never both — « B2 » or « 4 ».
- **Pairing rule**: the existing tab notation, adopted as-is — « Avoir Physique ⟹ Ne
  pas avoir Chimie » (U+27F9; no « (souple) » — softness is a property, not identity;
  list views append it themselves).
- **Slot pairing rule**: « Physique : [utilisé] Séverus Rogue - lundi 14h00 ⟹
  [non utilisé] Minerve McGonagall - mardi 15h00 » — subject fronted once (a warning
  lacks the tab's per-subject context), each part in the slot format minus its
  subject parens.

**As landed** (this commit, `ops/src/rendering.rs`, no tests — they are 5.2's).
*The signatures below were superseded by commit 6.0 — every renderer now takes
the document parts it reads instead of `&Data`; the formats and the miss policy
are untouched. Read §5-C6's 6.0 before this paragraph's `data: &Data`.* The
module is `pub mod rendering;` with **no** glob re-export, unlike every other `ops/`
module: the renderers are a named vocabulary and read better as
`collomatique_ops::rendering::render_week(…)` at gtk4's call sites than dumped into
the crate root. Two things not in the sketch above. The list gained a twelfth
renderer, `render_slot_in_subject` — « Séverus Rogue - lundi 14h00 », `render_slot`
minus its subject parentheses, of which the public one is now just that plus the
parentheses. It arrived as a private helper (the slot pairing rule notation fronts
the subject once and then needs the shorter form twice) and was made public on the
user's call: gtk4's slot pairings tab is grouped by subject throughout and its
`build_slot_description` is this exact format, so commit 6.1 has a real call site
for it. And the eleven `From<XxxId>` impls are a local `macro_rules!` rather
than eleven hand-written blocks: they exist only so a renderer can write
`.ok_or(id)?` and let `?` lift the id into [MissingId], which is not eleven separate
decisions.

Every format was eyeballed against the frozen hogwarts base before the commit, on a
throwaway test since deleted: « 1 (du 04/09/1995 au 01/10/1995 - semaines 1 à 4) »,
« 2 (du 11/09/1995 au 17/09/1995) », « Severus Rogue - jeudi 13h00 (Potions) »,
« Métamorphose : [utilisé] Rolanda Bibine - jeudi 12h00 ⟹ [non utilisé] Rolanda
Bibine - vendredi 18h00 ». One gap for 5.2 to keep in mind: hogwarts holds **no
pairing rule at all**, so `render_pairing_rule` is the one renderer with no live
example behind it — the property walk is what will first exercise it.

Ruled and recorded so it isn't re-litigated:
`constraints-colloscopes/src/types/user_readable.rs` overlaps but stays untouched, as
format *reference* only — its miss policy is deliberately different (solver
diagnostics degrade to a `{:?}` fallback and must stay printable; ops rendering is
strict, the strictness being the frame-rule instrument), and the only clean shared
home would be a new crate. Not worth it for ~150 lines of stable lookups.

### 5.1 — `ops/src/warning_text.rs`, the `Fix` renderer

`pub(crate) fn render(data: &Data, fix: &Fix) -> Result<String, MissingId>`,
exhaustive over `Fix` with **no wildcard arm** — a new fix shape is a compile error
here (D6/D15). The renderer never sees an invariant: one variant, one meaning, one
template, built from the 5.0 vocabulary with bare `?`. `CascadeWarning::text` is a
one-line wrapper forwarding its own `Fix` — there is no second warning shape to match
on first (D12 void), and it stays the crate's only public rendering door (`render`
stays `pub(crate)`). `resolution.rs`'s module doc gains: *"Every `Fix` variant has a
French description in `ops/src/warning_text.rs` — a new or changed variant must
update its text."*

Template conventions: « » quotes around free-form names that can't be read without
context (group lists, week patterns, incompat names, both rule notations); bare for
proper-noun-like names (subjects, people) and numbers. Templates naming groups branch
on cardinality — « Le groupe B2 sera retiré … » / « Les groupes 3, A1 et B2 seront
retirés … » via `join_french`.

**The catalogue** — this table IS the `Fix` enum (commit 1a built the type from it,
commit 5.1 the templates). Conventions: fields in braces; `rebuilt` marks a
payload-carrying variant (the variant reduces to a whole-value op — `Update`,
`SetRow`, `SetGroupList` — and carries the rebuilt value so `to_annotated_op` stays
pure, D15); all other variants are id-only and translate structurally. The
"translates to" column is the map's current op output (survey-verified) —
`to_annotated_op` reproduces it verbatim. The "produced by" column is the
`fix_invariant` arm inventory: which invariants answer this variant (`Conv:` =
`Convergence`). The French wording was walked row by row with the user (August 2)
and is settled alongside the *partition* (which invariants share a variant, driven
by one-variant-per-rendered-meaning); residual micro-polish at implementation is
fine, but the forms below are the reviewed ones. Placeholders are the 5.0 renderers: `{semaine}` = `render_week`,
`{période}` = `render_period`, `{créneau}` = `render_slot`, `{règle}` = the matching
rule renderer, `{groupes}` = `join_french` over `render_group`; `{sujet}`,
`{colleur}`, `{élève}`, `{liste}`, `{motif}`, `{nom}` are the name renderers.
A coordinate implied by *every* producer may drop, but only from the complement
position — the sentence's grammatical subject always stays ({élève} leads the
prefill row even though only a student deletion produces it). The four
period-exclusion fixes fire only when a period dies (the user knows which), so they
name no period; `ClearAssignmentRow` also fires once per period on a *subject*
deletion, so there the period is the discriminating information and stays. The week
in `RemoveWeekPatternExclusion` survives the same test by the composite case:
`DeletePeriodAndWeeks` can lift several exclusions of one motif in one dialog, and
without `{semaine}` those lines would be identical and dedup-collapsed. Periods
join a sentence as « sur la période {période} », never as a parenthetical —
`render_period`'s own output carries parens, and nesting them reads badly.

| `Fix` variant | translates to | produced by | template |
| --- | --- | --- | --- |
| `DeleteWeek { week }` | `WeekOp::Remove` | `Period@WeekPeriodFk` | « La semaine {semaine} sera supprimée » |
| `RemoveSubjectPeriodExclusion { subject, period, rebuilt }` | subject `Update` | `Period@SubjectExcludedPeriods` | « {sujet} : l'exclusion de période sera levée » |
| `RemoveStudentPeriodExclusion { student, period, rebuilt }` | student `Update` | `Period@StudentExcludedPeriods` | « {élève} : l'exclusion de période sera levée » |
| `RemovePairingRulePeriodExclusion { rule, period, rebuilt }` | pairing-rule `Update` | `Period@PairingRuleExcludedPeriods` | « Règle « {règle} » : l'exclusion de période sera levée » |
| `RemoveSlotPairingRulePeriodExclusion { rule, period, rebuilt }` | slot-pairing-rule `Update` | `Period@SlotPairingRuleExcludedPeriods` | « Règle de créneaux « {règle} » : l'exclusion de période sera levée » |
| `ClearAssignmentRow { period, subject }` | `SetRow(period, subject, ∅)` | `Period@AssignmentsKey`, `Subject@AssignmentsKey`, `Conv:AssignmentForSubjectNotRunningOnPeriod` | « Les inscriptions en {sujet} sur la période {période} seront supprimées » |
| `UnassignGroupList { period, subject }` | `AssignToSubject(period, subject, None)` | `Period@AssociationEntry`, `Subject@AssociationEntry`, `GroupList@AssociationEntry`, `Conv:AssociationForSubject{WithoutInterrogations,NotRunningOnPeriod}` | « L'association de la liste « {liste} » en {sujet} sur la période {période} sera supprimée » |
| `RemoveWeekPatternExclusion { pattern, week, rebuilt }` | week-pattern `Update` | `Week@WeekPatternExcludedWeek` | « Motif « {motif} » : l'exclusion de la semaine {semaine} sera levée » |
| `ClearInterrogationCell { slot, week }` | `SetInterrogation(slot, week, ∅)` | `Week@ColloscopeInterrogation`, `Slot@ColloscopeInterrogation`, `Conv:InterrogationSlotNotRunningOnPeriod`, `Conv:InterrogationOnInactiveWeek` | « La colle du créneau {créneau} en semaine {semaine} sera supprimée » |
| `RemoveTeacherSubject { teacher, subject, rebuilt }` | teacher `Update` | `Subject@TeacherSubjects`, `Conv:TeacherSubjectWithoutInterrogations` | « {colleur} n'interviendra plus en {sujet} » |
| `DeleteSlot { slot }` | `SlotOp::Remove` | `Subject@SlotSubject`, `Teacher@SlotTeacher`, `Conv:SlotTeacherDoesNotTeachSubject`, `Conv:SlotForSubjectWithoutInterrogations` | « Le créneau {créneau} sera supprimé » |
| `DeleteOverflowingSlot { slot }` | `SlotOp::Remove` | `Conv:SlotOverflowsDay` | « Le créneau {créneau} sera supprimé (il déborderait sur le jour suivant) » |
| `DeleteIncompat { incompat }` | `IncompatOp::Remove` | `Subject@IncompatSubject` | « L'incompatibilité « {nom} » sera supprimée » |
| `DeletePairingRule { rule }` | pairing-rule `Remove` | `Subject@PairingRuleAntecedent/Consequent` | « La règle « {règle} » sera supprimée » |
| `ClearSubjectBalancing { subject }` | `SetSubject(subject, None)` | `Subject@BalancingSubjectKey`, `Conv:BalancingForSubjectWithoutInterrogations` | « L'équilibrage de {sujet} prendra les valeurs par défaut » |
| `RemoveStudentFromGroupListPrefill { group_list, student, rebuilt }` | group-list `Update` | `Student@GroupListPrefilledStudent` | « {élève} sera retiré(e) des groupes préremplis de « {liste} » » |
| `RemoveStudentGroupListExclusion { group_list, student, rebuilt }` | group-list `Update` | `Student@GroupListExcludedStudent` | « {élève} : l'exclusion de la liste « {liste} » sera levée » |
| `ClearStudentSettings { student }` | `SetStudent(student, None)` | `Student@SettingsStudentKey` | « Les paramètres de {élève} prendront les valeurs par défaut » |
| `RemoveStudentFromAssignmentRow { period, subject, student, rebuilt }` | `SetRow` | `Student@AssignmentsStudent`, `Conv:AssignedStudentNotPresentForPeriod` | « L'inscription de {élève} en {sujet} sur la période {période} sera supprimée » |
| `RemoveStudentColloscopePlacement { group_list, student, rebuilt }` | `SetGroupList` | `Student@ColloscopeGroupListStudent`, `Conv:ColloscopeStudentExcluded`, `Conv:ColloscopeStudentGroupOutOfBounds` | « {élève} sera retiré(e) de son groupe de « {liste} » dans le colloscope » |
| `ClearSlotWeekPattern { slot, rebuilt }` | slot `Update` | `WeekPattern@SlotWeekPattern` | « Le créneau {créneau} ne suivra plus de motif : il aura lieu toutes les semaines » |
| `ClearIncompatWeekPattern { incompat, rebuilt }` | incompat `Update` | `WeekPattern@IncompatWeekPattern` | « L'incompatibilité « {nom} » ne suivra plus de motif : elle s'appliquera toutes les semaines » |
| `DeleteSlotPairingRule { rule }` | slot-pairing-rule `Remove` | `Slot@SlotPairingRuleAntecedent/Consequent`, `Conv:PairedSlotsNotInSameSubject` | « La règle de créneaux « {règle} » sera supprimée » |
| `ClearColloscopeGroupListRow { group_list }` | `SetGroupList(list, ∅)` | `GroupList@ColloscopeGroupListKey`, `Conv:ColloscopeGroupListPrefilled` | « La répartition en groupes de « {liste} » dans le colloscope sera supprimée » |
| `RemoveGroupsFromInterrogationCell { slot, week, groups, rebuilt }` | `SetInterrogation` | `Conv:InterrogationGroupsOutOfBounds` | « Le groupe {groupes} sera retiré / Les groupes {groupes} seront retirés de la colle du créneau {créneau} en semaine {semaine} » (cardinality branch) |

Rendering note: chained lookups are the norm, not the exception — a fix carrying only
an id often needs material *behind* it (`DeletePairingRule` reads the rule to name its
two subjects; every `{créneau}` reads the slot, then its subject and teacher). All are
sound for the same reason: the entity a fix touches is present in the composite
pre-state, and a miss is a frame-rule violation surfacing as `Err` per D7. The least
obvious case is `UnassignGroupList`: the template names the list, which the fix does
not carry — the renderer reads the association entry at `(period, subject)` from the
pre-state (the entry the fix clears, so it is present there; its absence is
`MissingId::Association`).

The catalogue above is the **whole** warning vocabulary: with D12 void there is no
composite-emitted template beside it.

The old many-to-one collapses (several invariants → one effect → one sentence) are now
explicit in the vocabulary itself: they are the multi-entry "produced by" cells.

**As landed** (this commit, `ops/src/warning_text.rs`, no tests — they are 5.2's).
The twenty-five templates went in as the catalogue has them. Three things worth
recording.

*The module is private.* `mod warning_text;` — not `pub mod`, unlike `rendering`:
`render` is `pub(crate)` and [CascadeWarning::text] is the only door out, so there
is no reason for the module path to exist in the public API at all.

*Two lookups the placeholder notation hid.* `{groupes}` reads as "just call
`render_group`", but `RemoveGroupsFromInterrogationCell` carries no group list —
group numbers in a cell are bounded by the list associated at the cell's
`(period, subject)` coordinate, and the fix names that coordinate only through its
slot and its week. So the arm resolves slot → subject, week → period, then the
association, in a private `cell_group_list` helper; `UnassignGroupList` needs the
last step of the same chain and they share the `association` helper under it. Both
are sound for the pre-state reason the catalogue's rendering note gives, and both
`Err` as `MissingId::Association` — the variant 5.0 added for exactly this.

*Eyeballed against the frozen hogwarts base* on a throwaway test since deleted,
the same way 5.0 checked its formats: « Le créneau Severus Rogue - jeudi 13h00
(Potions) sera supprimé », « Severus Rogue n'interviendra plus en Potions »,
« L'association de la liste « Liste principale » en Potions sur la période 1 (du
04/09/1995 au 01/10/1995 - semaines 1 à 4) sera supprimée », « Les groupes 1 et 3
seront retirés de la colle du créneau Severus Rogue - jeudi 13h00 (Potions) en
semaine 3 (du 18/09/1995 au 24/09/1995) ». Eleven variants had a live example
there; the other fourteen did not, because hogwarts excludes nobody from anything,
holds no pairing rule and no slot pairing rule, and has no per-student settings —
the same gap 5.0 recorded for `render_pairing_rule`, wider. **That is what 5.2's
property walk is for**: it is the first thing to render the period-exclusion, rule
and settings templates at all, so a run that shows those `Fix` variants never
drawn would mean the walk, not the renderer, needs work.

### 5.2 — all the tests (5.0 and 5.1 land test-less)

Two pieces, and deliberately **no per-variant French text pins** — with typed ids a
cross-field mix-up doesn't compile, and pinning UI wording only manufactures fixture
churn on every polish:

- Extend `ops/tests/property_update_ops.rs` (§5-C4): render every collected warning
  with `w.text(pre_state.get_data())`, panicking on `Err` with both the `MissingId`
  and the `Fix`. This is the main verification — rendering totality over every
  reachable fix, against the true pre-state, at the walk's full width.
- Unit tests in `rendering.rs` for the branchy leaves: `join_french` (one / two /
  three items) and the period renderer (multi-week, single-week, « (vide) », each
  with and without a start date).

**As landed** (this commit). Both pieces went in as planned, and the walk grew a
third guard on top of them.

*The walk renders, and the rendering is checked against the right document.*
Each collected warning is rendered with `w.text(state.get_data())` where `state`
is still the pre-state — the assignment `state = result.new_state` comes after —
and an `Err` panics with both the `MissingId` and the `Fix`. Mutation-checked by
rendering against `result.new_state` instead: **red on seed 0 after five ops**,
`Slot id (SlotId(26)) is invalid (fix: DeleteSlot { slot: SlotId(26) })` — which
is the point of D7 in one line, a repair's own target being gone from the state
it produced.

*A per-`Fix`-variant coverage guard, pinned both ways.* Rendering totality is
only worth what the walk actually renders, so the run tallies warnings per `Fix`
variant, prints the tally, and asserts every variant of `FIX_VARIANTS` was
rendered at least once — and that nothing outside the list was, so a
twenty-sixth variant cannot arrive unnoticed. `report()` now runs *before*
`assert_covered()`, so a run that fails a guard still prints the numbers that
explain it. Two variants were missing on the first run, for opposite reasons:

- **`DeleteOverflowingSlot` was the walk's fault, and is fixed here.**
  `testgen`'s synthesizers keep interrogation durations at 30 or 60 minutes and
  slot starts between 8:00 and 18:00 *precisely so* a slot can never overflow its
  day (`synth.rs:103-106`), so nothing built from them ever reaches
  `Convergence::SlotOverflowsDay`. The ops walk now stretches an
  `UpdateSubject`'s duration to ten hours one time in ten: every slot of the
  subject starting after 14:00 then runs past midnight, and the cascade deletes
  it. 11 renders over the run — thin, but the shape is reached, and the pre-
  existing-material path is exactly the one the « il déborderait sur le jour
  suivant » sentence is written for.
- **`DeleteWeek` is unreachable from `ops/`, by design.** It answers a dangling
  `Week::period_id`, which only a bare `PeriodOp::Remove` leaves — and no
  composite emits a bare one, because `DeletePeriodAndWeeks` authors its weeks'
  removal first (★ D8). Rather than quietly leaving it off the list, it sits in
  its own `OPS_UNREACHABLE_FIX_VARIANTS` const, with the reasoning, and is
  asserted **absent**: the day a composite grows a bare period removal, the walk
  says so and the name moves. Its French template stays — the state-layer path
  that reaches it is fixture 1b of `state-colloscopes/tests/cascade.rs`.

The other twenty-three were all rendered without any generator work, most in the
hundreds; the thinnest are `RemoveGroupsFromInterrogationCell` (5),
`RemoveSlotPairingRulePeriodExclusion` (10) and `ClearInterrogationCell` (18).
So the fourteen variants 5.1 recorded as having no live example in hogwarts —
the period-exclusion, rule and settings templates — are read here for the first
time, which is what that note asked for.

*The unit tests.* `join_french` on one / two / three items (plus the empty
branch, unreachable from any fix but present in the code, so it says what it
does), and `render_period` over the six planned shapes, built from
`Data::default()` by elementary ops — three periods of four, one and zero weeks,
rendered with and without the document start date, the « (vide) » branch pinned
to win over the dates. One test beyond the plan's list: `render_period` on a
removed period id returns `Err(MissingId::Period(id))`, the miss policy the whole
module is built on, pinned once where it is cheap. All four mutation-checked —
dropping the empty-period short-circuit, forcing the plural « semaines n à m »,
mis-computing the period end date and changing `join_french`'s separator each go
red on exactly the assertion that covers them.

## 5-C6/C7. Commits 6.0–6.4 and 7

### 6.0 — the rendering helpers take the document parts they read

*(Not in the original plan. Designed with the user August 2 2026, after surveying
gtk4 for 6.1 and finding that commit could not be written as specified.)*

**The finding.** `ops::rendering` landed taking `&Data`, and **no gtk4 panel holds
a `Data`.** Only `EditorPanel` does (`gtk4/src/editor.rs`, field
`data: AppState<Data, Desc>`); every panel below it receives *subset clones* —
a `Periods` and a `Weeks`, a `Subjects` and a `Pairings` — pushed in on each
update (`editor.rs:325-577`). The row factories below *those* hold neither, just
precomputed numbers, and several hold no id at all
(`subjects_display::PeriodSwitchData` is `{global_first_week, period_num,
first_week_in_period, week_count, state}`; `week_patterns::dialog::PeriodData` and
`students::dialog::PeriodData` have the same shape). Read literally, 6.1's
"call the ops renderers where the full data is at hand and pass rendered strings
down" therefore meant rendering every period and week title in `editor.rs` and
threading strings through two or three layers of relm4 messages — fifteen files of
message churn for a dedup commit.

**What unblocks it.** Every renderer went through one private accessor,
`fn params(data: &Data) -> &Parameters`, and then touched one or two fields of
`Parameters`. `render_period` reads `periods` and `weeks`; `render_slot_in_subject`
reads `slots` and `teachers`; `render_group_name` reads `group_lists`. `&Data` was
never the input these functions need — it is the widest thing that contains it.

**The change**, `ops/` only:

- Each renderer takes the parts it reads, **in `Parameters` field order, then the
  ids** — `render_week(periods, weeks, id)`,
  `render_slot(subjects, teachers, slots, id)`,
  `render_slot_pairing_rule(subjects, teachers, slots, slot_pairings, id)`,
  `render_group(group_lists, id, index)`. `MissingId`, its `From` impls and
  `join_french` are untouched, and so is every rendered string.
- `warning_text::render` takes `&Parameters`, as do its two private helpers
  (`association`, `cell_group_list`). Its twenty-five arms grow one field
  reference per call and are otherwise unchanged.
- **`CascadeWarning::text(&self, data: &Data) -> Result<String, MissingId>` keeps
  its exact signature** (user constraint) and does the projection itself:
  `crate::warning_text::render(&data.get_inner_data().params, &self.fix)`. That
  line is now the only place `Data` meets the renderers.

Rejected while planning: a private `&Parameters` adapter in `warning_text.rs` (a
newtype with fifteen one-line forwarding methods) to keep the arms as short as they
were. It buys ~15 characters per call and costs a whole indirection layer that
exists for nothing else. The arms stay explicit.

No new tests: this is a pure signature refactor, so the existing suite is the proof
it moved code without changing it — the framing commit 1a used. The load-bearing
ones are 5.2's property walk (renders every collected warning at full width and
pins per-`Fix`-variant coverage) and the four `rendering.rs` unit tests, whose
expected strings are unchanged, so any format shift turns them red. No `Cargo.lock`
change, so no `cargoHash` refresh.

**What this makes possible in 6.1.** Checked panel by panel, every 6.1 call site
now has its arguments in hand: `students.rs`, `group_lists.rs`, `subjects.rs`,
`week_patterns.rs`, `assignments.rs` and `general_planning.rs` all hold `periods`
and `weeks`; `pairings.rs` holds `subjects` and `pairings`; `slot_pairings.rs`
holds `subjects`, `teachers`, `slots` and `slot_pairings`; `colloscope.rs` and its
`config_dialog.rs` hold the whole `Parameters`. So a panel calls ops directly and
passes one finished `title: String` down to its row factories — which *shrinks*
those structs, since `global_first_week` / `first_week_num` / `week_count` exist
there only to feed a title.

### 6.1 — gtk4 helper dedup

gtk4's local renderers are deleted and their call sites switched to `ops::rendering`.
The sites: `generate_week_title` / `generate_period_title` (`editor.rs`),
`GroupEntry::generate_group_name`
(`editor/colloscope/interrogation_dialog.rs` — rebuilds « Groupe {n} : {name} » on top
of `render_group_name`), the group/week bind logic
(`editor/colloscope/colloscope_display.rs`), both rule `generate_summary`s
(re-appending « (souple) ») and `generate_excluded_periods_info`'s hand-rolled list
joining → `join_french` (`editor/pairings/pairings_display.rs`,
`editor/slot_pairings/slot_pairings_display.rs`), and
`SlotPairings::build_slot_description` (`editor/slot_pairings.rs:48`) →
`render_slot_in_subject`, which 5.0 made public for it. Commit 6.0 is what makes
these calls possible at all — a panel holds the sub-structures each renderer takes,
and passes one finished title down to its row factories. The 6.2 smoke covers this
commit too.

Two corrections to the paragraph above, from the August 2 2026 survey, and one rule
it never wrote down. With these three and 6.0 landed, the commit is planned.

- **The original claim of "zero visual change" was wrong and is struck.** 5.0 chose
  its formats deliberately and those choices stand (user ruling, August 2), so the
  divergences below are voluntary and simply land with this commit:
  `Semaine 5 du 28/09/2026 au 04/10/2026` → `Semaine 5 (du 28/09/2026 au
  04/10/2026)`; `Période 1 du 31/08/2026 au 27/09/2026 (semaines 1 à 4)` →
  `Période 1 (du 31/08/2026 au 27/09/2026 - semaines 1 à 4)`; and
  `Rolanda Bibine - Jeudi 12h00` → `Rolanda Bibine - jeudi 12h00`, because
  `build_slot_description` calls `start_time.capitalize()`
  (`gtk4/src/editor/slot_pairings.rs:57`) and `render_slot_in_subject` does not.
- **`generate_week_succession_title` survives**, and is no longer on the deletion
  list above. `subject_params::Block::generate_title_text` renders a *block* inside
  a subject — a week succession with no `PeriodId` — so no ops renderer can serve
  it. The helper stays in gtk4 for that one caller; `generate_week_title` and
  `generate_period_title` still go.
- **The new gtk4 code `.expect`s a missing id. It does not degrade.** Settled long
  ago and simply never written down here; it is D7's rule — which this plan only
  ever spelled out for the warning dialog — holding everywhere gtk4 calls a
  renderer. A `MissingId` is a *logic bug*: the id came out of the document the UI
  is showing, so a miss means the UI and the document disagree, and the right
  answer is to fail fast and loud rather than paint a plausible screen over it.
  The three sites that answer `"???"` today all lose that fallback with the switch
  — `SlotPairings::build_slot_description` on a missing teacher
  (`gtk4/src/editor/slot_pairings.rs:56`), `pairings_display::Entry::subject_name`
  on a missing subject (`editor/pairings/pairings_display.rs:62`), and
  `slot_pairings_display::Rule::slot_desc` on a slot absent from the
  `slot_desc_map` its own panel built (`.../slot_pairings_display.rs:236`).

**As landed** (this commit, gtk4 only; no test moves — gtk4 has none, and the
workspace suite is the proof nothing below it shifted). Nineteen files, and the
whole change is one shape repeated: a panel that holds the document renders, and
its row factories receive a finished `String`. Five things worth recording.

*The row structs shrank, exactly as 6.0 predicted.* `global_first_week`,
`first_week_num`, `week_count`, `first_week_in_period` and `period_num` existed
in those structs only to recompute a title one layer too low, and most of them
are gone: `assignments_display::PeriodEntryData`, `subjects_display::{PeriodData,
EntryData, PeriodSwitchData}`, `students::dialog::PeriodData`,
`week_patterns::dialog::{PeriodData, WeekData}` and
`periods_display::{EntryData, WeekData}` all carry a `title: String` now. What
stayed is what feeds an *output* message rather than a label —
`week_num_in_period`, `period_num`, `assignments`' `first_week_num` (a
`set_visible` test), and the `DynamicIndex` of the factories that report their
own position. Two fields died outright with their last reader:
`assignments_display::PeriodEntry::index` and
`week_patterns::dialog::PeriodEntry::index`.

*Two panels changed shape rather than shrinking.*
`interrogation_dialog::DialogInput::Show` used to carry a whole `GroupList` and
no id, so `render_group_name` could not be called there at all — it now carries
the finished `Vec<String>` of row titles, built in `colloscope.rs` where the id
is (« Groupe 3 » / « Groupe 3 : B2 », the dialog's own format, on top of the
renderer). Same reason in `colloscope_display`: `SlotPeriodData::slots` was
`Vec<Option<BTreeMap<u32, Option<NonEmptyString>>>>` and the `bind` did the
name-or-number choice by hand; it is now `Vec<Option<Vec<String>>>` filled by
`render_group` in `update_view_wrapper`, and `bind` is a `join(",")`.

*The pairings and slot-pairings tabs took different halves of the vocabulary,
and that is deliberate.* The pairings row uses `render_pairing_rule` whole (its
existing notation, adopted verbatim by 5.0, so the string is unchanged) and
appends « (souple) » itself. The slot-pairings row does **not** use
`render_slot_pairing_rule`: that renderer fronts the subject once, which a
warning needs and this tab — grouped by subject throughout — would only repeat.
It keeps its own `[utilisé] … ⟹ [non utilisé] …` composition and takes
`render_slot_in_subject` underneath, through `build_slot_description`, which is
the call site 5.0 made that renderer public for.

*Two more format divergences than the three listed above, both the same one.*
`students`' period filter droplist (« La période … ») and `group_lists`'
association headers (« Associations pour la période … ») were the other two
callers of `generate_week_succession_title`, and they name a *period*, so they
moved to `render_period` with the noun in front. They pick up the period-format
change already accepted above. The visible residual: `subject_params::Block`
keeps the old « Bloc 1 du … (semaines 1 à 4) » shape, because a block has no
`PeriodId` and no renderer can serve it — so blocks and periods now print their
week spans differently. Recorded rather than fixed: aligning them means either
teaching `ops::rendering` about a coordinate that is not an id, or hand-copying
the format into gtk4, and neither is worth it for one label.

*The three `"???"` fallbacks are gone, as ruled.* `build_slot_description` and
`Rule::slot_desc` `.expect` instead; `Entry::subject_name` disappeared entirely
with the summary moving up to the panel. Every new `ops::rendering` call site
`.expect`s with the same sentence — the id came out of the document being
displayed.

### 6.2 — gtk4 warning-dialog switch

The single consumer site (`gtk4/src/editor.rs:1061`):

```rust
EditorInput::UpdateOp(op) => {
    match op.cascade_dry_apply(&self.data) {
        Ok(result) => {
            if result.warnings.is_empty() {
                sender.input(EditorInput::CommitUpdateOp(result.new_state));
            } else {
                // self.data still holds the pre-state the UI is showing —
                // exactly the state CascadeWarning::text must render against.
                let mut seen = std::collections::BTreeSet::new();
                let texts: Vec<String> = result
                    .warnings
                    .iter()
                    .map(|w| {
                        w.text(self.data.get_data())
                            .expect("warning must render against the pre-state")
                    })
                    .filter(|t| seen.insert(t.clone()))
                    .collect();
                self.state_to_commit = Some(result.new_state);
                self.warning_op_dialog
                    .sender()
                    .send(warning_op::DialogInput::Show(texts))
                    .unwrap();
            }
        }
        Err(e) => { /* unchanged: error_dialog Show(e.to_string()) */ }
    }
}
```

Application order kept (meaningful); exact-duplicate strings dropped (preserves today's
`BTreeSet` dedup). `warning_op.rs`, `ContinueOp`/`CancelOp`, `state_to_commit` and the
undo-label plumbing untouched. User runs a gtk4 smoke here (covering 6.1 as well).

### 6.3 — python

`python/src/glue.rs:1356`: `op.apply(&mut *state)` → `op.cascade_apply(&mut *state)`.
One line; behaviour switches here, so the **three contract scripts run at this commit**
(user).

### 6.4 — rpc-engine

Drop `collomatique-ops` from `rpc-engine/Cargo.toml` (declared, zero references in
source). Cargo.lock changes ⇒ **cargoHash refresh (user)**.

### 7 — delete the old world, rename

Delete from `ops/`: `UpdateWarning` + its 15 `From` impls and dispatch
(`lib.rs:127-259`), the 15 family warning enums + `build_desc_from_data` ×16,
`get_next_cleaning_op` ×16 + dispatch, `CleaningOp` + helpers (`lib.rs:261-282`),
`rec_apply_no_session`, `apply_no_cleaning` ×16 + dispatch, `RecApplyResult`,
`DryResult`, `dry_apply`, `apply`. Rename `cascade_dry_apply` → `dry_apply`,
`cascade_apply` → `apply` (D11); gtk4 + python call sites follow (glue.rs returns to its
original text). Survives: `get_desc`, `OpCategory`, the `UpdateError` vocabulary — minus
the **D14 cleanup**, done here: delete `UpdateSlotError::InvalidSubjectId` and
`UpdatePeriodWeekCountError::SubjectImpliesMinimumWeekCount` (both dead), and fix
`MoveSlotDown` to return its own `MoveSlotDownError::InvalidSlotId` (fixture in this
commit pins it).

Test re-cuts, conscious not mechanical: `ops/tests/found_bugs.rs` (imports the warning
vocabulary — its regression scenarios re-assert on `CascadeWarning` accessors or on
resulting state); `ops/tests/assignments_error_surface.rs` (its translation-order pins
survive verbatim — the new path kept the order — but its header prose describes the old
machinery); `general_planning_content.rs` drops the old-path halves. Delete
`docs/todos/fixme_ops.md` — its bug is now the green merge fixture.

### Close-out

Design doc: §8 step 7 marked complete; new **Appendix J** (step 7 as delivered: the
receipt, the Manager wrapper, the session struct, the doctrine, the divergence register
as landed, tests); this plan retired and deleted (pinned by `git show` per house
convention); topic memory updated.

---

## 6. Divergence register (new ≠ old, all deliberate)

1. **Merging periods preserves colloscope data** (closes `docs/todos/fixme_ops.md`).
   Old: the merge cleaning reconciled the two periods *before* the move and its emitted
   cleaning ops + recursive `DeletePeriodAndWeeks` erased every cell it could not carry
   (`general_planning.rs:670-893`, `:1381-1384`; the body's own comment at `:1333`
   admits it). New: weeks move with their cells; only genuinely-invalidated cells are
   cleared, warned. Identical group lists ⇒ full preservation.
2. **`SlotOverflowsDay` on a subject update no longer aborts the process**
   (`subjects.rs:758`); the cascade removes the overflowing slot and warns. (On a *slot*
   update targeting a bad start, the old rejection survives unchanged — the map's
   presence test answers `None` and the target is convicted; §5 commit 3.11 pins both.)
3. **Deleting a week pattern keeps referencing slots and incompats** (★ ruled July 28,
   H.3): rows survive with `week_pattern = None` where legacy deleted them and their
   colloscope data. The texts say what happens now ("il aura lieu toutes les
   semaines").
4. **Prefilled-shrink student loss warns nowhere — and that is *not* a divergence this
   step introduces** (D12 void). The warning was already deleted by the
   global-group-list work, before this step starts: with count and filling arriving in
   one payload the loss is authored by the caller, so both the old and the new `ops/`
   are silent about it and there is nothing here to diverge. Kept as an entry so the
   executing session does not re-add it — see D12 in §0.
5. **`DeletePeriodAndWeeks` / `MergeWithPreviousPeriod` stop reconciling exclusions.** Old
   cleaning re-included subjects (`UpdatePeriodStatus(.., true)`) and aligned student
   exclusions to the neighbour period before deleting; new: the dead period's
   exclusion-set members are simply dropped by the cascade. Same end state for the
   surviving document, different warnings (drop-phrased instead of reconcile-phrased).
6. **Four crashes become errors** (★ D13 + D5's growth rule): `DeletePeriodAndWeeks` on a dead
   id (`InvalidPeriodId` instead of `.expect` death), balancing options on a
   no-interrogation subject (`SubjectHasNoInterrogation` instead of `.expect` death —
   commit 3.3bis, §3.3), teacher ops naming a no-interrogation subject (the same variant
   name on its own family's enum, instead of
   `panic!("Unexpected invariant breaks…")`), and a colloscope group-list row aimed at a
   prefilled list (`PrefilledGroupListInColloscope` instead of the same panic — a
   *restoration*, the guard having been lost at step 4; see §3.12). The fourth one also
   moves an existing answer: a prefilled list named with a dead student reports the
   prefilled case, not `InvalidStudentId`, which is where the old guard's precedence
   put it. And at commit 7 (★ D14): two dead error variants deleted, `MoveSlotDown`
   returns its own `InvalidSlotId` instead of `MoveSlotUpError`'s.
7. **Warning granularity changes.** Legacy: deduplicated coarse statements ("Pertes des
   créneaux de colle du colleur X"). New: one entry per fix op, finer, in application
   order. gtk4 dedups exact-equal texts only.
8. **Per-composite warning-set differences are pinned family by family** in the
   commit-3 fixtures; any surprise found while writing them is brought back to the
   user, not silently accepted.
9. **One convergence placement moved during this step** (§3.13, commit 3.13bis): the
   interrogation-row predicates now outrank the association ones. A divergence from
   step 6 as delivered, not from legacy — no behaviour of the *old* world is involved.
   What changes is which repair the cascade picks where both break, and so the sentence
   a user reads when a subject stops running on a period: the colles are lost whole
   instead of group by group.

---

## 7. Testing doctrine

- **The default fixture base is a frozen hogwarts copy.**
  `ops/tests/fixtures/hogwarts.collomatique` — copied from
  `examples/hogwarts.collomatique` at commit 2b, deliberately decoupled so the living
  example can evolve without touching the tests (the precedent is
  `constraints-colloscopes/tests/pairing_build_regression.rs`, whose fixtures derive
  from hogwarts the same way and pin literal ids). Tests `include_str!` it, decode
  through `collomatique_storage::deserialize_data` (asserting no caveats), and wrap
  the result in `AppState::new`; corner shapes the document lacks are set up by
  applying ops on top of the loaded base, visible in the test. `collomatique-storage`
  becomes an `ops/` dev-dependency (no cycle — storage does not depend on `ops/`;
  ⇒ **cargoHash**). In-process `AppState::new(Data::default())` + elementary ops
  stays the right tool where the point is a tiny document whose whole state reads at
  a glance (the three existing `ops/tests/` files keep their style). Exact-state
  assertions on the hogwarts base stay cheap: build the expected document by applying
  the expected elementary ops, in cascade order, to a clone of the base — each is
  gate-valid in that order, exactly as the cascade lands them — and compare with
  `==`; that pins that the cascade landed precisely those ops.
- **Expected warning lists derived by hand before the test runs** (H.5), as `Fix`
  literals compared through `CascadeWarning::fix()` (content is private — read it
  through the accessor; payload-carrying variants include their expected rebuilt
  value, derived from the base document — read the entity, remove the element);
  sequence
  asserted only where the engine really chose it (an ordered literal is a tripwire on
  the derived `Ord`, not a confluence pin). Invariant→fix *attribution* is not pinned
  here — that lives in commit 1a's direct `fix_invariant` unit tests (D15).
- **Mutation-check every pin that passes on its first run** (house rule; the review's
  G1 lesson): for behaviour-divergence fixtures, sabotage the new composite or
  translation arm and watch red before trusting green.
- **Fail on the last conjunct** (H.5): compound asserts ordered so a map/composite that
  dropped the final effect cannot go green for the wrong reason.
- **The property walk is coverage-guarded**: landed > 0, warned > 0, errored > 0 — a
  walk that never cascaded proves nothing.
- Full workspace suite once per commit, background, captured to the scratchpad and
  grepped.

---

## 8. Gates

- Per commit: workspace suite green, no warnings.
- Milestones: property harness at 500 seeds after commit 3.16 and before close-out;
  storage byte-stability + `examples/` pristine throughout (this step touches no
  storage bytes and no elementary-op vocabulary — they must never move).
- User-run: contract scripts at 6.3; gtk4 smoke at 6.2 (covering 6.1) and after 7;
  cargoHash refresh at commits 2b, 4 and 6.4 (2b's may be a no-op — the storage
  dev-dep adds no external
  crate — but the lock file moves, so the check runs); ★ end-of-step acceptance
  before close-out.
