# State consolidation plan

Status: **agreed roadmap** (July 2026, branch `consolidate_state`).
Scope: the state stack — `state/`, `state-colloscopes/`, `ops/`, `storage/` — and the on-disk file format.

This document records the architecture review of the state stack, the decisions taken, and the
phased plan to (1) give Collomatique a real, version-stable file format and (2) simplify the
in-memory state machinery behind it. It is meant to survive across sessions: read this first
before touching any of the crates above.

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
`state/`); §§3–6 track what changed.*

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
2. **Invariant double duty** *(being addressed — see `docs/invariant_cascade_design.md`)*:
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
4. **Write fan-out from the params↔colloscope mirror** *(dissolves at step 1d of
   `docs/invariant_cascade_design.md`)*: e.g. adding a slot must insert an
   empty `ColloscopeSlot` into every colloscope period; period ops span ~330 lines. Every
   structural op mutates two parallel representations by hand.
5. **Inconsistent op granularity**: whole-struct `Update`s (Settings, Balancing, entities)
   next to 11 per-field `ExportConfigOp` variants and a single-bool `AssignmentOp`. No
   principle.
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
field-level reference lives in `docs/file_format.md`.

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
  `docs/file_format.md`, derived from the recovered `SCHEMA_SQL` and the format structs.

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
   `docs/invariant_cascade_design.md`): generic `Table`/`OrderedTable` in `state/` + the
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
3. **Invariant consolidation** — **SUPERSEDED, direction reversed** (July 15 2026) by
   `docs/invariant_cascade_design.md`, now the live roadmap: `check_invariants` becomes the
   *sole* enforcement — each elementary op is apply → check → rollback-on-failure, returning
   precise coordinate-bearing errors — and the per-op typed preconditions retire (the exact
   opposite of the demotion sketched below). The extended-scope note ("reroute the triplicated
   checks through the item-2 registry") and the registry plan's §6 hand-off notes are
   superseded with it: two of the three check families are deleted, not rerouted.
   *Original sketch (historical):* keep per-op precondition checks (typed errors); demote the
   whole-model `check_invariants()` after every op to `debug_assertions` and the phase-0
   property tests; full checks remain only at trust boundaries (file load, `GlobalUpdate`).
   Collapse `InvariantError`'s duplicated variants onto the per-entity error enums.
4. **Uniform op granularity**: every entity gets `Add / Remove / Update(whole entity)`
   (+ position ops where user-visible order exists, + association ops where relational).
   Collapse `ExportConfigOp`'s 11 per-field variants into one `Update`. Elementary ops only
   need to be *reversible and replayable*; user-facing granularity already lives in `ops/`
   descriptions. *Note (July 2026)*: independent of, but interacting with, the
   invariant-cascade design (its §9) — the resolution map wants elementary ops that can
   express "remove/clear this one reference" conveniently, and the cascade's step-1 reshapes
   re-cut the slot/week/colloscope op surfaces anyway; granularity uniformization can ride
   along per step or stay a later pass.
5. **Params↔colloscope synchronization** — **DISSOLVED** (July 15 2026) by the
   invariant-cascade design (step 1d + the cascade): the colloscope goes sparse, so there is
   no fan-out left to centralize — cleanup becomes cascade resolution. The spec-2 format was
   deliberately shaped for exactly this re-keying and does not move.
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
7. **Python glue**: the write path is already insulated behind `ops::UpdateOp`; the read-path
   pyclass mirrors break compile-visibly and are regenerated mechanically per struct change.
   No Python API redesign in this phase (a full redesign is expected later, with the MVVM UI
   work).

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

- Phase 2 item 2 (`Table` + relationship registry) — **DONE (July 16 2026)**; see §6 item 2.
  The detailed plan doc is retired (pinned at `git show 77695338:docs/table_registry_plan.md`);
  the inventories it carried live on as Appendix A of `docs/invariant_cascade_design.md`.
- The live roadmap for the remaining phase-2 work is **`docs/invariant_cascade_design.md`**
  (agreed July 15 2026), a 7-step plan: reshape the dense copies (1a assignments sparse,
  1b `WeekId`, 1c slots no-reshape, 1d colloscope sparse) → precise checker alongside the old
  one → completeness audit → differential fuzz → switch elementary ops to
  apply/check/restore → the cascade → the `ops/` remaster. It supersedes item 3 (direction
  reversed) and dissolves item 5; item 4 can ride along its reshapes. Next concrete work: the
  step-1 session plans, starting with 1a.
