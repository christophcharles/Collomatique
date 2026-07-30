# Pre-step-7 review fixes: state/, state-colloscopes/, storage/

## Context

Steps 1–6.5 of the invariant-cascade roadmap are complete. Before starting step 7 (the
`ops/` remaster), a five-agent review of `state/`, `state-colloscopes/` and `storage/` was
run against the design record (`docs/plans/invariant_cascade_design.md`, Appendices A–I)
and the format spec (`docs/file_format/file_format.md`). Every serious finding was
re-verified by reading the code directly.

The verdict: no corruption-class or logic bug anywhere in the in-memory layer. The
resolution map, the checker (28/28 relationship coverage), the sealed types and the paired
structures all match their design record. The real defects are two storage decode holes
(spec violations), one spec/code drift on week-pattern length, a termination hole in the
cascade engine for fix chains that never land, and two gate-hardening warts. The rest is
documentation drift, dead code, vocabulary asymmetry and test gaps.

The user reviewed the findings and ruled:

- Everything is fixed **before step 7** — including items originally proposed for the
  step-7 register. Step 7 starts on a clean base.
- `#[ord(total)]`'s well-foundedness obligation is a **documentation fix only** — the
  attribute is useful and the obligation cannot be checked mechanically.
- The `IdIssuerHelper::get_new_id` finding is **withdrawn** — the absence of a guard is
  intended. `new`/`skip_to_id` reject anything past `u64::MAX >> 1`, which *guarantees*
  at least 2^63 issuances of headroom; even issuing one id per CPU cycle could not
  exhaust it in the lifetime of the program. The point is precisely that `get_new_id`
  needs no error path. Do not touch `state/src/tools.rs`.
- `GroupList::students` is confirmed dead (workspace-wide grep: the only `fn students`
  definitions are this one and `Assignments::students`, and every `.students(` call site
  is the two-argument assignments one) — **delete it** rather than fix its doc.

Two design decisions were resolved during planning:

- **Week-pattern length**: the decoder will **enforce** the spec's exact-length rule
  rather than the spec being weakened. The writer has always emitted full-length arrays
  (it projects `excluded_weeks` positionally via `walk`), so no legitimate file is
  rejected by the change, and the spec's strictness principle ("records are exact") wins.
- **Payload-id tier consistency** (ruling revised in discussion, superseding the first
  draft of this plan): the asymmetry between `AssignmentOp::SetRow` (which sweeps its
  payload student ids) and `ColloscopeOp::SetGroupList` (which does not) is resolved by
  **removing the `SetRow` payload sweep**, not by adding one to `SetGroupList`. The
  principle: a precheck resolves the op's *address* (the coordinates that say where in
  the document the op acts — dead addresses stay rejected everywhere in the domain,
  consistently with the op-target-existence family), while the op's *content* — ids the
  op writes into the document — belongs to the dangling-FK net. Prechecking content
  duplicates the checker. The FK route is safe against spurious fixes by construction:
  the gate rolls the failing target back *before* the resolution map is consulted, the
  pre-state is valid so the bad material is absent from it, every `Assignments*` map
  arm's presence test (`resolution.rs:188/:292/:376`) then answers `None`, and the
  engine reports the remembered break (the conviction route). `SetRow` *keeps* its two
  key checks (period, subject): the key pair is the op's address, and with an empty
  payload (`SetRow(key, ∅)` clears the row) nothing lands in the document, so a dead
  address there is the one case the FK net is structurally blind to. This does not touch
  the cascade path for `StudentOp::Remove`: that one flows through `DanglingFk @
  AssignmentsStudent` computed on the *live* state and never met the precheck.

General notes for the implementer:

- No commit in this plan adds a dependency, so `Cargo.lock` never changes and no Nix
  `cargoHash` refresh is needed.
- Bug fixes follow the test-first workflow: the failing regression test is committed
  alone (verified failing), then the fix is a separate commit.
- Never run the test suite twice in one command; capture output once to a scratchpad
  file and grep it. Full `cargo test --workspace` runs go in the background.
- `gtk4/src/.../file_loader.rs` matches `DecodeError` exhaustively — every new or
  reshaped `DecodeError` variant needs a French message arm there. Similarly, reshaped
  precheck variants may be matched in `ops/` or `gtk4/`; let the compiler enumerate the
  sites and fix them mechanically (do not reshape `ops/` logic — that is step 7).

---

# Part A — storage decode: spec compliance (test-first pairs)

## Commit A1 — failing regression tests: neutral rows escape key validation

The spec says keys outside a derived key set are invalid regardless of row content
(`docs/file_format/file_format.md:143`: "Keys outside that set are invalid"), and a
neutral (empty) entry is only the redundant spelling of an *absent* row when its key is
*inside* the set. Two decode functions normalize the neutral row to absence **before**
validating the key, so an invalid file decodes cleanly.

Add four tests to `storage/tests/spec2_format.rs`, modeled on the file's existing
malformed-input tests (reuse its helpers for building a minimal valid document and
asserting a `DecodeError`):

1. A `Slots` block row `{"subject_id": 9999, "slots": []}` where no subject 9999 exists.
   Expect an error (the exact variant is introduced in A2; until then assert
   `deserialize_data(...).is_err()`).
2. A `Slots` block row with an empty `slots` list keyed by a subject that exists but has
   `interrogation_parameters: null` (the derived key set for §4.7 is subjects *with*
   interrogations). Expect an error.
3. An `Assignments` block row `{"period_id": <valid>, "subject_id": 9999, "students": []}`
   with an unknown subject. Expect an error.
4. The same with a subject that exists but lists that period in `excluded_periods` (the
   §4.5 derived key set is period × subject-not-excluded). Expect an error.

Run the four tests, confirm all four **fail** (the current decoder accepts every one),
and commit the tests alone.

## Commit A2 — fix: validate the key before the emptiness drop

`storage/src/decode/spec2.rs`. The assignments function already does this for the period
half of its key — with the correct rationale in the comment — and the subject half was
missed. Old code (`reconstruct_assignments`, lines 504–520):

```rust
    for row in block.into_inner() {
        let period_id = id::<PeriodId>(row.period_id);
        if periods.find_period_position(period_id).is_none() {
            // An empty assignments row keyed by an unknown period decodes to an
            // absent row (canonical-absent rule) and would otherwise vanish
            // silently before the final gate can see it, so it is rejected here.
            return Err(DecodeError::UnknownPeriodInAssignments(row.period_id));
        }
        let students = id_set(row.students);
        if students.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        // A row on an unknown or excluded subject is inserted anyway:
        // layer 3 rejects it
        entries.push(((period_id, id(row.subject_id)), students));
    }
```

New code — the subject half gets the same treatment before the drop. The function needs
the decoded subjects available; extend its signature with `subjects:
&mem::subjects::Subjects` (the decode pipeline builds subjects before assignments —
verify the call order in `decode` and pass the already-built value):

```rust
    for row in block.into_inner() {
        let period_id = id::<PeriodId>(row.period_id);
        if periods.find_period_position(period_id).is_none() {
            // An empty assignments row keyed by an unknown period decodes to an
            // absent row (canonical-absent rule) and would otherwise vanish
            // silently before the final gate can see it, so it is rejected here.
            return Err(DecodeError::UnknownPeriodInAssignments(row.period_id));
        }
        // Same reasoning for the subject half of the key: an empty row keyed
        // outside the derived set (unknown subject, or subject excluded from
        // this period) would vanish before the final gate can see it (§4.5).
        let subject_id = id::<SubjectId>(row.subject_id);
        let Some(subject) = subjects.find_subject(subject_id) else {
            return Err(DecodeError::UnknownSubjectInAssignments(row.subject_id));
        };
        if subject.excluded_periods.contains(&period_id) {
            return Err(DecodeError::AssignmentOnExcludedPeriod {
                period_id: row.period_id,
                subject_id: row.subject_id,
            });
        }
        let students = id_set(row.students);
        if students.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        entries.push(((period_id, subject_id), students));
    }
```

Note the check now runs for *every* row, not only empty ones. That is deliberate and
matches the period half: for non-empty rows it merely front-runs layer 3 with a sharper
diagnostic (the old comment "layer 3 rejects it" stays true as the backstop, per the
single-trust-boundary doctrine in `storage/src/decode.rs`). Verify `Subject` exposes
`excluded_periods` to this crate (the field is read as `subject.excluded_periods`
elsewhere in decode; mirror whatever access path `reconstruct_*` already uses).

Same shape in `reconstruct_slots`. Old code (lines 562–596, elided middle):

```rust
fn reconstruct_slots(block: format::slots::Slots) -> Result<mem::slots::Slots, DecodeError> {
    ...
    for row in block.into_inner() {
        let subject_id = id::<SubjectId>(row.subject_id);
        let ordered_slots: Vec<(SlotId, mem::slots::Slot)> = row
            .slots
            ...
            .collect();
        if ordered_slots.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        rows.insert(subject_id, ordered_slots);
    }
```

New code — take `subjects: &mem::subjects::Subjects` as a parameter and validate the key
first (the §4.7 derived key set is "subjects with interrogations"):

```rust
    for row in block.into_inner() {
        let subject_id = id::<SubjectId>(row.subject_id);
        // The derived key set is "subjects with interrogations" (§4.7). An
        // empty row keyed outside it would decode to absence and vanish
        // before the final gate can see it, so it is rejected here.
        let Some(subject) = subjects.find_subject(subject_id) else {
            return Err(DecodeError::UnknownSubjectInSlots(row.subject_id));
        };
        if subject.parameters.interrogation_parameters.is_none() {
            return Err(DecodeError::SlotsForSubjectWithoutInterrogations(
                row.subject_id,
            ));
        }
        let ordered_slots: Vec<(SlotId, mem::slots::Slot)> = row
            .slots
            ...
            .collect();
        if ordered_slots.is_empty() {
            // Neutral row: drop it to keep the canonical (absent) form.
            continue;
        }
        rows.insert(subject_id, ordered_slots);
    }
```

Also update the function's header comment, whose last sentence ("Rows on subjects
without interrogations are inserted anyway: layer 3 rejects them") is superseded.

New `DecodeError` variants in `storage/src/decode.rs`, next to
`UnknownPeriodInAssignments` (keep its phrasing style):

```rust
    #[error("The assignments reference an unknown subject (subject id {0})")]
    UnknownSubjectInAssignments(u64),
    #[error(
        "The assignments have a row for subject id {subject_id} on period id {period_id}, but the subject is excluded from that period"
    )]
    AssignmentOnExcludedPeriod { period_id: u64, subject_id: u64 },
    #[error("The slots reference an unknown subject (subject id {0})")]
    UnknownSubjectInSlots(u64),
    #[error("The slots have a row for subject id {0} which has no interrogations")]
    SlotsForSubjectWithoutInterrogations(u64),
```

Then: tighten the A1 tests to assert the specific variants; add French message arms in
gtk4's `file_loader.rs`; run the storage suite plus byte-stability
(`populated_round_trip`) and the examples pristine-load test — the fixes are decode-side
only, so writer output must be byte-identical.

## Commit A3 — failing regression tests: week-pattern bitmask length

Spec `docs/file_format/file_format.md:403-404`: "`weeks` has exactly one element per week
of the schedule … — no shorter, no longer." The decoder currently zips and tolerates both
directions. Two tests in `storage/tests/spec2_format.rs`: a document with a 7-week
schedule and a `WeekPatterns` row whose `weeks` has 1 element (too short), and one whose
`weeks` has 8 (too long). Both must fail decode; both currently pass. Commit the failing
tests alone.

## Commit A4 — fix: enforce the exact length

`storage/src/decode/spec2.rs`, `reconstruct_week_patterns`. Old code (lines 527–560,
comment and zip):

```rust
    // The frozen positional bitmask carries one bit per week in global walk
    // order; a `false` bit excludes that week. Zipping against the walk order
    // maps each bit back to its synthesized week id (and gracefully ignores any
    // trailing bits past the schedule — such a file is rejected by layer 3).
    let week_ids: Vec<WeekId> = weeks
        .walk(periods)
        .map(|(_period_id, week_id, _week)| week_id)
        .collect();
    mem::week_patterns::WeekPatterns {
        week_pattern_map: block
            .into_inner()
            .into_iter()
            .map(|week_pattern| {
                let excluded_weeks = week_ids
                    .iter()
                    .zip(week_pattern.weeks)
                    .filter_map(|(&week_id, active)| (!active).then_some(week_id))
                    .collect();
                ...
```

The parenthetical is false: since commit `d169df71` (B2) the in-memory `WeekPattern` has
no length invariant, so layer 3 has nothing to reject — surplus bits vanish in the zip
and short arrays silently default to active. New code — the function becomes fallible and
checks the length up front:

```rust
fn reconstruct_week_patterns(
    block: format::week_patterns::WeekPatterns,
    weeks: &mem::weeks::Weeks,
    periods: &mem::periods::Periods,
) -> Result<mem::week_patterns::WeekPatterns, DecodeError> {
    // The frozen positional bitmask carries one bit per week in global walk
    // order; a `false` bit excludes that week. The spec (§4.6) requires
    // exactly one element per week of the schedule — no shorter, no longer —
    // and the in-memory type has no length to re-check later, so the length
    // is enforced here.
    let week_ids: Vec<WeekId> = weeks
        .walk(periods)
        .map(|(_period_id, week_id, _week)| week_id)
        .collect();
    let week_pattern_map = block
        .into_inner()
        .into_iter()
        .map(|week_pattern| {
            if week_pattern.weeks.len() != week_ids.len() {
                return Err(DecodeError::WrongWeekCountInWeekPattern {
                    week_pattern_id: week_pattern.id,
                    expected: week_ids.len(),
                    found: week_pattern.weeks.len(),
                });
            }
            let excluded_weeks = week_ids
                .iter()
                .zip(week_pattern.weeks)
                .filter_map(|(&week_id, active)| (!active).then_some(week_id))
                .collect();
            Ok((
                id::<WeekPatternId>(week_pattern.id),
                mem::week_patterns::WeekPattern {
                    name: week_pattern.name,
                    excluded_weeks,
                },
            ))
        })
        .collect::<Result<_, _>>()?;
    Ok(mem::week_patterns::WeekPatterns { week_pattern_map })
}
```

New variant in `storage/src/decode.rs`:

```rust
    #[error(
        "Week pattern id {week_pattern_id} has {found} week entries but the schedule has {expected} weeks"
    )]
    WrongWeekCountInWeekPattern {
        week_pattern_id: u64,
        expected: usize,
        found: usize,
    },
```

Fix the caller in `decode` (the function's result is now `Result`), and fix the false
doc comment on the format struct, `storage/src/format/week_patterns.rs:17-21`. Old:

```rust
    /// Positional: `weeks[w]` = pattern active on global week `w`. Well-formed
    /// files carry exactly one element per week of the schedule (as produced by
    /// the encoder); decode maps each bit to its week in global order, so any
    /// surplus bits are ignored and missing ones default to active.
    pub weeks: Vec<bool>,
```

New:

```rust
    /// Positional: `weeks[w]` = pattern active on global week `w`. Exactly one
    /// element per week of the schedule (spec §4.6 — no shorter, no longer);
    /// decode rejects any other length, since the in-memory type keeps only
    /// the exclusion set and could not re-check a length later.
    pub weeks: Vec<bool>,
```

gtk4 French arm for the new variant. A3's tests now pass; byte-stability and pristine
examples must stay green (the writer already emits exact-length arrays).

## Commit A5 — failing regression tests: envelope accepts unknown fields

Spec §3: a record "with a missing field or an unknown field is invalid"; the envelope
header and entry are fixed-field records (§2). Two tests (place them next to the header
tests — `storage/tests/header_check.rs` exists and already pins header behavior): a
document whose `header` object carries an extra `"junk": 1` field, and one whose entry
carries a fourth field. Both must fail; both currently decode. Commit failing.

## Commit A6 — fix: `deny_unknown_fields` on the envelope

`storage/src/json.rs`. Old (lines 19–30 and 51–70, the four deserializable structs):

```rust
#[derive(Debug, Deserialize)]
pub struct RawJsonData {
    pub header: Header,
    pub entries: Vec<RawEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RawEntry {
    pub minimum_spec_version: u32,
    pub needed_entry: bool,
    pub content: Box<serde_json::value::RawValue>,
}
...
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header { ... }
...
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Version { ... }
```

New: add `#[serde(deny_unknown_fields)]` to all four (`RawJsonData`, `RawEntry`,
`Header`, `Version`). `RawEntry.content` being a `RawValue` is compatible with the
attribute (it is a named field, not a flatten). Note the resulting error is serde-level
(`DeserializationError::InvalidJson`-family, matching how other envelope malformations
already surface) — assert that in the A5 tests.

---

# Part B — storage diagnostics and test hardening

## Commit B1 — error-surface diagnostics

Three related diagnostic improvements in `storage/src/decode.rs` + `storage/src/json.rs`,
plus the matching gtk4 arms. No test-first pair — behavior (accept/reject) is unchanged;
only diagnostics improve. Add or adjust unit tests in the same commit.

**(a) Duplicate ids name the block and the id.** Old (`decode.rs:65-66`):

```rust
    #[error("Duplicated ID")]
    DuplicatedID,
```

Keep `DuplicatedID` for the global cross-kind check (the `From<FromInnerDataError>` arm,
where `IdError::DuplicatedId` genuinely carries no id), and add:

```rust
    #[error("Duplicated ID {id} in block {block:?}")]
    DuplicatedIdInBlock { block: &'static str, id: u64 },
```

Reroute the four in-block sites, whose inner error types all carry the offending id:
`decode/spec2.rs:371` (`Periods::from_ordered_ids`, error wraps
`DuplicatedPeriodIdError(id)`), `:372` (`Weeks::from_period_rows`), `:397`
(`OrderedTable` `try_into`, error is `DuplicatedIdError<I>(pub I)`), `:601`
(`Slots::from_subject_rows`, error is `DuplicatedSlotIdError(pub SlotId)`). Pattern:

```rust
    let periods = mem::periods::Periods::from_ordered_ids(first_week, period_ids)
        .map_err(|e| DecodeError::DuplicatedIdInBlock {
            block: "GeneralPlanning",
            id: e.0.inner(),
        })?;
```

(Use each block's canonical name string as it appears in the block tagging; use the id
type's `inner()` accessor.)

**(b) Spec-requirement mismatch names the entry.** Old (`decode.rs:32-33`):

```rust
    #[error("An entry has the wrong spec requirements")]
    MismatchedSpecRequirementInEntry,
```

New — carry the block name where the check fires (it fires in `collect_blocks` on a
*recognized* block, so the name is at hand; verify at the construction site in
`decode/spec2.rs` and thread the `&'static str` through):

```rust
    #[error("Entry for block {0:?} has the wrong spec requirements")]
    MismatchedSpecRequirementInEntry(&'static str),
```

**(c) Unknown file_type vs unknown file_content.** Today an unrecognized `file_content`
is tolerated at parse (`FileContent::UnknownFileContent(serde_json::Value)`) and surfaces
as the misnamed `DecodeError::UnknownFileType(Version)` (its payload is the writer's app
version, not the offending value), while an unrecognized `file_type` string fails serde
inside `RawJsonData` and surfaces as generic invalid-JSON. Make the two symmetric. In
`json.rs`, mirror the untagged-unknown pattern for `FileType`. Old (lines 106–121):

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Collomatique,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileContent {
    ValidFileContent(ValidFileContent),
    UnknownFileContent(serde_json::Value),
}
```

New:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileType {
    ValidFileType(ValidFileType),
    UnknownFileType(serde_json::Value),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidFileType {
    Collomatique,
}
```

(The serializer side must keep emitting exactly `"Collomatique"` — untagged serializes
the inner variant transparently; pin with a writer byte test if not already covered by
byte-stability.) In `decode.rs`, rename the misnamed variant and check both fields in
`check_header`. Old (lines 28–29 and 112–127):

```rust
    #[error("Unknown file type - this might be from a more recent version of Collomatique")]
    UnknownFileType(Version),
...
pub(crate) fn check_header(
    header: &Header,
    caveats: &mut BTreeSet<Caveat>,
) -> Result<(), DecodeError> {
    if let FileContent::UnknownFileContent(_value) = &header.file_content {
        return Err(DecodeError::UnknownFileType(
            header.produced_with_version.clone(),
        ));
    }
    ...
```

New:

```rust
    #[error(
        "Unknown file type - this might be from a more recent version of Collomatique (file written by version {0})"
    )]
    UnknownFileType(Version),
    #[error(
        "Unknown file content - this might be from a more recent version of Collomatique (file written by version {0})"
    )]
    UnknownFileContent(Version),
...
    if let FileType::UnknownFileType(_value) = &header.file_type {
        return Err(DecodeError::UnknownFileType(
            header.produced_with_version.clone(),
        ));
    }
    if let FileContent::UnknownFileContent(_value) = &header.file_content {
        return Err(DecodeError::UnknownFileContent(
            header.produced_with_version.clone(),
        ));
    }
    ...
```

`tests/header_check.rs` pins the old generic-JSON behavior for unknown `file_type` —
update that test to expect the new clean variant. gtk4 arms for the renamed/split
variants. Note `deny_unknown_fields` from A6 and untagged enums interact: an untagged
enum member match is by shape, unaffected by the container attribute — but run the A5/A6
tests after this commit too.

## Commit B2 — storage test hardening

Three additions, one commit (tests only, no behavior change):

1. **Missing-field pins for every `explicit_option` site not yet covered.** The 15
   `Option` record fields all carry `#[serde(deserialize_with = "explicit_option")]`, but
   only some have a missing-field regression test. Add the five missing ones, each
   following its block module's existing `missing_field_is_rejected`-style test (drop the
   field from the block's `spec_example()` JSON and assert rejection):
   `format/balancing.rs` `teacher_rotation` and `slot_rotation` (the existing test drops
   a plain bool instead), `format/students.rs` `tel` and `email` (existing test drops
   `excluded_periods`), `format/export_config.rs` `orientation`
   (`PerStudentGroupsConfig`). Rationale: losing one `deserialize_with` attribute
   silently reintroduces serde's `None` default; only a per-field pin catches it.
2. **A committed golden byte fixture.** Byte stability is currently pinned only against
   same-build output, so a formatting/ordering regression in the writer would go
   unnoticed. Add `storage/tests/fixtures/spec2_populated_golden.json`: serialize the
   `populated_round_trip` builder document once, write the bytes to the fixture, and add
   a test asserting `serialize_data(&builder_doc) == include_str!(...fixture)`. On a
   legitimate format evolution the fixture is regenerated consciously (document that in
   a comment at the top of the test).
3. **Tombstone and §6 coverage are already green** (`spec1_empty.json`,
   `spec_dispatch.rs`, the §6 example test) — no action, listed here so the implementer
   does not duplicate them.

---

# Part C — cascade engine: close the non-landing termination hole, deepen the toys

## Commit C1 — failure-scoped no-progress guard

**The hole.** The step-6.5 strictly-below assertion runs only in the `Ok` arm of
`apply_cascade` — it meters fixes that *land*. On `Err(BrokenInvariants)` the engine just
pushes another fix, with no state change and no ledger. A resolution map caught in a
dependency cycle ("remove A" keeps failing because B must go first, "remove B" keeps
failing because A must go first) re-pushes the same ops forever on an unchanged state: the
stack grows, no panic fires, the loop hangs. The module doc's "panics instead of hanging"
(`state/src/cascade.rs:20`) is currently true only for fixes that land; the actual
termination premise for non-landing chains — the reference graph is acyclic — lives in
the domain (design doc §5), not in the mechanism.

**The guard.** Track the invariants picked since the last landing; if the same invariant
is picked twice with no `Ok` in between, panic. This is exactly sound: state only changes
on landings and `fix_invariant` is a pure function of `(&self, invariant)`, so a repeated
pick on an unchanged state reproduces the same fix, which fails the same way, forever.
Conversely the legitimate N-round repair path always lands a fix between re-picks of the
same invariant, so the ledger (cleared on every `Ok`) never fires on it.

In `state/src/cascade.rs`, old code (the loop set-up and the two relevant arms):

```rust
    let mut last_target_break: Option<BTreeSet<T::Invariant>> = None;

    loop {
        ...
            Ok(backward) => {
                if let Some(before) = before {
                    ...
                }
                stack.pop();
                applied.push(ReversibleOp {
                    forward: front,
                    backward,
                });
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
```

New code:

```rust
    let mut last_target_break: Option<BTreeSet<T::Invariant>> = None;
    // Picks since the last landed op. State only changes on landings and the
    // map is a pure function of (state, invariant), so re-picking the same
    // invariant with no landing in between reproduces the same failing fix
    // forever — a stuck map. Any landing resets the ledger; the legitimate
    // N-round repair path always lands between re-picks.
    let mut picks_since_landing: BTreeSet<T::Invariant> = BTreeSet::new();

    loop {
        ...
            Ok(backward) => {
                if let Some(before) = before {
                    ...
                }
                picks_since_landing.clear();
                stack.pop();
                applied.push(ReversibleOp {
                    forward: front,
                    backward,
                });
            }
            Err(ApplyError::BrokenInvariants(set)) => {
                let pick = set
                    .first()
                    .expect("a BrokenInvariants error carries a non-empty set")
                    .clone();
                if is_target {
                    last_target_break = Some(set);
                }
                if !picks_since_landing.insert(pick.clone()) {
                    panic!(
                        "cascade made no progress: invariant {pick:?} was picked \
                         twice with no fix landing in between (failing op \
                         {front:?}) — the resolution map is stuck in a cycle"
                    );
                }
                match data.fix_invariant(&pick) {
                    Some(fix) => stack.push(fix),
```

(`T::Invariant` is already `Ord + Clone` — it lives in `BTreeSet`s and is cloned for the
pick today.)

**Doc updates in the same commit.** The module doc sentence (old, `cascade.rs:16-20`):

```rust
//! Termination rests on the resolution map's contract, and the engine holds it
//! to it in-flight: after every fix, the new state must compare **strictly
//! below** the pre-fix state in the document order ([ContentOrd], step 6.5). A
//! fix landing equivalent (the old perfect-no-op panic), above, or incomparable
//! panics instead of hanging.
```

New:

```rust
//! Termination rests on the resolution map's contract, and the engine holds it
//! to it in-flight, on both routes a map bug could hang it. Fixes that land:
//! after every fix, the new state must compare **strictly below** the pre-fix
//! state in the document order ([ContentOrd], step 6.5) — landing equivalent
//! (the old perfect-no-op panic), above, or incomparable panics. Fixes that
//! never land: re-picking the same invariant with no landing in between panics
//! (the no-progress ledger) — state only changes on landings and the map is a
//! pure function of state, so such a re-pick is a cycle, not a repair. The one
//! shape neither check can catch in-flight is a map that answers a failing fix
//! with ever-fresh invented material instead of the state's own; the
//! presence-test frame rule (design doc H.3) is what excludes it, and its
//! material is finite, so a conforming map either lands or repeats a pick.
```

Amend the `fix_invariant` doc-comment's termination sentence (old: "Because the order is
well-founded, this contract is the cascade's termination proof") to say the order bounds
the number of *landed* fixes, and the no-progress ledger bounds non-landing chains for
any map that only names material present in the state.

**Test.** New `EvilMode` in `state/src/test_utils.rs`:

```rust
    /// Answers the dangling invariant of quote `a` by "fixing" quote `b` and
    /// vice versa, each fix creating the other dangling quote — a two-cycle
    /// of never-landing fixes. Without the no-progress ledger this hangs the
    /// cascade forever.
    PingPong { a: u64, b: u64, author: u64 },
```

with the `Fixable` arm:

```rust
            EvilMode::PingPong { a, b, author } => {
                let other = if quote == a { *b } else { *a };
                Some(QuoteOp::SetQuote {
                    quote: other,
                    author: *author,
                })
            }
```

and the engine test in `cascade.rs`'s test module (trace: target `SetQuote{10, 7}` breaks
`D(10)`; evil fix `SetQuote{20, 7}` breaks `D(20)`, rolled back; its evil fix
`SetQuote{10, 7}` breaks `D(10)` — `D(10)` re-picked with no landing → panic):

```rust
    // 12. A map stuck in a never-landing two-cycle: without the no-progress
    //     ledger this loops forever with no state change; with it, the second
    //     pick of the same invariant panics.
    #[test]
    #[should_panic(expected = "made no progress")]
    fn a_never_landing_fix_cycle_panics() {
        let mut data = EvilQuoteData(
            quote_data(&[1], &[]),
            EvilMode::PingPong {
                a: 10,
                b: 20,
                author: 7,
            },
        );
        let (target, ()) = data.annotate(QuoteOp::SetQuote {
            quote: 10,
            author: 7,
        });

        let _ = apply_cascade(&mut data, target);
    }
```

Run the full `state/` suite plus the `state-colloscopes` cascade fixtures and
`property_cascade` (background, output captured once): the guard must not fire on any
legitimate cascade.

## Commit C2 — toy `UpdateQuote` + the conviction-route test

The engine's most user-visible error behaviour — `InvalidOp` on the target with a
remembered `BrokenInvariants` set (`cascade.rs:149-157`, the D4 "SlotOverflowsDay trace")
— has no test on either the toy or the domain. The toy cannot reach it today because no
`QuoteOp` both prechecks its target's existence and carries a payload that breaks an
invariant (`SetQuote` is a precheck-free upsert, kept that way on purpose for test 5).

Add an update-only op. In `state/src/test_utils.rs`, extend `QuoteOp`:

```rust
    /// Rewrites an existing quote's author. Unlike [QuoteOp::SetQuote] the
    /// quote must exist (precheck) — which is what lets a cascade fix consume
    /// the target's own target and drive the retried target into `InvalidOp`
    /// with a remembered break (the D4 conviction route).
    UpdateQuote { quote: u64, author: u64 },
```

extend `QuoteInvalidOp`:

```rust
    #[error("unknown quote {0}")]
    UnknownQuote(u64),
```

extend the precheck match in `QuoteData::apply` (after the two student arms):

```rust
            QuoteOp::UpdateQuote { quote, .. } if !self.quotes.contains_key(quote) => {
                return Err(ApplyError::InvalidOp(QuoteInvalidOp::UnknownQuote(*quote)));
            }
```

and the force arm (the precheck guarantees `insert` returns `Some`):

```rust
            QuoteOp::UpdateQuote { quote, author } => {
                let old = next
                    .quotes
                    .insert(*quote, *author)
                    .expect("prechecked: the quote exists");
                QuoteOp::UpdateQuote {
                    quote: *quote,
                    author: old,
                }
            }
```

The engine test, with the *honest* map (no evil mode): data `students {1}`,
`quotes {(10, 1)}`; target `UpdateQuote { quote: 10, author: 7 }`. Round 1: precheck OK,
applies, breaks `D(10)`, gate rolls back, engine remembers the set. The map finds quote
10 present and answers `RemoveQuote(10)`; the fix lands strictly below. The retried
target now hits `UnknownQuote(10)` — `InvalidOp`, `is_target`, remembered break present:

```rust
    // 13. The remembered-error conviction (D4): a fix consumes the target's
    //     own target, the retried target hits a precheck, and the user is told
    //     what the target kept breaking — not a baffling "unknown quote" for a
    //     quote they can see.
    #[test]
    fn a_fix_consuming_the_targets_target_reports_the_remembered_break() {
        let original = quote_data(&[1], &[(10, 1)]);
        let mut data = original.clone();
        let (target, ()) = data.annotate(QuoteOp::UpdateQuote {
            quote: 10,
            author: 7,
        });

        let err = apply_cascade(&mut data, target).unwrap_err();

        assert_eq!(
            err,
            ApplyError::BrokenInvariants(BTreeSet::from([QuoteInvariant::DanglingQuoteAuthor(
                10
            )])),
        );
        assert_eq!(data, original);
    }
```

The `None => ApplyError::InvalidOp(e)` sibling arm is already pinned by test 4.

## Commit C3 — a third toy level + the depth-3 success test

Every green cascade in the engine tests is a depth-2 alternation; a fix that itself
breaks an invariant *and then everything lands* is structurally unreachable (removing a
quote never breaks anything), so the depth-first success ordering `[fix-of-fix, fix,
target]` is untested. Add a third level to `QuoteData`: notes referencing quotes.

In `state/src/test_utils.rs`:

- `QuoteData` gains `pub notes: BTreeMap<u64, u64>` (note id → quote id). The two
  `quote_data` helpers (test_utils' own `partial_order_tests` and `cascade.rs`'s test
  module) gain `notes: BTreeMap::new()`; add a `quote_data_with_notes` helper where
  needed.
- `QuoteOp` gains `SetNote { note: u64, quote: u64 }` and `RemoveNote(u64)`, exact
  mirrors of `SetQuote`/`RemoveQuote` (upsert, no precheck; remove-absent is a perfect
  no-op whose inverse is itself).
- `QuoteInvariant` gains `DanglingNoteQuote(u64)` **after** `DanglingQuoteAuthor` (the
  derive `Ord` pick order matters: quote invariants sort before note invariants — state
  the reason in a comment).
- The checker in `apply` gains the second sweep:

```rust
        let broken: BTreeSet<QuoteInvariant> = next
            .quotes
            .iter()
            .filter(|(_, author)| !next.students.contains(author))
            .map(|(quote, _)| QuoteInvariant::DanglingQuoteAuthor(*quote))
            .chain(
                next.notes
                    .iter()
                    .filter(|(_, quote)| !next.quotes.contains_key(quote))
                    .map(|(note, _)| QuoteInvariant::DanglingNoteQuote(*note)),
            )
            .collect();
```

- The honest map gains the arm (presence test, same shape as the quote arm):

```rust
            QuoteInvariant::DanglingNoteQuote(note) => self
                .notes
                .contains_key(note)
                .then(|| QuoteOp::RemoveNote(*note)),
```

- `ContentOrd for QuoteData` destructures the new field (the compile error from the
  destructuring is the designed forcing function) and adds
  `map_inclusion(notes, &other.notes, |a, b| discrete(a, b))` to the `combine` list.
- The `EvilQuoteData` passthroughs are unchanged; evil modes still only speak
  `DanglingQuoteAuthor` — add a wildcard-free match arm delegating
  `DanglingNoteQuote` to the honest map (`self.0.fix_invariant(invariant)`), so existing
  evil tests are untouched.

The success test (engine test module): data `students {1}`, `quotes {(10, 1)}`,
`notes {(5, 10)}`; target `RemoveStudent(1)`.
Trace: target breaks `D(10)`; map answers `RemoveQuote(10)`; that fix breaks
`DanglingNoteQuote(5)`; map answers `RemoveNote(5)`, which lands; `RemoveQuote(10)`
lands; the target lands.

```rust
    // 14. Depth 3: a fix that itself needs a fix. The applied list is the
    //     depth-first unwinding — deepest fix first, target last.
    #[test]
    fn a_fix_needing_its_own_fix_unwinds_depth_first() {
        let mut data = quote_data_with_notes(&[1], &[(10, 1)], &[(5, 10)]);
        let (target, ()) = data.annotate(QuoteOp::RemoveStudent(1));

        let applied = apply_cascade(&mut data, target).expect("cascade resolves");

        assert_eq!(
            forward_ops(&applied),
            vec![
                QuoteOp::RemoveNote(5),
                QuoteOp::RemoveQuote(10),
                QuoteOp::RemoveStudent(1),
            ],
        );
        assert!(data.students.is_empty());
        assert!(data.quotes.is_empty());
        assert!(data.notes.is_empty());
    }
```

Also extend test 2 (undo round-trip) — or add a sibling — to run on the depth-3 fixture
so the compound reverse is pinned at depth 3 as well. Verify all existing toy-based tests
(engine tests, `partial_order_tests` in test_utils) still compile and pass with the new
field.

## Commit C4 — ContentOrd law checks over generated triples

The trait laws (reflexivity, transitivity, antisymmetry up to equivalence —
`state/src/partial_order.rs:37-39`) are only example-tested; the compositions where a
violation would be least obvious are `prefix_pointwise` and the `OrderedTable` order
(subsequence keys × pointwise values). Add `state/tests/partial_order_laws.rs`:

- A small deterministic generator (reuse the LCG walk pattern from
  `state/tests/cascade_on_derived_order.rs` — no new dependency) producing small values
  of a few representative shapes: `BTreeSet<u8>`, `BTreeMap<u8, Option<u8>>`,
  `Vec<u8>` under the subsequence blanket, small `OrderedTable`s, a two-field derived
  struct, and `Vec<Option<u8>>` under `vec_prefix`.
- For every generated triple `(a, b, c)` within a universe small enough to enumerate
  (a few hundred values per shape), assert: `a.content_cmp(&a) == Some(Equal)`;
  `content_cmp(a,b)` and `content_cmp(b,a)` are mutual inverses; if `a ≤ b` and
  `b ≤ c` then `a ≤ c` (transitivity through `content_le`); if `a ≤ b` and `b ≤ a`
  then `content_eq(a, b)`.

Keep the universes small enough that the test runs in well under a second; print nothing.

---

# Part D — gate hardening (state-colloscopes)

## Commit D1 — `GlobalUpdate` annotate: checked increment

Old (`state-colloscopes/src/ops.rs:832-840`):

```rust
            Op::GlobalUpdate(inner_data) => {
                if let Some(max_id) = inner_data.ids().max() {
                    id_issuer.skip_to_id(max_id + 1).expect(
                        "GlobalUpdate: ID space exhausted. \
                         This is either a critical bug or a malicious data payload.",
                    );
                }
                (AnnotatedOp::GlobalUpdate(inner_data), None)
            }
```

A forged payload containing raw id `u64::MAX` makes `max_id + 1` wrap to 0 in release:
`skip_to_id(0)` is a no-op, the exhaustion `expect` never fires, and if the payload is
otherwise valid the gate's `Ok` arm panics **after the mutation landed** in
`assert_id_issuer_high_water` with a misleading message and no rollback. New:

```rust
            Op::GlobalUpdate(inner_data) => {
                if let Some(max_id) = inner_data.ids().max() {
                    let next = max_id.checked_add(1).expect(
                        "GlobalUpdate: ID space exhausted. \
                         This is either a critical bug or a malicious data payload.",
                    );
                    id_issuer.skip_to_id(next).expect(
                        "GlobalUpdate: ID space exhausted. \
                         This is either a critical bug or a malicious data payload.",
                    );
                }
                (AnnotatedOp::GlobalUpdate(inner_data), None)
            }
```

The panic-on-forged-payload policy itself is unchanged (any id above `u64::MAX >> 1`
already panics through the `skip_to_id` `expect`); the fix only makes the extreme value
take the same loud, pre-mutation path instead of a post-mutation high-water panic. Add an
in-crate `#[should_panic(expected = "ID space exhausted")]` test next to the ops code
(`annotate` is `pub(crate)`, and building an `InnerData` holding an id of `u64::MAX`
needs the crate-private field access): construct a default `InnerData`, insert a student
with `unsafe { StudentId::new(u64::MAX) }`, and call `Op::annotate(Op::GlobalUpdate(...))`
with a fresh issuer. Note in the test why the id is forged (`unsafe new` is the
documented test-forgery door).

## Commit D2 — precheck-arm restore: mechanism over discipline

Old (`state-colloscopes/src/lib.rs:349-354`):

```rust
        let snapshot = self.inner_data.clone();
        let issuer_snapshot = self.id_issuer.lock().unwrap().clone();

        // Precheck failures return before any mutation (by construction of the
        // `force_apply_*` copies), so the state is untouched on this arm.
        let backward = self.force_apply(op)?;
```

"Failed apply leaves the state bit-identical" currently rests on all 16 `force_apply_*`
copies checking every precheck before their first mutation. The snapshot is already in
hand — restore it on the error arm and the invariant becomes unconditional:

```rust
        let snapshot = self.inner_data.clone();
        let issuer_snapshot = self.id_issuer.lock().unwrap().clone();

        // Precheck failures return before any mutation (by construction of the
        // `force_apply_*` copies); the restore below makes that a mechanism
        // rather than a 16-file discipline — a copy that ever mutated before
        // erroring would still leave the state bit-identical.
        let backward = match self.force_apply(op) {
            Ok(backward) => backward,
            Err(e) => {
                self.inner_data = snapshot;
                *self.id_issuer.lock().unwrap() = issuer_snapshot;
                return Err(e.into());
            }
        };
```

(`From<PrecheckError> for Error` exists at `lib.rs:284-288`, so `e.into()` preserves the
exact error value the `?` produced.) No behavior change on any currently-reachable path;
the whole workspace suite is the regression net.

## Commit D2b — retire the id-issuer `Mutex` (`annotate` takes `&mut self`)

Added during plan review. `Data.id_issuer` lives in a `std::sync::Mutex` for exactly one
reason: the `InMemoryData` trait declares `fn annotate(&self, op)` while annotation must
advance the issuer, and the trait's `Sync` bound then forces the interior mutability to
be thread-safe. That reason is obsolete. The only production caller of `annotate` is
`Manager::apply` (`state/src/traits.rs:155`), which already holds the data mutably:

```rust
        let (annotated_op, new_info) = self.get_in_memory_data_mut().annotate(op);
```

Every other caller in the workspace is a test that owns its `data` mutably. The
resolution map never annotates (`fix_invariant` takes `&self` and constructs deletive
`AnnotatedOp`s directly — fixes carry no fresh ids), and the cascade engine receives its
target already annotated. Nobody annotates through a shared reference. Beyond the
mechanics there is an honesty gain: `annotate(&self)` looks pure but consumes ids
through the back door; `&mut self` declares the side effect in the signature.

**Trait change.** `state/src/traits.rs`, old (line 57, with the doc comment above it
unchanged):

```rust
    fn annotate(&self, op: Self::OriginalOperation) -> (Self::AnnotatedOperation, Self::NewInfo);
```

New — and append one sentence to the doc comment ("Takes `&mut self`: annotation may
consume ids from the implementor's issuer, and the signature declares that side
effect."):

```rust
    fn annotate(&mut self, op: Self::OriginalOperation) -> (Self::AnnotatedOperation, Self::NewInfo);
```

The caller at `traits.rs:155` compiles unchanged (it already goes through
`get_in_memory_data_mut()`).

**Trait implementors.** Five toy impls change `&self` to `&mut self` in the signature
only (their bodies don't touch an issuer): `state/src/test_utils.rs:67` (`FakeData`),
`:156` (`QuoteData`), `:301` (`EvilQuoteData`, which delegates to the inner honest
impl), and `state/tests/cascade_on_derived_order.rs:67` (`LibraryData`) plus `:161` (the
derived-order wrapper, whose body `self.inner.annotate(op)` now needs the receiver
mutable — it is a plain field, so nothing else changes).

**`state-colloscopes/src/lib.rs`.** The field, old (lines 183-191):

```rust
#[derive(Debug, ContentOrd)]
pub struct Data {
    // The document order does not see the issuer: two `Data` with equal
    // inner data are content-equivalent even when their issuers differ —
    // the same quotient the hand-written `PartialEq` below takes.
    #[ord(ignore)]
    id_issuer: std::sync::Mutex<IdIssuer>,
    inner_data: InnerData,
}
```

New — `Clone` joins the derive list, the comment stays:

```rust
#[derive(Clone, Debug, ContentOrd)]
pub struct Data {
    // The document order does not see the issuer: two `Data` with equal
    // inner data are content-equivalent even when their issuers differ —
    // the same quotient the hand-written `PartialEq` below takes.
    #[ord(ignore)]
    id_issuer: IdIssuer,
    inner_data: InnerData,
}
```

Delete the hand-written `Clone` impl (lines 193-203) — it existed only because `Mutex`
is not `Clone`; the derived impl is equivalent (`IdIssuer` is already `Clone`, it is
cloned through the guard today). The hand-written `PartialEq` (the issuer-quotient) is
untouched.

The `annotate` impl, old (lines 320-323):

```rust
    fn annotate(&self, op: Op) -> (AnnotatedOp, Option<NewId>) {
        let mut guard = self.id_issuer.lock().unwrap();
        AnnotatedOp::annotate(op, &mut guard)
    }
```

New:

```rust
    fn annotate(&mut self, op: Op) -> (AnnotatedOp, Option<NewId>) {
        AnnotatedOp::annotate(op, &mut self.id_issuer)
    }
```

In `apply` (as it stands *after D2*), the snapshot line and the three restore sites lose
their locks. Old:

```rust
        let issuer_snapshot = self.id_issuer.lock().unwrap().clone();
...
                *self.id_issuer.lock().unwrap() = issuer_snapshot;
```

New (the snapshot line once, the restore in the D2 precheck arm and in the two
rolled-back checker arms):

```rust
        let issuer_snapshot = self.id_issuer.clone();
...
                self.id_issuer = issuer_snapshot;
```

`assert_id_issuer_high_water`, old (lines 439-448):

```rust
    fn assert_id_issuer_high_water(&self) {
        let max_id = self.inner_data.ids().max();

        if let Some(id) = max_id {
            let guard = self.id_issuer.lock().expect("No error on lock");
            if id >= guard.get_internal_counter() {
                panic!("IdIssuer internal counter is not greater than all internal ids");
            }
        }
    }
```

New:

```rust
    fn assert_id_issuer_high_water(&self) {
        let max_id = self.inner_data.ids().max();

        if let Some(id) = max_id {
            if id >= self.id_issuer.get_internal_counter() {
                panic!("IdIssuer internal counter is not greater than all internal ids");
            }
        }
    }
```

And `from_inner_data`, old (lines 482-485):

```rust
        let data = Data {
            id_issuer: std::sync::Mutex::new(id_issuer),
            inner_data,
        };
```

New:

```rust
        let data = Data { id_issuer, inner_data };
```

`Data` remains `Send + Sync` for free (`IdIssuer` is plain data), so the
`InMemoryData` bound is still satisfied — the compiler checks this at the impl. No
behavior change anywhere; the poison-path `unwrap()`s disappear with the locks. Sweep
the crate for any remaining `id_issuer.lock()` site (grep) — the borrow checker plus
grep is the completeness check. Consumers (`ops/`, `gtk4/`, tests) go through `Manager`
or own their `Data` mutably; the compiler may ask for a few `let mut` bindings in tests,
nothing more. Whole workspace suite as the regression net.

## Commit D3 — pin the `Data` construction boundary

Two test additions, no production change:

1. `FromInnerDataError::Logic` and `::BrokenInvariants` (`lib.rs:474-477`) — the decode
   trust boundary of G.4 — are constructed but never tested. Add two in-crate tests
   (the forgery needs `pub(crate)` field access): a default `InnerData` given one
   colloscope interrogation row with an empty group set (a canonical-absent
   `LogicError`, the same forgery `colloscopes.rs`'s `empty_interrogation_row_rejected`
   test already uses) must produce `Err(FromInnerDataError::Logic(_))` from
   `Data::from_inner_data`; an `InnerData` with a slot whose `teacher_id` dangles must
   produce `Err(FromInnerDataError::BrokenInvariants(_))`.
2. `assert_id_issuer_high_water`'s panic path (`lib.rs:439-448`) has no pin. Add an
   in-crate `#[should_panic(expected = "not greater than all internal ids")]` test: build
   a valid `Data` via ops, then feed a *forged* `AnnotatedOp` transplant through raw
   `apply` — or, simpler and stable, construct the `Data` whole with a hand-rolled low
   issuer via the crate-private constructor path used by tests, whichever the module's
   existing test utilities make shortest. The point of the pin is the message and the
   fact the check runs on the accepted path.

## Commit D4 — fuzz honesty: count checker rejections per corruption kind

`state-colloscopes/tests/property_apply_gate.rs`. Old (the error arm, lines 131–152, and
the guard, lines 194–206):

```rust
                    Err(e) => {
                        rejected.set(rejected.get() + 1);
                        rejected_by_kind[i].set(rejected_by_kind[i].get() + 1);
                        ...
                        match e {
                            // Precheck bounced before any mutation.
                            Error::InvalidOp(InvalidOp::Precheck(_)) => {}
                            Error::InvalidOp(InvalidOp::Logic(set)) => {
                                logic_seen.set(logic_seen.get() + 1);
                                ...
                            }
                            Error::BrokenInvariants(set) => {
                                assert!(
                                    !set.is_empty(),
                                    "an Invariants error carries a non-empty set",
                                );
                            }
                        }
                    }
...
        if kind.corrupting() {
            assert!(
                rejected_by_kind[i].get() > 0,
                "corrupting kind {kind:?} was never rejected across all seeds",
            );
        }
```

`rejected_by_kind` counts *any* `Err`, so a corrupting kind whose probes all bounced at
precheck would satisfy its guard without the checker-rejection path ever running — the
exact failure mode H.5's coverage ruling exists to prevent (only `ForceLogic` has a
tier-specific counter today). Add a `broken_by_kind` array of counters incremented in the
`Error::BrokenInvariants` arm, and strengthen the guard: for every corrupting kind whose
designed outcome is a checker rejection (all of them except the kind that targets the
`Logic` tier — read `CorruptionKind::ALL` and each kind's construction to name them
precisely in the assert), require `broken_by_kind[i].get() > 0`. Keep the existing
counters; this is an addition, not a replacement. Run the harness at the house seed
configuration and confirm the new guards hold (if a kind genuinely cannot reach the
checker tier, that is a finding to surface, not to assert around — check with the user).

## Commit D5 — remove the `SetRow` payload-student sweep (address/content split)

Per the (revised) Context decision: the op's address keeps its prechecks, the op's
content belongs to the dangling-FK net. `SetRow`'s payload students are content — the
op writes them into the document, `refs.rs:471` walks them as
`StudentRefSite::AssignmentsStudent`, and the checker reports any dead one as
`DanglingFk`. The precheck sweep duplicates that and is removed. The two key checks
(period, subject) are the op's address and **stay** — see the Context bullet for the
full rationale (the `∅`-payload clear is the case the FK net cannot see).

This sweep was the only payload-content sweep in the state layer (verified by grep:
`settings.rs`'s `InvalidStudentId` guards the per-student settings row *address*, and
`students.rs`'s guard op-target existence — both stay; the group-list ops never swept
their payload students).

In `state-colloscopes/src/assignments.rs`, remove the sweep from
`force_apply_assignment`. Old (lines 109–124):

```rust
                // stripped: SubjectDoesNotRunOnPeriod semantic guard

                // Every id in the incoming row must exist (coordinate carve-out).
                for student_id in students {
                    if !self
                        .inner_data
                        .params
                        .students
                        .student_map
                        .contains(student_id)
                    {
                        return Err(AssignmentPrecheckError::InvalidStudentId(*student_id));
                    }
                }

                // stripped: StudentIsNotPresentOnPeriod semantic guard
```

New — the whole loop goes; leave a stripped-marker in the house style of the two
neighbors, naming where the check now lives:

```rust
                // stripped: SubjectDoesNotRunOnPeriod semantic guard

                // stripped: payload-student existence sweep — the students are
                // op *content*, owned by the FK net (`DanglingFk @
                // StudentRefSite::AssignmentsStudent`); only the address
                // (period, subject) is prechecked.

                // stripped: StudentIsNotPresentOnPeriod semantic guard
```

Remove the now-unconstructed variant. Old (`assignments.rs:59-72`):

```rust
/// Precondition errors of the forced assignment op — the carve-out subset
/// (step-3 survey Table 2). The three coordinate-existence checks are
/// dual-listed (also invariant twins) and kept per Appendix D.3; the two
/// semantic guards (subject-runs / student-present) are stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentPrecheckError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),
}
```

New:

```rust
/// Precondition errors of the forced assignment op — the carve-out subset
/// (step-3 survey Table 2, as revised by the pre-step-7 review). The two
/// *address* checks (the row's period/subject key) are kept — with an empty
/// payload nothing lands in the document, so the FK net cannot see a dead
/// key. The payload-student sweep and the two semantic guards (subject-runs /
/// student-present) are stripped: they are op *content*, owned by the checker
/// (`DanglingFk @ AssignmentsStudent`, `Convergence::AssignmentForSubject-
/// NotRunningOnPeriod`, `Convergence::AssignedStudentNotPresentForPeriod`).
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentPrecheckError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),
}
```

Also update `force_apply_assignment`'s own doc comment (`assignments.rs:76-82`), which
says "the three coordinate-existence checks are kept" — same rewording as the enum doc.

**Consumer sites.** `ops/src/assignments.rs:159-160` maps
`AssignmentPrecheckError::InvalidStudentId(id)` into `AssignError::InvalidStudentId(id)`.
Do **not** reshape the `ops/` error surface (that is step 7): keep
`AssignError::InvalidStudentId` and construct it from the new rejection route instead —
the gate now reports a dead payload student as `Error::BrokenInvariants` carrying
`DanglingFk @ AssignmentsStudent`, so the `ops/` adapter matches that set and translates
the first assignments-student dangle back into the same public error it produced before.
If that translation turns out awkward (the set-shaped error does not pattern-match as
directly as the enum did), the fallback is to keep a pre-flight sweep *in `ops/`* (where
UI-facing validation is legitimate) — either way the `ops/` public error type is
unchanged. Let the compiler surface any other match sites (`gtk4` French arms included)
and fix them mechanically.

**Tests.**

1. In-crate gate test: a `SetRow` whose payload names a dead student is rejected as
   `Error::BrokenInvariants` with the set containing the `DanglingFk @
   AssignmentsStudent` entry, and the state is bit-identical afterwards (the gate rolled
   back).
2. Deterministic cascade fixture (next to the existing `state-colloscopes` cascade
   fixtures): the same op as an `apply_cascade` *target*. Expected trace: the target is
   rolled back, the map's `AssignmentsStudent` presence test fails on the (valid)
   pre-state, the map answers `None`, and the engine convicts — the returned error
   carries the remembered `DanglingFk` set and the state is unchanged. This pins the
   conviction route on the domain (C2 pins it on the toy).
3. Any existing test asserting `InvalidStudentId` from a `SetRow` precheck moves tier:
   assert the `BrokenInvariants` rejection instead (grep for the variant in
   `state-colloscopes` tests; `ops/` tests keep passing unchanged if the adapter
   translation above is used).
4. Check `collomatique-testgen-colloscopes`: if the invalid-op generator forges
   dead-payload-student `SetRow` ops under an "invalid" category, the expected rejection
   tier moves from precheck to invariants — the walk harnesses only count `Err` vs `Ok`,
   but any tier-specific assertion or category label mentioning the precheck must be
   updated.

**Design doc.** Amend D.3's "Parameter targeting" bullet in the same commit (D.3 is the
normative register for this family): the bullet keeps address coordinates ("op inputs
that say *where* the op acts must resolve") and gains a sentence recording the revision:
payload ids the op writes into the document are *content*, deliberately not prechecked —
the FK net reports them, and the map's presence tests guarantee a conviction (never a
spurious fix) when such an op is a cascade target. Note the `SetRow` payload sweep was
removed by this review, and that `SetGroupList`'s placements were never swept — the two
ops are now symmetric.

---

# Part E — entity-surface cleanup

## Commit E1 — dead code removal

Six deletions. The first was verified caller-free by hand (workspace-wide grep on the
name and on `.students(` call shapes); the other five come from a systematic scan: a
script extracted every `pub fn` name in the three reviewed crates (270 definitions) and
searched the whole workspace for any non-definition occurrence, classifying hits into
code, test, and comment-only. All six have zero code uses, zero test uses, and zero
doc-link mentions. (The scan is conservative — a name shared with any live method is
never flagged, which is why `students` needed the hand check.)

1. `GroupList::students` (`state-colloscopes/src/group_lists.rs:344-355`) — its doc
   ("students that are **not** already in a prefilled group") also states the opposite
   of the `Prefilled` arm's behavior, so it is both dead and a trap. Delete the method.
2. `Weeks::find_period_position_and_total_number_of_weeks`
   (`state-colloscopes/src/weeks.rs:341-358`). Delete.
3. `Weeks::get_first_week_and_length_for_period` (`weeks.rs:360-377`). Delete.
   (`find_period_position_and_first_week` at `:324-339` has a live caller in
   `ops/src/general_planning.rs:327` and stays.)
4. `GroupListFilling::check_duplicated_student` (`group_lists.rs:175-191`) — a
   pre-consolidation validate-era leftover; the no-duplicate rule is enforced by the
   checker's own code, which does not call this. Delete.
5. `GroupListFilling::groups_len` (`group_lists.rs:240-246`) — trivial accessor, and
   trap-ish ("0 for Automatic" although an automatic list has a real group count in its
   parameters). Delete.
6. `Weeks::periods_with_weeks` (`weeks.rs:262-269`) — dead, despite being the
   documented twin of the live `Slots::subjects_with_slots`. Ruled in discussion:
   a dead twin is symmetry on paper only, and it is a three-line re-add if a consumer
   ever wants it. Only its own doc carries the twin note, so nothing else needs
   editing. Delete.

Deliberately **kept** although the scan surfaced it: `Parameters::resolve`
(`colloscope_params.rs:305`) is *test-only* (16 test uses), not dead — it is the
infallible half of the designed `Lookup` surface and a likely step-7 consumer API.

`cargo build --workspace --tests` is the completeness check (any missed caller is a
compile error).

## Commit E2 — position-precheck unification

Three spellings of one concept, and one check-order divergence. Unify on
`PositionOutOfBounds` with named fields carrying scope, position and size (scope where
the list is scoped); test target-existence **before** bounds everywhere.

`state-colloscopes/src/weeks.rs` — old (lines 529–531):

```rust
    /// The target position is out of range for the destination period
    #[error("invalid position ({1}) in period ({0:?})")]
    InvalidPosition(PeriodId, usize),
```

new:

```rust
    /// The target position is out of range for the destination period
    #[error("position {position} is out of range for period {period:?} (size = {size})")]
    PositionOutOfBounds {
        period: PeriodId,
        position: usize,
        size: usize,
    },
```

(update every construction site inside `force_apply_week`/its helpers to carry the
period's current week count as `size`).

`state-colloscopes/src/slots.rs` — old (lines 346–348):

```rust
    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),
```

new (the slot list is per-subject, so name the subject):

```rust
    /// A position is outside the subject's slot list
    #[error("position {position} is outside the slot list of subject {subject:?} (size = {size})")]
    PositionOutOfBounds {
        subject: SubjectId,
        position: usize,
        size: usize,
    },
```

`state-colloscopes/src/subjects.rs` — old (lines 286–288):

```rust
    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),
```

new (the subject list is global; named fields for uniformity):

```rust
    /// A position is outside the subject list
    #[error("position {position} is outside the list (size = {size})")]
    PositionOutOfBounds { position: usize, size: usize },
```

Check-order fix in `subjects.rs` `ChangePosition` — old (lines 336–346):

```rust
            AnnotatedSubjectOp::ChangePosition(id, new_pos) => {
                if *new_pos >= self.inner_data.params.subjects.ordered_subject_list.len() {
                    return Err(SubjectPrecheckError::PositionOutOfBounds(
                        *new_pos,
                        self.inner_data.params.subjects.ordered_subject_list.len(),
                    ));
                }
                let Some(old_pos) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectPrecheckError::InvalidSubjectId(*id));
                };
```

new (existence first — the slots order; a doubly-bad op reports the dangling target):

```rust
            AnnotatedSubjectOp::ChangePosition(id, new_pos) => {
                let Some(old_pos) = self.inner_data.params.subjects.find_subject_position(*id)
                else {
                    return Err(SubjectPrecheckError::InvalidSubjectId(*id));
                };
                let size = self.inner_data.params.subjects.ordered_subject_list.len();
                if *new_pos >= size {
                    return Err(SubjectPrecheckError::PositionOutOfBounds {
                        position: *new_pos,
                        size,
                    });
                }
```

Audit `weeks.rs`'s position checks for the same precedence rule while renaming (fix if
divergent, note in the commit message either way). Let the compiler enumerate consumer
match sites (`ops/`, `gtk4/`) and update them mechanically. Update the design doc's D.3
"Position bounds" bullet from "`InvalidPosition`, `PositionOutOfBounds`" to the single
name. Add one deterministic test per entity asserting the doubly-bad-op precedence
(dangling id + out-of-range position reports the id).

## Commit E3 — delete the dead `validate_*_id` family

Revised during plan review: this commit was originally a style unification (the
`validate_period_id`/`validate_subject_id` linear scans rewritten in the
promote-then-lookup style of their siblings). Before executing it, a workspace-wide
grep for `validate_[a-z_]*_id` (all `.rs` files, `target/` excluded) returned exactly
ten hits — the ten definitions in `state-colloscopes/src/colloscope_params.rs` — and
not one call site anywhere: not in `ops/`, not in `gtk4/`, not in `python/`, not in any
test. The whole family is dead, so it is deleted rather than polished (the same ruling
as `GroupList::students` in E1: no caller anywhere → remove).

Delete all ten methods from `colloscope_params.rs` (each is a small
"promotes an u64 to a typed id" helper returning `Option<Id>`):

- `validate_period_id` (line 109), `validate_student_id` (:120),
  `validate_subject_id` (:131), `validate_teacher_id` (:142),
  `validate_week_pattern_id` (:152), `validate_slot_id` (:166),
  `validate_incompat_id` (:176), `validate_group_list_id` (:186),
- `validate_pairing_rule_id` (:354) and `validate_slot_pairing_rule_id` (:364) in the
  later impl block.

This deletion swallows the two findings the original commit existed to fix (the two
linear scans and the style asymmetry). If some future consumer (the step-7 `ops/`
surface, or the Python bindings) needs a u64-promotion surface again, it can be
reintroduced deliberately, in one style, with callers to hold it accountable.

`cargo build --workspace --tests` is the completeness check, same as E1 — any missed
caller is a compile error.

## Commit E4 — `Colloscope::is_empty` rename + `is_empty` honesty docs

Revised during plan review: the `Colloscope` item was originally a doc-only fix, but a
doc comment cannot fully defuse the trap — a method named `is_empty` on a two-table
struct *reads* as "the whole struct is empty" no matter what its doc says (a colloscope
holding only group-list placements answers `true` today). Rename it so the name tells
the truth, making the pair symmetric: each method names the half it reads, and no
method claims to speak for the whole struct. Deliberately do **not** add a new
whole-struct `is_empty`: no caller wants one today (gtk4 and the round-trip builder
test both use the two halves separately), and a cleanup pass adds no speculative API.

`state-colloscopes/src/colloscopes.rs:32-39` — old:

```rust
impl Colloscope {
    pub fn is_empty(&self) -> bool {
        self.interrogations.is_empty()
    }

    pub fn are_group_lists_empty(&self) -> bool {
        self.group_lists.is_empty()
    }
}
```

new:

```rust
impl Colloscope {
    /// Whether there are no interrogation rows. Reads the interrogations table
    /// only — its twin [Self::are_group_lists_empty] covers the other half of
    /// the struct.
    pub fn are_interrogations_empty(&self) -> bool {
        self.interrogations.is_empty()
    }

    /// Whether no group list has any placements. Reads the group-lists table
    /// only — its twin [Self::are_interrogations_empty] covers the other half
    /// of the struct.
    pub fn are_group_lists_empty(&self) -> bool {
        self.group_lists.is_empty()
    }
}
```

Known callers of the renamed method (mechanical rename at each site; the compiler
catches any the grep missed): `storage/tests/populated_round_trip/builder.rs:952`,
`testgen-colloscopes/src/generator.rs:1501`, `gtk4/src/editor/colloscope.rs:335`.

The `Slots::is_empty` item stays doc-only — that method really does answer for its
whole struct (the ordering sidecar and `slot_map` cover the same slots in lockstep),
so only the *why-trustworthy* note is missing.
`state-colloscopes/src/slots.rs:160-163` — old:

```rust
    /// Whether no subject has any slots.
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty()
    }
```

new (mirror the lockstep note its weeks twin carries at `weeks.rs:271-279`, adapted to
the container this one reads):

```rust
    /// Whether no subject has any slots.
    ///
    /// Reads the ordering sidecar: the compound mutators keep it in lockstep
    /// with `slot_map`, so the two containers cover the same slots in every
    /// ops-reachable state (force ops included); only test forgery can split
    /// them. (The weeks twin reads its entity table instead — either side of
    /// the lockstep is equally authoritative.)
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty()
    }
```

---

# Part F — documentation batch

## Commit F1 — code doc drift (no behavior change, code comments only)

1. **`weeks.rs:17-27` and `slots.rs:14-24` module headers** claim the ordering
   `LogicError`s check row-key liveness (weeks: "and that period exists"; slots: "and
   that subject exists and has interrogations"). The checker deliberately does the
   opposite (Appendix F.4): row-key liveness is the op-reachable dangling state,
   reported as `DanglingFk` through the entity FK sites and repaired by the cascade,
   and "has interrogations" is `Convergence::SlotForSubjectWithoutInterrogations`.
   Reword both headers: the ordering `LogicError`s pin sparsity (no empty rows) and the
   duplicate-free-permutation mirror; key liveness and the interrogation flag belong to
   the fixable tier. Old (weeks, the two bullets):

```rust
/// - `ordering` is sparse: a row is present exactly when the period has at
///   least one week (canonical form — no empty rows), and that period exists,
///   and
/// - `ordering[p]` is a duplicate-free permutation of
///   `{ id | week_map[id].period_id == p }`.
```

   New (weeks; mirror for slots with its own vocabulary):

```rust
/// - `ordering` is sparse: a row is present exactly when the period has at
///   least one week (canonical form — no empty rows), and
/// - `ordering[p]` is a duplicate-free permutation of
///   `{ id | week_map[id].period_id == p }`.
///
/// Row-key *liveness* (the period exists) is deliberately not part of these
/// `LogicError`s: a row keyed by a removed period is the op-reachable dangling
/// state, reported as `DanglingFk` through the per-week `WeekPeriodFk` sites
/// and repaired by the cascade (design doc Appendix F.4).
```

2. **`slots.rs` stale pre-sparse wording** on three read methods (`:165`, `:180`,
   `:199`): "or `None` if the subject has no interrogations" → "or `None` if the
   subject has no slots (no ordering row)" — the weeks twins' phrasing. A subject
   *with* interrogations but no slots also reads `None`; `subjects_with_slots`
   (`:150-155`) already documents the distinction.
3. **`refs.rs:47-48`**: "in surface order (period → slot → week)" — the period layer
   died in step 1; `walk_colloscope` iterates flat `(slot, week)` composite-key order.
   Reword: "in `(slot, week)` key order — each row emits its slot key site then its
   week key site". Also unify "sparse mirrors" (module doc step 13, `:42`) vs "the
   dense mirrors" (`walk_refs` doc, `:522`) — the mirrors are sparse; fix `:522`.
4. **`ops.rs` copy-paste rustdoc**: `:333-334` says "Operation on slots" above
   `Incompat(AnnotatedIncompatOp)` → "Operation on incompatibilities". `SlotOp`
   (`:189-192`) and `AnnotatedSlotOp` (`:603-606`): "Move a subject" → "Move a slot",
   "parameters of an existing subject" → "of an existing slot". `AnnotatedSubjectOp`
   (`:522-524`): "Add a period after an existing period / First parameter is the period
   id" → subject wording.
5. **`invariants.rs:4`** cites the deleted `docs/plans/plan_step_2.md`; retired plans
   are reachable only via their pins. Replace with the pin:
   "(plan §3, pinned at `git show 49b4f77d:docs/plans/plan_step_2.md`)" — verify the pin
   hash against the state-consolidation topic file before writing it.
6. **`resolution.rs:44-48`** (frame rule 4's op enumeration) lists `AssignToSubject`
   and `SetGroupList` among ops that "carry the target inside the op". False for
   `GroupListRefSite::AssociationEntry` (target = the group-list id; the emitted
   `AssignToSubject(period, subject, None)` does not carry it — that arm correctly has
   an explicit `*assigned != group_list` identity test) and for the
   `ColloscopeGroupListStudent` arm's `SetGroupList` (covered by the membership test).
   Since rule 4 is the audit-by-shape criterion, a loose list can wave a targetless arm
   through. Qualify: the list holds only where the target *is* the coordinate the op
   names, and name the two exceptions with their tests.
7. **`property_cascade.rs:14-17`** — old:

```rust
//!   exists. The engine holds the map to its contract with three panics (a fix
//!   rejected as invalid, a fix-created invariant the map then disowns, a fix
//!   that applies as a perfect no-op). With the round fuse gone, those panics
//!   plus the by-hand audit of every arm are what stands between a map bug and a
//!   production hang, until step 6.5 adds the `PartialOrd` in-flight check.
```

   New (step 6.5 shipped, and its check is deliberately **not** `PartialOrd` — I.1;
   after C1 the panic count also changes):

```rust
//!   exists. The engine holds the map to its contract in-flight: the precheck
//!   and disowned-invariant panics, the [ContentOrd] strictly-below assertion
//!   after every landed fix (step 6.5 — deliberately *not* `PartialOrd`, whose
//!   std container impls are lexicographic), and the no-progress ledger on
//!   never-landing fix chains. Every fix a green walk lands has passed the
//!   strictly-below assertion.
```

8. **`partial_order.rs:40-43`** (the well-foundedness law) — old:

```rust
/// * **Well-foundedness on document data**: every strict decrease removes
///   an element from a finite container or moves an `Option` from `Some` to
///   `None` — so there is no infinite strictly-decreasing chain, and strict
///   monotonicity of fixes is a termination proof.
```

   New (the law as stated excludes `#[ord(total)]` fields — reword to the actual
   obligation):

```rust
/// * **Well-foundedness on document data**: every strict decrease happens in
///   a well-founded coordinate — removing an element from a finite container,
///   moving an `Option` from `Some` to `None`, or stepping down a field whose
///   own order admits no infinite descending chain — so there is no infinite
///   strictly-decreasing chain, and strict monotonicity of fixes is a
///   termination proof.
```

9. **`#[ord(total)]` docs** (`state-derive/src/lib.rs:119-121`, and the matching
   sentence in `state-derive/src/content_ord.rs:31`) — append the obligation:
   "The field's `Ord` must itself be well-founded (no infinite strictly-descending
   chain): integers are, `String` is **not** (`"b" > "ab" > "aab" > …`) — a
   non-well-founded `total` field silently voids the termination proof. This cannot be
   checked mechanically; it is part of the field's design decision." (User ruling: doc
   only, no mechanism.)
10. **`state/src/partial_order.rs` `Table::content_cmp` (`:325-342`)** — add a one-line
    comment: "Same order as [map_inclusion] with a [combine]d value rule, expressed
    through `Table`'s public surface (`keys`/`contains`/`get`/`iter`) — keep the two in
    step if either changes." (Delegation was considered and rejected: it would need a
    `pub(crate)` accessor leaking the private map for zero behavior gain.)

## Commit F2 — design-doc and spec riders

1. **Appendix B.3 rider** (`docs/plans/invariant_cascade_design.md:805-807`): the line
   recording the colloscope writers as "(panic on impossible coordinates, empty payload
   clears the row)" is superseded — the delivered writers are plain upserts that never
   panic (`colloscopes.rs:84-86`), coordinate existence lives in
   `force_apply_colloscope`'s prechecks. Add an italic rider in the F-supersession
   style: "*(Superseded: since the step-5/6 gate the writers are plain upserts; the
   coordinate checks live in `force_apply_colloscope`'s prechecks — see G/H.4.)*"
2. **H.5 wording**: the innocent-test census says "one per *comparison*"; the
   `GroupListExcludedStudent` arm's variant guard shares its test with the membership
   half (acknowledged in the block comment at `innocent_tests.rs:1619-1624`, and
   verified near-worthless to split — a plausible regression still answers `None`
   either way). Add a parenthetical to H.5 naming the one deliberate doubling-up.
3. **Spec reader-headroom note** (`docs/file_format/file_format.md`, §3 scalars): a
   file whose largest defining id is exactly 2^63−1 and which contains at least one
   week is spec-valid but rejected by this reader (`WeekId` synthesis allocates above
   the maximum id, and the issuer's secure range ends at 2^63−1). Add one sentence:
   "Readers may reserve id headroom above the largest defining id (for synthesized
   ids); writers should stay far below the 2^63−1 ceiling — the 32-bit guidance above
   makes the reservation invisible in practice."
4. **This plan's own close-out**: when every commit has landed, update the
   state-consolidation topic file's memory entry (review complete, fixes landed,
   step 7 unblocked) and retire `docs/plans/plan_review_fixes.md` with a `git show`
   pin, per the house close-out pattern.

---

# Part G — deterministic test pins (entity ops)

## Commit G1 — reorder and edge-undo pins

Four deterministic tests, currently fuzz-only paths. Follow `week_ops.rs`'s style
(build a small document through the public op surface, apply, assert both structures,
undo, assert identity):

1. **`SlotOp::ChangePosition`** (in a new or existing slots test section): subject with
   three slots; move the first to position 2; assert the new order via
   `slots_for_subject` and the slot's own `find_slot_subject_and_position`; apply the
   returned reverse; assert the original order. This pins `move_slot`'s
   remove-then-insert index semantics (`new_pos` interpreted after detachment).
2. **`SubjectOp::ChangePosition`**: same shape over `ordered_subject_list`.
3. **`WeekOp::Move`, same-period branch** (`tests/week_ops.rs`): a period with three
   weeks; move week 0 to position 2 *within the same period*; assert order and
   `week_position`; undo; assert identity. This pins the `weeks.rs:477` row-keepalive
   guard (`order.is_empty() && src_period != dest_period`) and the `dest_len_post`
   adjustment (`:690-696`) on the `dest == src` path. Also add the one-week variant
   (move a period's only week to its own position 0) — the transiently-empty-row edge.
4. **Remove-first-week undo** (`tests/week_ops.rs`): remove the week at position 0 and
   undo — the reverse op is the `AddFront` arm (`weeks.rs:632-635`), currently only
   exercised by the property harness (the existing `remove_week_then_undo…` test removes
   a middle week, pinning only `AddAfter`). Mirror for slots: remove a subject's first
   slot and undo (the `AddAfter(id, None, slot)` reverse at `slots.rs:444-457`).

---

## Execution order and verification

Commit order: A1–A6, B1–B2, C1–C4, D1, D2, D2b, D3–D5, E1–E4, F1–F2, G1. Within each part the
order above is load-bearing (test-first pairs); across parts A→G is the recommended
sequence (spec compliance first, engine next, cleanup last) but parts are independent.

After each commit: targeted suite for the touched crate (`cargo test -p …`), output
captured once to the scratchpad and grepped. After each part: full
`cargo test --workspace` in the background. The storage commits must keep
byte-stability (`populated_round_trip`), `spec2_format`, and the examples pristine-load
green throughout — they are decode-side only. The engine commits must keep the two
50-seed cascade walks and `property_content_ord` green (the C1 guard must never fire on
a legitimate cascade). No `Cargo.lock` change is expected at any point; if one appears,
stop — something pulled a dependency this plan does not intend.

User-run acceptance at the end (standing gate, at the user's cadence): gtk4 smoke + the
three contract scripts.
