# File format: implementation notes

Companion to `file_format.md`, which specifies the Collomatique file format in the
abstract. This document covers everything specific to the Collomatique codebase:
where the code lives, how validation is layered, error reporting, the writer's
choices among spec-conforming alternatives, and the history and rationale behind
the format's design. Nothing here is needed to read or write a conforming file.

The broader context (why the format was redesigned, phase ordering, migration
plan) is in `state_consolidation_plan.md`.

## Code layout

Everything lives in the `storage/` crate:

- `storage/src/json.rs` — envelope types (`JsonData`, `Header`, `Entry`) and
  `CURRENT_SPEC_VERSION`.
- `storage/src/format/` — one module per block, containing **dumb format structs**
  that mirror the spec's JSON shapes exactly (serde derives, no methods beyond
  conversion). These structs are the spec made executable; they never appear
  outside the storage crate.
- `storage/src/decode.rs` / `storage/src/encode.rs` — conversion between the
  format structs and the in-memory model
  (`collomatique_state_colloscopes::InnerData`).

The in-memory model itself lives in `state-colloscopes/`; the format structs are
deliberately decoupled from it so that in-memory refactors never touch the format.

## Validation layers

Loading a file goes through three layers with a clear trust boundary:

1. **JSON structure** (serde, on the format structs): the envelope shape, block
   name recognition, record strictness (`deny_unknown_fields`, no `serde(default)`
   on spec-2 structs), and local scalar constraints — time/date syntax,
   Monday-ness, non-empty strings, `min <= max`, non-zero integers, duplicated
   keys in keyed collections. (Neutral entries in derived-key-set collections —
   empty `students`/`slots`/`assigned_groups` rows — are valid per spec; they
   decode to the same state as their absence.)
2. **Reconstruction** (format structs → `InnerData`): the decoder rebuilds what
   the file deliberately omits —
   - default state for absent blocks;
   - the complete forced key sets (an assignments map entry for every period ×
     non-excluded subject, a slots entry for every interrogation subject, a
     colloscope group-list entry for every automatic list), inserting empty
     values for absent rows;
   - the colloscope interrogation skeleton: each slot's per-week `Some`/`None`
     cells, from the merged pattern (period week flags AND slot week pattern, via
     `Parameters::merge_pattern`) — sparse-row placement errors (unknown slot,
     out-of-range or inactive week) surface here;
   - the id issuer, seeded past the maximum id in use (an exhausted id space is
     the `EndOfTheUniverse` error).
3. **Invariants** (`Data::from_inner_data`, in `state-colloscopes`): global id
   uniqueness and every cross-block referential constraint of the spec's §4 —
   dangling ids, week-pattern lengths, teacher/subject compatibility,
   group-number bounds, colloscope-vs-parameters consistency. The authoritative
   lists are the `InvariantError` and `ColloscopeError` enums in
   `state-colloscopes/src/lib.rs`.

Layer 3 is the **single trust boundary**: it validates any `InnerData` regardless
of provenance, so the decoder never needs to be trusted for semantic integrity —
even where it happens to catch a problem earlier, the invariant layer would catch
it too.

## Errors and diagnostics

Decoding failures are reported through `DecodeError` (`storage/src/decode.rs`);
partial-success situations through `Caveat`:

- `Caveat::CreatedWithNewerVersion` — header `produced_with_version` is newer than
  the application.
- `Caveat::UnknownEntries` — unrecognised blocks with `needed_entry: false` were
  skipped (and will be dropped on re-save).
- `DecodeError::UnknownNeededEntry` — an unrecognised block with
  `needed_entry: true`.
- `DecodeError::MismatchedSpecRequirementInEntry` — a recognised block declaring
  non-canonical `minimum_spec_version`/`needed_entry`.

**Diagnostics requirement.** When a block declaring
`minimum_spec_version <= CURRENT_SPEC_VERSION` fails to parse, the error must
surface the underlying serde diagnostics (field name, expected type, position).
The spec-1 implementation had a bug here worth remembering: the hand-written
`EntryContent::deserialize` captured the payload as a `RawValue`, tried the typed
parse, and mapped *any* failure to `UnknownEntry` — so a typo in a known block was
misreported as `ProbablyIllformedEntry` with no detail. The spec-2 decoder must
distinguish "unknown block name" (tolerance rules apply) from "known block, bad
payload" (fail with the serde error).

## Writer policy

The spec deliberately allows several encodings of the same state (absent vs
explicit default blocks, collection order); any choice is conforming. Our writer
produces the spec's **canonical form** (`file_format.md` §3): blocks in default
state omitted, neutral entries in derived-key-set collections omitted, canonical
block order, sorted collections, 2-space pretty-printing
(`serde_json::to_string_pretty`).

Omitting default-state blocks is not just cosmetic: it means a file demands the
spec level of the features actually *used*, not of the application that wrote it.
When a future spec adds a block, documents that never touch the feature keep
opening cleanly in older versions.

Byte stability — the same state always serialises to the same bytes — is a
regression-tested guarantee, pinned by the golden test
`storage/tests/populated_round_trip.rs::reserialize_is_stable`.

## Spec 1: history and migration

Spec 1 had a single block, `InnerDataDump`, whose payload was a raw serde dump of
the live `InnerData` type. It only ever existed during pre-alpha development (all
of it under version 0.1.0) and had two fatal flaws: the format changed whenever
the in-memory type did, and defaults (`serde(default)`) silently patched over
missing data.

Migration, strictly in this order (see `state_consolidation_plan.md` §4):

1. implement spec 2 (read + write) while the v1 decoder still works;
2. bulk-convert every existing file (private test files, everything the Python
   import scripts produced) via a one-time conversion command;
3. delete the v1 decoder, leaving only a tombstone: a file containing an
   `InnerDataDump` block fails with a dedicated "unsupported pre-alpha development
   format" error rather than a generic parse failure;
4. only then is `InnerData` free to change (the v1 decoder read the live type, so
   any earlier change would have broken it).

Old spec-1 files get no long-term reading path; nothing released ever produced
them.

## Evolution rationale

The spec's forward-compatibility rules (`file_format.md` §5) have implementation
consequences:

- **Frozen block names = frozen decoders.** When a future block name supersedes
  `Subjects`, the `Subjects` format struct and its conversion are kept forever,
  converting into the *current* in-memory model. This is the one place defaults
  are legitimate: an old block cannot express newer state, and the decoder for
  the old name fills the gap explicitly.
- **Frozen defaults.** An absent block must mean the same state forever, and
  omit-on-write makes absent blocks common. Changing a block's default is a shape
  change: new block name, new spec number.
- **Adding an optional feature** costs one new format module and no spec bump for
  existing blocks; readers older than the feature skip it (`needed_entry: false`)
  or refuse the file (`true`) exactly as specified.

## Test corpus

Phase 1.5 of `state_consolidation_plan.md` adds themed example files under
`examples/` (fully populated, human-readable documents). They serve as golden
fixtures for the decoder, integration-test inputs, and user-facing documentation
of the format.
