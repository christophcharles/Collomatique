# Invariant checking & cascade resolution — design + plan of action

**Status:** direction agreed, plan of action drafted (July 15 2026, branch `consolidate_state`).
This doc started as an exploration after phase C of `table_registry_plan.md` shipped; it now
records the agreed design *direction* and a step-by-step plan. Per the house rule of
`state_consolidation_plan.md` §6, **each step below still gets its own detailed session plan
(and user sign-off) before implementation** — this doc fixes the direction, the ordering, and
the decisions already taken, not the per-commit mechanics.

It supersedes how `docs/state_consolidation_plan.md` **item 3** (invariant consolidation) and
**item 5** (params↔colloscope synchronization) were going to be tackled, and reuses/retires
parts of the phase-C reference registry. §9 details the impact on the existing plans.

Read `docs/state_consolidation_plan.md` and `docs/table_registry_plan.md` first — this builds
on their inventory (28 ID-based relationships, the triplicated checks, the dense mirrors).

---

## 1. The problem this addresses

Every referential/consistency rule is currently expressed **three times**
(`table_registry_plan.md` §1): candidate validation before an op, delete-blocking scans in the
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
  Completeness is audited explicitly (plan step 3). Note: the commented-out multi-colloscope
  block in `lib.rs:172` is *stale dead code* from the multi-colloscope era, not a live gap —
  the live colloscope is checked at line 171; the audit resolves it (delete or revive).
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
- **Phases D/E of `table_registry_plan.md`** (consumer migration to the read API, `Deref`
  removal) — **orthogonal and unaffected**; they can proceed in parallel with any step here.

---

## 8. Plan of action (agreed July 15 2026)

Each step may span several commits/sessions and gets its own detailed plan first. Standing
gates throughout: the property harness (100 seeds; 500-seed reference at milestones), the
`found_bugs.rs` asserts, storage byte-stability + the `examples/` smoke test, and the three
contract scripts at milestones (run by the user).

**Step 1 — reshape: remove a maximum of dense copies.** Done *under the old architecture* (the
triplicated checks still exist), so each reshape hand-updates the old check families and the
`ops/` cleaning paths one last time — accepted cost, bounded, and the reshapes *delete* more
check code than they touch (the colloscope fan-out above all). Rationale for reshape-first: the
new checker of step 2 is then written **once against the final data model** — and the reshapes
remove precisely the dense/denseness invariants whose precise "expected-vs-actual, which entry
is missing" errors were this design's hardest open problem. The additive cascade dies here,
unwritten.

  Sub-steps, roughly by size (each with its own session plan):
  - **1a — assignments sparse** (§6a; smallest; deletes decode densification, byte-stable).
  - **1b — `WeekId`** (§6b; medium; op re-cut + `constraints-colloscopes` global-week walk;
    on-disk positional encoding kept, ids synthesized at decode).
  - **1c — slots: no reshape** (decision recorded in §6); optionally sparse-ify the `ordering`
    sidecar rows.
  - **1d — colloscope sparse** (§6a; the big one; wants 1b first so interrogations re-key by
    `WeekId`; dissolves the params→colloscope fan-out = item 5). Real consumer blast radius:
    gtk4/xlsx/python read the dense shapes; pyclass mirrors and the contract scripts are
    updated in the same change (house rule, `state_consolidation_plan.md` §7).

**Step 2 — the precise checker, alongside the old one.** Write the *second*
`check_invariants` without removing the first: the enriched coordinate-bearing
`InvariantError` (§3), a generic dangling-reference sweep driven by `for_each_ref` (§7), the
duplicate-id check, plus the hand-written residue — the tier-3 convergence checks (§6) and any
structural leftovers not encapsulated per §6c. It returns **all** broken invariants in
canonical (`Ord`) order.

**Step 3 — completeness audit.** Survey the old `check_invariants` *and* the
`validate_*_internal` candidate checks (the side-constraint inventory of
`table_registry_plan.md` §3.3 is the checklist) for anything the new checker misses; resolve
the stale `lib.rs:172` block. At the end of this step the new checker is ground truth.

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
  item-2 registry") and the **§6 hand-off notes of `table_registry_plan.md`** (transcribing
  per-op check orders, site→typed-error mappings for delete-blocking) are superseded with it —
  there is nothing left to reroute; two of the three families are deleted.
- **`state_consolidation_plan.md` §6 item 5 — dissolved** by step 1d + the cascade: with a
  sparse colloscope there is no fan-out left to centralize; cleanup is cascade resolution. The
  spec-2 format was shaped for exactly this re-keying and does not move.
- **`state_consolidation_plan.md` §6 item 4 (uniform op granularity) — independent, but
  interacts.** The resolution map wants elementary ops that can express "remove/clear this one
  reference" conveniently; step-1 reshapes re-cut the slot/week/colloscope op surfaces anyway.
  Granularity uniformization can ride along per step or stay a later pass.
- **`table_registry_plan.md` phase C artifacts** — `Join` kept; `References`/`for_each_ref`
  recast as the step-2 sweep engine; `RefSite`/`references_to_*` likely retired (§7, pending a
  gtk4 claim); `Lookup`/`resolve`/`all_ids` unaffected. Phases D/E proceed independently.
- **`ops/` — step 7 is the promised remaster.** Until then, decision 6 of the registry plan
  (touch `ops/` minimally) stands. Supporting evidence for computed-over-hand-written
  consequences: a suspected dormant drift bug in `general_planning.rs`
  (`UpdatePeriodWeekCount`'s colloscope-cleaning loop iterates
  `old_week_count..*week_count`, an empty range under its own guard — bounds swapped, so the
  cleaning op can never fire; the elementary layer's blocking then surfaces as a hard error
  instead of an auto-clean). To verify/fix independently; the class disappears at step 7.
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
- exact in-memory keying of the sparse colloscope (nested vs flat composite keys) — step 1d;
- whether the slots `ordering` sidecar goes sparse — step 1c;
- the `WeekDesc` container shape and the re-cut week op surface — step 1b;
- the `Ord` used for the canonical cascade pick (derive order on the invariant enum is the
  natural candidate) — step 6.
