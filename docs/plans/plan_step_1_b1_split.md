# Plan: split commit B1 (WeekId introduction) into 6 contained commits

## Context

Step 1 of `docs/plans/invariant_cascade_design.md` is underway on `consolidate_state`
(phases 0–3 of `docs/plans/plan_step_1.md` are committed: bugfix, decoupling, 1a sparse
assignments, 1c sparse slots ordering). Phase 4 commit B1 — "weeks become entities" — was
attempted and turned out far too wide to land as one commit: it simultaneously changes the
`Periods` representation, re-cuts the op family, rewrites every positional week walk in 7
crates, and adds transitional colloscope/pattern maintenance.

This plan replaces B1 with **six commits (0–5)**, each green, byte-stable, and either
wide-but-shallow or deep-but-narrow — never both. Commit B2 (week patterns → exception
set) is unchanged and follows after commit 5.

The key structural insight (verified against the code): `Periods.ordered_period_list` is a
`pub` field read directly by ~50 sites across state-colloscopes, ops, storage,
constraints-colloscopes, gtk4, python, xlsx and testgen — many via hand-rolled
`global_week += desc.len()` accumulation loops. Any payload change ripples through all of
them. So the bulk lands first as a behavior-identical **read-surface commit** (the same
"prep on the old shape" pattern as commit D0 of phase 5), and every later commit stays
narrow.

## Decisions ledger (all settled)

| # | Decision |
|---|---|
| S1 | **Commit 0 introduces the final read surface on the current representation** and privatizes `ordered_period_list` (slots-style). Consumers migrate exactly once. |
| S2 | **Transitional representation (commits 1–4)**: `ordered_period_list: OrderedTable<PeriodId, Vec<(WeekId, WeekDesc)>>` — ids inline, no sidecar, no mirror invariant to check beyond global id-uniqueness (already covered by `InnerData::check_no_duplicate_ids` once week ids join `ids()`). |
| S3 | **`walk()` yields a 3-tuple**: `(PeriodId, &WeekDesc)` in commit 0, `(PeriodId, WeekId, &WeekDesc)` from commit 1, `(PeriodId, WeekId, &Week)` from commit 5. Richer than the design doc's `(PeriodId, WeekId)` — same role, avoids a second lookup at every call site. Since `Week` keeps `WeekDesc`'s field names, the commit-5 item-type change is invisible to field-access-only call sites. |
| S4 | **`annotate()` has no data access** (signature: `(Op, &mut IdIssuer)`), so transitional `AnnotatedPeriodOp::Update` carries `Vec<(WeekId, WeekDesc)>` where `annotate` issues a fresh id for *every* position; `apply_period` keeps existing ids on the common prefix and consumes annotated ids only for growth. Wasted counter values are harmless (monotonic issuer); replay/redo stays deterministic. Dies in commit 4. |
| S5 | **`WeekOp::Move` carries content**: pattern bits and colloscope cells travel with the week (this is what lets commit 3 delete the cut/merge save/clean machinery). Guard only where content *cannot* travel: destination period lacks the slot (subject excluded there), or a non-empty cell's group numbers exceed the destination `(period, subject)` association bounds → error, same guard family as today. |
| S6 | **`WeekOp::Remove` requires every pattern bit `true` at that week** (`can_remove_weeks`-style, same UX as today's period shrink) and all colloscope cells empty. Consequence: undo (re-`Add` at the same spot with the same id) restores the exact prior state, since `Add*` splices a `true` bit into every pattern and recreates cell activity purely from `desc.interrogations`. |
| S7 | **`PeriodOp::Remove` becomes "must be week-empty"** (commit 4), mirroring the design doc's `remove_period` precondition. Composite `DeletePeriod` empties the period via `WeekOp::Remove` first. Period-level reference guards (subjects/students/pairings/…) stay on `PeriodOp::Remove`. |
| S8 | **`ChangeStartDate` stays in `PeriodOp`** (the design doc's slimmed enum lists only Add/Remove; start-date is period-adjacent and has no reason to move). |
| S9 | **`WeekId` transitional entity attribute is `#[entity(WeekDesc)]`**; re-targeted to `#[entity(Week)]` in commit 5. `Lookup<WeekId>` is only introduced in commit 5 (nothing needs it earlier; `find_week` inherent method covers commits 1–4). |
| S10 | **`apply_week` lives in `periods.rs`** next to `apply_period` (weeks are the periods submodule's entities; no new file — consistent with every other op family). |
| S11 | **Decode id synthesis**: a small `max_used_id(&format doc)` pre-scan over every id field, then `reconstruct_periods` assigns `max+1, max+2, …` in walk order. Encode never writes week ids ⇒ byte-stability is unconditional. |
| S12 | **`from_period_rows` gets its final signature in commit 1** — `(first_week, Vec<(PeriodId, Vec<(WeekId, WeekDesc)>)>)` — so commit 5 changes only its body, not its callers. (Commit 0 introduces it id-less; commit 1 re-signs it — decode is its only caller.) |

House rules honored: Edit tool only; no `serde(default)`; `foo.rs + foo/` style; `Table`
types never leave state-colloscopes; test-first where a behavior bug is involved (none
here — this is restructuring; regression pins are added *with* each behavior-bearing
commit); no new dependencies ⇒ no `Cargo.lock` change ⇒ no Nix `cargoHash` refresh.

---

## Commit 0 — read surface + privatization (wide, shallow, behavior-identical)

**Goal:** no consumer outside `periods.rs` touches `ordered_period_list` or assumes its
payload type. Zero behavior change, zero new types.

### 0.1 New surface on `Periods` (`state-colloscopes/src/periods.rs`)

The field goes private (Rust privacy is module-level, so `apply_period` — same file —
keeps direct access; **no `pub(crate)` mutators are needed in this commit**):

```rust
pub struct Periods {
    pub first_week: Option<collomatique_time::WeekStart>,   // stays pub: scalar, no invariant
    /// Period order + per-period weeks (private; read through the surface below)
    ordered_period_list: OrderedTable<PeriodId, Vec<WeekDesc>>,
}

/// Error returned when building [Periods] from rows with a duplicated period id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("duplicated period id {0:?}")]
pub struct DuplicatedPeriodIdError(pub PeriodId);

impl Periods {
    /// Builds a [Periods] from period rows (used by storage decode).
    pub fn from_period_rows(
        first_week: Option<collomatique_time::WeekStart>,
        rows: Vec<(PeriodId, Vec<WeekDesc>)>,
    ) -> Result<Self, DuplicatedPeriodIdError> {
        let ordered_period_list = rows
            .try_into()
            .map_err(|collomatique_state::DuplicatedIdError(id)| DuplicatedPeriodIdError(id))?;
        Ok(Periods { first_week, ordered_period_list })
    }

    // ---- Read surface ----

    /// Period ids in display order.
    pub fn period_ids(&self) -> impl Iterator<Item = PeriodId> + '_ {
        self.ordered_period_list.keys()
    }

    /// Number of periods.
    pub fn period_count(&self) -> usize {
        self.ordered_period_list.len()
    }

    /// The canonical global week order: every week of every period, in
    /// period-then-position order. `walk().enumerate()` gives the global week
    /// index — this replaces every hand-rolled accumulate-`len()` loop.
    pub fn walk(&self) -> impl Iterator<Item = (PeriodId, &WeekDesc)> + '_ {
        self.ordered_period_list
            .iter()
            .flat_map(|(period_id, weeks)| weeks.iter().map(move |desc| (period_id, desc)))
    }

    /// Weeks of one period, in order; `None` if the period id is invalid.
    pub fn weeks_of(&self, id: PeriodId)
        -> Option<impl Iterator<Item = &WeekDesc> + '_>
    {
        Some(self.ordered_period_list.get(&id)?.iter())
    }

    /// Owned copy of a period's weeks (op-payload building in `ops/` and gtk4).
    pub fn weeks_vec_of(&self, id: PeriodId) -> Option<Vec<WeekDesc>> { … }

    /// Number of weeks of one period; `None` if the period id is invalid.
    pub fn week_count_of(&self, id: PeriodId) -> Option<usize> { … }
}
```

Kept as-is (already encapsulating): `count_weeks`, `find_period_position`,
`find_period_position_and_first_week`, `find_period_position_and_total_number_of_weeks`,
`get_first_week_and_length_for_period`. Demoted to `pub(crate)`: `find_period` (its only
out-of-module caller becomes the `Lookup<PeriodId>` impl in `colloscope_params.rs:174-179`
and one gtk4 site that switches to `weeks_of`/`weeks_vec_of`).

### 0.2 Consumer sweep (mechanical, grep-driven; the full inventory)

Every site below moves to the surface; the transformation is one of three shapes —
(a) accumulate-loop → `walk().enumerate()`, (b) `find_period`/payload read → `weeks_of` /
`week_count_of` / `weeks_vec_of`, (c) `.keys()`/`.len()`/`get_at(pos).0` on the table →
`period_ids()` / `period_count()` / existing position helpers:

- **state-colloscopes internal** (other modules than `periods.rs`):
  `colloscope_params.rs:42-70` (`merge_pattern` → walk-based), `:76,:287,:932,:1021,:1051,:1107`;
  `colloscopes.rs:63,:85,:132,:177,:330-343,:391-400` (the splice walkers keep their
  per-period windowing but source it from `period_ids()` + `week_count_of`);
  `subjects.rs:547`; `refs.rs:296` (`walk_week_pattern_coupling` spans).
- **ops/**: `general_planning.rs:338,461,683,921,954,1020,1210,1220,1231,1292,1333`
  (desc clones → `weeks_vec_of`; `iter().last()` → `period_ids().last()`),
  `assignments.rs:182`, `week_patterns.rs:212-228`, `slots.rs:221-237`,
  `group_lists.rs:819,1186`.
- **constraints-colloscopes**: `helpers.rs:59-73`, `tools.rs:22-96`,
  `convert.rs:32-151`, `config.rs:138`, `types/user_readable.rs:748`,
  `balancing/period_rotation.rs:31-61`, `misc/limits.rs:14-16`,
  `pairings/subject.rs:30-50`, `periodicity/*.rs`, `extras.rs:209-211`,
  `tests/property_build.rs:46,53` — all accumulate-loops become `walk().enumerate()`
  (or `weeks_of` where period-scoped).
- **gtk4**: `editor.rs:566-569`, `editor/general_planning.rs` + `periods_display.rs`
  (UI copies via `weeks_vec_of`), `editor/subjects.rs:148-151,:200`,
  `editor/students.rs` + `dialog.rs`, `editor/assignments.rs:60,152`,
  `editor/group_lists.rs:324`, `editor/{slot_pairings,pairings}/*_params.rs`
  (`get_at(i).map(|(id,_)| id)` → `period_ids().nth(i)`), `editor/week_patterns.rs:149`
  + `dialog.rs:296-358`, `editor/colloscope/{config_dialog,colloscope_display}.rs`.
- **storage**: `decode/spec2.rs:261-291` → `Periods::from_period_rows`;
  `decode/spec2.rs:720-726` (colloscope `week_table`) and `encode/spec2.rs:155-169,:542-578`
  → `walk()`-based; `tests/populated_round_trip/builder.rs:790-794`.
- **python**: `glue/params.rs:51,79`. **xlsx**: `colloscope_sheet.rs:98-140`.
  **testgen**: `generator.rs:102`, plus `synth.rs` untouched.

### 0.3 Gates

`cargo build --workspace && cargo test --workspace`; 100-seed property harness;
byte-stability suite + `all_examples_load_pristine` (hogwarts). All trivially expected to
pass — any failure means the conversion wasn't behavior-preserving.

---

## Commit 1 — `WeekId` exists; nobody outside `periods.rs`/storage/ops.rs cares

**Goal:** weeks get identities; representation gains them inline; decode synthesizes them;
undo/redo preserves them exactly. No new op family yet.

### 1.1 The id (`ids.rs`)

```rust
/// This type represents an ID for a week
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
         Serialize, Deserialize, EntityId)]
#[entity(WeekDesc)]                 // transitional; re-targeted to Week in commit 5
pub struct WeekId(u64);
```

Plus: `IdIssuer::get_week_id()`, `NewId::WeekId(WeekId)` variant (+ `inner()` arm +
`From<WeekId> for NewId`). Sweep the (few) exhaustive `match`es on `NewId` in ops/gtk4 —
they mostly `panic!("Unexpected result")` on wrong variants, so additions are cheap.

### 1.2 Representation + surface (`periods.rs`)

```rust
pub struct Periods {
    pub first_week: Option<collomatique_time::WeekStart>,
    /// Weeks carry their id inline (transitional shape until the week_map split).
    ordered_period_list: OrderedTable<PeriodId, Vec<(WeekId, WeekDesc)>>,
}
```

Surface changes (the *only* out-of-crate ripple, all mechanical destructuring):

```rust
pub fn walk(&self) -> impl Iterator<Item = (PeriodId, WeekId, &WeekDesc)> + '_ { … }
pub fn weeks_of(&self, id: PeriodId)
    -> Option<impl Iterator<Item = (WeekId, &WeekDesc)> + '_> { … }
// stable: weeks_vec_of still returns Vec<WeekDesc> (strips ids — op payloads
// stay id-less), week_count_of / count_weeks / position helpers unchanged.

// New id-centric reads (used by commits 2–3; harmless now):
pub fn find_week(&self, id: WeekId) -> Option<(PeriodId, &WeekDesc)> { … }   // linear scan, transitional
pub fn week_position(&self, id: WeekId) -> Option<(PeriodId, usize)> { … }
pub fn week_id_at(&self, period: PeriodId, pos: usize) -> Option<WeekId> { … }
/// Global position of a week (index in `walk()` order).
pub fn global_week_position(&self, id: WeekId) -> Option<usize> { … }
```

`from_period_rows` re-signed per **S12**: rows become
`Vec<(PeriodId, Vec<(WeekId, WeekDesc)>)>` (decode is the only caller).
`find_period` (kept **pub** since commit 0, for the `read_api` pointer-identity oracle)
now returns `&Vec<(WeekId, WeekDesc)>`; the `Lookup<PeriodId>` impl and `ids.rs`
`#[entity(…)]` follow suit (nothing calls `.lookup()` with a `PeriodId` directly —
verified — only `Join` derives resolve against it).

**Deviations landed (vs the sketch above), to keep commit 1 minimal:**

- **`weeks_of` keeps yielding `&WeekDesc`** (ids stripped), *not* `(WeekId, &WeekDesc)`.
  Its ~7 consumers (constraints, storage encode, gtk4, xlsx) only read the description,
  so leaving the item type unchanged means they are **untouched** this commit. `walk()`
  is still promoted to the 3-tuple (S3) — it is the global iterator where the id is
  worth carrying, and its ~5 accumulate-loop consumers each gain one `_`. Commit 5 stays
  invisible either way (`&WeekDesc` → `&Week` is field-compatible; the tuple shape is not
  what the consumers destructure). A dedicated period-scoped id iterator is added when
  commit 3 first needs it, rather than overloading `weeks_of`.
- **The id-centric reads (`find_week`/`week_position`/`week_id_at`/`global_week_position`)
  are deferred to the commit that first uses them** (commit 2's `apply_week`). Only
  `week_ids()` (feeding `all_ids`) is added now, since §1.3 needs it. Shipping unused
  `pub` reads early is harmless but adds-when-used keeps each commit's surface honest.

### 1.3 Ids join the global id space (`lib.rs` / `colloscope_params.rs`)

`Parameters::ids()` must yield week ids too — this is what makes
`check_no_duplicate_ids` cover them, seeds `IdIssuer::new(inner_data.ids())` in
`Data::from_inner_data`, and keeps the `check_invariants` high-water-mark panic
(`lib.rs:338-346`) honest. No separate week-id checker is needed for the inline shape:
it structurally cannot desynchronize.

### 1.4 Ops keep working, ids survive undo (`ops.rs`, `periods.rs`)

Public `PeriodOp` is **unchanged** (payloads stay `Vec<WeekDesc>`). The annotated form
carries ids — same pattern as `AddFront` already annotating a `PeriodId`:

```rust
pub enum AnnotatedPeriodOp {
    ChangeStartDate(Option<collomatique_time::WeekStart>),
    AddFront(PeriodId, Vec<(WeekId, WeekDesc)>),
    AddAfter(PeriodId, PeriodId, Vec<(WeekId, WeekDesc)>),
    Remove(PeriodId),
    /// Transitional (dies in commit 4): annotate issues a fresh id per
    /// position; apply keeps existing ids on the common prefix and consumes
    /// annotated ids only for growth (annotate cannot see the data — S4).
    Update(PeriodId, Vec<(WeekId, WeekDesc)>),
}
```

`AnnotatedPeriodOp::annotate` (`ops.rs:809-828`): `AddFront`/`AddAfter`/`Update` map each
`WeekDesc` to `(id_issuer.get_week_id(), desc)`. `apply_period`:

- `AddFront`/`AddAfter`: insert the annotated `(WeekId, WeekDesc)` vec verbatim (fails on
  duplicate id via the post-apply `check_invariants`, as everywhere).
- `Remove`: reverse op carries the removed vec **with its original ids** — undo restores
  them exactly (`InnerData::Eq` now includes ids, so the property harness pins this).
- `Update`: `new_vec[i].0 = old_vec[i].0` for `i < min(len)`, annotated ids for the tail;
  reverse = `Update(period_id, old_vec)`.

Redo replays the stored annotated op ⇒ same prefix ids from state, same growth ids from
the annotation ⇒ deterministic.

### 1.5 Storage decode synthesis (`decode/spec2.rs`)

Per **S11**:

```rust
/// Highest id used anywhere in the file (periods, subjects, teachers, students,
/// week patterns, slots, incompats, group lists, pairings, slot pairings).
fn max_used_id(file: &format::File) -> u64 { … }   // ~20 lines, one arm per block

fn reconstruct_periods(block: …, next_id: &mut u64) -> Result<mem::periods::Periods, DecodeError> {
    // walk order; ids max_used+1, max_used+2, …
    let rows = block.periods.into_iter().map(|period| {
        (id(period.id),
         period.weeks.into_iter().map(|week| {
             let week_id = unsafe { WeekId::new(*next_id) };
             *next_id += 1;
             (week_id, mem::periods::WeekDesc { interrogations: week.interrogations,
                                                annotation: week.annotation })
         }).collect())
    }).collect();
    mem::periods::Periods::from_period_rows(first_week, rows)
        .map_err(|_| DecodeError::DuplicatedID)
}
```

Encode is untouched beyond commit 0's `walk()` (it never sees ids ⇒ **bytes identical**).

### 1.6 Call-site ripple + gates

`walk()`/`weeks_of` destructuring gains one `_` element at the ~25 commit-0 call sites
(e.g. `for (period_id, desc) in …` → `for (period_id, _week_id, desc) in …`). Nothing
else outside `periods.rs`/`ops.rs`/`ids.rs`/`lib.rs`/decode changes.

Gates: full workspace tests; 100-seed harness (now also exercising id preservation
through period ops + undo); byte-stability + hogwarts pristine. **Watch item:** if any
round-trip test compares `InnerData` for *equality* across save/load (rather than bytes),
it will now fail — ops-issued week ids differ from decode-synthesized ones. Expected
resolution: such a test should compare re-encoded bytes (the established norm); adjust if
found.

---

## Commit 2 — `WeekOp` family: the primitives, fuzz-covered before anyone uses them

**Goal:** the new elementary ops exist, enforce all guards, maintain patterns + colloscope
transitionally, and are hammered by the property harness. No composite op or UI uses them
yet.

### 2.1 The ops (`ops.rs`)

```rust
/// Week operation enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeekOp {
    /// Add a week at the front of a period
    AddFront(PeriodId, periods::WeekDesc),
    /// Add a week right after an existing week
    AddAfter(WeekId, periods::WeekDesc),
    /// Remove an existing week
    Remove(WeekId),
    /// Update status/annotation of an existing week
    Update(WeekId, periods::WeekDesc),
    /// Move a week (same or different period), preserving its id.
    /// The position is interpreted after the week is detached.
    Move(WeekId, PeriodId, usize),
}

pub enum AnnotatedWeekOp {
    AddFront(WeekId, PeriodId, periods::WeekDesc),   // new id first, like AddAfter below
    AddAfter(WeekId, WeekId, periods::WeekDesc),     // (new_id, after_id, desc)
    Remove(WeekId),
    Update(WeekId, periods::WeekDesc),
    Move(WeekId, PeriodId, usize),
}
```

Wire-up: `Op::Week(WeekOp)` / `AnnotatedOp::Week(AnnotatedWeekOp)` variants, `From` impl,
`annotate` arm (Add* issue `NewId::WeekId`), `lib.rs` apply dispatch arm
(`AnnotatedOp::Week(op) => AnnotatedOp::Week(self.apply_week(op)?)`), `Error::Week`
variant.

### 2.2 `apply_week` (`periods.rs`) — semantics per variant

New error enum:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekError {
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),
    #[error("invalid week id ({0:?})")]
    InvalidWeekId(WeekId),
    #[error("week id ({0:?}) already exists")]
    WeekIdAlreadyExists(WeekId),
    #[error("invalid position ({1}) in period ({0:?})")]
    InvalidPosition(PeriodId, usize),
    #[error("week pattern {1:?} is not trivial on week {0:?}")]
    NonTrivialWeekPattern(WeekId, WeekPatternId),
    #[error("slot {1:?} in colloscope blocks the operation on week {0:?}")]
    NotCompatibleSlotInColloscope(WeekId, SlotId),
}
```

All variants translate `WeekId → (period, per-period pos)` via `week_position()` and to
the global position via `global_week_position()`.

- **`AddFront(period, desc)` / `AddAfter(week, desc)`** — validate the anchor; splice the
  `(id, desc)` entry in; then transitional maintenance at global position `g`:
  - every `WeekPattern`: `add_weeks(g, 1)` (splices `true` — exactly what `apply_period`
    does today at `periods.rs:210-218`);
  - every slot in the period's `ColloscopePeriod.slot_map`: insert one cell at the
    per-period position — `Some(default)` iff `desc.interrogations` (all pattern bits at
    `g` are `true` by construction), else `None`.
- **`Remove(week)`** — guards (S6): every pattern `can_remove_weeks(g, 1)` else
  `NonTrivialWeekPattern`; every slot's cell at the position is `None`/`Some(empty)` else
  `NotCompatibleSlotInColloscope`. Then splice out the entry, the pattern bit, and each
  slot's cell. Reverse: `AddFront`/`AddAfter` (depending on `pos == 0`) **with the
  original id** — exact restoration holds because the guards pinned the removed state to
  the trivial one (bit `true`, cells empty ⇒ recreated identically from
  `desc.interrogations`).
- **`Update(week, desc)`** — annotation changes are unconditional. `interrogations`
  `true→false`: reuse today's guard/refresh pair exactly as `apply_period::Update` does
  (`periods.rs:462-484` then `:515-526`): per slot,
  `build_pattern_for_new_period` on the period + `check_empty_on_removed_weeks`, else
  `NotCompatibleSlotInColloscope`; after the write, loop
  `update_slot_to_match_week_pattern` over all slots. Reverse: `Update(week, old_desc)`.
- **`Move(week, dest_period, dest_pos)`** — content travels (S5):
  - guards: `dest_period` valid; `dest_pos <= week_count_of(dest_after_detach)` else
    `InvalidPosition`; for each slot with a **non-empty** cell at the source position:
    dest `slot_map` must contain the slot AND the cell's `assigned_groups` must satisfy
    the dest `(dest_period, subject)` association bounds (mirroring
    `apply_colloscope::UpdateInterrogation` validation) — else
    `NotCompatibleSlotInColloscope`;
  - effect: detach `(id, desc)`; splice each pattern's *actual bit* from old global pos to
    new global pos (no triviality guard — no information is lost); per slot: if the slot
    exists on both sides, the cell travels verbatim (activity is `desc.interrogations ∧
    pattern-bit`, and both travel ⇒ unchanged); if the slot exists only at the source
    (subject excluded on dest), the guard already ensured emptiness — drop the cell; if
    only at the dest, create `Some(default)`/`None` from activity;
  - reverse: `Move(week, source_period, source_pos)`.

This is ~250 lines of transitional maintenance; it leans on the existing helpers
(`build_pattern_for_new_period`, slot-level `update_slot_for_week_pattern`,
`check_empty_on_removed_weeks`) and dies in 1d (cells) / B2 (patterns).

### 2.3 Coverage in the same commit

- **testgen** (`generator.rs`): `Pools` gains `week_ids: Vec<WeekId>` (from `walk()`);
  new `gen_week(rng, inner, pools, invalid)` emitting all five variants (invalid arm:
  dangling `WeekId`/`PeriodId`, out-of-range positions — mirroring `gen_period`); wired
  into the `gen_op` distribution. The 100-seed harness now drives the splice logic against
  `check_invariants` **before any UI depends on it** — this is the de-risking payoff of
  the split.
- **unit tests** (`state-colloscopes/tests/`): targeted scenarios the fuzzer may not
  reliably hit — `Move` of a week with non-empty cells to a compatible period preserves
  content; `Move` blocked when dest lacks the slot; `Remove` blocked by a `false` pattern
  bit; `Remove`+undo restores the id (assert via `week_id_at`); `Update(false)` blocked by
  a non-empty cell.
- **error display sweep**: `Error::Week`/`WeekError` arms in gtk4/python error rendering
  (mechanical; no caller can produce them yet).

Gates: standard (workspace tests, 100 seeds, byte-stability — untouched by this commit).

**Deviations landed (vs the sketch above):**
- Two small primitives were added rather than open-coding their effect inline:
  `OrderedTable::get_mut` (state crate — mutable access to a period's inline week
  vec; value-only, cannot disturb key order) and `WeekPattern::move_week(from, to)`
  (relocates an arbitrary bit, which is exactly the pattern splice `WeekOp::Move`
  needs — `add_weeks`/`remove_weeks` only handle trivial `true` bits).
- `gen_week` takes `(rng, pools, invalid)` (no `inner`): the live pools already
  carry everything it needs (`period_ids`, `week_ids`). Its invalid arm covers a
  dangling period (`AddFront`), a dangling week (`Remove`) and a dangling-week
  `Move`; positions are drawn small so most valid-branch ops land, and an
  occasional out-of-range `Move` exercises `InvalidPosition` as a rejected outcome.
- **No error-display sweep was needed**: the top-level `Error` is consumed via
  `if let Error::X(..)` sites, never an exhaustive `match`, so the new `Error::Week`
  arm compiles everywhere untouched; nothing produces a `WeekError` yet anyway.

---

## Commit 3 — composite ops emit week ops; cut/merge preserve content

**Goal:** `ops/src/general_planning.rs` re-cut. `GeneralPlanningUpdateOp` enum unchanged
(gtk4 contract). Period ops used only as `ChangeStartDate` / `AddFront(vec![])` /
`AddAfter(id, vec![])` / `Remove(week-empty)`.

Per variant of `apply_no_cleaning` (`:873-1361`):

- **`AddNewPeriod(n)`** (`:910`): `PeriodOp::AddFront(vec![])` or `AddAfter(last, vec![])`
  → new period id; then `WeekOp::AddFront(new_id, WeekDesc::new(true))` for the first week
  and `WeekOp::AddAfter(prev_week_id, WeekDesc::new(true))` for the rest (each `apply`
  returns `NewId::WeekId` to chain on).
- **`UpdatePeriodWeekCount(p, n)`** (`:941`): grow = `WeekOp::AddAfter` × (n−m) off the
  last week id (or `AddFront` if empty); shrink = `WeekOp::Remove` on the tail ids,
  last-to-first. The pre-existing cleaning-op machinery (`get_next_cleaning_op`, including
  the phase-0 bugfix loop) keeps running first at the ops layer and re-cuts its scans from
  positional walks to `weeks_of(p)` — the phase-0 regression test
  (`ops/tests/found_bugs.rs`) pins that behavior across the re-cut.
- **`DeletePeriod(p)`** (`:991`): cleaning as today, then `WeekOp::Remove` for every week
  of `p` (last-to-first), then `PeriodOp::Remove(p)`. (Transitionally `apply_period`'s
  Remove still carries its own week machinery; it no-ops on an already-empty period.
  Commit 4 deletes it.)
- **`CutPeriod(p, k)`** (`:1007`): keep the reference-propagation block exactly as today
  (subject/student `excluded_periods`, assignments, group-list associations —
  `:1053-1164`) **but reorder**: create the new period (`AddAfter(p, vec![])`) →
  propagate exclusions/assignments/associations → then `WeekOp::Move(week_id, new_id,
  next_pos)` for each tail week in order. The ordering matters: `Move`'s
  association-bounds guard needs the dest `(period, subject)` associations in place
  before non-empty cells travel. **Delete** the `save_then_clean_end_of_period` /
  `restore_end_of_period` calls here — content now survives by construction.
- **`MergeWithPreviousPeriod(p)`** (`:1190`): `WeekOp::Move(week_id, previous_id,
  append_pos)` for each week of `p` in order, then the recursive
  `DeletePeriod(p)` (now week-empty) as today. Delete the save/restore calls.
- **`UpdateWeekStatus(p, w, s)` / `UpdateWeekAnnotation(p, w, a)`** (`:1279`, `:1320`):
  `let week_id = periods.week_id_at(p, w)`; emit `WeekOp::Update(week_id, new_desc)`
  (clone the current desc via `weeks_of`, flip one field).
- **Delete outright** once unreferenced: `save_then_clean_end_of_period` /
  `restore_end_of_period` (`:1364-1526`) and `WeekPattern::clean_weeks`
  (`week_patterns.rs:55-64`, its only caller).

**Behavior note (pin with tests):** today cut/merge already round-trips content via
save/clean/restore, so user-visible state is near-identical — what changes is that week
identities survive (invisible until B2/1d makes them load-bearing) and the undo history
records moves instead of clean/restore churn. Add an ops-level regression test:
build a doc, fill a colloscope cell + a `false` pattern bit in the tail of a period, cut
the period through the composite op, assert the cell and the bit landed in the new period
(then merge back and re-assert). This is the contract 1d/B2 will rely on.

Gates: standard + the new regression tests; gtk4 compiles untouched (enum unchanged).

---

## Commit 4 — slim `PeriodOp`; delete the transitional period-op week machinery

**Goal:** exactly one writer for week data (`apply_week`); `PeriodOp` reaches its final
shape (S7/S8). Almost pure deletion.

```rust
pub enum PeriodOp {
    ChangeStartDate(Option<collomatique_time::WeekStart>),
    AddFront,                    // created empty
    AddAfter(PeriodId),
    Remove(PeriodId),            // must be week-empty
}

pub enum AnnotatedPeriodOp {
    ChangeStartDate(Option<collomatique_time::WeekStart>),
    AddFront(PeriodId),
    AddAfter(PeriodId, PeriodId),
    Remove(PeriodId),
}
```

- `apply_period`: `Add*` insert an empty period + an empty `ColloscopePeriod`
  (`new_empty_from_params` on a week-less period = slots with zero-length vecs; still
  needed until 1d) and **no pattern splicing** (zero weeks). `Remove` gains the
  week-empty precondition (new `PeriodError::PeriodStillHasWeeks(PeriodId)`), keeps all
  reference guards (`:311-376`), and **loses** the week-pattern/colloscope machinery
  (`:290-309`, `:413-421`) and the whole `Update` arm (`:429-528`).
  The `Update` reverse-op id-preservation wart (S4) dies with it.
- `PeriodError`: delete `NonTrivialWeekPattern` and `NotCompatibleSlotInColloscope`
  variants if now unreferenced (they moved to `WeekError`); sweep gtk4/python error
  `match`es.
- testgen `gen_period` (`generator.rs:316-348`): re-cut to the new variants (no desc
  payloads; `Update` arm gone — `gen_week` from commit 2 already covers week mutation);
  `synth::week_desc_vec` likely becomes unused → delete.
- `ops/` and gtk4: emission sites already conform (commit 3); only enum-shape mechanical
  fixes (`AddFront(vec![])` → `AddFront`).
- `weeks_vec_of`: delete if the last payload-building caller is gone (check gtk4 dialogs
  first — they may still build `Vec<WeekDesc>` UI models from it; keep if so).

Gates: standard. The 100-seed harness re-validates that period ops + week ops compose
(generator emits both).

---

## Commit 5 — backend swap: `week_map` + ordering sidecar (`Week` entity is born)

**Goal:** the design-doc structure (decision 7), invisible outside `periods.rs` + storage
decode + the checker, because every consumer sits on the commit-0/1 surface.

```rust
/// Description of a single week
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join)]
#[join(error = NewId)]
pub struct Week {
    /// Period this week belongs to (authoritative; the ordering sidecar groups it here)
    #[fk(name = period)]
    pub period_id: PeriodId,
    pub interrogations: bool,
    pub annotation: Option<non_empty_string::NonEmptyString>,
}

pub struct Periods {
    pub first_week: Option<collomatique_time::WeekStart>,
    /// Period order + per-period week order (private, slots-style encapsulation)
    ordered_period_list: OrderedTable<PeriodId, Vec<WeekId>>,
    week_map: Table<WeekId, Week>,
}
```

- **Mirror invariant** (slots precedent): `week_map` keyset == ∪ ordering vecs;
  `week.period_id` == owning period. Compound `pub(crate)` mutators used by
  `apply_period`/`apply_week` (which now live partly outside the struct's direct reach —
  same-module, so direct field access technically remains; still route through compound
  helpers `insert_week_at` / `remove_week` / `move_week` / `update_week` /
  `insert_period_at` / `remove_period` so no future call site can desync the pair).
  New `check_periods_data_consistency` in `colloscope_params.rs` validates the mirror
  (the role `check_slots_data_consistency` plays for slots), wired into
  `Parameters::check_invariants`.
- **`WeekDesc` survives as op payload / DTO only** (`{ interrogations, annotation }`,
  no FK) — used by `WeekOp`, gtk4 dialogs, python glue. Conversions at the op boundary:
  `Week { period_id, interrogations: desc.interrogations, annotation: desc.annotation }`,
  plus a `Week::desc(&self) -> WeekDesc` helper for the reverse.
- **Surface re-implementation**: `walk()` → iterate ordering, look up `week_map`, item
  `(PeriodId, WeekId, &Week)`; `weeks_of` item `(WeekId, &Week)` — per S3, field-name
  compatibility makes this invisible at field-access call sites (sweep the few sites that
  *name* `WeekDesc` in bindings). `find_week` becomes O(log n) via `week_map` and returns
  `(week.period_id, &week)`. `from_period_rows` keeps its commit-1 signature; body builds
  both structures and reports `DuplicatedWeekIdError`.
- **ids/lookup**: `#[entity(Week)]` on `WeekId`; `#[entity(Vec<WeekId>)]` on `PeriodId`;
  `impl Lookup<WeekId> for Parameters` (→ `&Week`) and `Lookup<PeriodId>` re-targeted to
  `Vec<WeekId>`.
- **refs registry** (`refs.rs`): `Week.period_id` is a new FK edge — add a
  `RefSite::WeekPeriodFk`-style variant and walk it (one loop over `week_map`), so
  `references_to_period` keeps its completeness contract.
- **storage decode**: only `reconstruct_periods`' row-building closure changes shape-side
  (it already emits `(WeekId, WeekDesc)` rows; unchanged per S12). Encode untouched.
  **Bytes identical.**

Gates: standard, **plus the B1-closing sweep**: 500-seed harness run, byte-stability +
hogwarts pristine, and a user-run gtk4 smoke (edit weeks/periods, cut/merge with a filled
colloscope, undo/redo, save/reload) + the 3 contract scripts. (The design doc puts the ★
milestone at 1d, but commit 5 closes the riskiest reshape of 1b — cheap insurance now.)

---

## What comes after (unchanged from plan_step_1.md)

Commit B2 (week patterns → `excluded_weeks: BTreeSet<WeekId>`) proceeds exactly as
written in `docs/plans/plan_step_1.md` §Phase 4/B2 — commit 2's pattern-splice
maintenance in `apply_week` is precisely what B2 deletes. Then phase 5 (1d) deletes the
colloscope cell-splice maintenance.

Also: record this split — supersede the B1 section of `docs/plans/plan_step_1.md` with a
pointer to this document (committed under `docs/plans/`, e.g.
`docs/plans/plan_step_1_b1_split.md`) in the first commit.

## Verification (every commit)

1. `cargo build --workspace` + `cargo test --workspace` (no clippy — house rule).
2. Property harness 100 seeds (in `cargo test -p collomatique-state-colloscopes`);
   500 seeds after commit 5.
3. Byte-stability: `spec2_format.rs` re-serialize tests, `populated_round_trip.rs`,
   `all_examples_load_pristine` (hogwarts, zero caveats).
4. After commit 5, user-run: gtk4 smoke + 3 contract scripts (`import.py`,
   `import_pronote_web_2026_05_06.py`, `custom_export_xlsx.py`).
5. No new dependencies ⇒ no `Cargo.lock` change ⇒ no Nix `cargoHash` refresh.

## Risks & watch items

- **Commit 0 is wide** (~50 sites, 8 crates) but every hunk is one of three mechanical
  shapes; the compiler drives the sweep (field goes private ⇒ every missed site is a
  build error, not a latent bug).
- **`InnerData`-equality round-trip tests** (commit 1): decode-synthesized week ids
  differ from ops-issued ones; any test comparing `InnerData` across save/load must
  compare re-encoded bytes instead. Audit `populated_round_trip.rs` when landing.
- **`Move` guard completeness** (commit 2): `apply` panics via post-op
  `check_invariants` if a guard is missing (e.g. association bounds). The fuzzer +
  the targeted unit tests are the net; treat any harness panic as a missing guard, not
  a checker bug.
- **Composite-op ordering in `CutPeriod`** (commit 3): associations must be propagated
  before weeks move, or `Move`'s bounds guard fires spuriously. Pinned by the
  cut-preserves-content regression test.
- **Error-variant churn** (commits 2/4): `WeekError` additions and `PeriodError`
  deletions touch gtk4/python display `match`es — sweep in the same commits; the big
  vocabulary collapse still belongs to step 5 of the cascade plan, don't gold-plate.
- **Undo id-exactness** (commits 1–2): `InnerData::Eq` includes `WeekId`s, so the
  history-replay property tests catch any id churn on undo/redo — this is a feature
  (it pins S4/S6), but expect the first harness runs to find mistakes here.
