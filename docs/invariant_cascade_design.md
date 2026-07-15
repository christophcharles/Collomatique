# Invariant checking & cascade resolution — a design exploration

**Status:** exploratory / nothing is settled (July 2026, branch `consolidate_state`). This doc
records a design *direction* discussed after phase C of `table_registry_plan.md` shipped. It
reconsiders how `docs/state_consolidation_plan.md` **item 3** (invariant consolidation) and
**item 5** (params↔colloscope synchronization) should be tackled, and what that implies for the
reference registry built in phase C. It decides nothing; it frames choices and records leanings.

Read `docs/state_consolidation_plan.md` and `docs/table_registry_plan.md` first — this builds on
their inventory (28 ID-based relationships, the triplicated checks, the dense mirrors).

---

## 1. The problem this addresses

Every referential/consistency rule is currently expressed **three times**
(`table_registry_plan.md` §1): candidate validation before an op, delete-blocking scans in the
`Remove`/`Update` paths, and the whole-model `check_invariants`. They drift; the drift produced
real bugs (the whole `found_bugs.rs` family). The registry's item-3 plan was to *reroute all
three families through one declared registry*. This doc explores a different collapse:

> Make `check_invariants` the **single source of truth** for "is this state valid?", enforce it at
> every op by apply-then-check-then-maybe-rollback, and drive automatic repair from the *precise*
> errors it returns. The checks then exist **once**, and the same function serves every consumer:
> elementary ops, file load, the property tests, and the `ops/` auto-repair.

The goal is *consistent checks everywhere* from one definition, not a smaller number of checks.

## 2. `check_invariants` as the single source of truth

One function defines validity. It is already the trust-boundary check on file load, and the
property harness (`tests/property_ops*.rs`) already uses it as the oracle after every generated op
— so this direction *promotes the architecture the safety net already relies on* into production.

Two hard requirements follow:

- **It must be complete.** Today a stray precondition check might catch something the invariant
  misses; once validity is defined solely by `check_invariants`, that function becomes
  load-bearing. Gaps (e.g. the commented-out multi-colloscope block in `lib.rs:172`) silently stop
  being enforced. Completeness must be audited before the switch.
- **It must return precise, coordinate-bearing errors.** Today `check_subjects_data_consistency`
  returns a bare `InvariantError::InvalidSubject` — no id, no site. That is useless for repair and
  mediocre for users. The error must say *which entity references *which* dangling target at *which*
  site* (e.g. `TeacherReferencesDanglingSubject(TeacherId, SubjectId)`).

## 3. A detailed invariant enum + a resolution map

Enrich `InvariantError` into a precise enum (one variant per relationship/site, carrying
coordinates). Pair it with a **static map**: invariant variant → the elementary op that resolves it
(which may itself cascade). This map is the "declare each relationship's repair once" artifact — the
conceptual descendant of the registry, in a different shape.

Invariants split into **three kinds**, and the split dictates the machinery:

1. **Referential / resolvable** — a reference dangles (a teacher points at a removed subject). Has a
   map entry; the cascade resolves it.
2. **Structural / representational** — nonsense state a correct `apply` never produces (e.g.
   `WrongSubjectCountInAssignments`: a subject exists but has no assignments row). **No** map entry
   → **panic**. A broken structural invariant means the op is written wrong; surface it loudly.
3. **Path-convergence** — every reference exists, but two reference *paths* must **agree** (the group
   assigned in a colloscope cell must belong to the group list associated to that cell's
   period × the slot's subject). Not existence, not shape — *agreement*. Handled by the cascade like
   tier 1, but detected by a hand-written multi-hop check and usually resolved *lossily* (clear the
   now-invalid assignment). See §6 for why this tier is (mostly) irreducible.

## 4. Elementary ops: apply / check / restore

Each elementary op:

1. Snapshot (clone `InnerData` — trivial at this scale; or rely on the existing reverse `AnnotatedOp`).
2. Apply the mutation optimistically, including any deterministic structural fan-out.
3. Run `check_invariants`.
4. On failure, restore the snapshot and **route the precise error out** — which tells the caller
   exactly *what invariant would break* if the op were applied.

This deletes candidate validation and the precondition half of delete-blocking entirely: there is no
separate "can I?" check to drift from the invariant, because you *try and roll back*. The whole
"delete-blocking scan disagrees with the consistency check" bug class becomes unrepresentable.

Two things `check_invariants` **cannot** see, because they are properties of the *transition*, not
the *state* — these stay as a small, permanent precondition carve-out:

- **No-clobber**: inserting on an already-used id can land in a valid state yet destroy data and
  break reversibility. "Fresh id is fresh" must be checked explicitly.
- **Reversibility**: the emitted reverse op must actually reverse.

**Bonus:** because step 3 validates the *result*, it also validates the structural fan-out (e.g. the
colloscope maintenance) for free — a safety net over the most fragile, least-covered code today.

## 5. The cascade and the `ops/` dry-run

`apply_cascade(op)`:

1. Force-apply `op`.
2. Loop: run `check_invariants`; if it reports a resolvable/convergence break, look up the map,
   apply the resolving op, and record it; repeat until clean.
3. Return the exact list of ops it emitted (and a compound reverse op for the history stack).

**Termination.** Deletive resolutions strictly shrink the data → monotone termination. Additive
resolutions (filling a derived/dense set) fill a bounded set → terminate. The reference graph is
acyclic, which also bounds it. **Confluence** (does the emitted list / final state depend on the
order simultaneous breaks are resolved?) is the subtler property — distinct removals commute so the
final state is safe, but the *displayed op list* could reorder; pin it with a test.

**`ops/` use = dry-run.** Run `apply_cascade` on a throwaway session (or clone), read the emitted op
list, discard. Show the user the *actual* consequences ("this removal will also delete 3 slots and
clear 14 colloscope assignments") and let them accept or reject. This **retires the `Warning`
machinery**: hand-written consequence descriptions are replaced by computed ground truth that cannot
drift. `ON DELETE RESTRICT` vs `CASCADE` collapses into "the user declined vs accepted the preview,"
not a per-relationship policy declaration.

Rejected variant: allowing a transient *inconsistent* live state and converging later. The read
surface is laced with `.expect("… should be valid")` (slots/colloscope accessors); a knowingly-broken
live state is a panic minefield. Not worth it — keep every intermediate state valid (each resolving
op is itself a valid elementary op).

## 6. Making the data "reference-friendly" (possible changes)

The tier of §3 that a constraint lands in is a *consequence of the data model*, and reshaping moves
constraints between tiers. **The complexity is conserved; the art is putting each constraint in the
tier with the best tooling.** Four techniques:

**(a) Sparse-ify dense mirrors into references.** A dense mirror (one entry per period × non-excluded
subject) forces a tier-2 structural denseness invariant that some `apply` must hand-maintain. Making
it sparse turns "subject removed" from a wrong-count into a genuine dangling reference (tier 1).
*Assignments is the clean case:* storage already writes sparse rows and omits empty ones
(`encode/spec2.rs:288`); the dense in-memory mirror is reconstructed at decode
(`decode/spec2.rs:424-431`) purely to feed the representation. Sparse-ifying deletes that decode
densification and leaves the on-disk bytes identical. Storage gets *simpler*.

**(b) Promote positional / index data to entities.** Introduce `WeekId`, `GroupId`, etc., so
index-based checks become existence edges. A week pattern as a set of disabled `WeekId`s has no
length invariant; groups-as-ordered-entities kill the group-number bound. Caveat: this can *introduce*
new structure (a period's weeks must stay a contiguous sequence) or a convergence check (see the
`GroupId` case below) — i.e. it relocates rather than deletes.

**(c) Encapsulate so invalid states are unrepresentable.** Some checks leave `check_invariants`
entirely if the type can't represent the bad state: a `WeekPattern` behind a private field with
validating accessors carries no separately-wrong length. This is the "make illegal states
unrepresentable" technique — distinct from references, and the *cleanest* outcome where it applies
(the invariant doesn't move to another tier, it *disappears*).
**Note from the user:** Bad example. WeekPattern would just be a BTreeSet<WeekId>, no issues there
No this would be for periods. WeekId would need to be ordered *within* periods, just like slots are in
subjects right now. But all this can be put in the Periods type and guaranteed at this level.
It is *way more* local and easier to maintain.

**(d) Reference a pre-validated combination to dissolve a convergence check.** "A slot's teacher must
teach the slot's subject" is a tier-3 convergence today. If a slot references a single node that
already encodes a valid (teacher, subject) pairing — e.g. the slot points at the teacher and the
subject is *taken from* that reference — the convergence constraint vanishes by construction. (Cost:
the per-subject slot ordering becomes more convoluted to maintain. Not clear if it is worth it)

**The irreducible residue.** Techniques (a)–(d) can eliminate *existence*, *shape/length/count*, and
*some* convergence constraints. What resists is the genuine two-path-convergence core — most sharply
the colloscope group assignment: the assigned `GroupId`'s list must equal the list associated to
(the cell's period, the slot's subject). Introducing `GroupId` does not remove this; it *transmutes*
it from a contextual index-bound into a path-equality. Other members: paired-slots-share-a-subject,
assigned-student-present-for-period, association-subject-runs-on-period. These are exactly the
"side-constraints attached to references" the retired SQL schema could not express as plain FKs.

**This is fine.** Full folding is not the goal. A registry/checker that catches *most* invariants
generically, plus a handful of hand-written tier-3 checks, is already a large, clean win. The tier-3
checks are where **`Join` earns its keep**: they read as following the reference paths and comparing
them (`slot.join(params)` → teacher, subject in hand), which is *the* argument for keeping `Join`
beyond consumer ergonomics.

## 7. What survives of the phase-C registry

- **`Join` — keep.** It is the vocabulary for tier-3 convergence checks *and* consumer ergonomics.
- **`References` / `for_each_ref` — keep, recast.** Not for reverse-lookups, but as the generic
  tier-1 existence-sweep engine behind the precise checker, plus `all_ids`/duplicate detection.
- **`RefSite` + `references_to_*` (reverse lookup) — likely retire.** The precise-error cascade
  discovers referrers by tripping the checker, not by enumerating them. The `RefSite` *taxonomy*
  survives reincarnated as the precise-`InvariantError` variant set.
- **Item 3 shrinks dramatically.** You no longer reroute three check families through one registry;
  you *delete two of them* (candidate validation, precondition delete-blocking) and keep one enriched
  `check_invariants` + the map + the small no-clobber carve-out.

## 8. Hierarchy of changes

**Almost certainly worth doing**
- Enrich `InvariantError` → precise, coordinate-bearing variants. *Prerequisite for everything.*
- apply/check/restore at the elementary-op layer; retire candidate validation + precondition
  delete-blocking.
- Keep the reverse-op round-trip test as a universal safety net (regardless of the rest).
- Sparse-ify **assignments** (storage already sparse; deletes decode densification; byte-stable).
- Keep `Join`.

**Probably**
- The invariant→op map + `apply_cascade` + `ops/` dry-run; retire the `Warning` types.
- Recast `for_each_ref` as the checker's existence-sweep; retire `RefSite`/`references_to_*`.
- `WeekPattern` reshape/encapsulation (technique c/b) to lift the length check out of
  `check_invariants` (low risk — the slot machinery it mirrors already exists).

**Speculative**
- `GroupId` (surfaces the association-reinterpretation hazard the index model hides, but pays the
  3-hop convergence check).
- `WeekId` (relocates length → week-contiguity structure; do it only if it also simplifies the
  order-critical global-week walk in `constraints-colloscopes`).
- Slot → teacher-only / teaching-assignment reference to dissolve teacher-teaches-subject
  (technique d).
- Full **colloscope** sparse-ification (the real item-5 prize; biggest lift; needs format review).

**Open / hardest design work**
- Precise errors for *completeness/denseness* invariants: the checker must compute expected-vs-actual
  and report the *specific missing entry*, not a wrong count (needed for the additive cascade).
- A confluence test for the cascade.

## 9. Risks & open questions

- **Completeness becomes load-bearing** — `check_invariants` must be total before it is the sole
  authority (audit the multi-colloscope gap).
- **Dry-run cost** — a clone + full cascade + N checks per preview; fine interactively at this scale,
  watch for eager/live previews.
- **Confluence** — cascade output must not depend on internal iteration order.
- **Frozen on-disk format** — assignments reshape is byte-safe; week/group/colloscope reshapes need a
  format review before commitment.
- **Irreducible tier-3 residue** — accept it; ensure each residual constraint is *expressible*
  (via `Join`) and *resolvable* (via the cascade + preview), not eliminated.
