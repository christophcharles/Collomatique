# Invariant checking & cascade resolution — design + plan of action

**Status:** direction agreed July 15 2026 (branch `consolidate_state`); **step 1 completed
July 18 2026** — its detailed plan is retired (pinned at
`git show 62949404:docs/plans/plan_step_1.md`), the delivered state is recorded in
Appendix B. Next up: step 2.
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
                if any broken invariant has no map entry: PANIC   // structural tier
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
  detection.
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

**Step 2 — the precise checker, alongside the old one.** Write the *second*
`check_invariants` without removing the first: the enriched coordinate-bearing
`InvariantError` (§3), a generic dangling-reference sweep driven by `for_each_ref` (§7), the
duplicate-id check, plus the hand-written residue — the tier-3 convergence checks (§6) and any
structural leftovers not encapsulated per §6c. It returns **all** broken invariants in
canonical (`Ord`) order.

**Step 3 — completeness audit.** Survey the old `check_invariants` *and* the
`validate_*_internal` candidate checks (the side-constraint inventory of Appendix A.2 is the
checklist) for anything the new checker misses. (The stale `lib.rs:172` block was already
deleted in step 1, phase 1.) At the end of this step the new checker is ground truth.

**Step 4 — differential fuzz.** A way to build arbitrary (including invalid) `InnerData`:
random elementary ops applied through a `force_apply` door *without* checking, deliberately
landing in inconsistent states. Assert the two checkers agree on the **verdict** (old rejects
iff new reports non-empty) — not on variants: the old checker returns its first error in its
own order, the new vocabulary is richer. Encapsulated invariants (§6c) can't be broken this
way — by design, out of the fuzz's scope.

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
- the `Ord` used for the canonical cascade pick (derive order on the invariant enum is the
  natural candidate) — step 6.

---

## Appendix A — inventories inherited from the retired table-registry plan

Copied (July 16 2026) from §3.2/§3.3 of the retired `docs/table_registry_plan.md` (item 2's
detailed plan, delivered in full; the whole document is pinned at
`git show 77695338:docs/table_registry_plan.md`). Step 2 uses A.1 as the existence-sweep
target set; step 3 uses A.1 + A.2 as the completeness-audit checklist. File/line references
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
  empty; `Remove` requires week-empty (`PeriodStillHasWeeks`).
- **Colloscope ops are upserts**: `SetInterrogation(SlotId, WeekId, BTreeSet<u32>)` /
  `SetGroupList(GroupListId, BTreeMap<StudentId, u32>)`; empty payload = remove row;
  reverse = `Set…` with the prior payload (or empty).

### B.3 Read surface and oracles

- **`Periods`**: `walk()` is the canonical global order (`walk().enumerate()` = global week
  index); `weeks_of`, `weeks_vec_of`, `find_week` (owning period via `week.period_id`),
  `week_id_at`, `week_position`, `global_week_position`, `period_ids()`, `week_count_of`;
  `find_period(PeriodId) -> Option<&Vec<WeekId>>` is pub and pinned by the `read_api`
  pointer-identity test. `Lookup<WeekId> → Week`, `Lookup<PeriodId> → Vec<WeekId>`.
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
- **Refs registry**: gained `RefSite::WeekPeriodFk` (walked first in `walk_params_refs`);
  `RefSite::WeekPatternLengthCoupling` stays **period-keyed** with `non_trivial` = "the
  pattern excludes ≥1 of this period's weeks" (mirrors the transitive delete guard);
  remodelling it to a genuine week-ref is deferred to step 7.
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
