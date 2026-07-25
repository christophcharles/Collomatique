# Invariant checking & cascade resolution — design + plan of action

**Status:** direction agreed July 15 2026 (branch `consolidate_state`); **step 1 completed
July 18 2026** — its detailed plan is retired (pinned at
`git show 62949404:docs/plans/plan_step_1.md`), the delivered state is recorded in
Appendix B. **Step 2 completed July 18 2026** — its session plan is retired (pinned at
`git show 49b4f77d:docs/plans/plan_step_2.md`), the delivered state is recorded in
Appendix C. **Step 3 completed July 19 2026** (doc-only) — its session plan is retired
(pinned at `git show 26d88024:docs/plans/plan_step_3.md`), the audit record is
Appendix D. **Step 4 completed July 21 2026** — its session plan is retired (pinned at
`git show fbc4ae6d:docs/plans/plan_step_4.md`), the delivered state is recorded in
Appendix E. **Pre-step-5 loose ends completed July 25 2026** — a small consolidation phase
run before step 5 (periods/weeks module split, sealed `GroupList`, consolidated
`GroupListOp`, mirror-consistency `LogicError`s); its session plan is retired (pinned at
`git show 25fdc50b:docs/plans/plan_loose_ends.md`), the delivered state is recorded in
Appendix F. Next up: step 5.
This doc started as an exploration after phase C of the table-registry plan shipped (item 2's
detailed plan, since delivered in full and retired; pinned at
`git show 77695338:docs/table_registry_plan.md`); it now
records the agreed design *direction* and a step-by-step plan. Per the house rule of
`state_consolidation_plan.md` §6, **each step below still gets its own detailed session plan
(and user sign-off) before implementation** — this doc fixes the direction, the ordering, and
the decisions already taken, not the per-commit mechanics.

It supersedes how `docs/plans/state_consolidation_plan.md` **item 3** (invariant consolidation) and
**item 5** (params↔colloscope synchronization) were going to be tackled, and reuses/retires
parts of the phase-C reference registry. §9 details the impact on the existing plans.

Read `docs/plans/state_consolidation_plan.md` first — this builds on the retired table-registry
plan's inventory (28 ID-based relationships, the triplicated checks, the dense mirrors),
inlined below as Appendix A.

---

## 1. The problem this addresses

Every referential/consistency rule is currently expressed **three times**
(retired table-registry plan §1): candidate validation before an op, delete-blocking scans in the
`Remove`/`Update` paths, and the whole-model `check_invariants`. They drift; the drift produced
real bugs (the whole `found_bugs.rs` family). The registry's item-3 plan was to *reroute all
three families through one declared registry*. This design does a different collapse:

> Make `check_invariants` the **single source of truth** for "is this state valid?", enforce it at
> every op by apply-then-check-then-maybe-rollback, and drive automatic repair from the *precise*
> errors it returns. The checks then exist **once**, and the same function serves every consumer:
> elementary ops, file load, the property tests, and the `ops/` auto-repair.

The goal is *consistent checks everywhere* from one definition, not a smaller number of checks.
Drift between a precondition and the invariant it approximates becomes **unrepresentable**, not
merely unlikely.

## 2. `check_invariants` as the single source of truth

One function defines validity. It is already the trust-boundary check on file load, and the
property harness (`tests/property_ops*.rs`) already uses it as the oracle after every generated op
— so this direction *promotes the architecture the safety net already relies on* into production.

Better: **production already pays for this**. `Data::apply` runs the full `check_invariants`
after *every* op today and panics on failure (`lib.rs:347`). The switch converts that panic into
a rollback-with-error and *deletes* the separate candidate-validation pass that ran before it —
per-op cost goes down, not up.

Two hard requirements follow:

- **It must be complete.** Once validity is defined solely by `check_invariants`, that function
  becomes load-bearing; anything only a precondition caught silently stops being enforced.
  Completeness is audited explicitly (plan step 3). (The commented-out multi-colloscope
  block in `lib.rs:172` — stale dead code from the multi-colloscope era, not a live gap —
  was deleted as a step-1 phase-1 rider, `d3c56e9f`; nothing left for the audit there.)
- **It must return precise, coordinate-bearing errors.** Today `check_subjects_data_consistency`
  returns a bare `InvariantError::InvalidSubject` — no id, no site. That is useless for repair and
  mediocre for users. The error must say *which* entity references *which* dangling target at
  *which* site (e.g. `TeacherReferencesDanglingSubject(TeacherId, SubjectId)`).
- (For the cascade, a third:) **it must be able to return *all* broken invariants**, in a
  deterministic canonical order — not just the first — so the cascade's pick is well-defined
  (§5).

## 3. A detailed invariant enum + a resolution map

Enrich `InvariantError` into a precise enum (one variant per relationship/site, carrying
coordinates — the `RefSite` taxonomy of phase C reincarnated as errors). Pair it with a
**static map**: invariant variant → the elementary op that resolves it (constructed from the
error's coordinates + current state; the resolution may itself cascade). This map is the
"declare each relationship's repair once" artifact — the conceptual descendant of the registry,
in a different shape.

A stated consequence (decided July 15 2026): the precise `InvariantError` **becomes the op-error
vocabulary**. Once preconditions are deleted (step 5), the per-op typed error enums
(`PeriodError`'s eight "referenced by …" variants, `TeacherStillHasAssociatedSlots`, …) are no
longer reachable and retire in its favor, keeping only the transition carve-out errors of §4.
This is a deliberate UI-visible change (gtk4/Python error surfaces, `found_bugs.rs` exact-variant
asserts) and is handled in step 5.

Invariants split into **three kinds**, and the split dictates the machinery:

1. **Referential / resolvable** — a reference dangles (a teacher points at a removed subject). Has a
   map entry; the cascade resolves it.
2. **Structural / representational** — nonsense state a correct `apply` never produces (e.g. a
   mirror desynchronized from its source). **No** map entry → **panic**. A broken structural
   invariant means the op is written wrong; surface it loudly. The reshapes of step 1 shrink
   this tier to nearly nothing, and encapsulation (§6c) hides most of the remainder inside the
   owning types.
3. **Path-convergence** — every reference exists, but two reference *paths* must **agree** (the group
   assigned in a colloscope cell must belong to the group list associated to that cell's
   period × the slot's subject). Not existence, not shape — *agreement*. Handled by the cascade like
   tier 1, but detected by a hand-written multi-hop check and usually resolved *lossily* (clear the
   now-invalid assignment). See §6 for why this tier is (mostly) irreducible.

## 4. Elementary ops: apply / check / restore

Each elementary op:

1. Snapshot (clone `InnerData` — trivial at this scale).
2. Apply the mutation optimistically (`force_apply` — the same primitive the differential fuzz
   of step 4 uses), including any deterministic structural fan-out, computing the reverse op as
   today.
3. Run `check_invariants`.
4. On failure, restore the snapshot and **route the precise errors out** — which tell the caller
   exactly *what invariants would break* if the op were applied.

This deletes candidate validation and the precondition half of delete-blocking entirely: there is no
separate "can I?" check to drift from the invariant, because you *try and roll back*. The whole
"delete-blocking scan disagrees with the consistency check" bug class becomes unrepresentable.

Three things `check_invariants` **cannot** see, because they are properties of the *transition*
(or of the op's inputs), not the resulting *state* — these stay as a small, permanent
precondition carve-out:

- **No-clobber**: inserting on an already-used id can land in a valid state yet destroy data and
  break reversibility. "Fresh id is fresh" must be checked explicitly.
- **Parameter-targeting**: op inputs whose invalidity leaves no trace in the result — e.g.
  `AddAfter` with a dangling anchor id would produce a perfectly valid state at some default
  position; the bad anchor must be rejected up front.
- **Reversibility**: the emitted reverse op must actually reverse.

Carve-out errors are hard errors: the cascade (§5) never tries to resolve them.

**Bonus:** because step 3 validates the *result*, it also validates whatever structural fan-out
remains after the step-1 reshapes — a safety net over the most fragile, least-covered code today.

## 5. The cascade (settled July 15 2026)

The cascade never force-applies and never lets an invalid state escape a single elementary
`try_apply`: **discovery happens through failure**. It is a retry queue (in practice a stack —
new work goes to the front, so resolution is depth-first):

```text
apply_cascade(target_op):
    queue   = [target_op]          // front = next to try
    applied = []                   // (op, reverse) pairs, in application order

    loop until queue is empty:
        op = queue.front()
        match try_apply(op):                    // the §4 elementary apply — full gate
            Ok(reverse) =>
                queue.pop_front()
                applied.push((op, reverse))
            Err(carve-out error) =>
                undo `applied`, return the error // bad input, not resolvable
            Err(broken_invariants) =>            // state already restored by try_apply
                if any broken invariant has no map entry: PANIC   // structural tier — since
                                                 // step 2, this arm is unrepresentable: the
                                                 // checker's Result split routes tier 2 out as
                                                 // Err(LogicError) before any fixable set is
                                                 // built (Appendix C.1), so the map is total
                                                 // over FixableInvariant
                pick = min(broken_invariants)    // canonical Ord on the invariant enum
                resolver = map[pick](coordinates, current_state)
                queue.push_front(resolver)       // depth-first: resolver runs before op retries

    return applied      // the exact op list; compound reverse = reverses, reversed
```

Worked example: apply A → fails needing B; try B → applies cleanly (recorded in `applied`);
retry A → now fails needing C; try C → applies; retry A → applies. `applied = [B, C, A]` — the
queue never re-holds B; the `applied` Vec **is** the answer sent to the caller.

Properties:

- **Every committed intermediate state is valid** — each landed op passed the full §4 gate.
  There is no force-apply mode anywhere, so the "transient inconsistent live state" variant
  (rejected in the original exploration because the read surface is laced with
  `.expect("… should be valid")`) is not merely rejected — it is *unrepresentable*.
- **Undo is valid stepwise for free**: replaying the reverses in reverse order re-creates the
  target first, then re-attaches the references; each intermediate state is a state that
  actually existed on the way down.
- **Termination**: resolutions are deletive (the step-1 reshapes eliminate the additive/dense
  cases, see §6), so each resolver strictly shrinks the data; the reference graph is acyclic.
  As a backstop against a buggy map entry (a resolver that does not actually resolve its
  invariant), a **no-progress guard** panics if the same (op, picked invariant) pair is seen
  twice.
- **Confluence by construction**: the checker returns all broken invariants in canonical order,
  the pick is `min` under a derived `Ord`, and the map is a function — the emitted op list is
  fully deterministic. One pin test freezes it against checker-refactor drift.

**`ops/` use = dry-run.** Run `apply_cascade` on a session/clone, read the emitted op
list, discard. If `applied` contains only the target op, commit directly; otherwise show the
user the *actual* consequences ("this removal will also delete 3 slots and clear 14 colloscope
assignments") and let them accept or reject. This **retires the `Warning` machinery**:
hand-written consequence descriptions are replaced by computed ground truth that cannot drift.
`ON DELETE RESTRICT` vs `CASCADE` collapses into "the user declined vs accepted the preview,"
not a per-relationship policy declaration. (What remains to write is a *rendering* layer —
human-readable descriptions of elementary ops — replacing the Warning messages' text.)

## 6. Making the data "reference-friendly" (step-1 reshapes)

The tier of §3 that a constraint lands in is a *consequence of the data model*, and reshaping moves
constraints between tiers. **The complexity is conserved; the art is putting each constraint in the
tier with the best tooling.** Four techniques, now with their decisions:

**(a) Sparse-ify dense mirrors into references.** A dense mirror forces a tier-2 structural
denseness invariant that some `apply` must hand-maintain. Making it sparse turns "target removed"
from a wrong-count into a genuine dangling reference (tier 1) that the cascade resolves.

- *Assignments* — **decided, do it**. Storage already writes sparse rows and omits empty ones
  (`encode/spec2.rs`, spec §4.5 "empty row = redundant"); the dense in-memory mirror is
  reconstructed at decode purely to feed the representation. Sparse-ifying deletes that decode
  densification and leaves the on-disk bytes identical.
- *Colloscope* — **decided, do it (the hard case, and the real item-5 prize)**. The spec-2
  format was deliberately shaped for this: interrogations are stored sparse keyed by
  `(slot_id, week)` and filled group lists sparse keyed by `group_list_id`, both with
  "empty row = absent row" semantics. The in-memory dense skeletons (`period_map` per period,
  `slot_map` per slot, `interrogations: Vec<Option<…>>` per week, `group_lists` per
  non-prefilled list) are decode-time densification. Sparse-ifying dissolves the entire
  ~330-line structural fan-out from param ops (item 5's problem): a removed period/slot/week
  leaves dangling colloscope keys → tier 1 → cascade. The exact in-memory keying (nested vs
  flat composite keys, `WeekId` per (b) below) is settled in that step's session plan.

**(b) Promote positional / index data to entities.** **Decided for weeks: introduce `WeekId`.**
Weeks become entities owned by periods, ordered *within* their period exactly like slots are
ordered within subjects today — and that ordering is **encapsulated in the `Periods` type**
(technique c), so it never reaches `check_invariants`. `WeekPattern` becomes a
`BTreeSet<WeekId>` — the length-coupling invariant (#8, the ugliest tier-2 residue, emitted per
pattern × period) *disappears*; a removed week is a dangling `WeekId` (tier 1). Colloscope
interrogations re-key by `WeekId` (see (a)). On disk nothing changes: the format stays
positional (`GeneralPlanning.periods[].weeks`, week-pattern bool vectors, colloscope global week
indices); decode synthesizes `WeekId`s (seeded into the `IdIssuer` like every id), encode
projects back to indices via the period walk — byte-stable. Cost to absorb in that step's plan:
the week-touching op surface is re-cut, and `constraints-colloscopes`' order-critical
global-week walk is adapted.

**(c) Encapsulate so invalid states are unrepresentable.** Some checks leave `check_invariants`
entirely if the owning type can't represent the bad state: order-within-owner invariants
(weeks within periods, slots within subjects) live behind private fields with validating
accessors. This is distinct from references, and the *cleanest* outcome where it applies (the
invariant doesn't move to another tier, it *disappears* — and the differential fuzz of step 4
cannot even generate the broken state, correctly shrinking its search space).

**(d) Reference a pre-validated combination to dissolve a convergence check.** **Examined and
rejected (July 15 2026).** The candidate was a teaching-assignment junction
(`Table<TeacherSubjectId, (TeacherId, SubjectId)>`) with slots referencing `TeacherSubjectId`,
dissolving "a slot's teacher must teach the slot's subject" by construction. Rejected because
the moment a slot's subject sits behind a join, the per-subject slot ordering can no longer be
validated *locally* (it becomes a two-hop check — the worst tier), and the price includes a new
entity kind, new ops, and decode-time id synthesis — all to dissolve **one** tier-3 check that
is cheap to express with `Join` and cheap to keep. Complexity conserved, moved to a worse place.

**Slots: keep the current form** (decided July 15 2026). The post-B3 shape — flat
`slot_map: Table<SlotId, Slot>` with `subject_id` FK + `ordering` sidecar behind compound
`pub(crate)` helpers — already *is* technique (c): the mirror invariant is encapsulated at the
mutation boundary. (An earlier idea — flat `OrderedTable` with a "same-subject slots are
consecutive" canonical form — rested on the false premise that a teacher has one subject; with
that gone it buys little over the sidecar.) One optional cheap tweak, to be decided in the
step-1 session: make the sidecar rows **sparse** (a row only for subjects that have slots),
dropping the denseness coupling to "subject has interrogations"; the format explicitly permits
it (spec §4.7: "a row with an empty `slots` array is valid but redundant").

**The irreducible residue.** Techniques (a)–(d) can eliminate *existence*, *shape/length/count*, and
*some* convergence constraints. What resists is the genuine two-path-convergence core — most sharply
the colloscope group assignment: the assigned group's list must equal the list associated to
(the cell's period, the slot's subject). Other members: paired-slots-share-a-subject,
assigned-student-present-for-period, association-subject-runs-on-period,
teacher-teaches-subject (kept, per (d) above). These are exactly the "side-constraints attached
to references" the retired SQL schema could not express as plain FKs.

**This is fine.** Full folding is not the goal. A checker that catches *most* invariants
generically, plus a handful of hand-written tier-3 checks, is already a large, clean win. The tier-3
checks are where **`Join` earns its keep**: they read as following the reference paths and comparing
them (`slot.join(params)` → teacher, subject in hand), which is *the* argument for keeping `Join`
beyond consumer ergonomics.

## 7. What survives of the phase-C registry

- **`Join` — keep.** It is the vocabulary for tier-3 convergence checks *and* consumer ergonomics.
- **`References` / `for_each_ref` — keep, recast.** Not for reverse-lookups, but as the generic
  tier-1 existence-sweep engine behind the precise checker (step 2), plus `all_ids`/duplicate
  detection. **Delivered**: `InnerData::for_each_reference` is the layer-B sweep engine of
  `broken_invariants` (Appendix C.1).
- **`RefSite` + `references_to_*` (reverse lookup) — likely retire.** The cascade discovers
  referrers by tripping the checker, not by enumerating them. The `RefSite` *taxonomy* survives
  reincarnated as the precise-`InvariantError` variant set. Hold the deletion until gtk4's needs
  are known — a UI "who references X?" display may still want the reverse lookups,
  independently of the cascade.
- **Phases D/E of the table-registry plan** (consumer migration to the read API, `Deref`
  removal) — **completed July 16 2026** (`47a3a5ac`…`4543bb46`): consumers migrated, the
  compat layer deleted; the internal table representation is now free to change.

---

## 8. Plan of action (agreed July 15 2026)

Each step may span several commits/sessions and gets its own detailed plan first. Standing
gates throughout: the property harness (100 seeds; 500-seed reference at milestones), the
`found_bugs.rs` asserts, storage byte-stability + the `examples/` smoke test, and the three
contract scripts at milestones (run by the user).

**Step 1 — reshape: remove a maximum of dense copies — COMPLETED July 18 2026.** Done *under
the old architecture* (the triplicated checks still exist), so each reshape hand-updated the
old check families and the `ops/` cleaning paths one last time — accepted cost, bounded, and
the reshapes *deleted* more check code than they touched (the colloscope fan-out above all).
Rationale for reshape-first: the new checker of step 2 is now written **once against the final
data model** — and the reshapes removed precisely the dense/denseness invariants whose precise
"expected-vs-actual, which entry is missing" errors were this design's hardest open problem.
The additive cascade died here, unwritten.

  The detailed session plan (decision ledger, per-phase mechanics, landed-state notes) is
  retired from the tree; pinned at `git show 62949404:docs/plans/plan_step_1.md`. **The
  delivered state — final shapes, op surface, oracles, and what steps 2–7 build on — is
  Appendix B.** Sub-steps as landed (order: bugfix → decoupling → 1a → 1c → 1b → 1d):
  - **Phase 0 — bugfix**: the `UpdatePeriodWeekCount` colloscope-clean bounds bug (the §9
    "suspected dormant drift" — confirmed real), test-first (`1418d4bf`, fix `f8e34128`).
  - **Phase 1 — decoupling** (`d3c56e9f`): consumers stopped *relying* on denseness; riders
    deleted the stale multi-colloscope block + `FromDataError`. (Adjacent: property-test
    generator extracted to `collomatique-testgen-colloscopes`, `ca450d19`; fuzz-build net
    `cfce7f1f`.)
  - **1a — assignments sparse** (`9f4471e2`): row iff non-empty; decode densification deleted.
  - **1c — slots ordering sidecar sparse** (`b681cdac`): the §6 optional tweak, taken.
  - **1b — `WeekId`** — B1 as six commits (`2a0ec129`…`2de37eae`): Week entity,
    `week_map` + ordering sidecar, `WeekOp` family, content-carrying `Move`; B2
    (`d169df71`): patterns → `excluded_weeks: BTreeSet<WeekId>`, invariant #8 gone.
  - **1d — colloscope sparse** — D0 surface + consumer migration (`c32a431b`, fixes
    `9b10b655`/`bf191907`); D1 the swap as six commits (`996eb89d`…`0d5cc34b`, registry
    rider `2903e5de`); D2 cleanup (`62949404`). The ~330-line params→colloscope fan-out is
    gone — item 5 dissolved as designed.

  ★ End-of-step gate: 500-seed harness clean, byte-stability + hogwarts pristine, contract
  scripts + gtk4 smoke passed (July 18 2026).

**Step 2 — the precise checker, alongside the old one — COMPLETED July 18 2026.** Written
against the step-1 model, fully tested, and deliberately **unwired** — no production caller
(wiring is steps 3–6's business); the old checker stays authoritative and untouched except
the stage-6 backfill below. The session plan (decisions ledger, full variant tables, legacy
conversion table) is retired; pinned at `git show 49b4f77d:docs/plans/plan_step_2.md`.
**The delivered contract steps 3–7 build on is Appendix C.**

  Two deliberate divergences from the §3 sketch: the vocabulary is **new types**
  (`invariants.rs`: `LogicError` / `Convergence` / `FixableInvariant` over the refs
  registry's `Reference` edge), not an enriched `InvariantError` — the old vocabulary stays
  with the old checker until step 5; and the tier split is a **`Result`**
  (`broken_invariants() -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>>`,
  tier-2 logic errors short-circuit as `Err`), which makes the §5 "no map entry → PANIC"
  arm unrepresentable for step 6. Stages as landed:
  - **Stage 1 — empty ranges unrepresentable** (`d250c16b`, `d33100ed`, `220e8b9e`):
    `NonEmptyRangeInclusive` newtype, five field swaps, four error shapes deleted; decode
    hard-errors on an empty range in a file; bytes untouched (§6c applied to value shapes).
  - **Stages 2–5 — vocabulary + the three layers** (`7667091b`, `ea62dead`, `7663844b`,
    `2bcf0a83`): `Ord` on `Reference`/site enums; layer A (logic errors, `Err` path),
    layer B (generic dangling sweep via `for_each_reference`), layer C (hand-written
    convergence walks, skip-on-dangling).
  - **Stage 6 — old-checker backfill** (`8f3bb093`): the two missing colloscope
    canonical-absent checks added to the *old* architecture
    (`ColloscopeError::{EmptyInterrogationRow, EmptyGroupListRow}`) — the old checker is
    now the *complete* ground truth this step's differential and step 3's audit run against.
  - **Stage 7 — legacy bridge + differential** (`be9eac90`, `158567a5`, `3d139d3a`): total
    `to_legacy`, `is_necessarily_logic_error`, the three-part differential over every
    fixture — a delivered head start on step 4 (the fuzz will drive the same harness).
  - **Beyond plan** (`49b4f77d`): a parallel property harness on the new checker
    (`property_ops_broken_invariants.rs`, forward direction only); the old harness stays
    the oracle, untouched.

  End-of-step gate: `cargo test --workspace` green, `Cargo.lock` unchanged (July 18 2026).

**Step 3 — completeness audit — COMPLETED July 19 2026.** Certify the **old** `check_invariants` as the reliable
*reference oracle* for the step-4 differential fuzz — the fuzz asserts verdict agreement,
so a disagreement is only meaningful if the old checker is known-complete. (An earlier
version of this step aimed the audit at the *new* checker and ended with "the new checker
is ground truth" — that was wrong; the new checker earns trust *through* steps 3–4, it does
not confer it.) The audit is two composed arrows: **old ⊆ new** (the July-19 review pass
below) and **ops ⊆ old** (the session-plan survey: no elementary op enforces an *invariant*
the old checker misses — transition checks like no-clobber, op-target existence and the
other §4 carve-outs are expected checker-absent and are inventoried as such). Together:
every invariant enforced anywhere is visible to both checkers. The session plan is
retired per the house pattern (pinned at `git show 26d88024:docs/plans/plan_step_3.md`);
it holds the full row-by-row tables: every invariant-guarding op check → its old-checker
twin, the carve-out register, and a field-by-field coverage sweep of the whole data model
(the "missed by everything" backstop, checked against Appendix A.1 + A.2). **Doc-only
step**: findings were recorded, not fixed. Result: **no gaps** — the survey record and
its five observations are Appendix D.

  *The old ⊆ new arrow — review pass, July 19 2026* (ahead of the step-3 session plan): the old
  checker's complete condition set — 57 conditions: 3 top-level (`lib.rs:175`), 42
  params-side (`colloscope_params.rs`), 12 colloscope-side (`validate_against_params`) —
  each map to a `LogicError` / `DanglingFk` site / `Convergence` variant; counts match
  Appendix C.2 (9 + 16 + the registry sweep). The refs-registry edge inventory and the
  layer-B site set were derived independently and agree, colloscope tables included. The
  only non-sweeps are the two encapsulated mirror families (Appendix C.3) — verified
  private-field + compound-mutator sound. **No gaps found.** Four decisions confirmed in
  review (documented in code where noted):
  - **Incompat subjects need no interrogations** — intended, in both checkers: an
    incompatibility may block slots for students declared in a subject whose own schedule
    creates the unavailability, without that subject running colles. Documented on
    `Incompatibility::subject_id` (`incompats.rs`).
  - **Mirror desync = `LogicError`** — *superseded by the loose-ends phase (Appendix F).*
    This bullet originally planned a fail-fast read-path panic once the old checker retired.
    The loose-ends review (July 25 2026) ruled otherwise: since the old checker carries the
    full ordering↔table mirror and retires at step 5, the new checker must carry it too, as
    `LogicError`s (eight variants). A desync is unreachable by ops but decidable from the
    data and code-at-fault-if-present — exactly `LogicError`. Row-key liveness alone stays
    out (it is the op-reachable dangle owned by layer B). See Appendix F.
  - **The id-issuer high-water check stays**: it is `Data`-level state outside `InnerData`,
    so the step-5 wiring keeps `Data::check_invariants`' issuer assert as a separate
    companion to `broken_invariants`. Documented on `Data::check_invariants` (`lib.rs`).
  - **`Err` short-circuit confirmed intended**: the fixable sweeps cannot be trusted over a
    logically-broken state, so `Err(LogicError)` deliberately says nothing about
    co-occurring fixable breaks. Module docs strengthened accordingly.
  With the ops ⊆ old survey delivered (July 19 2026, Appendix D), the old checker is
  certified complete: it is the reference oracle step 4's differential fuzz measures the
  new checker against.

**Step 4 — differential fuzz — COMPLETED July 21 2026.** A way to build arbitrary
(including invalid) `InnerData`: random elementary ops applied through a `force_apply` door
*without* checking, deliberately landing in inconsistent states. The fuzz asserts the two
checkers agree on the **verdict** (old rejects iff new reports non-empty) — not on variants:
the old checker returns its first error in its own order, the new vocabulary is richer.
Encapsulated invariants (§6c) can't be broken this way — by design, out of the fuzz's scope.
Delivered as depth-1 corruption probes off a validated walk (in step 5 `force_apply` only
ever runs on a *consistent* state, so `{valid state} + {one forced op}` is the target
distribution). Two things earned here that step 5 builds on: the governing rule
**`force_apply` fixes nothing** (it does only what the op asks, never repairs the rest of
the state), and the per-domain precheck vocabulary (the carve-out subset that survives the
step-5 deletion). **The delivered `force_apply` door, precheck vocabulary, strip/keep rule,
and differential fuzz are Appendix E.**

**Pre-step-5 loose ends — COMPLETED July 25 2026.** A small consolidation phase run before
step 5 rewires production, so step 5 lands on a cleaner op surface. What changed under step
5's feet (full delivered state in Appendix F):

- **B.1/B.2/B.3 (periods/weeks)** — `Periods` shrank to existence-only
  (`OrderedTable<PeriodId, ()>` + `first_week`); week data moved to a new `Weeks` module
  (twin of `slots.rs`: `week_map` + a sparse `ordering` sidecar). `PeriodOp::Remove` is no
  longer week-empty-gated in the *force* path (checked apply keeps its guard until step 5) —
  removing a week-bearing period now leaves dangling `WeekPeriodFk`s for the cascade. Read
  surface re-homed onto `Weeks`/`Parameters` (slots naming).
- **C.3 / D.3 (GroupList + the empty-first trio)** — the `GroupList` smart-constructor churn
  C.3 parked for step 5 is done: `GroupList` is sealed (private fields, validating `new()`),
  and the elementary `GroupListOp` carries a whole consistent `GroupList` (elementary
  `SetFilling` gone; the high-level `ops/` API is frozen and translated onto `Update`). This
  makes the D.3 empty-first trio unrepresentable: `RemainingFilling`,
  `PrefillGroupCountMismatch`, `NonEmptyGroupsWhenReducing` are all deleted.
- **E.3 (precheck enums)** — `GroupListPrecheckError`/`GroupListError` shrank accordingly.
- **D.4-F1 unchanged** — checked `apply_*` keeps its guards (incl. `NotEmptyPeriodInColloscope`)
  until step 5; only the force copies lost the invariant guards.
- **New-checker mirror coverage** — the new checker now carries the full ordering↔table
  mirror as `LogicError`s (superseding C.3's "trusts them unconditionally" and §8's
  "fail-fast panic"), so the old checker can retire at step 5 without leaving validation
  behind.

**Step 5 — switch elementary ops to apply/check/restore (§4).** `apply` becomes
`force_apply` + new checker + rollback — the same primitives step 4 built. The item-1 canary
pattern: one commit runs old validation and the new gate side by side with the property harness
asserting verdict agreement across generated valid+invalid ops; the next commit deletes
candidate validation and delete-blocking. The per-op error enums collapse into the precise
`InvariantError` + the §4 carve-out errors; `found_bugs.rs` exact-variant asserts and any
gtk4/python error matching migrate in the same change.

**Step 6 — the cascade (§5).** Resolution map + retry queue + no-progress guard; the compound
reverse feeds the history stack; a confluence pin test freezes the emitted op list on a
hand-built document.

**Step 7 — migrate `ops/` (the remaster).** Each natural op becomes: open a session, run
`apply_cascade`, present the extra ops to the user (dry-run preview, §5), commit or cancel.
The `Warning` enums, `get_next_cleaning_op`, and the hand-written consequence detection retire;
an op-list rendering layer replaces the warning texts.

## 9. Impact on the existing plans

- **`state_consolidation_plan.md` §6 item 3 — superseded, direction reversed.** Item 3 planned
  to *demote* the whole-model check to debug assertions and keep per-op typed preconditions;
  this design does the opposite (the whole-model check becomes the sole enforcement, the
  preconditions retire). The extended scope note ("reroute the triplicated checks through the
  item-2 registry") and the **§6 hand-off notes of the retired table-registry plan** (transcribing
  per-op check orders, site→typed-error mappings for delete-blocking) are superseded with it —
  there is nothing left to reroute; two of the three families are deleted.
- **`state_consolidation_plan.md` §6 item 5 — dissolved** by step 1d + the cascade: with a
  sparse colloscope there is no fan-out left to centralize; cleanup is cascade resolution. The
  spec-2 format was shaped for exactly this re-keying and does not move.
- **`state_consolidation_plan.md` §6 item 4 (uniform op granularity) — independent, but
  interacts.** The resolution map wants elementary ops that can express "remove/clear this one
  reference" conveniently; step-1 reshapes re-cut the slot/week/colloscope op surfaces anyway.
  Granularity uniformization can ride along per step or stay a later pass.
- **Table-registry-plan phase C artifacts** — `Join` kept; `References`/`for_each_ref`
  recast as the step-2 sweep engine; `RefSite`/`references_to_*` likely retired (§7, pending a
  gtk4 claim); `Lookup`/`resolve`/`all_ids` unaffected. Phases D/E completed July 16 2026.
- **`ops/` — step 7 is the promised remaster.** Until then, decision 6 of the registry plan
  (touch `ops/` minimally) stands. Supporting evidence for computed-over-hand-written
  consequences: the suspected dormant drift bug in `general_planning.rs`
  (`UpdatePeriodWeekCount`'s colloscope-cleaning loop iterated
  `old_week_count..*week_count`, an empty range under its own guard — bounds swapped, so the
  cleaning op could never fire) **was confirmed real and fixed in step 1 phase 0**
  (test-first, `1418d4bf` + `f8e34128`, regression pinned in `ops/tests/found_bugs.rs`);
  the class disappears at step 7.
- **The safety net** — the property harness stays the oracle throughout and gains the step-4
  differential fuzz; `found_bugs.rs` keeps its regression *scenarios* but its exact-variant
  asserts are rewritten at step 5 to the new error vocabulary.
- **Storage — the frozen format does not move.** Every step-1 reshape is byte-stable by
  construction (the format is already sparse/positional where the memory model changes);
  byte-stability tests + the golden fixture gate every reshape commit.
- **Python contract scripts** (`state_consolidation_plan.md` §7) — read-shape changes land at
  1a/1d (dense mirrors leave the read surface) and at step 5 (error surface); scripts are
  updated in the same change and run by the user as acceptance.

## 10. Risks & open questions

- **Completeness becomes load-bearing** — addressed head-on by steps 3–4 (audit + differential
  fuzz) *before* the step-5 switchover.
- **Error-vocabulary migration is UI-visible** — the step-5 collapse of per-op error enums
  must sweep gtk4/python error handling in the same change; budget for it in that step's plan.
- **Step 1 pays the old-architecture tax** — each reshape updates the triplicated checks one
  last time. Accepted: bounded, and the alternative (new checker first, reshapes after) would
  mean writing precise dense-completeness errors and an additive cascade only to delete them.
- **Dry-run cost** — a clone + full cascade + N checks per preview; fine interactively at this
  scale, watch for eager/live previews.
- **Confluence** — by construction (§5: all-errors checker, canonical `Ord` pick, deterministic
  map); one pin test guards against checker-order refactors.
- **Cascade liveness** — a map entry that doesn't resolve its invariant would loop; the
  no-progress guard turns that into a loud panic (logic error, not a hang).
- **Frozen on-disk format** — all step-1 reshapes verified byte-safe against the shipped spec-2
  shapes (assignments/slots/colloscope sparse-permissive, weeks positional); any *future*
  reshape beyond them needs its own format review.
- **Irreducible tier-3 residue** — accept it; ensure each residual constraint is *expressible*
  (via `Join`) and *resolvable* (via the cascade + preview, lossily where needed).

Open (settled in the relevant step's session plan, not here):
- ~~exact in-memory keying of the sparse colloscope~~ — settled in 1d: **flat composite keys**
  (Appendix B.1);
- ~~whether the slots `ordering` sidecar goes sparse~~ — settled in 1c: **yes** (row iff ≥1 slot);
- ~~the `WeekDesc` container shape and the re-cut week op surface~~ — settled in 1b
  (Appendix B.1/B.2);
- ~~the `Ord` used for the canonical cascade pick (derive order on the invariant enum is the
  natural candidate)~~ — settled in step 2: **derive order** (`DanglingFk < Convergence` so
  `min()` prefers the precise row-removal fix; declaration order within each enum). Step 6
  may still reorder variants — a variant-order edit, not a mechanism change (Appendix C.1).

---

## Appendix A — inventories inherited from the retired table-registry plan

Copied (July 16 2026) from §3.2/§3.3 of the retired `docs/table_registry_plan.md` (item 2's
detailed plan, delivered in full; the whole document is pinned at
`git show 77695338:docs/table_registry_plan.md`). Step 2 used A.1 as the existence-sweep
target set (via the refs registry, B.5); step 3 used A.1 + A.2 as the completeness-audit
checklist (Appendix D). File/line references
are against the tree at commit `de8ed888` (July 13 2026) and have rotted since — the file +
function names are the stable part. The "block"/"twin" error columns describe the *old*
architecture this design replaces; they document exactly what the new checker must cover.

### A.1 The relationship inventory (28 ID-based relationships)

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
| 10 | `slots.ordering` key (dense) — slots themselves carry `Slot.subject_id` as a regular `#[fk]` | dense-mirror key | `SubjectStillHasAssociatedSlots` | `WrongSubjectCountInSlots` |
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

### A.2 Index-based and structural checks (outside the reference registry)

These are *value/shape* checks, not ID-existence checks; they were hand-coded under the old
architecture and each must find its home (tier or encapsulation) under this design:

- **Group-number bounds** (indices, not ids): `ColloscopeInterrogation.assigned_groups` and
  `groups_for_students` values vs the associated group list's `group_names.len()`
  (`ColloscopeError::InvalidGroupNumInInterrogation` / `InvalidGroupNumForStudentInGroupList`,
  `check_interrogations_group_bound` in `group_lists.rs:287`).
- **Structural counts** of the dense mirrors: `WrongSubjectCountInAssignments`,
  `WrongSubjectCountInSlots`, `WrongPeriodCountInSubjectAssociationsForGroupLists`, the
  colloscope shape/count/week-structure checks, `BadWeekPatternLength`. (The step-1 reshapes
  eliminate most of these.)
- **Pair-level predicates**: `SameSubjectInBothParts`, `SameSlotInBothParts`,
  `SlotsNotInSameSubject`.
- **Side-constraints attached to references** (the retired SQL schema encoded these by
  pointing FKs at `subject_interrogation_params` instead of `subjects`): "referenced subject
  must have interrogations" (`TeacherError::SubjectHasNoInterrogation`,
  `BalancingForSubjectWithoutInterrogations`, …), "teacher must teach the slot's subject"
  (`TeacherDoesNotTeachInSubject`), "subject must run on the period"
  (`SubjectAssociationForSubjectNotRunningOnPeriod`). These are the tier-3 convergence
  residue of §6.

---

## Appendix B — step 1 as delivered (July 18 2026)

Recorded when the step-1 session plan was retired (full plan with the decision ledger and
per-phase mechanics pinned at `git show 62949404:docs/plans/plan_step_1.md`; commit anchors
in §8). This is the ground truth steps 2–7 build on. Note that Appendix A's dense-mirror
rows (A.1 #5/7/10/21/26/28 payload shapes, A.2 structural counts) describe the *pre*-step-1
state; this appendix supersedes them as the description of the live model.

### B.1 Final data shapes (what the step-2 checker is written against)

- **Assignments**: `Table<(PeriodId, SubjectId), BTreeSet<StudentId>>` — a row exists iff
  its student set is non-empty. Whether a subject runs on a period is *not* encoded here;
  the only source is `Subject.excluded_periods`.
- **Periods/weeks**: private `ordered_period_list: OrderedTable<PeriodId, Vec<WeekId>>` +
  `week_map: Table<WeekId, Week>` with `Week.period_id` as the authoritative FK; the
  within-period ordering is encapsulated behind compound `pub(crate)` mutators (§6c) — the
  list↔map mirror is checked (`InvariantError::InvalidWeek`), the ordering itself never
  reaches `check_invariants`. `WeekDesc` survives as the FK-less op-payload/glue DTO.
  *Reshaped by the loose-ends phase (Appendix F): `Periods` is now existence-only and week
  data lives in a separate `Weeks` module; the ordering mirror is now checked by the new
  checker as `LogicError`s, not merely by the old checker.*
- **Slots**: unchanged flat `slot_map: Table<SlotId, Slot>` (+ `subject_id` FK) with the
  `ordering` sidecar, whose rows are now **sparse** (row iff the subject has ≥1 slot).
- **Week patterns**: `WeekPattern { name, excluded_weeks: BTreeSet<WeekId> }` — the
  exception set; absent = active. The length-coupling invariant (#8) is gone; a removed week
  is a dangling `WeekId` (tier 1). `excluded_weeks` is *not* canonicalized against
  `week.interrogations` (a file may exclude a non-interrogation week; the bit is preserved
  for byte-stability). Merged activity = `week.interrogations ∧ ¬excluded`.
- **Colloscope**: two **crate-private** sparse tables, flat composite keys —
  `interrogations: Table<(SlotId, WeekId), BTreeSet<u32>>` and
  `group_lists: Table<GroupListId, BTreeMap<StudentId, u32>>`. The period layer and the
  one-field wrapper types are gone; all access goes through the surface (B.3).
- **Canonical-absent everywhere**: no empty rows in assignments, either colloscope table, or
  the slots ordering sidecar. Enforced at op sites (an empty write clears the row); the
  step-2 checker must *assert* it — it is what keeps `InnerData::Eq` honest.

### B.2 Op surface (what the step-6 resolution map emits from)

- **`WeekOp { AddFront, AddAfter, Remove, Update, Move }`**; `apply_week` is the sole week
  writer. `Move` **carries content** (pattern membership travels with the id — no pattern
  work at all; colloscope cells travel verbatim), guarded only where content cannot travel
  (dest period lacks the slot / group numbers exceed the dest association bounds). `Remove`
  requires trivial state (no pattern excludes the week, cells empty), so undo re-adds with
  the original id. `WeekId` is **preserved across cut/merge** (re-parenting, not
  delete+recreate) — colloscope cells and pattern exclusions survive.
- **`PeriodOp { ChangeStartDate, AddFront, AddAfter, Remove }`** — a period is created
  empty; `Remove` requires week-empty (`PeriodStillHasWeeks`). *Loose-ends phase (Appendix
  F): the force path dropped this guard (checked apply keeps it until step 5); the
  elementary `GroupListOp` was also consolidated to carry a whole sealed `GroupList`.*
- **Colloscope ops are upserts**: `SetInterrogation(SlotId, WeekId, BTreeSet<u32>)` /
  `SetGroupList(GroupListId, BTreeMap<StudentId, u32>)`; empty payload = remove row;
  reverse = `Set…` with the prior payload (or empty).

### B.3 Read surface and oracles

- **`Periods`**: `walk()` is the canonical global order (`walk().enumerate()` = global week
  index); `weeks_of`, `weeks_vec_of`, `find_week` (owning period via `week.period_id`),
  `week_id_at`, `week_position`, `global_week_position`, `period_ids()`, `week_count_of`;
  `find_period(PeriodId) -> Option<&Vec<WeekId>>` is pub and pinned by the `read_api`
  pointer-identity test. `Lookup<WeekId> → Week`, `Lookup<PeriodId> → Vec<WeekId>`.
  *Loose-ends phase (Appendix F): this read surface re-homed onto `Weeks`/`Parameters`
  (slots naming); `find_period` was deleted, `Lookup<PeriodId>` now yields `()`, and the
  `read_api` pointer pin moved to `find_week`.*
- **Possibility oracles** (permanent, the single re-expression of the old dense "cell is
  `Some`" rule): `WeekPatterns::is_week_active(periods, week, pattern)` (homed there so
  gtk4 piece-clones can call it; `Parameters::is_week_active` delegates) and
  `Parameters::is_interrogation_possible(slot, week)`. Every re-derivation — constraints
  zero-fill, python dense views, decode trust-boundary checks — goes through these.
- **Colloscope surface**: readers `interrogation`, `interrogations_for_slot`, `iter`,
  `group_list`, `group_lists_iter`; writers `set_interrogation` / `set_group_list` (panic
  on impossible coordinates, empty payload clears the row — canonical form maintained).

### B.4 Deleted or dissolved by step 1

- The **~330-line params→colloscope structural fan-out** (item 5's problem) — a removed
  period/week/slot/group-list now leaves dangling rows for the cascade; the Remove guards
  survive re-cut to row-existence scans (range scan on the composite key for slots).
- `new_empty_from_params` (all levels), `update_slot_for_week_pattern` /
  `update_slot_to_match_week_pattern`, `check_empty_on_removed_weeks`,
  `save_then_clean_end_of_period` / `restore_end_of_period`, the pattern splice helpers
  (`add_weeks` / `remove_weeks` / `clean_weeks` / `can_remove_weeks` / `move_week`).
- Dense-count invariants (`WrongSubjectCountInAssignments`, `WrongSubjectCountInSlots`,
  `BadWeekPatternLength`, the colloscope shape/count/week-structure checks) and the 10 dead
  `ColloscopeError` variants; the last positional payload
  (`InvalidGroupNumInInterrogation`) re-cut to row vocabulary `(SlotId, WeekId)`.
- The stale multi-colloscope block (`lib.rs`) and the unused `FromDataError` (phase-1
  riders) — nothing left there for the step-3 audit.
- **No transitional code from step 1 survives** — the B1/B2 splices all died in 1d/D2.

### B.5 What still stands (the old architecture steps 2–7 replace)

- The **triplicated checks** (candidate validation, delete-blocking, `check_invariants`)
  still exist — step 1 hand-updated them one last time (the accepted tax); steps 2–5
  collapse them.
- **Refs registry**: gained `WeekPeriodFk` (walked first in `walk_params_refs`).
  The registry now holds only *direct* references, fine-grained: the old
  `WeekPatternLengthCoupling` (a materialized *transitive* pattern → period edge)
  was **removed** — it is derivable from `WeekPatternExcludedWeek` (pattern → week) composed
  with `WeekPeriodFk` (week → period), and the cascade derives period blocking through week
  deletion, so the step-7 remodel deferral dissolves. `AssignmentsKey` dropped its
  `non_trivial` flag (rows are canonical-absent, so a walked row is always non-trivial), and
  `SlotsOrderingKey` was removed (a subject has an ordering row iff it has ≥1 slot,
  with the key pinned to each slot's `subject_id`, so it adds nothing over `SlotSubject`).
  The pairing part sites split by role — `PairingRuleAntecedent` / `PairingRuleConsequent`
  (and `SlotPairingRuleAntecedent` / `SlotPairingRuleConsequent`) — so errors can name which
  side of a rule references a subject/slot. The god `RefSite` enum was then **split into
  eight per-kind site enums** (`PeriodRefSite`, `WeekRefSite`, …) plus a `Reference` edge
  enum (`{ target, site }`, one variant per kind): `RefVisitor` callbacks and
  `references_to_*` are now typed per kind, so a consumer matches exhaustively over one
  kind's real cases only — no impossible arms, no catchall. Payloads follow the
  **key-complement rule** (a site carries the referencing row's coordinates *minus* the
  target; the target is never duplicated into a payload, and derivable values aren't
  carried), so the two-sided rows (assignments `(period, subject)`, association entry,
  colloscope interrogation `(slot, week)`) each yield one site per id occurrence — resolved
  by the same row-removal op, arbitrated by the canonical order. `InnerData::for_each_reference`
  funnels the whole walk into the flat `Reference` stream.
- **Python glue keeps the dense-view pyclass contract** as *computed* views over the sparse
  core (`Colloscope::from_mem(&mem::Colloscope, &Parameters)`, assignment seeding, pattern
  projection) — throwaway scaffolding for the upcoming Python API rework; reference
  scripts untouched.
- **`constraints-colloscopes` keeps `GlobalWeek`** internally; one canonical
  `WeekId ↔ GlobalWeek` map built at model entry (the `walk()` order).
- **Storage, format frozen**: decode pre-scans `max_used_id` over all id-bearing blocks and
  synthesizes `WeekId`s `max+1, …` in walk order; encode projects back to positional
  indices and never writes week ids — bytes unconditionally identical.
  `populated_round_trip` compares **re-encoded bytes**, not `InnerData` equality
  (decode-synthesized ids differ from ops-issued ones by design).

### B.6 Tests pinning the step-1 contracts

- `state-colloscopes/tests/week_ops.rs` — move-preserves-content, both Move guards,
  remove-blocked-by-pattern-exclusion, undo restores the id.
- `ops/tests/general_planning_content.rs` — cut preserves the tail's colloscope cell +
  pattern membership; merge-back structure. **This is the cut/merge contract later steps
  must not regress.**
- `state-colloscopes/tests/read_api.rs` — `resolve == find_period` by pointer identity.
- `state-colloscopes/tests/colloscope_surface.rs` — the sparse surface semantics
  (absent/empty equivalence, writer canonicalization).
- `ops/tests/found_bugs.rs` — the phase-0 `UpdatePeriodWeekCount` regression (first test
  file in the crate); plus the long-standing `state-colloscopes` `found_bugs.rs` family.
- The property harness (`property_ops.rs`, 100 seeds committed; 500-seed reference at
  milestones) remains the oracle; its generator lives in `collomatique-testgen-colloscopes`
  and targets ops from **params** (via the B.3 oracles), not from the data shapes.

---

## Appendix C — step 2 as delivered (July 18 2026)

Recorded when the step-2 session plan was retired (full plan — decisions ledger, complete
variant tables, the per-site legacy conversion table — pinned at
`git show 49b4f77d:docs/plans/plan_step_2.md`; commit anchors in §8). This appendix records
the *contract*; the variant sets and the legacy table live in
`state-colloscopes/src/invariants.rs`, pinned by tests — they are deliberately not copied
here. This is what steps 3–7 build on.

### C.1 The checker

```rust
impl InnerData {
    pub fn broken_invariants(&self)
        -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>>;
}
```

- **Semantics**: `Ok` = the *code* is sound; the payload is what the *data* needs fixed
  (`Ok(empty)` = valid). `Err` = a state no elementary op can legitimately reach — the code
  (or a hand-forged file) is wrong; the step-6 cascade panics on it, decode hard-errors.
  Logic errors are collected **first and short-circuit**: they undermine the meaningfulness
  of the fixable sweep, so the two payloads never mix. Consequence for step 6: the
  resolution map is **total over `FixableInvariant`** — the §5 "no map entry → PANIC" arm
  is unrepresentable.
- **Three layers on the `Ok` side of the pipeline**: **A** — logic errors (duplicate-id
  sweep, canonical-absent rows, prefill count/duplicate-student, parts-share-an-id); **B** —
  the generic dangling sweep: eight per-kind existence sets, then `for_each_reference`;
  every edge whose target does not resolve yields `DanglingFk(Reference)`. **C** —
  hand-written convergence walks mirroring the old semantics (the differential is the
  referee). Layer C **skips, never unwraps**, when a prerequisite ref dangles
  (`let Some(x) = … else { continue }`) — the `DanglingFk` entry already reports it; B and
  C coexist in one `Ok` set.
- **Canonical order = derive order** on `BTreeSet` (dedup free). `DanglingFk < Convergence`
  so that when a row is both dangling and convergence-broken, `min()` picks the precise
  row-removal fix over the lossy one. Ordering is pinned by tests; step 6 may reorder
  variants (a variant-order edit, not a mechanism change).

### C.2 The vocabulary (`invariants.rs`, re-exported from `lib.rs`)

Classification is **mechanical**, per edge/predicate — the module docs state the rule:

- **`DanglingFk(Reference)`** — the edge is in the refs registry and its target id does not
  resolve. Type-guaranteed edges (`WeekPeriodFk`) stay in the sweep — generic over the
  registry — and simply never fire.
- **`LogicError`** (9 variants) — truth decidable from a row's *own value* (or whole-document
  id-uniqueness): no other entity's state can flip it, so no legitimate op produces it by
  side effect. Nothing that follows a reference belongs here.
- **`Convergence`** (16 variants) — a predicate over *existing* edges that legitimate ops can
  break indirectly (e.g. `UpdateSubject` turning interrogations off, or lengthening the
  duration past midnight — `SlotOverflowsDay` is convergence, not logic). The cascade
  resolves these lossily.
- `FixableInvariant = DanglingFk(Reference) | Convergence(Convergence)`. **No `dangling()`
  unwrap helper**: every consumer (the step-6 resolution map foremost) matches both variants
  exhaustively — no caller is entitled to panic on one of them.
- All types derive `Ord` (+ `Ord` added to `Reference` and the eight `*RefSite` enums) and
  thiserror `Display`. **No Serde** — step 5 revisits when the vocabulary becomes UI-visible.

### C.3 Unrepresentable / encapsulated (what the checker does *not* sweep)

- **Empty ranges** — unrepresentable: `NonEmptyRangeInclusive<T>`
  (`non_empty_range.rs`; validated `new -> Option`, `Deref` reads, serde
  `try_from`/`into` `RangeInclusive`) on the five range fields (four `Subject` ranges +
  `GroupListParameters.students_per_group`). The four empty-range error variants are
  deleted; an empty range in a *file* is a decode hard-error (honest-decode rule);
  representation identical, bytes untouched.
- **Mirrors** — type-encapsulated (§6c): the periods list↔map and slots ordering↔table
  mirrors are maintained inside `Periods`/`Slots`; the new checker trusts them
  unconditionally. (The old checker's mirror sweeps are pre-encapsulation vestiges and stay
  there.) *Reversed by the loose-ends phase (Appendix F): since the old checker retires at
  step 5, the new checker no longer trusts the mirrors — it carries the full ordering↔table
  mirror as `LogicError`s. Only row-key liveness stays out (op-reachable dangle, layer B).*
- **Remaining cross-field value shapes** (prefill count/duplicate-student, parts-share-an-id
  ×2) stay `LogicError` variants for now: encapsulating them means privatizing
  `GroupList`/`PairingRule`/`SlotPairingRule` behind smart constructors — public-API churn
  step 5 does anyway. Do that churn once, at step 5. *Loose-ends phase (Appendix F): the
  `GroupList` half was done early — `GroupList` is sealed, so the prefill
  count/duplicate-student shapes are now unrepresentable (variants deleted). `PairingRule`/
  `SlotPairingRule` remain for step 5.*

### C.4 Old-checker parity + the legacy bridge

- **The old checker is now complete**: stage 6 backfilled the two missing colloscope
  canonical-absent checks (`ColloscopeError::{EmptyInterrogationRow, EmptyGroupListRow}`,
  rejected in `validate_against_params`). Zero observable change on real data — ops
  canonicalize at write time and the spec-2 codec routes through the canonicalizing sparse
  surface — only in-crate corruption tests reach them. Step 3 audits against this complete
  ground truth.
- **`to_legacy(&self) -> InnerDataError`** on both `LogicError` and `FixableInvariant` —
  **total** (bought by the backfill). Codomain is `InnerDataError`, not `InvariantError`:
  colloscope-side conditions live on the `ColloscopeError` arm, `DuplicatedId` on the
  top-level `DuplicateIds`.
- **`InnerDataError::is_necessarily_logic_error`** — `true` for exactly the six variants
  whose *every* possible cause is tier-2; mixed-cause coarse variants classify `false`.
- **The three-part differential** (every stage-3..6 fixture + clean + compound states):
  (1) verdicts always agree (old `is_ok()` ⇔ new `Ok(∅)`); (2) if new is `Err(L)` and old's
  error is logic-classified, old's error ∈ `to_legacy(L)` — lenient otherwise (in a
  compound state old may trip a fixable error first); (3) if new is `Ok(F)` non-empty,
  old's error ∈ `to_legacy(F)` exactly. **Step 4's fuzz drives this same harness** over
  generated states instead of hand-built fixtures.

### C.5 Unwired, and the tests pinning the contracts

`broken_invariants` has **no production caller** — wiring is steps 3–6. Pinned by:

- `invariants.rs` in-crate tests: ordering pins (`DanglingFk < Convergence`, declaration
  order), one corruption fixture per variant (forged ids / crate-internal field access —
  ops can't reach these states, which is the point), short-circuit and skip-on-dangling
  pins, compound-state leniency pins, per-site differential coverage of the legacy table.
- `colloscopes.rs` stage-6 corruption tests (the backfilled checks).
- `tests/property_ops_broken_invariants.rs` — the forward-only property mirror
  (old-valid ⇒ new fully clean; `broken_invariants` consumes no RNG, so seeded
  trajectories stay identical). The reverse direction is out of scope until step 4;
  `property_ops.rs` stays the oracle, untouched.

Deferred hooks: Serde + value-shape encapsulation → step 5; resolution map (total over
`FixableInvariant`) + possible variant reorder → step 6; `references_to_*` retirement still
pending a gtk4 claim (§7 unchanged).

---

## Appendix D — step 3 as delivered (July 19 2026)

Recorded when the step-3 session plan was retired (the full row-by-row tables — every
invariant-guarding op check → its old-checker twin, the complete carve-out register with
per-error sites, and the field-by-field coverage sweep — are pinned at
`git show 26d88024:docs/plans/plan_step_3.md`; file/line references there are against the
tree at `0a1041b6`). **Doc-only step**: no code was touched. This appendix records the
certification and everything later steps need from it.

### D.1 The certification

The old checker (`InnerData::check_invariants`, `lib.rs:175`) is **certified complete**: it
is fit to serve as the reference oracle of the step-4 differential fuzz. Two composed
arrows, each audited independently:

- **old ⊆ new** (the §8 review pass): each of the old checker's 57 conditions maps to a
  `LogicError` / `DanglingFk` site / `Convergence` variant.
- **ops ⊆ old** (the retired survey): no elementary op enforces an *invariant* — a property
  of the resulting state — that the old checker misses, and a field-by-field walk of
  `InnerData` (against A.1's 28 relationships + A.2) found no invariant checked *nowhere*.

Together: every invariant enforced anywhere in the crate is visible to both checkers,
which is exactly what the step-4 verdict differential requires.

### D.2 Structure facts the survey leaned on (still true, worth knowing in steps 4–5)

- Every `apply_*` is validate-before-mutate; after every successful apply, `lib.rs:340`
  runs the full old checker as a panic net — so op/checker drift surfaces as a production
  panic *provided the checker knows the invariant* (the conditional the survey discharged).
- Add/Update payload validation goes through the **same** `validate_*` helpers the old
  checker's `check_*_data_consistency` families call, so for those checks op/checker drift
  is structurally impossible. The drift-able surface is only the hand-written Remove/Update
  guards — each individually reconciled in the pinned Table 1.

### D.3 The carve-out register (checker-absent by design — the step-5 keep-list)

Every op check without an old-checker twin falls into one of the §4 transition categories;
none is an invariant. These error families are what **survives** the step-5 precondition
deletion (everything twinned in Table 1 retires in favor of the precise vocabulary):

- **No-clobber** — `*IdAlreadyExists` in every Add.
- **Op-target existence** — `Invalid*Id` on the entity being updated/removed.
- **Parameter targeting** — op inputs that must resolve (`AddAfter` anchors incl.
  `PreviousSlotIsNotInRightSubject`, `Assign`/`SetInterrogation`/`SetGroupList`/
  `AssignToSubject` coordinates, `WeekMove` destination).
- **Position bounds** — `InvalidPosition`, `PositionOutOfBounds`.
- **Empty-first protocol** — `PeriodStillHasWeeks`, `RemainingFilling`,
  `NonEmptyGroupsWhenReducing` (op-ordering discipline; these three are *preconditions*,
  so step 5 decides their fate with the rest — the states they demand are valid either way).
  *Loose-ends phase (Appendix F): `RemainingFilling` and `NonEmptyGroupsWhenReducing` are
  deleted (the consolidated `GroupListOp` carries a whole `GroupList`, so the Remove/Update
  reshaping that needed them is gone); `PeriodStillHasWeeks` was dropped from the force path
  only (checked apply keeps it until step 5).*
- **Immutability** — `CannotChangeSubject` (a slot's subject is fixed at creation).
- **Payload shape** — `PrefillGroupCountMismatch` (dual-listed: also invariant-twinned).

### D.4 Findings F1–F5 (no gaps; observations for the record)

- **F1 — vacuous guard**: `NotEmptyPeriodInColloscope` (`periods.rs`) can never fire after
  the `PeriodStillHasWeeks` guard; commented in code as defensive redundancy. Disappears
  with all preconditions at step 5.
- **F2 — the one drift-risk spot**: `WeekMove` (`periods.rs`, destination checks)
  re-implements the checker's slot-runs-on-period + group-bound logic inline instead of
  calling shared helpers. Not refactored — step 5 deletes all preconditions — but **any
  pre-step-5 touch of that code must re-verify the pairing** against
  `SlotNotRunningOnPeriod` / `InvalidGroupNumInInterrogation`.
- **F3 — check-order quirk**: `GroupListAssignToSubject` tests the subject before the
  period; a dangling period harmlessly reaches `SubjectDoesNotRunOnPeriod` first.
  Error-variant choice only.
- **F4 — justified asymmetry**: `SubjectUpdate`(interrogations→off) re-checks
  balancing/teachers/associations/slots but *not* pairing/incompat references — correct:
  those edges don't require interrogations, in the validators and both checkers alike
  (documented on `Incompatibility::subject_id`).
- **F5 — a filled group list needs no association**: this concerns *only* the
  `Colloscope.group_lists` table — the per-list student→group fillings. Such a row may
  reference a list not (yet) associated to any subject×period: ops and both checkers
  agree this is valid (a filling can be prepared first, the list associated afterwards) —
  **the step-6 resolution map must not "fix" it**. It does NOT extend to
  `Colloscope.interrogations` rows: those *do* require the `(period, subject)`
  association — without one the group-number bound saturates to 0 (old checker
  `.unwrap_or(0)`, new checker `None => Some(0)`), so every interrogation placement at an
  association-less coordinate is invalid (`InvalidGroupNumInInterrogation` /
  `Convergence::InterrogationGroupOutOfBounds`). Clearing an association out from under
  live interrogation placements therefore lands a *broken* state, in both checkers
  (re-verified Jul 21 2026 when a mis-reading of this finding briefly suggested
  otherwise).

## Appendix E — step 4 as delivered (July 21 2026)

Recorded when the step-4 session plan was retired (the per-commit mechanics, the full
strip/keep row list, and the corruption-generator recipes are pinned at
`git show fbc4ae6d:docs/plans/plan_step_4.md`; its file/line references are against the
tree at `d3dcc4e5`, with the Table 1/2 line refs against `0a1041b6`). Step 4 built the
`force_apply` door and used it to **corroborate the new checker against the certified old
one** on invalid states via differential fuzzing. Both checkers are still wired only in
tests — step 5 rewires production.

### E.1 The `force_apply` door

`Data::force_apply` (`lib.rs:425`) applies one op and **never checks invariants**:

- Returns `Ok(reverse)` or `Err(PrecheckError)`; a failed call leaves the state unchanged.
- A *successful* call **may leave the state invalid** — that is the point. The caller owns
  checking (`InnerData::check_invariants` / `broken_invariants`) and restoring a snapshot on
  failure.
- It is an **independent thin copy** of `apply`: each arm calls a `force_apply_*` twin of
  `apply_*`, and `apply`/`apply_*` are byte-untouched. Three differences from `apply`:
  errors are `PrecheckError`; the `GlobalUpdate` arm drops the `check_invariants()?`
  pre-gate (this *is* the force door); the trailing `check_invariants()` panic net is
  omitted. It is the step-5 apply/check/restore primitive; the checked originals are deleted
  there, so the duplication is short-lived.

**The governing rule: `force_apply` fixes nothing.** It does only what the op asks, and
never touches state outside the op's direct target to keep the rest consistent. A broken
landing is valid; reporting it is the checker's job.

### E.2 The strip/keep rule as executed

Each `force_apply_*` is `apply_*` with its guards classified by the step-3 survey (pinned
Table 1/2):

- **Stripped** — every guard with an old-checker twin (the invariant guards): the
  `validate_*` calls and the hand-written Remove/Update reference scans.
- **Kept** — the carve-out register (D.3): no-clobber, op-target existence, position/anchor
  bounds, empty-first protocol (`PeriodStillHasWeeks`, `RemainingFilling`,
  `NonEmptyGroupsWhenReducing`), immutability (`CannotChangeSubject`), and the
  parameter-targeting checks. The dual-listed guards (Assign coordinates, the `SetFilling`
  prefill boundary) are kept.
- **Mutation copied verbatim**, including same-domain canonicalization that is *live* in
  checked apply — the assignments empty-row drop, the slots empty-ordering-row drop, the
  group-list Update filling truncate/extend, the sparse colloscope writers. These are op
  semantics, not repair, and the fuzz's forced ≡ checked pin locks them against drift.

**Refinement (fuzz-found, Jul 21): guard-dead cleanup loops strip too.** Cleanup reachable
only when a *stripped* guard would have rejected the op is not mutation — alive in the copy
it silently repairs. The one instance: `force_apply_period` Remove's association-row drop
(dead in `apply_period` behind the stripped
`PeriodStillHasNonTrivialGroupListAssociation` guard). Left in, it would land a *valid*
state on a removal checked apply rejects, and irreversibly. Stripped (commit `4e7563f5`);
a Jul-21 audit of all 16 copies found no other instance.

### E.3 The precheck vocabulary

Each domain gets a new `*PrecheckError` enum holding **only its keep-list** — this *is* the
step-5 carve-out error surface, born here (`apply`'s existing per-domain error enums are
untouched). The three value-only domains carry empty enums (`SettingsPrecheckError`,
`BalancingPrecheckError`, `ExportConfigPrecheckError` — kept for uniformity). The top-level
`PrecheckError` (`lib.rs:297`) mirrors `Error` 1:1 with `#[from]` transparency, minus the
infallible `GlobalUpdate` arm. *Loose-ends phase (Appendix F): `GroupListPrecheckError`
shrank to `{InvalidGroupListId, GroupListIdAlreadyExists, InvalidSubjectId, InvalidPeriodId}`
and `PeriodPrecheckError` lost `PeriodStillHasWeeks`, as the force copies shed those guards.*

### E.4 The differential fuzz

`state-colloscopes/tests/differential_force_apply.rs` — a validated random walk (existing
testgen harness, byte-untouched) interrupted every `PROBE_STRIDE = 10` successful ops by a
**depth-1 corruption probe**: snapshot → force one op → `assert_differential` → restore →
resume. In the step-5 architecture `force_apply` only ever runs on a *consistent* state, so
`{valid state} + {one forced op}` is the exact target distribution. `assert_differential`
(`invariants.rs:782`, moved to crate scope in commit 1) is the stage-7 three-part check that
the old and new checkers agree.

Probe kinds — `CorruptionKind` in `testgen-colloscopes` (`ForceRemove`, `ForceRetarget`,
`ForceSemantic`, `ForceLogic`, `ForceValid`). `ForceValid` carries the standing
**anti-drift pin**: on a truly valid op the thin copy must match checked apply exactly
(state + reverse).

Two hard-won test-protocol rulings (Jul 21):

- **No targeting prechecks in `force_apply`.** Pre-checking consistency is the very thing
  being removed. `gen_op`'s valid arm only guarantees a valid-*shaped* op, so checked apply
  may still reject a `ForceValid` probe (a stripped guard). When it does, the forced landing
  must be **broken** (the checker sees the damage) **or a perfect no-op** (the rejected input
  left no trace — e.g. clearing an association at a coordinate that cannot hold one). A
  state-*changing* valid landing would be hidden work: a silently repairing copy. This is a
  carve-out in the *test*, never a guard in `force_apply`.
- **Clean-landing reverse pin.** When a forced op lands a valid state, its reverse must
  restore the pre-state exactly (that reverse feeds history in step 5).

**Honesty guards** (cross-seed): ≥25% of landed probes were actually broken (old checker
`Err`); every kind attempted, each *corrupting* kind landed a broken state at least once.
Committed config: `seeds: 100, ops_per_run: 1000`; a 500-seed crank was run once locally
and is green.

### E.5 Delivered artifacts & standing rules for steps 5–6

- `Data::force_apply` (`lib.rs:425`) + 16 `force_apply_*` twins; `PrecheckError`
  (`lib.rs:297`) + 16 `*PrecheckError` enums; `assert_differential` at crate scope
  (`invariants.rs:782`); `CorruptionKind` + `gen_corruption_op` in `testgen-colloscopes`;
  the fuzz test. `apply` and all `apply_*` byte-untouched.
- **Rules later steps must honor:** `force_apply` fixes nothing (no cross-target
  consistency maintenance, no guard-dead cleanup); no consistency prechecks are ever added
  to it; and the step-6 resolution map must not "fix" a filling-without-association (F5,
  D.4) or any placement it did not itself create.

## Appendix F — pre-step-5 loose ends as delivered (July 25 2026)

Recorded when the loose-ends session plan was retired (per-commit mechanics pinned at
`git show 25fdc50b:docs/plans/plan_loose_ends.md`; commits: P1 `5b416763`, P2 `af543578`,
P3 split into `e66f62fe`..`286c242f` + follow-up `7d4b67c2`, G1 `5f731cd3`, G2 `7e4c3e71`,
mirror `LogicError`s `4b1b8f5f`). This phase ran *before* step 5 to clean the op surface it
lands on. Two op-surface warts were removed (periods-with-weeks removal; the split-value
group-list ops), and one checker gap the differential fuzz was masking was closed. Both
checkers are still wired only in tests — step 5 still does the production rewire.

### F.1 Periods and weeks (supersedes the periods/weeks parts of B.1/B.2/B.3)

- **`Periods` is existence-only**: `pub first_week: Option<WeekStart>` +
  `pub ordered_period_list: OrderedTable<PeriodId, ()>` (order and existence, mirroring
  `Subjects.ordered_subject_list`). A period owns nothing else. `PeriodId` is `#[entity(())]`;
  `Lookup<PeriodId>` yields `&()`. `Periods::from_ordered_ids(first_week, Vec<PeriodId>)`.
- **`Weeks` is a new module** (`state-colloscopes/src/weeks.rs`, twin of `slots.rs`):
  `week_map: Table<WeekId, Week>` + a sparse `ordering: Table<PeriodId, Vec<WeekId>>`
  sidecar (a row exists iff the period has ≥1 week — canonical-absent, like the slots
  ordering). The ordering-row key double-duties with the per-week `WeekPeriodFk`; no new
  registry edge (same argument that dropped `SlotsOrderingKey`). `Parameters` gained
  `pub weeks: weeks::Weeks`, ordered right after `periods`.
- **Read surface re-homed** (slots naming): single-container readers (`find_week`,
  `week_position`, `week_id_at`) and cross-container composites on `Weeks` taking
  `&Periods` (`walk`, `week_ids`, `count_weeks`, `global_week_position`, …), plus
  `Parameters::{walk_weeks, count_weeks, week_ids}` delegations. `weeks_of` →
  `weeks_for_period` etc. with slots-style `None = no row` semantics. `find_period` (the old
  borrowable-`Vec` accessor) is **deleted**; the `read_api` pointer-identity pin moved to
  `find_week`. `WeekPatterns::is_week_active(weeks: &Weeks, …)`.
- **`PeriodOp::Remove` no longer week-empty-gated in the force path.** Removing a
  week-bearing period is now representable and leaves each surviving week dangling at
  `WeekPeriodFk` for the cascade to repair. `PeriodPrecheckError::PeriodStillHasWeeks` and
  the guard in `force_apply_period` Remove are gone. **Checked `apply_period` keeps its
  guard** (it retires wholesale at step 5; stripping now would only turn the case into a
  `check_invariants` panic) — the 4.2 fuzz carve-out: a checked-rejected `ForceValid` probe
  must land broken, and it does.
- Storage wire format untouched: `spec2` decode builds both containers
  (`Periods::from_ordered_ids` + `Weeks::from_period_rows`), encode rebuilds the per-period
  week vecs; `populated_round_trip` stays byte-identical.

### F.2 Sealed `GroupList` (fulfils C.3's deferred smart-constructor churn)

`GroupList` has private fields (`params`, `filling`) and a validating
`GroupList::new(params, filling) -> Result<Self, GroupListBuildError>` that checks the
value-internal cross-field facts only (prefill group count matches the name count; no
student in two prefilled groups). Accessors `params()`/`filling()`/`is_prefilled()`/
`into_parts()`; serde via a private `RawGroupList` mirror whose `TryFrom` calls `new()`
(honest-decode, the `NonEmptyRangeInclusive` precedent). The one external constructor
(`storage` decode) maps `GroupListBuildError` to a hard decode error.

Consequences: `LogicError::{PrefillGroupCountMismatch, DuplicatedStudentInPrefilledGroups}`
and `GroupListError::DuplicatedStudentInPrefilledGroups` are **deleted** (unrepresentable);
the old checker's `validate_group_list_filling_internal` keeps only student-existence.
State-dependent facts (student existence) stay with the checker/walker as dangling FKs.

### F.3 Consolidated `GroupListOp` (elementary `SetFilling` gone)

The elementary op payload is now a whole sealed `GroupList`:
`GroupListOp::{Add(GroupList), Remove(id), Update(id, GroupList), AssignToSubject(period,
subject, Option<id>)}` (and the annotated twin `Add(id, GroupList)` for undo-of-Remove).
`apply_group_list` Update runs the merged colloscope-guard set driven by
`(old.is_prefilled(), new.is_prefilled())`. **`RemainingFilling` and
`NonEmptyGroupsWhenReducing` are deleted** — the row goes atomically, and the op carries a
complete consistent value, so the truncate/extend reshaping is gone. `GroupListPrecheckError`
shrank to `{InvalidGroupListId, GroupListIdAlreadyExists, InvalidSubjectId, InvalidPeriodId}`;
`GroupListError` dropped `RemainingFilling`, `NonEmptyGroupsWhenReducing`,
`PrefillGroupCountMismatch`.

**The high-level `ops/` API is frozen.** `GroupListsUpdateOp`, its error enums, and the
cleaning-op machinery are unchanged; the translators to low-level ops absorb the reshaping
(`AddNewGroupList` → `Add(new(params, default).expect(...))`; `UpdateGroupList` replicates
the grow/shrink pad-and-truncate then `Update(id, new(...).expect(...))`; the high-level
`SetFilling` survives, translated to a low-level `Update`). Same panic contracts as before.

### F.4 Mirror-consistency `LogicError`s (supersedes C.3's "Mirrors" bullet + §8's panic plan)

Because the old checker retires at step 5, the new checker now carries the full
ordering↔table mirror itself, for both sidecars (slots and weeks). Eight new `LogicError`
variants: `SlotOrderingUnknownId`, `SlotOrderingWrongSubject`, `SlotOrderingDuplicate`,
`OrphanSlot`, and the weeks twins `WeekOrderingUnknownId`, `WeekOrderingWrongPeriod`,
`WeekOrderingDuplicate`, `OrphanWeek`. The two empty-row loops in `logic_errors()` grew into
accumulating mirror sweeps: every ordered id must exist in the entity table, name the entity
that keys its row, appear exactly once, and every table entry must be covered. `to_legacy`
maps all four slots variants to `InvalidSlot` and all four weeks variants to `InvalidWeek`
(the old checker's first image); `is_necessarily_logic_error` is unchanged (those legacy
images are shared with the fixable dangles, so they are not "necessarily" logic errors).

**Row-key liveness is deliberately excluded.** A row keyed by a removed period/subject is the
op-reachable dangle (F.1's period removal; the analogous subject removal), reported per
entity as `DanglingFk(WeekPeriodFk)` / `SlotSubjectFk` and repaired by the cascade. A
short-circuiting `LogicError` there would block that repair, so it stays in the fixable
layer. Every desync the sweeps *do* catch is reachable only through the `#[cfg(test)]`
`forge_ordering_row` hatch — a code bug if it ever appears in production.

This is consistent with the `LogicError` definition, not a doctrine change: unreachable by
ops, decidable from the data, code-at-fault-if-present (`EmptyWeeksRow` was already the
precedent). The classification rustdoc widened to admit "consistency of an ordering sidecar
with its entity table" as a third decidable class, parallel to how `DuplicatedId` widened it.

### F.5 The `walk` vs `count_weeks` convention note

`Parameters::count_weeks` reads the week *table* while `walk`/`week_ids` are period-keyed
(they iterate the ordering). On a valid state the two agree; on a broken (dangling) state
they disagree — an orphan week is counted but never walked. Never mix the two conventions
off a validated state. (Motivating site: `constraints-colloscopes/src/helpers.rs` derives
indices from `walk_weeks().enumerate()` in one place and `count_weeks()` in another; safe
only because the solver sees validated states.) Documented on `count_weeks`.

### F.6 What step 5 inherits

- **Op surface** is smaller: no `PeriodStillHasWeeks`/`RemainingFilling`/
  `NonEmptyGroupsWhenReducing` in the force path, no elementary `SetFilling`; sealed
  `GroupList` carried whole by `GroupListOp`.
- **The new checker no longer leans on the old one** for mirror validation, so step 5 can
  delete the old checker without leaving any invariant behind. The differential fuzz stayed
  green (100 seeds) throughout — the new `LogicError`s are forge-only, outside its walk space.
- **Unchanged:** checked `apply_*` keeps all its guards until step 5 (D.4-F1 still holds —
  `NotEmptyPeriodInColloscope` stays vacuous-but-present); `PairingRule`/`SlotPairingRule`
  are not yet sealed (their two cross-field `LogicError`s remain for step 5).
