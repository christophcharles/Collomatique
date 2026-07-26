# Step 5 — rewire production onto force_apply + the new checker (session plan)

**Status:** open (July 25 2026). This plan delivers step 5 of `docs/plans/invariant_cascade_design.md`: production switches from the checked `apply_*` + old checker to the apply/check/rollback primitive built on `force_apply` + `broken_invariants`, and the old world (checked `apply_*`, the old `Error` vocabulary, `InnerData::check_invariants`, `InvariantError`, the legacy bridge, the differential fuzz) is deleted. File/line references are against the tree at `cc2f13b4`.

---

## 0. Context and the one prerequisite

The migration problem is an error-type problem. The old API surfaces `collomatique_state_colloscopes::Error` (17 per-domain enums full of guard vocabulary). The new primitive surfaces something entirely different: a precheck error, or a set of broken invariants. Since `InMemoryData::Error` is a trait associated type, we cannot change it "gradually" in place — the moment it changes, every consumer changes with it.

The chosen strategy (settled in the pre-planning discussion) is therefore: **add a second associated error type and a second apply method to `InMemoryData`, add a parallel `Manager::try_apply`, migrate consumers one file at a time while both APIs coexist, then delete the old API, and finally rename the new names back to the old ones.** Nothing blocks this: there are exactly two `InMemoryData` implementors (`Data` in state-colloscopes, `FakeData` in `state/src/test_utils.rs`), and the error type escapes the `state` crate in exactly one place (`Manager::apply`'s return type).

The loose-ends phase already removed the two structural obstacles we identified:
- **Weeks** are de-fused from periods (`Periods` is existence-only, `Week.period_id` can dangle, `WeekPeriodFk` fires), so period removal no longer needs an empty-first guard — a bad removal lands, the checker reports the dangles, and the primitive rolls back.
- **Group lists** are sealed (`GroupList::new` validates; ops carry a whole sealed value), so the filling guards and their `LogicError` twins are already gone.

**Prerequisite (in progress in a separate session):** `PairingRule`/`SlotPairingRule` are being sealed the same way (the G1/G2 recipe, with a plain-fields DTO at the ops boundary so gtk4's dialog code only sees a type swap). That work lands before this plan starts. This plan is written against the sealed state: `LogicError::PairingRulePartsShareSubject` / `SlotPairingRulePartsShareSlot` no longer exist, and tier-2 `Logic` errors are reachable only from external data (`GlobalUpdate` payloads, decode) — never from ordinary elementary ops. Where a reference below points at code the sealing session touches (`ops/src/pairings.rs`, `ops/src/slot_pairings.rs`, the invariants fixtures), re-verify against the landed tree when executing.

---

## 1. Goal and shape

At the end of this step:

- `InMemoryData::apply` (the only apply left, after the rename) is: **snapshot → `force_apply` → `broken_invariants` → rollback on any breakage**. A failed op leaves the state bit-identical to before. A successful op is guaranteed to land in a fully valid state.
- The op-error vocabulary is exactly three-tiered:
  - `Precheck(PrecheckError)` — bad input (no-clobber, dangling op target, bad position/anchor); the state was never touched.
  - `Logic(BTreeSet<LogicError>)` — the op would create logically impossible rows; rolled back. This arm is required because `GlobalUpdate` carries external data (solver results, Python subprocess output) that can be arbitrarily broken. With the sealing prerequisite done, ordinary elementary ops can never produce it.
  - `Invariants(BTreeSet<FixableInvariant>)` — the op would leave dangling references or broken convergence facts; rolled back. At step 6 this becomes the cascade's raw material; at step 5 it is simply a precise error.
- The old checker (`InnerData::check_invariants`, `Parameters::check_invariants`, `InvariantError`, `InnerDataError`), the 16 checked `apply_*` bodies, the 17 old per-domain `*Error` enums, the `to_legacy` bridge, `assert_differential`, and the differential fuzz are all deleted.
- The storage decode path validates loaded files with `broken_invariants` (hard error on *any* non-clean result — a loaded file must be fully valid, because broken states never exist outside the primitive).
- `ops/`'s public `UpdateError` vocabulary is **frozen** (gtk4 and Python sit on it); only the internal `map_err` translation from the state error changes. The ops-layer pre-cleaning logic (`get_next_cleaning_op` etc.) is untouched — replacing it with the cascade is step 6's job, not ours.

Naming: during the migration the new members are `type ApplyError` / `fn try_apply` (trait), `Manager::try_apply`, and the enum `ApplyError`. After the old API is deleted, one mechanical commit renames them to `type Error` / `fn apply` / `Error`. Reason: the migration window needs distinct names, but the lasting API should not carry migration scars — §8 of the design doc says "apply *becomes* force_apply + new checker + rollback", and the final vocabulary should read that way. The rename is one purely textual commit at the very end, when every call site has already been visited once anyway.

---

## 2. The new API surface (commit 1)

### 2.1 The trait (`state/src/traits.rs`)

Today (traits.rs:22-67, abbreviated):

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

We add, alongside (not replacing) the existing members:

```rust
    /// Error type for when [Self::try_apply] fails.
    type ApplyError: std::error::Error + Send + Sync + Clone;

    /// Apply an operation through the apply/check/rollback gate and return its inverse.
    ///
    /// On failure the data is left strictly unchanged: precheck failures never touch
    /// it, and invariant/logic failures are rolled back from a snapshot.
    fn try_apply(
        &mut self,
    	op: &Self::AnnotatedOperation,
    ) -> std::result::Result<Self::AnnotatedOperation, Self::ApplyError>;
```

The bounds on `ApplyError` are copied from `Error` (`std::error::Error + Send + Sync + Clone`) because `Manager::try_apply` propagates it the same way `Manager::apply` propagates `Error` today. The method is *required* (no default body) — a default is impossible anyway, since the gate semantics live in the implementor's data.

### 2.2 The new error enum (`state-colloscopes/src/lib.rs`)

`PrecheckError` (lib.rs:288-331) already exists and is exactly the carve-out vocabulary (Appendix E.3); it is unchanged. The new top-level enum wraps it:

```rust
/// Error surface of the apply/check/rollback gate ([Data::try_apply]).
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ApplyError {
    /// Bad op input (no-clobber, dangling target, bad position/anchor).
    /// The state was never touched.
    #[error(transparent)]
    Precheck(#[from] PrecheckError),
    /// The op would land logically impossible rows; rolled back.
    /// Only reachable from data built outside this crate (GlobalUpdate
    /// payloads, decode): ordinary elementary ops cannot construct
    /// logically impossible rows since the value types were sealed.
    #[error("the operation would leave logically impossible data: {}", format_error_set(.0))]
    Logic(std::collections::BTreeSet<LogicError>),
    /// The op would break fixable invariants; rolled back. At step 6 this is
    /// what the cascade resolves; at step 5 it is simply an error.
    #[error("the operation would break data invariants: {}", format_error_set(.0))]
    Invariants(std::collections::BTreeSet<FixableInvariant>),
}

fn format_error_set<T: std::fmt::Display>(set: &std::collections::BTreeSet<T>) -> String {
    set.iter().map(T::to_string).collect::<Vec<_>>().join("; ")
}
```

The `Display` strategy matters because gtk4's two direct call sites surface errors with `e.to_string()` into a dialog (editor.rs:1152, run_python_script.rs:372). Itemizing the set through each entry's own `Display` gives those dialogs something meaningful without gtk4 learning the new vocabulary (that is step 7's job).

`BTreeSet` (not `Vec`) because `broken_invariants` already returns `BTreeSet`s and the canonical `Ord` is load-bearing for step 6's confluence argument — we pass it through untouched.

### 2.3 `Data::try_apply` — the gate itself

Implemented directly in the `impl InMemoryData for Data` block (matching where `apply` lives today, lib.rs:352-408):

```rust
    fn try_apply(&mut self, op: &AnnotatedOp) -> Result<AnnotatedOp, ApplyError> {
        // Snapshot everything an op could conceivably touch. force_apply never
        // uses the id issuer today (ids are issued in annotate), but the issuer
        // is a single counter — snapshotting it keeps rollback total even if a
        // later change to a force_apply_* copy starts touching it.
        let snapshot = self.inner_data.clone();
        let issuer_snapshot = self.id_issuer.lock().unwrap().clone();

        // Precheck failures return before any mutation (by construction of the
        // force_apply_* copies), so the state is untouched on this arm.
        let backward = self.force_apply(op)?;

        match self.inner_data.broken_invariants() {
            Err(logic) => {
                self.inner_data = snapshot;
                *self.id_issuer.lock().unwrap() = issuer_snapshot;
                Err(ApplyError::Logic(logic))
            }
            Ok(fixable) if !fixable.is_empty() => {
                self.inner_data = snapshot;
                *self.id_issuer.lock().unwrap() = issuer_snapshot;
                Err(ApplyError::Invariants(fixable))
            }
            Ok(_empty) => {
                // Data-level companion: the id-issuer high-water assert cannot
                // live in broken_invariants (the issuer is not part of InnerData).
                self.assert_id_issuer_high_water();
                Ok(backward)
            }
        }
    }
```

Four notes on this body:

- **The snapshot is a clone of `InnerData` plus a clone of the `IdIssuer`** — the design doc (§4) explicitly calls the data clone trivial at our scale, and it is the same primitive the step-6 cascade will use. The issuer clone is defensive, not needed today: `force_apply` never touches the issuer, but it costs one `u64` (`IdIssuer` is `#[derive(Debug, Clone)]`, ids.rs:134) and keeps rollback total against future changes. Note what it does *not* (and must not) undo: `annotate` runs before `try_apply` and its issued ids stay burned on failure, exactly as today — history ids are never reused.
- **`GlobalUpdate` needs no special casing.** `force_apply`'s GlobalUpdate arm is the force door (no pre-gate, infallible); the checker gate after it replaces the old `new_inner_data.check_invariants()?` pre-gate (lib.rs:401). A broken solver/Python payload now comes back as `Logic`/`Invariants` with the state rolled back — exactly the "we can always panic a level up, but the type carries it" behavior we agreed on.
- **`assert_id_issuer_high_water`** is the issuer half of today's `Data::check_invariants` (lib.rs:464-487), split into its own private method in this commit. The `inner_data.check_invariants()` half of that function (the old-checker panic net) survives until the removal commit, still called from the *old* `apply` — the two paths keep their own nets during coexistence.
- **The high-water check stays a panic, not an `ApplyError` arm** (settled July 25). Reasoning: `annotate` fuses id issuance — normal ops draw their fresh ids from this very issuer, and the `GlobalUpdate` annotate arm absorbs foreign payload ids up front via `id_issuer.skip_to_id(max_id + 1)` (ops.rs:820-827) — so through the `Manager` surface the check cannot fire. The only trigger is an `AnnotatedOp` transplanted from another `Data` instance through the raw `try_apply`, which is a caller bug: an error arm for it would mean "the program is wrong", not "the op is wrong", and every consumer would have to carry a dead `panic!("cannot happen")` match arm for it — the exact boilerplate this step deletes. The method's doc comment should cite the `skip_to_id` fusion as the reason the assert is unreachable in production.

### 2.4 `Manager::try_apply` (`state/src/traits.rs`)

A verbatim mirror of `Manager::apply` (traits.rs:100-122) with `try_apply`/`ApplyError` substituted:

```rust
    /// Apply an operation through the apply/check/rollback gate and keep the
    /// modification history consistent.
    fn try_apply(
        &mut self,
        op: <<Self as private::ManagerInternal>::Data as InMemoryData>::OriginalOperation,
        desc: <Self as private::ManagerInternal>::Desc,
    ) -> Result<
        <<Self as private::ManagerInternal>::Data as InMemoryData>::NewInfo,
        <<Self as private::ManagerInternal>::Data as InMemoryData>::ApplyError,
    > {
        let (annotated_op, new_info) = self.get_in_memory_data_mut().annotate(op);
        let backward = self.get_in_memory_data_mut().try_apply(&annotated_op)?;
        let rev_op = crate::history::ReversibleOp { forward: annotated_op, backward };
        let aggregated_op = crate::history::AggregatedOp::new(vec![rev_op]);
        self.get_modification_history_mut().store(aggregated_op, desc);
        Ok(new_info)
    }
```

History recording is identical because errors only exist on failure — a failed op stores nothing either way. The history module itself (`state/src/history.rs`) is error-type-agnostic and needs no change at all.

**The replay path stays on the old `apply` during the migration window.** `private::update_internal_state_with_aggregated` (traits.rs:233-280) — the undo/redo engine — keeps calling `apply` until the deactivation commit R1 switches it. See §3 for why this is safe.

### 2.5 `FakeData` (`state/src/test_utils.rs:47-75`) and the traits tests

`FakeData` gains `type ApplyError = FakeError;` and a `try_apply` with the same body as its `apply` (it has no invariants to check; reusing `FakeError` is deliberate — the trait does not require the two error types to differ). The traits test module (traits.rs:316-460) gains three twins of the existing tests:

- `try_apply_changes_data_and_stores_history` (twin of the test at :375);
- `try_apply_failing_leaves_state_untouched_and_stores_nothing` (twin of :388);
- one interleaving test: an op applied through `try_apply`, then `undo` + `redo` — pinning that history written by the new method replays correctly through the old-`apply` replay path. This is the coexistence contract in miniature.

---

## 3. Why coexistence is safe (the one-directional contract the canary enforces)

During the migration window, ops recorded by `Manager::try_apply` are replayed (undo/redo, `AppSession::cancel`). **The replay path is switched from old `apply` to `try_apply` up front, in commit 3.0, before any ops module migrates** — so history recorded by *either* API replays through the same gate that accepts it. This ordering is what makes the coexistence honest; the reasoning below is the *agreement* contract the canary pins, not a claim that the two apply paths are interchangeable in both directions.

The contract is **one-directional**:

1. **Everything old checked `apply` accepts, `try_apply` also accepts — with the same resulting state and the same computed reverse.** Step 3 certified the old checker as complete ground truth (ops ⊆ old, no gaps), so any op that survives every old guard lands a state that is clean under `broken_invariants`; and `try_apply` on the accepted path *is* `force_apply` plus that clean check. The step-4 differential fuzz's `ForceValid` pins already assert valid-op equivalence of state + reverse between `apply` and `force_apply` (tests/differential_force_apply.rs:160-174). This is the direction that matters for soundness: the `debug_assert_eq!` in replay (traits.rs:245-248: recomputed inverse must equal stored backward) rests on it.

2. **The reverse direction has one known, bounded exception.** An op that old `apply` *rejects* may still be *accepted* by `try_apply` — but only as a **perfect no-op**: the state is left bit-identical and the returned reverse is itself that same no-op. This is the step-4 divergence: old checked apply rejects a harmless clear (e.g. clearing a group-list association on a non-interrogation subject, `SubjectHasNoInterrogation`), which the gate accepts because clearing a non-existent association changes nothing. Because such a recorded op is a no-op in *both* replay directions, storing it is sound the moment replay runs through `try_apply` (commit 3.0) — and it was never sound through old `apply`, which is exactly why 3.0 comes first. Any *state-changing* `try_apply`-Ok against an old-`apply`-Err would be hidden work (a force copy silently repairing what a stripped guard policed) and is a hard failure.

The forbidden direction is **old `apply` Ok / `try_apply` Err** — a genuine new-checker-stricter regression. The canary turns any such case into a hard failure with a replayable seed, which is precisely the kind of finding we want surfaced before the old world is deleted, not after.

The **canary test** (§6) re-verifies this whole contract continuously, op-by-op, across the generated op space — including corruption ops that probe the stripped-guard territory — for the entire life of the migration.

---

## 4. Commit sequence overview

| # | Commit | Crates touched |
|---|--------|----------------|
| 1 | API commit: trait members, `ApplyError`, `Data::try_apply`, `Manager::try_apply`, `FakeData`, traits tests | state, state-colloscopes |
| 2 | Canary: twin-execution verdict-agreement fuzz | state-colloscopes (test) |
| 2.5 | Canary carve-out: relax to the one-directional contract (asymmetric perfect-no-op tolerance); plan §3/§6 rewrite | state-colloscopes (test), docs |
| 3.0 | Replay path onto `try_apply`: `update_internal_state_with_aggregated` (both call sites, return type → `ApplyError`) | state |
| 3.1–3.11 | ops migration, one module per commit: teachers, students, week_patterns, incompatibilities, pairings, slot_pairings, subjects, group_lists, assignments, colloscope, slots | ops |
| 3.12 | ops migration: the four `.expect`-only modules (general_planning, settings, balancing, export_config) | ops |
| 4 | gtk4 direct `GlobalUpdate` sites (2 sites, `e.to_string()` only) | gtk4 |
| 5.1–5.4 | state-colloscopes test migration, split per `plan_step_5_commit_5.md`: period_consistency (5.1), week_ops (5.2), found_bugs (5.3), new `property_try_apply.rs` (5.4). Tests only; the old invariant checks are untouched — old commit 5's `src/` fixture edits moved to R1.5 | state-colloscopes (tests) |
| 6 | Decode rewiring: `from_inner_data` onto `broken_invariants`, `FromInnerDataError` arms, `DecodeError` arms, spec2 edits, gtk4 file_loader arm | state-colloscopes, storage, gtk4, rpc-engine (Display only) |
| R1 | Deactivate: property harness oracles switch; canary + old-fuzz deleted; old API caller-free (replay path already moved in 3.0) | state, state-colloscopes, constraints-colloscopes |
| — | **Testing pause** (user-run: full suite once to scratchpad, 500-seed crank, smoke, contract scripts) | |
| R1.5 | Retire the old-checker test scaffolding (invariants fixtures, colloscopes.rs stage-6, lib.rs force_apply_tests) — deferred out of commit 5, see `plan_step_5_commit_5.md` §6 | state-colloscopes |
| R1.6 | Migrate the remaining scaffold `apply` callers onto `try_apply` (gap: the parent sweep tracked only the error vocabulary, not the happy-path scaffolds) | testgen-colloscopes, state-colloscopes (tests), ops (tests), storage (tests), state, constraints-colloscopes (tests) |
| R2 | Remove: checked `apply_*`, old `Error` + per-domain enums, old checkers, legacy bridge, old trait members | state, state-colloscopes |
| — | **Testing pause** | |
| R3 | Mechanical rename: `try_apply` → `apply`, `ApplyError` → `Error` everywhere | workspace |

Commits 3.x–6 can be reordered freely among themselves (they are independent per-file migrations); the order above goes simplest-first so the teachers commit establishes the template. Every commit must build green with the canary and the full harness passing. Per house rules, each renaming/removal is done with the Edit tool (no `sed` on source files), and the suite is run once per pause with output captured to a scratchpad file and grepped.

---

## 5. Commit 1 — the API commit

Everything in §2, in one commit: both crates must move together because `Data` and `FakeData` must implement the new members the moment the trait declares them. Deliverables checklist:

- `state/src/traits.rs`: `type ApplyError` + `fn try_apply` on `InMemoryData`; `Manager::try_apply` provided method; three new traits tests.
- `state/src/test_utils.rs`: `FakeData` impl extended.
- `state-colloscopes/src/lib.rs`: `ApplyError` enum + `format_error_set`; `try_apply` in the `InMemoryData` impl; `Data::check_invariants` split so `assert_id_issuer_high_water` exists as its own method (old `apply` keeps calling the combined net).
- New unit tests in `lib.rs`'s test module: `try_apply` rollback atomicity on an `Invariants` landing (state bit-identical to pre-op after the error), and `Err(Logic)` on a `GlobalUpdate` whose payload carries a duplicated id — the external-data route, which post-sealing is the only route into the `Logic` arm (the fixture from `force_global_update_drops_the_pre_gate`, lib.rs:663, already builds this payload).

---

## 6. Commit 2 — the canary

**File:** `state-colloscopes/tests/canary_try_apply.rs`. `testgen-colloscopes` is already a dev-dependency (the differential fuzz uses it).

The design doc's §8 asks for exactly this: "one commit runs old validation and the new gate side by side with the property harness asserting verdict agreement across generated valid+invalid ops". Shape (modeled on `tests/differential_force_apply.rs`'s walk; exact plumbing adapted at implementation):

- One seeded walk per `for_each_seed` iteration (`RunConfig { seeds: 100, ops_per_run: 1000, invalid_fraction: 0.15 }` — house scale), bootstrapped via `harness::bootstrap`.
- For **every** generated op (both `gen_op`'s valid/invalid walk ops and, every `PROBE_STRIDE` committed ops, a `gen_corruption_op` probe): annotate once, then run old `apply` and new `try_apply` on **twin clones of the same pre-state**, and assert the **one-directional agreement contract of §3** (see the carve-out note below):
  - on double-`Ok`: resulting `InnerData` equal and computed reverses equal (the replay-interchangeability contract of §3);
  - on double-`Err`: both states bit-identical to the pre-state (old-apply guard atomicity and new-gate rollback atomicity), and a new-side `Invariants`/`Logic` error carries a non-empty set;
  - on an **old-`Ok`/new-`Err`** split: a hard failure printing the op and both results (new-checker-stricter — never tolerated);
  - on an **old-`Err`/new-`Ok`** split: tolerated **only** as a perfect no-op — the new twin is bit-identical to the pre-state *and* the returned reverse is itself that same no-op (verified by re-applying it and checking it too changes nothing); any state-changing new-`Ok` against an old-`Err` is a hard failure (hidden work).
- The walk **commits through the old path** (still the authoritative one), so trajectories stay comparable with `property_ops.rs` seeds.
- The corruption-probe interleave is the important part: `gen_op`'s invalid arm produces op-shaped invalidity, while `gen_corruption_op`'s five kinds (`ForceRemove`/`ForceRetarget`/`ForceSemantic`/`ForceLogic`/`ForceValid`) target exactly the space the stripped guards used to police — where old-vs-new verdicts could plausibly split.

**The carve-out (commit 2.5).** Commit 2 lands the canary with strict `old Ok ⇔ new Ok` agreement, and it immediately catches the known step-4 divergence (a no-op clear the gate accepts but old `apply` rejects) — landing red on purpose. Commit 2.5 relaxes it to the asymmetric contract above: the old-`Err`/new-`Ok` perfect-no-op exception is tolerated, while the old-`Ok`/new-`Err` direction and every state-changing split stay fatal. This is a test + plan-only change (this §3/§6 rewrite is that plan half). The underlying replay soundness for those no-op ops is delivered by commit 3.0 (§6.5), which moves replay onto `try_apply` up front.

**Lifetime:** the canary lands right after commit 1, must stay green (with the 2.5 carve-out) through every 3.x/4/5/6 commit, gets one 500-seed crank at the first testing pause, and is **deleted in R1** — it structurally requires both APIs, and its job (verdict-equivalence evidence) is complete the moment the old API loses authority. Its module doc says this explicitly so nobody "ports" it later.

---

## 6.5 Commit 3.0 — replay onto `try_apply`

Moving the replay path onto the gate **before** any ops module migrates is what makes the §3 coexistence honest: history recorded by `try_apply` (including the tolerated perfect-no-op ops of §3) then replays through the same gate that accepts it, instead of through old `apply`, which would reject those no-ops.

`private::update_internal_state_with_aggregated` (state/src/traits.rs:233-280) is the undo/redo/`cancel` replay engine. It applies each stored `ReversibleOp` — the forward op on redo, the backward op on undo — through `self.get_in_memory_data_mut().apply(...)` at both call sites (:243 forward, :265 backward), and its `debug_assert_eq!` recomputes the inverse and pins it against the stored backward. Commit 3.0:

- switches both `apply(...)` call sites to `try_apply(...)`;
- changes the function's return type from the `Error`-typed result to the `ApplyError`-typed one (it is `pub(crate)`-private and its callers panic on `Err`, so nothing outside moves);
- the `debug_assert_eq!` inverse pin is unchanged — on the accepted path `try_apply`'s reverse equals `force_apply`'s, which the step-4 pins already tie to the checked reverse.

The old `apply` remains for the not-yet-migrated ops modules; only the *replay* of already-recorded ops moves. This is the single item peeled out of R1's original "replay path switches to `try_apply`" bullet (§11), landed early instead.

---

## 7. Commits 3.1–3.12 — the ops migration

### 7.1 The translation doctrine

Every ops module today calls `Manager::apply` and translates failures with a `map_err` that matches old `Error` variants into its own frozen per-op `UpdateError` variants, panicking on anything unexpected (two syntactic shapes exist: `if let Error::X(inner) = e { match inner { … } } else { panic!() }` and `match e { Error::X(inner) => match inner { … }, _ => panic!() }` — both get rewritten to one shape). Under the new API the same information arrives differently, and the translation follows three rules:

1. **Carve-out guards arrive as `ApplyError::Precheck(PrecheckError::Domain(...))`** — a direct variant-for-variant rewrite (e.g. `TeacherError::InvalidTeacherId` → `TeacherPrecheckError::InvalidTeacherId`).
2. **Stripped guards arrive as `ApplyError::Invariants(set)`.** (`Logic` never reaches an ops `map_err`: ops issues no `GlobalUpdate`, and post-sealing nothing else can produce it — so `Logic` sits in every catch-all panic arm.) The key soundness fact: **the pre-op state is always valid** (it passed the same gate), so *every* entry in the set was caused by the op at hand — the ops layer may attribute set entries to the op and synthesize its typed error from set membership, without cross-checking ids. Example: a teacher→subject dangle in the set after `AddNewTeacher` can only be this add's bad subject id.
3. **Where one op can trip several stripped guards at once, synthesize in the old validator's precedence order** — explicit priority passes over the set, not "first entry in canonical order" — because `Reference`'s derive order (Period < Week < Subject < …) does not match the old first-error-wins order, and the frozen `UpdateError` consumers may care which variant they see. Where the new set entry carries less payload than the old error did (e.g. `Convergence::SlotTeacherDoesNotTeachSubject(slot)` vs the old `(teacher, subject)` payload), the missing ids are synthesized **from the op payload in scope**, which is where they came from anyway.

The `panic!("… should have been cleaned …")` arms (the ops-layer cleaning contract) stay panics: they become panic-on-`Invariants` arms whose message prints the set — the cleaning phases that keep them unreachable are untouched at step 5.

The four `.expect(...)`-only modules (general_planning — 27 expects, settings, balancing, export_config) migrate by re-pointing the same `.expect` at `try_apply`'s `Result`; their cleaning phases keep those expects honest exactly as today, and the canary proves acceptance behavior is unchanged.

### 7.2 Representative rewrites

**Old code** (teachers.rs:148-162, the archetype):

```rust
.map_err(|e| {
    if let collomatique_state_colloscopes::Error::Teacher(te) = e {
        match te {
            collomatique_state_colloscopes::TeacherError::InvalidSubjectId(subject_id) =>
                AddNewTeacherError::InvalidSubjectId(subject_id),
            _ => panic!("Unexpected teacher error during AddNewTeacher: {:?}", te),
        }
    } else {
        panic!("Unexpected error during AddNewTeacher: {:?}", e);
    }
})?;
```

**New code** — the old `InvalidSubjectId` was a *stripped* guard (checked apply scanned the subject list); under the new API the bad add lands, the checker reports the dangle, and the gate rolls back:

```rust
.map_err(|e| {
    use collomatique_state_colloscopes::{ApplyError, FixableInvariant, Reference, SubjectRefSite};
    match e {
        // The pre-op state was valid, so any teacher->subject dangle in the set
        // was introduced by this Add; the dangling target is the bad input id.
        ApplyError::Invariants(set) => {
            for inv in &set {
                if let FixableInvariant::DanglingFk(Reference::Subject {
                    target,
                    site: SubjectRefSite::TeacherSubjects(_),
                }) = inv
                {
                    return AddNewTeacherError::InvalidSubjectId(*target);
                }
            }
            panic!("Unexpected invariant breaks during AddNewTeacher: {set:?}");
        }
        _ => panic!("Unexpected error during AddNewTeacher: {e:?}"),
    }
})?;
```

**Mixed precheck + convergence, exhaustive** (assignments.rs `Assign`, :132):

```rust
.map_err(|e| {
    use collomatique_state_colloscopes::{
        ApplyError, AssignmentPrecheckError, Convergence, FixableInvariant, PrecheckError,
    };
    match e {
        ApplyError::Precheck(PrecheckError::Assignment(pe)) => match pe {
            AssignmentPrecheckError::InvalidPeriodId(id) => AssignError::InvalidPeriodId(id),
            AssignmentPrecheckError::InvalidStudentId(id) => AssignError::InvalidStudentId(id),
            AssignmentPrecheckError::InvalidSubjectId(id) => AssignError::InvalidSubjectId(id),
        },
        ApplyError::Invariants(set) => {
            // Old validator order: subject-not-running before student-not-present.
            for inv in &set {
                if let FixableInvariant::Convergence(
                    Convergence::AssignmentForSubjectNotRunningOnPeriod(period, subject),
                ) = inv
                {
                    return AssignError::SubjectDoesNotRunOnPeriod(*subject, *period);
                }
            }
            for inv in &set {
                if let FixableInvariant::Convergence(
                    Convergence::AssignedStudentNotPresentForPeriod { period, student, .. },
                ) = inv
                {
                    return AssignError::StudentIsNotPresentOnPeriod(*student, *period);
                }
            }
            panic!("Unexpected invariant breaks during Assign: {set:?}");
        }
        _ => panic!("Unexpected error during Assign: {e:?}"),
    }
})?;
```

**Several stripped guards trippable by one op** (slot_pairings.rs `AddNewSlotPairingRule`, :106 — the richest case: two kinds of `DanglingFk` plus a `Convergence`; `rule` is the op payload in scope — field access adjusts to whatever DTO the sealing pre-step gives this module):

```rust
.map_err(|e| {
    use collomatique_state_colloscopes::{
        ApplyError, Convergence, FixableInvariant, PeriodRefSite, Reference, SlotRefSite,
    };
    match e {
        ApplyError::Invariants(set) => {
            // Old precedence: slot dangles, then same-subject, then period dangles.
            for inv in &set {
                if let FixableInvariant::DanglingFk(Reference::Slot {
                    target,
                    site: SlotRefSite::SlotPairingRuleAntecedent(_)
                        | SlotRefSite::SlotPairingRuleConsequent(_),
                }) = inv
                {
                    return AddNewSlotPairingRuleError::InvalidSlotId(*target);
                }
            }
            for inv in &set {
                if let FixableInvariant::Convergence(Convergence::PairedSlotsNotInSameSubject(_)) = inv {
                    return AddNewSlotPairingRuleError::SlotsNotInSameSubject(
                        rule.antecedent.slot_id,
                        rule.consequent.slot_id,
                    );
                }
            }
            for inv in &set {
                if let FixableInvariant::DanglingFk(Reference::Period {
                    target,
                    site: PeriodRefSite::SlotPairingRuleExcludedPeriods(_),
                }) = inv
                {
                    return AddNewSlotPairingRuleError::InvalidPeriodId(*target);
                }
            }
            panic!("Unexpected invariant breaks during AddNewSlotPairingRule: {set:?}");
        }
        _ => panic!("Unexpected error during AddNewSlotPairingRule: {e:?}"),
    }
})?;
```

### 7.3 Per-module mapping tables

Classes: **(a)** carve-out → `Precheck`; **(b)** stripped guard → `Invariants` (the listed entry is what the set contains for that op on an otherwise-clean state); **(c)** cleaning-contract panic arm → panic-on-`Invariants` (set content listed so the panic message is meaningful). "Catch-all" arms stay panics throughout. Variants not listed for a module were already unreachable-by-construction there. The parts-share variants (`SameSubjectInBothParts`/`SameSlotInBothParts`) appear in no table: the sealing pre-step rejects them at construction time inside ops, before any apply call.

**teachers** (3.1):

| op | old matched variant | class | new match target |
|---|---|---|---|
| AddNewTeacher :148 | `InvalidSubjectId` | b | `DanglingFk(Subject@TeacherSubjects)` → same `UpdateError` variant |
| UpdateTeacher :179 | `InvalidTeacherId` | a | `TeacherPrecheckError::InvalidTeacherId` |
| | `InvalidSubjectId` | b | as Add |
| | `TeacherStillHasAssociatedSlotsInSubject` (panic) | c | panic on `Convergence(SlotTeacherDoesNotTeachSubject)` |
| DeleteTeacher :213 | `InvalidTeacherId` | a | precheck |
| | `TeacherStillHasAssociatedSlots` (panic) | c | panic on `DanglingFk(Teacher@SlotTeacher)` |

**students** (3.2):

| op | old matched variant | class | new match target |
|---|---|---|---|
| AddNewStudent :397 | `InvalidPeriodId` | b | `DanglingFk(Period@StudentExcludedPeriods)` |
| UpdateStudent :428 | `InvalidStudentId` | a | precheck |
| | `InvalidPeriodId` | b | as Add |
| | `StudentStillHasNonTrivialAssignments` (panic) | c | panic on `Convergence(AssignedStudentNotPresentForPeriod)` |
| DeleteStudent :462 | `InvalidStudentId` | a | precheck |
| | 3 × `StudentIsStill…` (panics) | c | panics on `DanglingFk(Student@GroupListExcludedStudent / GroupListPrefilledStudent / AssignmentsStudent)` |

**week_patterns** (3.3):

| op | old matched variant | class | new match target |
|---|---|---|---|
| AddNewWeekPattern :283 | `WeekPatternExcludesInvalidWeek` | b | `DanglingFk(Week@WeekPatternExcludedWeek)` |
| UpdateWeekPattern :315 | `InvalidWeekPatternId` | a | precheck; `…ExcludesInvalidWeek` as Add |
| DeleteWeekPattern :345 | `InvalidWeekPatternId` | a | precheck |
| | `…StillHasAssociatedIncompat/Slots` (panics) | c | panics on `DanglingFk(WeekPattern@IncompatWeekPattern / SlotWeekPattern)` |

**incompatibilities** (3.4): AddNewIncompat :92 `InvalidSubjectId`/`InvalidWeekPatternId` (b → `DanglingFk(Subject@IncompatSubject)` / `DanglingFk(WeekPattern@IncompatWeekPattern)`); Update :116 / Delete :140 `InvalidIncompatId` (a).

**pairings** (3.5): AddNewPairingRule :95 — `InvalidSubjectId` (b, `DanglingFk(Subject@PairingRuleAntecedent|Consequent)`), `InvalidPeriodId` (b, `DanglingFk(Period@PairingRuleExcludedPeriods)`); Delete :130 / Update :161 `InvalidPairingRuleId` (a), Update's rest as Add. Line anchors to re-verify after the sealing session lands.

**slot_pairings** (3.6): as the representative rewrite above; Delete :147 / Update :181 `InvalidSlotPairingRuleId` (a).

**subjects** (3.7): the only map_err is DeleteSubject :773 `InvalidSubjectId` (a). All Remove scans are catch-all (cleaned first) → panic-on-`Invariants`. Add/Update/Move/UpdatePeriodStatus keep their `.expect`s.

**group_lists** (3.8): the only map_err is DeleteGroupList :944 `RemainingAssociatedSubjects` (c → panic on `DanglingFk(GroupList@AssociationEntry)`); everything else pre-checks in the ops layer or `.expect`s. The frozen `GroupListsUpdateError` vocabulary is untouched.

**assignments** (3.9): as the representative rewrite above. The old guard order (subject-not-running before student-not-present, state-colloscopes assignments.rs:107-145) is preserved by the priority passes.

**colloscope** (3.10):

| op | old matched variant | class | new match target |
|---|---|---|---|
| UpdateColloscopeGroupList :112 | `InvalidGroupListId` | a | `ColloscopePrecheckError::InvalidGroupListId` |
| | `ExcludedStudentInGroupList` | b | `Convergence(ColloscopeStudentExcluded)` |
| | `InvalidStudentId` | b | `DanglingFk(Student@ColloscopeGroupListStudent)` |
| | `InvalidGroupNumForStudentInGroupList` | b | `Convergence(ColloscopeStudentGroupOutOfBounds)` |
| UpdateColloscopeInterrogation :153 | `InvalidWeekId`/`InvalidSlotId` | a | precheck |
| | `SlotNotRunningOnPeriod` | b | `Convergence(InterrogationSlotNotRunningOnPeriod)` |
| | `InterrogationOnInactiveWeek` | b | `Convergence(InterrogationOnInactiveWeek)` |
| | `InvalidGroupNumInInterrogation` | b | `Convergence(InterrogationGroupOutOfBounds)` |

**slots** (3.11 — the richest module, done last with the template mature):

| op | old matched variant | class | new match target |
|---|---|---|---|
| AddNewSlot :317 | `InvalidSubjectId`, `SubjectHasNoInterrogation` (panics; ops pre-checks :278/:285) | c | panics |
| | `InvalidTeacherId` | b | `DanglingFk(Teacher@SlotTeacher)` |
| | `InvalidWeekPatternId` | b | `DanglingFk(WeekPattern@SlotWeekPattern)` |
| | `TeacherDoesNotTeachInSubject` | b | `Convergence(SlotTeacherDoesNotTeachSubject)` — payload from op |
| | `SlotOverlapsWithNextDay` | b | `Convergence(SlotOverflowsDay)` |
| UpdateSlot :360 | `InvalidSlotId` | a | precheck; rest as AddNewSlot (incl. `SubjectHasNoInterrogation` → `Convergence(SlotForSubjectWithoutInterrogations)`) |
| DeleteSlot :388 | `InvalidSlotId` | a | precheck |

Slot's `CannotChangeSubject` and `PreviousSlotIsNotInRightSubject` are kept carve-outs — they arrive as `Precheck(Slot(...))` and currently sit in catch-all arms; they stay panics in ops (ops never sends them).

Each 3.x commit also runs that module's own tests plus the canary before landing. The tables above are the design; at implementation each map_err is rewritten against the actual arm list in the file (the tables were built by reading them, but the file is the authority — house rule: never argue from a paraphrase).

---

## 8. Commit 4 — the gtk4 direct sites

Two sites, both `Op::GlobalUpdate` with `e.to_string()`-only error handling, so the migration is a one-line method swap each:

- `gtk4/src/editor.rs:1139` — `match Manager::apply(&mut self.data, op, desc)` → `Manager::try_apply(...)`. A logic-broken solver payload now produces a rolled-back state and a dialog itemizing the breakage instead of the old `InnerDataError` Display.
- `gtk4/src/editor/run_python_script.rs:368` — `Manager::apply(app_session, op, desc)` → `try_apply`; the error keeps flowing to Python as `ResultMsg::GlobalError(e.to_string())`.

---

## 9. Commit 5 — state-colloscopes test migration

> **REVISED (Jul 25 2026).** Commit 5 is split into four **test-only** commits (5.1–5.4), detailed with full old+new snippets in `plan_step_5_commit_5.md` — implement from that document, not from this section. Ruling: **the old invariant checks are untouched at this point** — tests switch to `try_apply` only where they drive the public op surface, never where the two worlds are compared. Accordingly, the "In-crate fixtures" subsection below (`invariants.rs`, `colloscopes.rs`, `lib.rs` force_apply_tests) is deferred to the new commit R1.5 (§11). The expected outcomes worked out below remain valid as the reference they always were.

All scenario builds are untouched; only apply calls switch to `try_apply` and assert tails change to the new vocabulary. Exact expected outcomes (worked out against the checker):

### found_bugs.rs (5 asserts)
- **:80** remove-student-with-settings, was `Err(Error::Student(_))` → `Err(ApplyError::Invariants({DanglingFk(Student { target: student_id, site: SettingsStudentKey })}))` — assert exact single-entry set (a strict upgrade over the old wildcard).
- **:159** automatic→automatic group-list update excluding a placed student, was `NotCompatibleGroupListInColloscope` → `Err(Invariants({Convergence(ColloscopeStudentExcluded(group_list_id, student))}))`.
- **:327** shrink `group_names` under a placed group, was `InvalidGroupInSubjectSlotInColloscope` → `Err(Invariants({Convergence(InterrogationGroupOutOfBounds(slot_id, week0))}))`.
- **:475** `AssignToSubject` with dangling list id — this guard is a *kept* carve-out → `Err(ApplyError::Precheck(PrecheckError::GroupList(GroupListPrecheckError::InvalidGroupListId(id))))`.
- **:572** `CannotChangeSubject` — kept carve-out → `Err(Precheck(Slot(SlotPrecheckError::CannotChangeSubject(...))))`.

### week_ops.rs (3 asserts)
- **:161** remove a pattern-excluded week, was `NonTrivialWeekPattern` → `Err(Invariants({DanglingFk(Week { target: weeks[1], site: WeekPatternExcludedWeek(pattern_id) })}))`. The test currently discards the pattern id at :144 and must start capturing it.
- **:301** deactivate a week under a filled cell → `Err(Invariants({Convergence(InterrogationOnInactiveWeek(slot, weeks[0]))}))`.
- **:530** move a filled week to a subject-excluded period → **two** convergences fire: `{Convergence(InterrogationSlotNotRunningOnPeriod(slot, week)), Convergence(InterrogationGroupOutOfBounds(slot, week))}` (the destination has no association, so the group bound saturates to 0 — the F5/D.4 rule). Assert the exact two-entry set; this doubles as a pin on the bound-0 behavior.

### period_consistency_in_subjects.rs (:79-88)
`PeriodOp::Remove` on a subject-excluded period, was `PeriodIsReferencedBySubject` → `Err(Invariants({DanglingFk(Period { target: id2, site: SubjectExcludedPeriods(subject_id) })}))`.

### differential_force_apply.rs → rewritten as `tests/property_try_apply.rs`
The old-vs-new differential dies with the old checker, but this file's fuzz skeleton is the only randomized coverage of the exact primitive step 5 ships — so we **rewrite rather than delete**. Same walk shape and config; probes every `PROBE_STRIDE` via `gen_corruption_op`; the probe assertions become properties of `try_apply` alone:

- `Err(Precheck(_))` → state untouched;
- `Err(Logic(_) | Invariants(_))` → state **bit-identical** to the pre-op snapshot (rollback atomicity) and the set non-empty;
- `Ok(reverse)` → `broken_invariants() == Ok(∅)` (the gate is honest) and applying `reverse` restores the snapshot exactly (the clean-landing reverse pin, carried over from step 4);
- the `ForceValid` arm keeps the step-4 carve-out rule transposed: land `Ok` with a state change, or `Ok` as a perfect no-op, or `Err` with state untouched;
- honesty counters re-keyed: every `CorruptionKind` attempted, each corrupting kind produced ≥1 `Err`, `ForceLogic` produced ≥1 `Err(Logic)`. (Post-sealing, `ForceLogic`'s surviving recipes are external-data-shaped — duplicated-id `GlobalUpdate` and friends; the sealing session keeps the fuzz green, but verify at least one `Logic`-producing recipe survives when rebasing on it.)

This rewrite lands **in commit 5** (it only needs the new API) while the old differential file survives until R1, so fuzz coverage never gaps.

### In-crate fixtures
- `invariants.rs` :944-954 wrapper: delete only the `assert_differential(data);` line — every fixture's exact-set `broken_invariants` assert survives verbatim. Delete the handful of explicit `data.check_invariants() == Err(...)` asserts inside compound fixtures (:3455-3458, :3491-3494) and the two legacy-bridge unit tests (:3506-3639) whole.
- `colloscopes.rs` :571-610: the three stage-6 corruption tests become new-checker asserts on the same forged states (`Err({EmptyInterrogationRow})`, `Err({EmptyColloscopeGroupListRow})`, and the forged-row test asserts the exact two-entry `DanglingFk` set); drop the `assert_differential` calls.
- `lib.rs` force_apply_tests (:538-731): drop the old-checker asserts and `assert_differential` calls; the two `forced_valid_*_equals_checked_apply` anti-drift pins retarget as `try_apply` happy-path pins (valid op through `try_apply` equals `force_apply`-on-a-twin in state and reverse) — after R3 they read naturally as `apply` happy-path tests.

---

## 10. Commit 6 — decode and `from_inner_data`

**Design choice: validate once, at the end, on the full `InnerData` (option a).** The alternative — keeping a mid-decode params-only validation — is rejected because `broken_invariants` deliberately lives on `InnerData`, and the only ways to run it mid-decode are wrapping params in a throwaway `InnerData` (validating a state that is not the file's) or keeping the params-side old checker alive solely for decode (defeating the deletion).

Safety was verified by reading the reconstruction code: `reconstruct_colloscope` (spec2.rs:813-889) touches params only through total functions (`walk_weeks`, `find_slot`, `is_interrogation_possible` — a chain of `let Some(..) else { return false }`), and the decoder builds the ordering↔table mirrors consistent by construction. **No check needs hoisting.** The only observable change is diagnostic ordering on multiply-corrupt files (a params dangle may now be reported after a colloscope-cell error); the acceptance domain is unchanged in both directions, and no storage test asserts an `InnerDataError`-shaped failure.

Mechanics:

1. **`Data::from_inner_data`** (lib.rs:508-521). Old:

```rust
pub fn from_inner_data(inner_data: InnerData) -> Result<Data, FromInnerDataError> {
    inner_data.check_invariants()?;
    let id_issuer = IdIssuer::new(inner_data.ids())?;
    let data = Data { id_issuer: std::sync::Mutex::new(id_issuer), inner_data };
    data.check_invariants();
    Ok(data)
}
```

New — hard error on `Err(LogicError)` **and** on any non-empty fixable set (a loaded file must be fully valid; broken states never exist outside the primitive):

```rust
pub fn from_inner_data(inner_data: InnerData) -> Result<Data, FromInnerDataError> {
    match inner_data.broken_invariants() {
        Err(logic) => return Err(FromInnerDataError::Logic(logic)),
        Ok(set) if !set.is_empty() => return Err(FromInnerDataError::BrokenInvariants(set)),
        Ok(_) => {}
    }
    let id_issuer = IdIssuer::new(inner_data.ids())?;
    let data = Data { id_issuer: std::sync::Mutex::new(id_issuer), inner_data };
    data.assert_id_issuer_high_water();
    Ok(data)
}
```

with `FromInnerDataError` (lib.rs:335-340) losing its `InnerDataError` arm and gaining `Logic(BTreeSet<LogicError>)` / `BrokenInvariants(BTreeSet<FixableInvariant>)` arms (Display itemized via `format_error_set`). `rpc-engine/src/lib.rs:74` only `.to_string()`s this error — no change needed there.

2. **`storage/src/decode/spec2.rs`**: delete the mid-decode gate `params.check_invariants().map_err(InnerDataError::from)?` (:327, with its comment block). The fabricated error at :517 is **replaced, not deleted**: the check at :512-519 must stay, because an *empty* assignments row keyed by an unknown period is dropped by the canonical-absent rule and would otherwise vanish silently — the final gate cannot see it. It becomes a decoder-owned variant: `return Err(DecodeError::UnknownPeriodInAssignments(row.period_id).into())` (better diagnostics too: the raw id travels).

3. **`storage/src/decode.rs`**: `DecodeError::InnerDataError` (:64) splits into `LogicError(BTreeSet<LogicError>)`, `BrokenInvariants(BTreeSet<FixableInvariant>)`, `UnknownPeriodInAssignments(u64)`; the `From<FromInnerDataError>` impl (:67-76) maps the new arms across. Both set types derive `Eq`/`Ord`, so `DecodeError`'s derives survive.

4. **`gtk4/src/loading/file_loader.rs:158`**: the one arm rendering `DecodeError::InnerDataError` splits into three, keeping the French envelope wording and formatting the sets through their Display.

---

## 11. The removal commits

### R1 — deactivate (old API loses its last callers; code still present)

1. (Replay path already moved to `try_apply` in commit 3.0 — see §6.5 — so R1 no longer touches `update_internal_state_with_aggregated`.)
2. `state-colloscopes/tests/property_ops.rs`: the walk switches to `Manager::try_apply` and the oracle from `check_invariants()` to `broken_invariants() == Ok(∅)`. This makes `tests/property_ops_broken_invariants.rs` redundant (the two harnesses become identical) — delete it here, noting the merge in the commit message.
3. `constraints-colloscopes/tests/property_build.rs:146`: same oracle swap.
4. Delete the **canary** (`tests/canary_try_apply.rs`) and the old differential fuzz (`tests/differential_force_apply.rs`) — `property_try_apply.rs` (commit 5) already carries the fuzz coverage forward.

After R1 the old API is intact but caller-free (pub items and trait methods don't warn as dead code, so every intermediate state compiles cleanly).

**→ First testing pause.** User-run: full workspace suite once (captured to a scratchpad file, then grepped), a 500-seed crank of `property_try_apply` + `property_ops`, the examples smoke tests, and a first pass of the contract scripts.

### R1.5 — retire the old-checker test scaffolding

Deferred out of commit 5 (ruling of Jul 25 2026: the test-migration commits do not touch the old invariant checks). One commit, after the first testing pause and before R2, so R2 stays a mechanical deletion:

1. `invariants.rs`: the fixture wrapper drops its `assert_differential(data)` call (every fixture's exact-set `broken_invariants` assert survives verbatim); `assert_dangling_maps` loses its old-checker `InnerDataError` argument and assert (callers drop the argument); the explicit `data.check_invariants() == Err(...)` asserts inside compound fixtures (:3455-3458, :3491-3494) and the two legacy-bridge unit tests (:3506-3639) are deleted whole.
2. `colloscopes.rs` :571-610: the three stage-6 corruption tests become new-checker asserts on the same forged states (`Err({EmptyInterrogationRow})`, `Err({EmptyColloscopeGroupListRow})`, and the forged-row test asserts the exact two-entry `DanglingFk` set); the `assert_differential` calls are dropped.
3. `lib.rs` force_apply_tests (:538-731): drop the old-checker asserts and `assert_differential` calls; the two `forced_valid_*_equals_checked_apply` anti-drift pins retarget as `try_apply` happy-path pins (valid op through `try_apply` equals `force_apply`-on-a-twin in state and reverse) — after R3 they read naturally as `apply` happy-path tests.

R1.5 got its own detailed snippet-level pass in `plan_step_5_r1_5.md` (against tree `cb13427d`); landed as `6c33375d`.

### R1.6 — migrate the remaining scaffold `apply` callers

A gap discovered while planning R1.5 (detailed in `plan_step_5_r1_5.md` §6): R2 deletes `Manager::apply` and `InMemoryData::apply` themselves, but the parent-plan sweep only tracked the dying *error vocabulary*, so it missed the happy-path scaffolds that call old `apply` purely to build state (`Ok` expected, panic otherwise). These are pure method swaps — no assert rewrites — and without them R2 does not compile. R1.6 migrates every such caller onto `try_apply`:

- `testgen-colloscopes/src/harness.rs` — `bootstrap`'s internal closure (the shared dev-dependency of every property harness) and its doc comment.
- `state-colloscopes/tests/` — the scaffold macros in `read_api.rs`, `colloscope_surface.rs`, `refs_registry.rs`.
- `ops/tests/` — the `Manager::apply` scaffold calls in `general_planning_content.rs` and `found_bugs.rs` (the ops-layer `UpdateOp::apply(&mut app_state)` calls, distinguished by their `(&mut app_state)` argument, are the frozen surface and stay untouched).
- `storage/tests/populated_round_trip.rs` + `populated_round_trip/builder.rs` — the post-reload id-issuer probe and the `build_rich_data` helper.
- `state/src/state.rs` and `state/src/traits.rs` test modules — the `FakeData`-driven history/undo/redo scaffolds (the two coexistence twin tests `apply_changes_data_and_stores_history` / `apply_failing_leaves_state_untouched` stay on old `apply` — they die with the API in R2).
- `constraints-colloscopes/tests/property_build.rs` — the walk's commit call (R1 swapped only its `broken_invariants` oracle, leaving the `apply` call as a leftover this commit clears).

The two `constraints-colloscopes` and `state/src/traits.rs` non-twin sites were not in the `plan_step_5_r1_5.md` §6 enumeration but are the same class of leftover; migrating them here keeps R2's "old API caller-free" precondition genuinely true. Landed as R1.6.

### R2 — remove

In one commit (it is a single compilation unit of change — the trait members, the impls, and the vocabulary types reference each other):

1. The 16 checked `apply_*` bodies (students, periods, weeks, subjects, teachers, assignments, week_patterns, slots, incompats, pairings, slot_pairings, group_lists, settings, balancing, colloscopes, export_config) plus the old `InMemoryData::apply` body for `Data` and the `Error` enum (lib.rs:250-286).
2. The 17 old per-domain `*Error` enums. Nothing survives out of them — every variant either has a precheck twin already living in `*PrecheckError`, or died as a stripped guard. (Full twin/die split per enum is in the workings; representative: `PeriodError` twins `{InvalidPeriodId, PeriodIdAlreadyExists}`, dies `{PeriodIsReferencedBy… ×4, PeriodStillHasNonTrivialAssignments, PeriodStillHasNonTrivialGroupListAssociation, NotEmptyPeriodInColloscope, PeriodStillHasWeeks}` — the F4 vacuous guard and the F2 WeekMove drift-risk disappear here, as Appendix F predicted.)
3. `InnerData::check_invariants` + `check_no_duplicate_ids` + `InnerDataError` (lib.rs:162-206); the old-checker half of `Data::check_invariants` (the function dissolves — `assert_id_issuer_high_water` is already the surviving companion).
4. `colloscope_params.rs`: `Parameters::check_invariants` + `InvariantError` + the `check_*_data_consistency` family + the now-caller-free `validate_*` helpers (`validate_subject/teacher/student/slot/incompat/group_list/settings/balancing/week_pattern/pairing_rule/slot_pairing_rule` and their `_internal` twins) + private builders if caller-free after the sweep. The pub `validate_*_id` u64-promotion helpers (:111-198) are a different family and stay. `colloscopes.rs`: `validate_against_params` + sub-validators.
5. `invariants.rs`: the whole legacy bridge — `to_legacy` (both impls), `dangling_to_legacy`, `convergence_to_legacy`, `is_necessarily_logic_error`, `assert_differential`.
6. `state/src/traits.rs` + `test_utils.rs`: remove `type Error` / `fn apply` from the trait, `Manager::apply`, and both impls.
7. Rider: fix the stale doc comment on `force_apply_group_list` (group_lists.rs:680-687), which still lists `RemainingFilling` / `NonEmptyGroupsWhenReducing` / `PrefillGroupCountMismatch` as kept guards — all three died in the loose-ends phase; reword to match the shrunk `GroupListPrecheckError`.

Hidden-caller sweep (done during planning; re-grep at implementation): outside state-colloscopes, the dying vocabulary was referenced only by `storage/src/decode.rs:64`, `spec2.rs:27/:327/:517`, `gtk4/src/loading/file_loader.rs:158`, `constraints-colloscopes/tests/property_build.rs:146`, `state-colloscopes/tests/property_ops.rs:75` — all migrated by commits 6/R1 — and the in-crate tests migrated in commit 5. Python touches none of it (glue.rs:1363 goes through the frozen `ops::UpdateError`); `InvariantError`'s Serialize/Deserialize derives have no wire consumers.

**→ Second testing pause** (same protocol).

### R3 — mechanical rename

`try_apply` → `apply` (trait method, `Manager`, both impls, all call sites and test names), `type ApplyError` → `type Error`, the enum `ApplyError` → `Error`, `property_try_apply.rs` → e.g. `property_apply_gate.rs`. Zero semantic change. Done-check: `grep -rn 'try_apply\|ApplyError'` over the workspace returns empty; build + suite green. Edits via the Edit tool per file (no `sed`).

---

## 12. Acceptance gates (user-run)

- After commit 2 and at each pause: full workspace suite, run **once**, output captured to a scratchpad file and grepped.
- First pause: 500-seed crank (`COLLOMATIQUE_PROP_SEEDS=500`-style, per the harness's config override) of `property_try_apply` and `property_ops`.
- Byte-stability: `examples/hogwarts.collomatique` round-trips unchanged (no format change is intended anywhere in this step; the smoke tests cover load, the round-trip check covers save).
- The three contract scripts (`extra-scripts/import.py`, `scripts/import_pronote_web_2026_05_06.py`, `scripts/examples/custom_export_xlsx.py`) — the error surface changed (§9 of the design doc: "scripts are updated in the same change and run by the user as acceptance"). Expectation: no script edits should be needed, since Python sits on the frozen `ops::UpdateError` and the scripts drive valid data; the run is to *prove* that, plus exercise the new load-error path.
- gtk4 manual smoke: open a file, run the solver and inject its colloscope (the `GlobalUpdate` site), run a Python script through the runner (the other `GlobalUpdate` site), undo/redo across a few ops (the replayed-through-`try_apply` path after R1), and trigger one load failure if convenient.

---

## 13. Decision ledger

Decisions made in this plan (all settled — nothing deferred to implementation):

1. **Parallel-API migration with rename-back** — add `ApplyError`/`try_apply`, migrate, delete old, rename. (User's approach; the associated-type "flip" problem only forbids *replacing* gradually, not *adding*.)
2. **Three-tier `ApplyError`** with a `Logic` arm — required by `GlobalUpdate`'s external payloads, and (post-sealing) reachable *only* from external data. Sets pass through as `BTreeSet`, `Display` itemizes entries.
3. **`try_apply` = snapshot + `force_apply` + `broken_invariants` + rollback**, snapshotting *both* `InnerData` and the `IdIssuer` (the issuer clone is one `u64` of defensive insurance against future force-path changes; `annotate`-issued ids stay burned on failure, as today). The id-issuer assert is the surviving Data-level companion on the accepted path (Appendix D.4's pin).
4. **Replay moves to `try_apply` up front (commit 3.0), before any ops module migrates** — so history recorded by either API replays through the gate that accepts it (in particular the tolerated perfect-no-op ops of §3, which old `apply` would reject on replay). Safety by step-3 certification + step-4 `ForceValid` equivalence pins + the canary. (This was originally R1's first bullet; it is peeled out and landed early — the reason commit 2's red carve-out is safe to accept.)
5. **Ops translation doctrine**: pre-op validity ⇒ set-membership attribution; old-validator precedence via explicit priority passes; missing payloads synthesized from the op in scope; cleaning-contract panics stay panics; `UpdateError` and all ops-layer cleaning frozen (step 6/7 territory).
6. **Decode validates once, at the end, on the full `InnerData`** (reconstruction verified total on unvalidated params); the empty-assignments-row-on-unknown-period check stays as a decoder-owned error; loaded files hard-error on any non-clean checker result.
7. **Differential fuzz is rewritten, not deleted** (`property_try_apply.rs`: atomicity + honesty + reverse pins over the public gate); the canary is deliberately temporary and dies in R1; `property_ops_broken_invariants.rs` merges into `property_ops.rs` at R1.
8. **Exact-set asserts** in migrated tests wherever the expected set is known (stronger pins than the old single-variant matches), including the two-entry set at week_ops :530 pinning the F5 bound-0 rule.
9. **PairingRule/SlotPairingRule are sealed *before* step 5 starts** (separate session; G1/G2 recipe with a plain-fields DTO at the ops boundary). This plan assumes the sealed state: the two parts-share `LogicError` variants are gone, and the `Logic` tier means "external data only".
10. **The id-issuer high-water check stays a panic**, not an error arm: `annotate` fuses id issuance (normal ops draw from this issuer; the `GlobalUpdate` annotate arm absorbs payload ids via `skip_to_id(max_id + 1)`, ops.rs:820-827), so the check cannot fire through the `Manager` surface. Its only trigger is a cross-instance `AnnotatedOp` transplant through the raw `try_apply` — a caller bug every consumer would panic on anyway, so an error arm would just recreate dead `panic!("cannot happen")` boilerplate.
11. **The old↔new agreement contract is one-directional** (decided when commit 2 landed red, encoded by commit 2.5). Old `Ok` ⇒ new `Ok` with equal state and reverse is the soundness direction and is strict. The reverse direction tolerates exactly one bounded exception — the step-4 perfect-no-op divergence, where old `apply` rejects a harmless clear (`SubjectHasNoInterrogation`) that the gate accepts without changing anything and with a no-op reverse. Every *state-changing* new-`Ok`-against-old-`Err`, and every old-`Ok`/new-`Err`, stays fatal. The canary encodes this asymmetry; the replay soundness it relies on is delivered by commit 3.0.

## 14. Design-doc bookkeeping (close-out)

On completion: retire this plan from the tree (pin `git show <sha>:docs/plans/plan_step_5.md`), add **Appendix G** to `invariant_cascade_design.md` recording the delivered state (the final `ApplyError`→`Error` vocabulary, the gate semantics, the decode contract, what was deleted), mark §8's step-5 entry `— COMPLETED` with the commit anchors and the ★ end-of-step gate line, and fix §8's stale wording ("collapse into the precise `InvariantError`" — the delivered vocabulary is `LogicError`/`Convergence`/`FixableInvariant` + `PrecheckError`, per Appendix C). Steps 6 (cascade) and 7 (UI consequences) then build directly on `try_apply`-now-`apply`.
