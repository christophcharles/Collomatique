# State consolidation plan

Status: **complete** (July 11 → August 2 2026, branch `consolidate_state`). Every phase below
has landed. The last of them — the invariant-cascade roadmap — closed August 2 2026, and its
own design doc is retired into Appendix A.
Scope: the state stack — `state/`, `state-colloscopes/`, `ops/`, `storage/` — and the on-disk file format.

This document records the architecture review of the state stack, the decisions taken, and the
phased plan to (1) give Collomatique a real, version-stable file format and (2) simplify the
in-memory state machinery behind it. It is meant to survive across sessions: read this first
before touching any of the crates above.

Two halves, and they answer different questions. **§§1–9 are the plan and its landed record**,
written as the work went: read them to find out *why* something is the way it is, or which
commits did it. **Appendix A describes the code as it stands today** and the rules a change to
it must respect: read it first if you are here to write code.

---

## 1. Background: the state stack at review time (July 11 2026)

Four layers, cleanly separated in intent:

| Layer | Size | Role |
|---|---|---|
| `state/` | ~1,000 loc | Generic undo/redo: `InMemoryData` trait (`annotate` / `apply`, where `apply` returns the inverse op), `Manager`, `AppState`, `AppSession` (transactional commit/cancel), `ModificationHistory` |
| `state-colloscopes/` | ~8,200 loc | `Data { Mutex<IdIssuer>, InnerData }`; `InnerData { params, colloscope, export_config }`; elementary `Op`/`AnnotatedOp` (16 categories); all invariant checking. `lib.rs` alone is 3.9k loc |
| `ops/` | ~7,700 loc | "Natural" UI-level `UpdateOp`s composed of elementary ops; recursive cleaning-op cascade with `UpdateWarning`s; `dry_apply` via `AppSession` |
| `storage/` | ~700 loc | JSON file: versioned envelope (header + entries with `minimum_spec_version` / `needed_entry` / `Caveat`s) whose sole payload entry is `InnerDataDump(InnerData)` — raw serde of the in-memory struct |

*The table is the pre-work snapshot the review was based on — sizes and payload descriptions
have since moved (spec-2 format structs in `storage/`, `lib.rs` split, generic layer in
`state/`); §§3–6 track what changed, and Appendix A.1 describes the stack as it now stands.*

Consumers: `gtk4` (reads `get_inner_data`, writes via `ops`), `python` (pyo3: writes via `ops`,
reads via pyclass mirrors of `InnerData` types), `constraints-colloscopes`
(`build_config(&Parameters, &Colloscope)`), `xlsx`, `rpc`.

### What is good and must be preserved

- The **Original/Annotated op split** (`state/src/history.rs` doc comment): annotation freezes
  freshly issued IDs so undo→redo replays identically. This is the correct design.
- **`AppSession`** as a transaction (commit → one aggregated history slot; cancel → undo all),
  and `ops::dry_apply` built on it.
- The **cleaning-op cascade with warnings** in `ops/`: auto-fix dependent data instead of
  refusing, and tell the user.
- The **storage envelope**: semver `produced_with_version`, per-entry `minimum_spec_version` +
  `needed_entry`, `Caveat`s, tolerance of unknown entries. A solid foundation — kept as-is.
- **Typed IDs in a single global ID space** (one monotonic `u64` counter for all entity kinds).
- **Precise typed referential-integrity errors** (e.g. `PeriodError`'s 8 "referenced by …"
  variants). The information is right; only the mechanism is repetitive.

### Problems identified (review of July 2026)

1. **File format** *(resolved — phase 1)*: the envelope is honest but the payload is serde-derive output of
   `InnerData` — any field rename/retype silently changes the on-disk format. Old-file
   failures surface as a misleading `ProbablyIllformedEntry` because the custom
   `EntryContent::deserialize` swallows the underlying serde error. Only `#[serde(default)]`
   on late-added fields provides any compatibility.
2. **Invariant double duty** *(resolved — the invariant-cascade roadmap; Appendix A)*:
   each per-entity predicate exists twice
   (`validate_*_internal` for typed per-op errors, `check_*_data_consistency` discarding
   detail into the ~26-variant `InvariantError`); every referential relationship is
   hand-coded twice (remove path + consistency pass); duplicate-ID scanning exists in three
   implementations; and `Data::apply` re-validates the **entire document after every op** —
   including every undo/redo sub-op (O(history × doc size)), panicking on failure. The load
   path (`Data::from_inner_data`) checks twice more.
3. **apply/build_rev duplication** *(resolved — Phase 2 item 1)*: was two parallel 15-way method
   families (~1,900 loc) that had to agree, with the inverse builders almost entirely untested.
   `apply` now computes and returns the inverse itself; `build_rev_with_current_state` is gone.
4. **Write fan-out from the params↔colloscope mirror** *(resolved — step 1d of the
   invariant-cascade roadmap, delivered July 18 2026; Appendix A)*: was — adding a slot
   had to insert an empty `ColloscopeSlot` into every colloscope period; period ops spanned
   ~330 lines. The colloscope is now sparse; the fan-out is deleted.
5. **Inconsistent op granularity** *(resolved — phase 2 item 4)*: whole-struct `Update`s
   (Settings, Balancing, entities) next to 11 per-field `ExportConfigOp` variants and a
   single-bool `AssignmentOp`. No principle.
6. **Test coverage** *(resolved — phase 0)*: zero tests in `state/`; 3 integration tests in
   `state-colloscopes/`; storage tests use only empty data.
7. Read-path indirection (O(n) `find_*` scans, nested lookups) is **noise, not a performance
   problem** at real data sizes (~50 students); the only super-linear cost is item 2.

### Historical reference material (in git)

- The **retired SQLite schema** — a complete, normalized, already-debugged spec of the data
  model (table-per-entity, child tables for one-to-many, FKs with explicit CASCADE/RESTRICT,
  enums flattened with exactly-one CHECKs):
  `git show 6e18c3d1~1:sqlite-state/src/schema.rs`
- The **abandoned per-entity structured JSON format** (deleted by commit `95568c27`
  "Simplified storage to only use a simple data dump", Oct 2025, because maintaining it during
  internal churn was too costly): `git show 95568c27~1:storage/src/json/` and
  `.../storage/src/decode/`. The lesson was about *sequencing* (don't maintain a frozen format
  while the model churns weekly), not about the design.

---

## 2. Decisions taken

- **Order of work: file format first, then in-memory restructuring.** The format firewall is
  what makes internal refactoring safe and keeps all existing test files usable.
- **Substrate: JSON, keeping the existing envelope.** SQLite-as-file-format is rejected
  (binary, non-diffable fixtures, sqlx deliberately removed). The SQL schema is kept as a
  *design document* only.
- **The current dump format (spec 1) will NOT be supported long-term.** It never shipped.
  One-time bulk conversion of all existing files, then the v1 decoder is deleted. Spec number
  1 becomes a dead number (spec numbers are internal wire-protocol details; this is normal).
  A small *tombstone* stays behind: recognizing an `InnerDataDump` entry produces a clear
  "unsupported pre-alpha development format" error instead of a generic decode failure.
  There is no re-open path after the conversion window (everything is version 0.1.0 pre-alpha,
  so there is no released version to point people to) — which makes the bulk conversion step
  a hard prerequisite: **every file that matters must be converted before the v1 decoder is
  deleted**. The new format is spec 2; do **not** renumber it as 1 (old dev files also claim
  1, and confusing the two would produce garbage errors).
- **The file stores a snapshot only.** No undo history, no op log in the file for now. The
  entry mechanism leaves the door open (a serializable op-log entry with
  `needed_entry: false` is possible later; note `ops::UpdateOp` is already serde-capable).
- **Python scripts are a compatibility contract** (see §7). Scripts are updated in the same
  change when the Python API must move, and the user runs them as acceptance tests.

---

## 3. Phase 0 — safety net (before anything else)

Cheap, high-leverage, exploits the current redundant checks as a test oracle *while they
still exist*:

1. **Property-test harness** in `state-colloscopes/tests/`: generated/enumerated elementary-op
   sequences →
   - after every apply, `check_invariants` holds;
   - undo-all returns exactly the initial state;
   - applying an op then applying the inverse it returns is the identity;
   - redo after undo reproduces the same state (annotated-op ID stability).
2. **Populated round-trip storage tests** (encode→decode == identity on non-trivial data —
   current tests only cover empty colloscopes).
3. A few unit tests for `state/` history pointer math and the mid-aggregate rollback in
   `update_internal_state_with_aggregated`.

### Status (July 2026)

*This is the phase-0 record as written, and some of its details are of that era: the harness
originally asserted `check_invariants` (now `broken_invariants`), and the "16 op categories"
count predates the invariant-cascade re-cuts of the op surface. The harness moved with the
stack in the same changes each time. Appendix A.10 describes the safety net as it stands
today.*

- **Item 3 — DONE** (commit `7628f303`): in-crate `#[cfg(test)]` tests for `state/` —
  history pointer math and truncation, `Manager::apply` failure paths, mid-aggregate
  rollback (success, rollback-on-failure, panic-on-failed-rollback), session
  commit/cancel atomicity incl. nested sessions, `IdIssuerHelper`. A minimal
  `FakeData`/`FakeOp` implementation lives in `state/src/test_utils.rs`. Pinned as-is:
  a zero-op session commit stores an empty history slot; `max_history_size = Some(0)`
  stores nothing.
- **Item 1 — DONE** (commit `62951deb`): `state-colloscopes/tests/property_ops.rs` +
  `property_ops/{harness,generator,synth}.rs`. Five properties, each over 100
  deterministic seeds (`ChaCha8Rng`) × 1000 generated ops covering all 16 op categories
  (~15% deliberately invalid); on failure the seed + full op log replay exactly. Also
  covers error atomicity and random undo/redo/apply walks against a snapshot model.
  Costs ~40 s wall clock per `cargo test` (debug). The original 500-seed configuration
  (~3.5 min) stays as the slow reference for occasional milestone checks; 100 seeds
  was verified to still catch each of the four bugs below.

  The harness found **four real bugs**, each quarantined in the generator with a
  `TODO(phase0-bug)` marker (fix the bug ⇒ delete the quarantine so the path is
  covered again). Each fix is preceded by its own regression test in
  `state-colloscopes/tests/found_bugs.rs`, committed failing right before the fix:
  1. **FIXED** — `StudentOp::Remove` did not check `settings.students` → dangling
     per-student settings entry (invariant panic `InvalidStudentIdInSettings`). Removal
     is now rejected with `StudentError::StudentStillHasSettings`.
  2. **FIXED** — `GroupListOp::SetFilling` automatic→automatic did not check the new
     `excluded_students` against students already placed in the colloscope group list
     (`ExcludedStudentInGroupList`); the prefilled↔automatic transitions were checked.
     Now validated like `Update`, rejected with `NotCompatibleGroupListInColloscope`.
  3. **FIXED** — `GroupListOp::Update` did not check interrogations' `assigned_groups`
     when shrinking `group_names` (`InvalidGroupNumInInterrogation`). The walk from
     `AssignToSubject` is factored into `check_interrogations_group_bound` and applied
     to every subject associated with the list, rejecting with
     `InvalidGroupInSubjectSlotInColloscope`.
  4. **FIXED** — `build_rev_group_list(Remove)` rebuilt only `Add(id, params)`, losing
     the filling kind: undoing the removal of a prefilled (empty) group list restored
     it as automatic and re-registered a colloscope entry. `AnnotatedGroupListOp::Add`
     now carries the filling (default when annotating a plain `Add`) and the apply
     path only registers a colloscope entry for non-prefilled fillings.

  Latent (spotted by review, not triggered), **FIXED**: `GroupListOp::AssignToSubject`
  with a dangling group-list id panicked on an `.expect` (`lib.rs`, "Group list ID
  should be valid") before reaching its (dead) `InvalidGroupListId` check. It now
  returns `InvalidGroupListId` and the generator covers this invalid case.
- **Item 2 — DONE**: `storage/tests/populated_round_trip.rs` +
  `populated_round_trip/builder.rs`. A deterministic op script (no RNG) builds a
  document where every serialized section is non-trivially populated (asserted at
  the end of the builder, so silent degradation fails loudly). Three tests:
  encode→decode identity with no caveats, re-serialization byte-stability (the
  format-determinism canary for the phase-1 golden fixtures), and
  editability-after-reload (the rebuilt `IdIssuer` issues non-colliding ids).
  No new dependencies.

---

## 4. Phase 1 — the real file format (spec 2)

**DONE** (commits `453dbdf1` spec reference doc → `92ce5217` format structs → `c9970a40`
spec-version-dispatched decoding + legacy writer flag → `b1957d03` wire read/write, bump
`CURRENT_SPEC_VERSION` to 2 → `2841d1de` encapsulate the invariant-carrying scalars →
`1360abe0` save in spec-2 by default). Spec 2 is the format actually written and read; the
field-level reference lives in `docs/file_format/file_format.md`.

**Update — the legacy v1 path has now been removed.** It was kept, deprecated, through
phase 1 so existing files kept opening. Once the corpus was converted to spec 2, the writer
was retired first (`dd385261`: dropped the `legacy` flag and the spec-1 encoder), then the
reader (`04ff63b2`): a spec-1 file — any entry declaring `minimum_spec_version: 1` — is now
rejected with the clear `RetiredSpec1Format` tombstone error instead of decoding. This lifts
the ordering constraint below: the phase-2 changes to `InnerData` (items 2–5 of §6) are
unblocked, since no decoder reads the live `InnerData` via serde anymore.

### Design

- **Keep the envelope** exactly as is: `Header { file_type, produced_with_version,
  file_content }` + `Vec<Entry> { minimum_spec_version, needed_entry, content }`, `Caveat`s,
  unknown-entry tolerance. Bump `CURRENT_SPEC_VERSION` to 2; entries introduced by the new
  format declare `minimum_spec_version: 2`.
- **Replace the single `InnerDataDump` payload with per-section entries**, one per data-model
  area: general planning (start date + periods/weeks), subjects, teachers, students,
  assignments, week patterns, slots, incompatibilities, group lists (incl. period/subject
  associations), pairings, slot pairings, settings, balancing, colloscope, export config.
  Per-section entries exploit `needed_entry`: a future section (e.g. rooms) can be an
  unneeded entry that old readers skip with a `Caveat`.
- **Dedicated format structs** under `storage/src/format/` — plain dumb data, `u64` IDs, no
  reuse of `state-colloscopes` types, no clever serde. All conversion to/from `InnerData`
  lives in one module. This decoupling is the entire point: the in-memory struct becomes free
  to change without touching files.
- **Shape follows the SQL schema where it is flatter** than the in-memory struct:
  associations as arrays of `{period_id, subject_id, group_list_id}` rows; colloscope
  interrogations stored **sparse, keyed by week index** (not positional vectors), so a later
  in-memory re-keying never touches the format.
- **Better decode diagnostics**: when an entry with `minimum_spec_version <=
  CURRENT_SPEC_VERSION` fails to decode, surface the underlying serde error (today it is
  swallowed into `UnknownEntry` and reported as `ProbablyIllformedEntry`).
- **Validation stays where it is**: decode produces an `InnerData`, and
  `Data::from_inner_data` remains the single trust boundary at load.
- A full field-by-field format specification is written during this phase as
  `docs/file_format/file_format.md`, derived from the recovered `SCHEMA_SQL` and the format
  structs.

### Migration and freeze

- The legacy v1 decoder reads the *live* `InnerData` type via serde, so it only works while
  `InnerData` is unchanged. Therefore, strictly in this order:
  1. implement spec 2 (write + read) while v1 reading still works; **(done)**
  2. **bulk-convert every existing file** (all private test files, anything the Python
     scripts produced); **(done — the corpus was ported to spec 2)**
  3. delete the v1 decoder (leave the tombstone) — *before* any phase-2 change to
     `InnerData`. **(done — `RetiredSpec1Format` is the tombstone; a byte-accurate spec-1
     document survives as `storage/tests/fixtures/spec1_empty.json` to test rejection)**
- **Golden fixture tests**: commit small populated spec-2 files (the themed examples of
  phase 1.5, §5, are the main corpus); CI asserts they decode successfully and that
  re-encoding is stable. Accidental format drift then fails CI instead of silently breaking
  compatibility. After this point the format changes only by explicit
  new spec versions, whose old decoders are frozen forever (recent versions always open old
  files).

### Why this won't die like the 2025 attempt

The 2025 structured format was maintained while the data model churned weekly. Now: the model
has just been consolidated and is comparatively stable; the format structs are deliberately
dumb so conversions are mechanical; and golden tests turn "silent maintenance burden" into
"explicit, versioned evolution".

---

## 5. Phase 1.5 — example files in the repo

**DONE** (commit `c1e12e13` the Hogwarts fixture; `06fed862` the smoke test loading and
building every `examples/` file).

**Decision — one broad fixture, not a themed set.** Rather than several themed files each
covering a slice of the format, a single **Hogwarts** example (`examples/hogwarts.collomatique`)
was built to exercise *most* features at once. Additional fixtures (the Scientists /
Scholastics themes originally sketched below) may still be added later but are **not a
priority** — one good fixture already gives CI a golden file and the stack an integration
input. The originally-planned themes are kept below as notes only.

The example lives at `examples/` (repo root) with non-sensitive, fun data, and serves three
purposes at once: golden fixture for CI (see §4 — this file *is* the fixture corpus for now),
integration-test input for the whole stack (load → edit via ops → solve → export), and
documentation-by-example for users.

Themes originally sketched (naming only — the theme carries no technical meaning); only
Hogwarts was built:

- **Hogwarts** *(built)*: Harry Potter characters; interrogations in Potions, Defense against
  the Dark Arts, etc.
- **Scientists** *(not built)*: known physicists/chemists with *plausible generations* (e.g.
  Rutherford as teacher, Bohr as student).
- **Scholastics** *(not built)*: St Thomas Aquinas, St Anselm, etc.

Feature coverage goal (now carried by the single Hogwarts file): the format is exercised
broadly — group lists (both prefilled and automatic filling, different lists on different
subjects), multiple periods, week patterns, incompatibilities, pairing and slot-pairing
rules, per-student/per-subject overrides in settings and balancing, export config, and a
colloscope. Any later fixtures would extend coverage rather than partition it.

Guidelines (met): hand-checkable size; created through the app or the Python API (not
hand-written JSON) so it is guaranteed valid; CI asserts the file decodes, passes
`Data::from_inner_data` validation, and re-encodes stably.

## 6. Phase 2 — in-memory restructuring (only after the format is frozen)

Direction agreed in outline; **each item below gets its own detailed plan (and user sign-off)
before implementation.** Ordered by leverage:

1. **Fuse `build_rev` into `apply`** — **DONE** (commit `90096d0d`). `state/` trait change:
   `fn apply(&mut self, op) -> Result<AnnotatedOperation, Error>` returns the inverse,
   computed while the old value is in hand. Halved the two 15-way families, eliminated the
   apply/build_rev agreement problem, removed one validation pass. Stored
   `ReversibleOp { forward, backward }` and the history machinery unchanged; a `debug_assert`
   canary on the undo/redo replay path checks the recomputed inverse against the stored one.
   The one guard that lived only in `build_rev` (`Colloscope::UpdateGroupList`'s
   colloscope-entry-existence check) was transplanted into the fused method.
2. **`Table<Id, T>` + declare-once relationship registry** — **DONE (July 16 2026)**. All
   five phases of the detailed plan delivered (the plan doc is retired; pinned at
   `git show 77695338:docs/table_registry_plan.md`, its inventories inlined as Appendix A of
   the invariant-cascade design doc, itself now retired — see Appendix A.0 for that pin):
   generic `Table`/`OrderedTable` in `state/` + the
   `EntityId`/`References`/`Join` derives (new `collomatique-state-derive` crate, reusable by
   the rooms side-project); the `RefSite` walker + `references_to_*` reverse lookups; the
   SQL-like read API (`Lookup`, `lookup`/`resolve`, `all_ids`, `Joined*` views); all consumers
   migrated off the `Deref` compat layer, then the layer deleted — the internal table
   representation is now free to change. The ★ D+E milestone (500-seed property reference +
   the three contract scripts) ran clean. Check-rerouting through the registry was handed off
   to item 3 and is now superseded with it by the invariant-cascade design (see item 3);
   `ops/` got mechanical read fixes only (slated for its own remaster, cascade step 7).
   Original sketch kept below for context.
   "FK" = foreign key, the SQL term for a declared reference ("slots hold a `TeacherId`").
   Today every such relationship is hand-coded at least twice (delete-blocking scan in the
   `Remove` path + matching consistency pass). The proposal: one generic table type replacing
   the `Vec<(Id, T)>` / `BTreeMap` mix — the ordered case need not keep today's
   representation; a keyed map with the user-visible order stored as a separate explicit
   order list is fine (the order is meaningful UI data, the container layout is not) — plus a
   registry
   where each relationship is declared once with a way to enumerate its references. Derived
   generically from the declarations: delete-blocking checks (same typed errors as today),
   the referential part of full validation, reverse lookups ("who references X?" — useful for
   the UI and the `ops/` cleaning cascades), duplicate-ID/counter maintenance. This is the
   SQL schema's `REFERENCES ... ON DELETE RESTRICT` brought in-memory.
   *Lighter fallback* if this feels over-engineered when detailed: keep hand-written checks
   but merge the two per-entity predicate families so each rule exists exactly once.
3. **Invariant consolidation** — **SUPERSEDED, direction reversed** (July 15 2026) by the
   invariant-cascade roadmap, and **delivered in full August 2 2026** (Appendix A):
   `check_invariants` became the
   *sole* enforcement — each elementary op is apply → check → rollback-on-failure, returning
   precise coordinate-bearing errors — and the per-op typed preconditions retired (the exact
   opposite of the demotion sketched below). The extended-scope note ("reroute the triplicated
   checks through the item-2 registry") and the registry plan's §6 hand-off notes are
   superseded with it: two of the three check families are deleted, not rerouted.
   *Original sketch (historical):* keep per-op precondition checks (typed errors); demote the
   whole-model `check_invariants()` after every op to `debug_assertions` and the phase-0
   property tests; full checks remain only at trust boundaries (file load, `GlobalUpdate`).
   Collapse `InvariantError`'s duplicated variants onto the per-entity error enums.
4. **Uniform op granularity** — **DONE (July 2026)**: every entity gets
   `Add / Remove / Update(whole entity)` (+ position ops where user-visible order exists,
   + association ops where relational). A workspace survey (July 31 2026) found every
   family already uniform after the cascade's step-1 re-cuts and the earlier reshapes
   (`AssignmentOp::SetRow` had long replaced the single-bool op; the group-list family is
   one sealed-`GroupList` payload; settings and balancing are whole-override-entry sets).
   The last residue — `ExportConfigOp`'s 11 per-field variants — collapsed into one
   whole-struct `Update(ExportConfig)` in the pre-step-7 sidework of July 31 2026
   (three commits; plan retired, pinned at
   `git show 15b59b1c:docs/plans/plan_export_config_op.md`).
   Elementary ops only need to be *reversible and replayable*; user-facing per-field
   granularity lives on in `ops/`' `ExportConfigUpdateOp` variants and their French history
   descriptions, exactly as this item prescribed. *Note (July 2026)*: independent of, but
   interacting with, the invariant-cascade design (its §9) — the resolution map wants
   elementary ops that can express "remove/clear this one reference" conveniently, and the
   cascade's step-1 reshapes re-cut the slot/week/colloscope op surfaces anyway; in the end
   granularity uniformization rode along those steps, with export config as a final pass.
5. **Params↔colloscope synchronization** — **DISSOLVED** (July 15 2026) by the
   invariant-cascade design (step 1d + the cascade): the colloscope goes sparse, so there is
   no fan-out left to centralize — cleanup becomes cascade resolution. The spec-2 format was
   deliberately shaped for exactly this re-keying and does not move. *Step 1d delivered
   July 18 2026 — the sparse half is real; cleanup became cascade resolution at steps 6–7
   (Appendix A.4, A.5).*
   *Original sketch (historical):* keep the dual representation (different access
   patterns) but centralize the fan-out (candidate: the registry owns "structural param ops
   propagate to colloscope"), and consider keying interrogations by week index instead of
   positional `Vec<Option<_>>`.
6. **Split the `lib.rs` god-file** — **DONE** (commits `29974361` error enums, `7210ead5`
   `apply_*` methods, plus `NewId`→`ids.rs`). Done ahead of items 2-5 because it needs no
   `InnerData` change (those are blocked on the legacy-file migration): each per-entity
   error enum and `apply_*` method moved verbatim into its existing entity module
   (`pub(crate)` methods in `impl Data` blocks; `InvariantError` to `colloscope_params.rs`,
   its construction site). Crate-root `pub use` re-exports keep every external path
   unchanged. `lib.rs` is now ~400 lines: `Data`/`InnerData`, the aggregate `Error`, and
   the `InMemoryData` dispatch. No new files were needed, so the style rule (`foo.rs` +
   `foo/` directory, never `foo/mod.rs`) was moot.
7. **Python glue** — **DONE / closed as a non-item (July 2026)**: the write path was and
   remains insulated behind `ops::UpdateOp`; the read-path pyclass mirrors break
   compile-visibly and have been regenerated mechanically inside the same change every time
   a struct moved (steps 1a/1c/1d glue notes). No Python API redesign happens in this phase
   (a full redesign is still expected later, with the MVVM UI work). The export config is
   not exposed in Python at all, so the July 31 2026 sidework of item 4 had zero Python
   surface. The three contract scripts of §7 remain the acceptance oracle for future
   changes.

---

## 7. Compatibility contract: the scripts

These scripts define the Python API surface that must keep working across all phases (or be
migrated *in the same change*, with the user running them as the acceptance test — some of
their input files contain private data and live outside the repo):

- **`extra-scripts/import.py`** — the main real-world import (in-repo copy for reference;
  its CSV inputs are private). Exercises: `current_session`, dialogs (`dialog_input`,
  `dialog_open_file`, `dialog_confirm_action`, `dialog_info_message`), `log`,
  `settings_update_global_limits`, `periods_add`, `subjects_add`, `group_lists_add` /
  `group_lists_update` / `group_lists_set_filling` / `group_lists_set_association`,
  `teachers_add` / `teachers_update`, `week_patterns_add`, `slots_add`, `incompats_add`,
  `students_add`, `assignments_set`; reads `get_main_params()` (`.subjects`, `.group_lists`,
  `.week_patterns`, `.teachers`, `.students`, `.group_lists_associations`,
  `.get_week_count()`); value types `SoftU32`, `SoftNonZeroU32`, `Limits`,
  `SubjectParameters`, `SubjectInterrogationParameters`, `SubjectPeriodicity`,
  `GroupListParameters`, `PrefilledGroup`, `GroupListFilling`, `WeekPattern`, `Teacher`,
  `Student`, `Incompat`, `Time`, `SlotStart`, `SlotParameters`, `SlotWithDuration`,
  `Weekday`.
- **`scripts/import_pronote_web_2026_05_06.py`** — Pronote web CSV import: `dialog_open_file`,
  `get_main_params`, `periods_add`, `subjects_add`, `students_add`, `assignments_set`.
- **`scripts/examples/custom_export_xlsx.py`** — read path (`get_colloscope`,
  `get_main_params`) + the `time` glue types.

These three scripts are the **complete** set to maintain — any other old script is retired.
When in doubt, ask the user to run the real scripts/files rather than guessing.

---

## 8. Explicitly rejected / deferred

- **SQLite as the on-disk format** — rejected (binary, non-diffable fixtures, sqlx removed).
  Schema kept as documentation only.
- **Persisting undo history / op log** — deferred; snapshot-only file for now, entry
  mechanism keeps the door open.
- **Micro-optimizing O(n) lookup scans *as a goal*** — performance is fine at real data
  sizes (confirmed by extensive use with sizable files); only the per-op whole-model recheck
  (phase 2, item 3) is a real cost. This does **not** protect the current containers: the
  binding requirement is only that user-visible ordering (subjects, periods, per-subject
  slots) is preserved *as data*. A uniform keyed model with the order stored separately
  (e.g. map + explicit order list) is an acceptable — even welcome — outcome of phase 2
  item 2.
- **Long-term support of the spec-1 dump format** — rejected; one-time conversion window,
  then tombstone.

## 9. Open points

**None left.** The two that stood here have closed:

- Phase 2 item 2 (`Table` + relationship registry) — **DONE (July 16 2026)**; see §6 item 2.
  The detailed plan doc is retired (pinned at `git show 77695338:docs/table_registry_plan.md`).
- The remaining phase-2 work ran as the **invariant-cascade roadmap** (agreed July 15 2026),
  a 7-step plan: reshape the dense copies (1a assignments sparse, 1b `WeekId`, 1c slots
  sidecar sparse, 1d colloscope sparse) → precise checker alongside the old one →
  completeness audit → differential fuzz → switch elementary ops to apply/check/rollback →
  the cascade → the `ops/` remaster. It superseded item 3 (direction reversed) and dissolved
  item 5; item 4 rode along its reshapes and closed July 31 2026. **All seven steps are
  complete** (July 18 → August 2 2026), each ★ gate ran clean, and the roadmap's own design
  doc is now retired in turn — **Appendix A** is what it left behind.

---

## Appendix A — the state stack as delivered (August 2 2026)

### A.0 What this replaces, and where the record went

The invariant-cascade roadmap ran from July 18 to August 2 2026 and delivered the
architecture described below. Its design doc — the design argument of §§1–7, the seven-step
record of §8, and the ten appendices A–J that recorded what each step delivered — is retired
and pinned at

    git show af774a54:docs/plans/invariant_cascade_design.md

Read that pin when you need the *why* behind a rule below, the argument that was weighed
against an alternative, or the commit-by-commit record of a step. Each step also had its own
session plan, retired behind its own pin. Here is the flat list, so that finding one does not
need a dig through the retired doc:

| step | what it delivered | session plan |
|---|---|---|
| 1 | reshape: sparse assignments, `WeekId` as an entity, sparse slots sidecar, sparse colloscope | `git show 62949404:docs/plans/plan_step_1.md` |
| 2 | the precise checker, written and fully tested but deliberately unwired | `git show 49b4f77d:docs/plans/plan_step_2.md` |
| 3 | completeness audit certifying the old checker as the differential oracle (doc-only) | `git show 26d88024:docs/plans/plan_step_3.md` |
| 4 | the `force_apply` door and the differential fuzz between the two checkers | `git show fbc4ae6d:docs/plans/plan_step_4.md` |
| — | pre-step-5 loose ends: periods/weeks split, sealed `GroupList` and pairing rules | `git show 25fdc50b:docs/plans/plan_loose_ends.md` |
| 5 | production switched to apply/check/rollback; the whole old checked world deleted | `git show b6f7bdbc:docs/plans/plan_step_5.md` |
| 6 | the cascade engine and the colloscope resolution map | `git show b35d6a56:docs/plans/plan_step_6.md` |
| 6.5 | the document order (`ContentOrd`), asserted in the loop after every fix | `git show 8bfd0b64:docs/plans/plan_step_6_5.md` |
| 7 | the `ops/` remaster: every user-facing composite runs on the cascade | `git show 89b1452f:docs/plans/plan_step_7.md` |

Two earlier plans of this document are retired the same way: the table-registry plan
(`git show 77695338:docs/table_registry_plan.md`, §6 item 2) and the export-config op reshape
(`git show 15b59b1c:docs/plans/plan_export_config_op.md`, §6 item 4).

What follows is not that record. It is the part that still governs code you can edit today.

### A.1 The shape of the stack

`state/` — generic, with no knowledge of colloscopes:

- `traits.rs` — `InMemoryData` (`annotate` / `apply`, where `apply` returns the inverse),
  `Manager` with `apply` and the defaulted `apply_cascade`, and the shared error enum
  `ApplyError<InvalidOp, Invariant>`.
- `state.rs`, `history.rs` — `AppState`, `AppSession` (transactional commit/cancel),
  `ModificationHistory`, the Original/Annotated op split.
- `cascade.rs` — the `Fixable` and `FixOp` traits, `apply_cascade`, `CascadeReceipt`.
- `partial_order.rs` — `ContentOrd`, the `ContentIdentity` marker, and the order combinators.
- `tables.rs`, `refs.rs`, `join.rs`, `ids.rs` — `Table`/`OrderedTable`, the reference
  registry and its walker, the `Join` read views, the id issuer.

`state-colloscopes/` — the domain:

- `lib.rs` — `Data`/`InnerData`, the `InvalidOp` enum, the `Error` alias, the `InMemoryData`
  dispatch. Per-entity modules hold their own ops, apply bodies and read surfaces.
- `invariants.rs` — the checker `broken_invariants` and its vocabulary.
- `resolution.rs` (+ `resolution/`) — `impl Fixable for Data`, the `Fix` enum, and the
  translation from `Fix` to elementary ops, all in one file on purpose.
- `partial_order_tests.rs` — in-crate, because several compared values sit behind private
  fields.

`ops/` — the user-facing composites that gtk4 and Python drive:

- `lib.rs` — `UpdateOp` (fifteen families) and the two entry points `dry_apply` / `apply`.
- `cascade.rs` — `CascadeSession`, `CascadeWarning`, `CascadeResult`.
- `rendering.rs` — the public, noun-less id-rendering vocabulary.
- `warning_text.rs` — private; the French warning texts, keyed on `Fix`.

`storage/` — the spec-2 file format, frozen. See `docs/file_format/file_format.md`.

### A.2 The gate: how one elementary op is applied

`Data::apply` is snapshot → `force_apply` → `broken_invariants` → rollback or keep:

1. Clone `InnerData` **and** the `IdIssuer`.
2. `force_apply` the op. It runs the *prechecks* of A.3 and then mutates. A precheck failure
   returns before any mutation.
3. Run `InnerData::broken_invariants(&self) -> Result<BTreeSet<FixableInvariant>,
   BTreeSet<LogicError>>`. `Ok(∅)` means valid. Anything else rolls the snapshot back.
4. On success, assert the id-issuer high-water mark and return the inverse op.

So a failed op leaves the document bit-identical and stores nothing in history, and a
successful op is guaranteed fully valid. The replay path (undo, redo, `AppSession::cancel`)
runs through the same gate.

The error surface has exactly two arms. `ApplyError::InvalidOp` means "this op cannot be made
sense of against this state" and is never resolvable. `ApplyError::BrokenInvariants(set)`
means the op is well-formed but the state does not yet satisfy what it needs — this is what
the cascade consumes. `collomatique_state_colloscopes::Error` is an alias for
`ApplyError<InvalidOp, FixableInvariant>`.

The vocabulary the checker returns:

- `DanglingFk(Reference)` — a declared reference whose target id does not resolve. Generic:
  driven by the reference registry, so a new `#[fk]` is swept without new checker code.
- `Convergence` (16 variants) — a predicate over *existing* edges that a legitimate op can
  break indirectly, for instance a subject update that turns interrogations off under a slot
  that needs them. The cascade resolves these, lossily.
- `LogicError` (14 variants) — truth decidable from a row's own value, or from
  document-wide id uniqueness, plus the ordering-sidecar mirrors. No legitimate op can
  produce one. They short-circuit as `Err`, because the fixable sweep says nothing
  meaningful over a logically broken state. In practice they arrive only from external data
  (file decode, a `GlobalUpdate` payload) or the `#[cfg(test)]` forge hatch.

Loading a file validates **once, at the end, on the full `InnerData`**:
`Data::from_inner_data` runs `broken_invariants` and hard-errors on any non-clean result. A
loaded document is fully valid, because broken states never exist outside the gate.

### A.3 What an elementary op may check

Elementary ops do **not** check invariants. That is the whole point of the gate: an invariant
exists once, in the checker. What an op still checks is the *carve-out* — facts about the op
itself rather than about the resulting state:

- **No-clobber** — `*IdAlreadyExists` on every `Add`.
- **Op-target existence** — `Invalid*Id` on the entity being updated or removed.
- **Parameter targeting** — inputs that say *where* the op acts must resolve: `AddAfter`
  anchors, `SetRow` / `SetInterrogation` / `SetGroupList` / `AssignToSubject` coordinates, a
  week move's destination.
- **Position bounds** — one named-field variant per entity carrying scope, position and size.
  Op-target existence is tested **before** bounds, so a doubly-bad op reports its dangling
  target.
- **Immutability** — `CannotChangeSubject`: a slot's subject is fixed at creation.

**Address, not content.** A precheck may verify where the op acts. Ids the op *writes into*
the document are content, and are deliberately not prechecked — the dangling-FK net reports
them. `AssignmentOp::SetRow` keeps both key checks (period, subject) because an empty payload
clears the row, so nothing lands and the FK net is structurally blind to a dead address.

**`force_apply` fixes nothing.** It does only what the op asks, and never touches state
outside the op's direct target to keep the rest consistent. A broken landing is legal;
reporting it is the checker's job. Two corollaries that cost real debugging to learn: never
add a consistency precheck to `force_apply`, and cleanup code reachable only when a *deleted*
guard would have rejected the op must be deleted with the guard — left in, it silently
repairs.

Position bounds and anchors aside, the general shape of a new elementary op is therefore:
check that the op is addressable, mutate, and let the gate decide whether the result is valid.

### A.4 The cascade

`apply_cascade` wraps the gate in a retry loop. The target op is applied; if it comes back
`BrokenInvariants(set)`, the engine picks the canonical minimum of the set (`BTreeSet::first()`
— the derive order, with `DanglingFk` before `Convergence` so a precise row removal is
preferred over a lossy rewrite), asks the resolution map for a fix, applies the fix, and
retries. Fixes are one step and recomputed every round. On success the whole cascade lands as
**one history slot**, so a single undo takes the document back. On failure the data is
restored bit-identically from an entry snapshot.

The engine's error behaviour, in full:

| failing op | outcome |
|---|---|
| map says `None`, op is the target | restore, `Err` with the target's **last** broken set |
| map says `None`, op is a fix | **panic** — the map disowned a break its own fix produced |
| `InvalidOp`, op is the target | restore, `Err` — the remembered break if there is one, else the `InvalidOp` |
| `InvalidOp`, op is a fix | **panic** |
| a fix lands equivalent, above, or sideways | **panic** (A.6) |

The **remembered-error rule** carries real weight. When a fix consumes the target's own
target — updating a slot to 23:00, whose only repair is to remove that slot — the user must
be told "this would break the end-of-day rule", not a baffling "invalid slot id" for a slot
they can see on screen.

**These contract panics are not a safety net.** They are instruments for the tests.
Correctness lives in the map's arms; never argue that a mistake is "caught anyway".

Termination rests on two mechanisms, not on a round fuse (there is none, deliberately: no
meaningful bound exists, and a bound loose enough to be safe detects nothing in useful time).
The strictly-below assertion of A.6 bounds the fixes that *land*; a no-progress ledger, keyed
on the invariants picked since the last landing and cleared by every landing, bounds chains
that never land.

Ops whose own payload breaks an invariant are bad **input**, not map bugs, and surface as
`Err`. gtk4 never offers them, but the same op surface is driven by Python and by UI code
racing a stale view, so the path must stay panic-free on data-dependent input.

### A.5 The resolution map: the frame rules

`impl Fixable for Data` (`state-colloscopes/src/resolution.rs`) is an exhaustive match with
**no wildcard arm** — totality is the compiler's business. The whole job of an arm is one
question: *can I remove, from the current state, the thing this invariant complains about?*
If yes `Some(fix)`, if no `None`. What the engine does with `None` is the engine's business.

Five rules govern every arm. They are not style; each was paid for.

1. **Presence, never predicate.** An arm asks whether the material it would remove is
   *there*. It never re-evaluates the invariant's condition, which may depend on the failed
   op's payload and is unknowable from the state. `InterrogationGroupsOutOfBounds(slot, week,
   {3})` asks "is group 3 still in that cell", never "is 3 beyond the group count".
2. **No `expect` on a state lookup — a miss is `None`.** The invariant set was computed on
   `self` *plus the op that just failed*, and that op was rolled back, so a row named by a
   site may simply not exist. Every arm is a chain of `?` lookups. The only `expect`
   permitted is on a sealed-constructor rebuild, where failure is impossible from the value.
3. **`self` is always valid at fix time**, so the ids a fix names are alive. This is what
   makes row-clearing fixes legal even though the dangling target is "gone" — it is not gone
   in `self`. The hole appears only once the retried target finally lands.
4. **The presence test names the target**, not merely "some value is there". The audit
   criterion is a shape you can check by eye: an arm needs an explicit identity test exactly
   when the target id does **not** appear in the op it emits.
5. **Pin the shape you are about to change, not merely its existence.** An invariant names an
   offending *configuration* — a row together with the field values that make it offending.
   Because the failing op is rolled back first, an arm testing only "the row is there" is
   looking at a row that is now innocent. Test only the fields the fix is about to destroy: a
   variant too poor to write that test must be enriched with a richer payload.

★ **Do not reason about what a missing shape test would lead to.** The downstream outcome
varies by arm, and working it out rests on guards in other files that nothing obliges to keep.
The test costs one comparison: write it in every arm, always — including arms whose `Some`
branch is unreachable on today's code.

The repair policy:

1. **Fixes are strictly monotonically decreasing.** Every fix removes a row or an entity,
   clears an optional edge, or rewrites a value *minus* the offending element. Nothing is
   invented; nothing lands equivalent.
2. **Where a targeted single-edge op exists, use it**; otherwise rewrite the whole value
   through the domain's `Update`, reading the current value from the pre-op state.
3. **Remove the reference; remove the entity only when the reference cannot go alone.** The
   test is purely structural: *is the offending reference expressible as absent?* An
   `Option`, a set member or a map-entry value — clear that one thing and the row stays. Only
   a bare mandatory id field, or half a row's key, forces the row to die.
4. **Legacy cleaning semantics are an aspiration, not a gate.** Where the map diverges, the
   divergence is recorded (A.9); it more likely captures an edge case the hand-written
   cleaning forgot than a regression.

One standing carve-out worth knowing, because it looks like a bug: **a colloscope group-list
filling may reference a list that is not associated to any subject×period, and the map must
not "fix" it.** A filling can legitimately be prepared before the association exists. This
does *not* extend to interrogation rows, which do require the association — without one the
group bound saturates to zero, so any placement there is genuinely invalid.

### A.6 The document order, and the obligation it creates

`ContentOrd` (`state/src/partial_order.rs`) materializes the partial order the termination
proof quantifies over. It is **deliberately not `PartialOrd`**: std implements `PartialOrd`
lexicographically on `BTreeSet`/`Vec`/`Option`, under which removing an element can make a
set sort *later*, and those impls cannot be replaced under coherence. No type anywhere gained
a new `PartialOrd`.

The order is **over the document's content, never over the meaning it denotes.** Several
conforming fixes shrink the data while *widening* the semantics — a subject that stops
excluding a dead period now applies more broadly; a slot whose week pattern is cleared now
runs every week. An id was removed and nothing added, so the document strictly decreased. An
order that compared meanings would reject these and break the proof.

**There is no bottom.** Configuration records (`Limits`, `BalancingOptions`, `ExportConfig`)
are atoms: a `None` field means "disabled", an active choice rather than absent content. So
`Default::default()` is *a* minimal element, not *the* one, and a document with an override is
incomparable to the default. Well-foundedness — not a universal minimum — is what termination
needs.

`ContentIdentity` is a marker asserting that `==` coincides with content equivalence. It is
required at container *matching positions* and nowhere else, and enrolment is opt-in and never
automatic: "safe to match by `==`" stays an explicit, auditable assertion.

The engine checks `content_cmp(after, before)` after every fix apply. `Some(Less)` proceeds;
`Some(Equal)`, `Some(Greater)` and `None` all panic. Only fixes are held to it — a no-op
*target* stays a legitimate success.

**The standing obligation for anyone touching `state-colloscopes/`: a new field on an ordered
type is a compile error until it gets a rule, and the rule is a design decision, not
boilerplate.** Ask where the element's identity is borne before reaching for an attribute:

| identity borne by | rule |
|---|---|
| the element's **value** | subsequence (the `Vec` blanket) |
| the element's **position** | prefix + pointwise (`#[ord(with = vec_prefix)]`) |
| **relations between elements** (a chain) | the whole list is one **atom** |

The domain's instances: `SubjectPeriodicity`'s `blocks` is one atom, because each block's
delay is measured from the previous one; prefilled groups and group names are prefix-ordered,
because group numbers are referenced by index from the colloscope; `Incompatibility::slots` is
a subsequence.

### A.7 `ops/`: composites, `Fix`, and where the French lives

A composite op opens a `CascadeSession`, applies its elementary ops one at a time through
`apply_cascade`, accumulates every fix as a `CascadeWarning`, and commits the lot as one
history slot (or cancels). `dry_apply` runs the same thing on a clone for the preview.

The map answers a **`Fix` enum** (25 variants), not a raw op. It is structurally deletive:
creation is unrepresentable, and neither the map nor the translation can reach the id issuer,
so a fix physically cannot carry a fresh id. Two design points do the load-bearing work:

- **Granularity is one variant per *rendered meaning*** — not per invariant, and not per op
  shape. Several invariants collapse into one variant when the sentence the user reads is the
  same: a dead subject and a dead teacher both give `DeleteSlot`. One op splits into two
  variants when the meaning differs: `ClearSlotWeekPattern` means "this slot now runs every
  week", not merely "updated". This is what makes the renderer **a function of `Fix` alone**.
- **Each variant carries everything its op needs**, so `to_annotated_op` is total, pure and
  testable in isolation, and the map stays a pure function of (state, invariant).

`rendering.rs` is a shared, noun-less id-rendering vocabulary — `render_week`, `render_slot`,
`render_group`, … — each taking *the document parts it reads* rather than a whole `Data`,
because no gtk4 panel holds one. That gives the property the module exists for: **a warning
and the screen behind it name the same entity with the same words.**

`warning_text.rs` is private, and matches on `Fix` with no wildcard, so a new fix shape is a
compile error in the renderer. Texts state the **effect only** (« L'interrogation de X sera
supprimée », never « … car son colleur a été supprimé »): the user just performed the action
and does not need to be told what they did. Rendering is lazy and runs against the
composite's *pre-state*, which is the document the user is looking at when the dialog appears.

### A.8 Doctrine for the `ops/` layer

- **The prefix-survival frame rule.** A composite must be written so each of its ops is valid
  against the state produced by its own earlier ops *and their cascades*. A composite whose
  later op is convicted because an earlier op's cascade ate its target is a bug in the
  composite, not bad user input. **Rendering corollary:** a composite's cascades must only
  touch material present in the composite's pre-state — a warning about material the same
  composite just created would be incomprehensible.
- ★ **The growth rule.** Two things pull in opposite directions and must not be conflated.
  **Prechecks must not grow**: an ops-level guard refusing input the cascade would happily
  repair is exactly what the remaster deleted, and re-adding one under a new error name puts
  the cleaning phase back by the side door. But **a panic on reachable input must be dealt
  with**: where a state-layer break can reach a residual catch-all `panic!`, the family's
  error vocabulary *gains a variant*. A crash is not a contract. The test is **reachability,
  not taste**.
- **The audit rule that makes the growth rule mechanical.** The engine rolls the failing
  target back *before* asking the map, and every arm is a presence test. So an invariant
  broken by a target's **own written content** always finds its material gone, answers `None`,
  and surfaces as that family's `BrokenInvariants` — while an invariant broken by invalidating
  **pre-existing** material survives the rollback and is repaired. Per family the question is
  finite: enumerate the reference sites originating in the row the family writes, plus the
  convergence predicates whose fields it writes, and check each is scanned or excluded by an
  address check.
- **The allocator is outside rollback** (★). The manager's rollback-managed state is
  *document + history*; the id issuer is monotone across the manager's whole life. `Err` means
  data and history unchanged, allocator possibly advanced. Burned ids never appear anywhere.
- **`UpdateError` scan order is public API.** The per-family translation of state-layer errors
  lives at the call sites and reproduces the old validator's first-error order, pinned by
  `ops/tests/assignments_error_surface.rs`. Python has roughly eighty exception-matching sites
  keyed on this vocabulary.
- **The hand-written-warning door is shut.** Every warning is a cascade `Fix`; no composite
  emits one of its own. Re-opening the door needs a fresh ruling.
- **Names split by layer** (★). Elementary ops are elementary: `PeriodOp::Remove` removes the
  period and nothing else, and its name must say no more than it does — the dangling weeks are
  a `DanglingFk` the cascade repairs. The semantic fact that a user deleting a period expects
  its weeks to go belongs to the user-facing layer and to its name:
  `GeneralPlanningUpdateOp::DeletePeriodAndWeeks` says the weeks go, and **authors their
  removal itself**. Authoring is what keeps the warning list down to the genuinely surprising
  effects, instead of one line per week restating the request.
- **A `Table` stays inside `state/`.** Ops never carry one through their payloads; consumer
  snapshots convert explicitly.
- **The map arm obligation.** A new map arm needs a `Fix` variant, and the variant is chosen
  by the sentence the user will read — not by which invariant fired and not by which op
  performs the repair. Then the renderer stops compiling until the new meaning has words,
  which is the intended order of events.

### A.9 Behaviour that differs from the pre-cascade application

All deliberate, all recorded when they landed:

1. **Merging periods preserves colloscope data**, closing a long-standing FIXME. Weeks move
   with their cells; only genuinely invalidated cells are cleared, and warned about.
2. **A subject update that makes a slot overflow its day no longer aborts** — the cascade
   removes the overflowing slot and warns. On a *slot* update naming a bad start time the
   rejection survives, because there the offending content is the target's own.
3. **Deleting a week pattern keeps the slots and incompatibilities that referenced it**, with
   `week_pattern = None`, where the old code deleted them and their colloscope data.
4. **Period deletion and merge no longer reconcile exclusion sets** — the dead period's
   members are simply dropped. Same end state, differently phrased warnings.
5. **Four crashes became errors**: a dead period id on `DeletePeriodAndWeeks`, balancing
   options on a subject whose interrogations are off, a teacher op naming a no-interrogation
   subject, and a colloscope group-list row aimed at a prefilled list.
6. **Warning granularity is finer**: one entry per fix op, in application order, where the old
   code emitted deduplicated coarse statements. gtk4 dedups exact-equal texts only.

One read-surface convention that survives and bites: `Parameters::count_weeks` reads the week
*table* while `walk`/`week_ids` iterate the period-keyed ordering. On a valid state they
agree; on a broken one they do not, because an orphan week is counted but never walked. Never
mix the two conventions off an unvalidated state.

### A.10 The safety net

- **`state-colloscopes/tests/property_ops.rs`** — the original oracle: generated elementary-op
  sequences, with `broken_invariants() == Ok(∅)` after every apply, undo-all identity, and
  redo stability. 100 seeds committed; a 500-seed crank is the milestone reference.
- **`property_apply_gate.rs`** — depth-1 corruption probes off a validated walk, asserting
  atomicity (every `Err` leaves the state bit-identical), honesty (`Ok` implies clean and an
  exactly-restoring inverse), and coverage.
- **`property_cascade.rs`**, **`property_content_ord.rs`** — the cascade fuzz, and the
  strictly-below contract checked over every invariant in a reported set, not just the pick.
- **`ops/tests/property_update_ops.rs`** — 100 seeds × 500 random composites, asserting no
  panic, a valid result, and that **every** warning renders against the true pre-state.
- **`resolution/innocent_tests.rs`** — one `None` test per comparison, on a valid document
  with an invariant derived from a corrupted twin. These are what mechanically catch a missing
  identity or shape test; the `Ok`-route fixtures cannot see one.
  **`resolution/attribution_tests.rs`** calls `fix_invariant` directly to pin which `Fix` each
  invariant maps to — the seam the map became unit-testable at when it started answering
  `Fix` instead of a raw op. The `ops/` fixtures deliberately do not pin attribution.
- **`found_bugs.rs`** in both crates — every bug the harnesses found, pinned test-first.
- Storage byte-stability, the golden `examples/` fixtures, and the three Python contract
  scripts of §7 (run by the user).

Testing rules that outlive the plan: derive expected op lists **by hand before the test
runs**; an ordered literal is a tripwire on a derived `Ord`, not a confluence pin, so assert
sequence only where the engine really chose; make a fixture fail on its *last* conjunct, so a
regression cannot go green for the wrong reason; a green fuzz run proves nothing without a
cross-seed guard counting the specific outcome it claims; and a pin that passes on its first
run proves nothing until the code under it has been broken and watched go red.

### A.11 Known gaps and standing future work

- **Test coverage is not exhaustive** — widening it has been a standing item since step 5.
  Notably, the resolution map's `None` branches are covered arm by arm, while the `Some`
  branches are not covered systematically. That asymmetry was a deliberate decision.
- **gtk4 shows warnings as a flat `Vec<String>` dialog.** A richer warning window is separate
  work; the `Fix` values it would need are already there.
- **Python discards warnings entirely.** The Python API revamp is separate work, expected
  alongside the MVVM UI work.
- **`RefSite` reverse lookups (`references_to_*`) are still unretired**, pending a gtk4 claim
  on a "who references X?" display. The cascade itself does not need them.
- **Any future data reshape beyond the step-1 shapes needs its own format review.** The
  spec-2 format is frozen and was shaped to absorb those reshapes; it does not automatically
  absorb the next one.
