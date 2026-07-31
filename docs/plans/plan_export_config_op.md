# Pre-step-7 sidework — one whole-struct `ExportConfigOp::Update`

Status: **agreed plan** (July 31 2026, branch `consolidate_state`).
Scope: `state-colloscopes/`, `ops/`, `testgen-colloscopes/`, `storage/` (tests only),
plus two documentation commits (`state_consolidation_plan.md`, `plan_step_7.md`).

## 0. Why this exists, and the decisions

This closes the last residue of `state_consolidation_plan.md` §6 item 4 ("uniform op
granularity") **before** step 7 rewrites `ops/`, so the remaster never has to touch the
old 11-variant surface. A workspace survey (July 31 2026) confirmed:

- Every other elementary op family already has uniform granularity: whole-entity
  `Add`/`Remove`/`Update` (students, teachers, week patterns, incompats, pairings,
  slot pairings), position ops where order exists (subjects, slots), association ops
  where relational (group lists), whole-override-entry sets (settings, balancing),
  whole-row `AssignmentOp::SetRow`, the step-1 period/week re-cut, and the two sparse
  colloscope upserts. `ExportConfigOp` (`state-colloscopes/src/ops.rs:294`) is the
  **only** per-field litany left.
- The export config is **not exposed in the Python API at all** (zero matches for
  `export_config`/`ExportConfig` under `python/`). The three contract scripts are
  unaffected; no Python change in any commit.
- The elementary family has exactly four consumers: `force_apply_export_config`
  (`state-colloscopes/src/export_config.rs`), the ops-layer passthrough
  (`ops/src/export_config.rs`), the testgen generator
  (`testgen-colloscopes/src/generator.rs:1116`), and the storage round-trip builder
  (`storage/tests/populated_round_trip/builder.rs:760-850`).
- gtk4 uses only the ops-level `ExportConfigUpdateOp`; storage encode/decode reads
  `InnerData.export_config` directly (format structs, not ops). **Neither gtk4 nor the
  on-disk format moves.**

Decisions, all settled now (none deferred):

- **D1 — elementary shape.** `ExportConfigOp` becomes a single variant
  `Update(export_config::ExportConfig)` carrying the whole struct. Precedent:
  `AssignmentOp::SetRow` is a single-variant enum. `AnnotatedExportConfigOp` mirrors it;
  annotation is the identity (no ids to freeze). The `ExportConfigPrecheckError` enum
  stays, still empty, kept for uniformity across the `PrecheckError` family (same as
  today — see `invariant_cascade_design.md` around line 1176).
- **D2 — ops-level family KEEPS its eleven variants.** `ExportConfigUpdateOp` in
  `ops/src/export_config.rs` is the *user-facing* granularity: each variant carries its
  own French history description ("Mettre à jour l'activation de l'export du
  colloscope", …). Per the roadmap doctrine (§6 item 4: "user-facing granularity
  already lives in `ops/` descriptions"), each variant now **reads the current config,
  patches its one field, and issues the single elementary `Update`**. The read goes
  through `data.get_data().get_inner_data().export_config` — the exact pattern
  `ops/src/assignments.rs` and `ops/src/colloscope.rs` already use. gtk4's
  `export_panel.rs` is untouched.
- **D3 — migration workflow.** Add-alongside (commit 1), migrate every consumer
  (commit 2), remove the old variants (commit 3). Commits 2 and 3 are separate per the
  established migration workflow; the whole workspace builds and tests green at every
  commit.
- **D4 — byte-stability of the round-trip builder.** The builder must produce the
  **identical final `ExportConfig` value** (the golden byte-stability test pins the
  written bytes). Its rewrite composes one struct literal reproducing exactly the state
  the six old ops left behind — including the fields the old script never touched,
  which keep their `Default` values (see commit 2, site C).
- **D5 — generator seeds may shift.** Collapsing `gen_export_config` to one whole-struct
  draw changes how many RNG draws that path consumes, so generated op walks after an
  export-config op differ from before. This is harmless: the property harness is
  self-checking (invariants, undo-identity, inverse-identity), not golden-pinned to
  specific walks. No quarantine markers involve export config.
- **D6 — ContentOrd unchanged.** `ExportConfig` is already a document-order **atom**
  (`impl_content_ord_atom!`, step-6.5 decision 13): two different configurations are
  incomparable. A whole-struct replace is exactly the operation that ruling anticipated;
  no cascade fix ever touches export config (it references nothing), so the
  monotonicity assertion never sees it. Nothing to change.

---

## 1. Commit 1 — the elementary op (`state-colloscopes/`)

Add the whole-struct variant **alongside** the old ones. Three sites, all in
`state-colloscopes/`. Builds green; the new variant is simply unused until commit 2.

### Site A — `src/ops.rs`, the two enums

Old (`ops.rs:294`):

```rust
pub enum ExportConfigOp {
    UpdateGlobalConfig(export_config::GlobalConfig),
    UpdateColloscopeEnabled(bool),
    UpdateAllGroupsEnabled(bool),
    UpdatePrefilledGroupsEnabled(bool),
    UpdateAutomaticGroupsEnabled(bool),
    UpdatePerGroupListEnabled(bool),
    UpdateColloscopeConfig(export_config::ColloscopeConfig),
    UpdateAllGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdatePrefilledGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdateAutomaticGroupsConfig(export_config::PerStudentGroupsConfig),
    UpdatePerGroupListConfig(export_config::PerGroupListConfig),
}
```

New — commit 1 *adds* one variant at the top (the old ones die in commit 3):

```rust
pub enum ExportConfigOp {
    /// Replace the whole export configuration at once
    Update(export_config::ExportConfig),
    UpdateGlobalConfig(export_config::GlobalConfig),
    // ... the ten other old variants, unchanged until commit 3 ...
}
```

`AnnotatedExportConfigOp` gains the mirror variant `Update(export_config::ExportConfig)`,
and `AnnotatedExportConfigOp::annotate` gains the identity arm:

```rust
ExportConfigOp::Update(v) => AnnotatedExportConfigOp::Update(v),
```

### Site B — `src/export_config.rs`, `force_apply_export_config`

The match gains one arm, same `std::mem::replace` shape as every existing arm but on
the whole struct:

```rust
AnnotatedExportConfigOp::Update(v) => {
    let old = std::mem::replace(&mut self.inner_data.export_config, v.clone());
    AnnotatedExportConfigOp::Update(old)
}
```

The doc comment on `force_apply_export_config` ("pure value data with no guards of any
kind, so this copy is byte-identical to the original") stays true and stays put.

### Site C — unit coverage

`state-colloscopes` has no per-family op unit-test file for export config today (the
property harness covers it via the generator, which only migrates in commit 2). Commit 1
therefore adds a minimal in-crate test next to the existing test patterns: apply
`Update` with a non-default config, assert the data holds it, apply the returned
backward op, assert the original config is restored. This is the reversibility pin for
the new arm while the generator still exercises only the old ones.

---

## 2. Commit 2 — migrate every consumer

Three sites in three crates. After this commit the old eleven variants have **zero
constructors left in the workspace** (only the enum definition and its
`force_apply`/`annotate` arms remain, deleted in commit 3).

### Site A — `ops/src/export_config.rs`, `apply_no_cleaning`

Old (shape repeated eleven times, one per variant — e.g. the first two):

```rust
Self::UpdateGlobalConfig(v) => {
    let result = data
        .apply(
            collomatique_state_colloscopes::Op::ExportConfig(
                ExportConfigOp::UpdateGlobalConfig(v.clone()),
            ),
            self.get_desc(),
        )
        .expect("ExportConfigOp::UpdateGlobalConfig should never fail");
    assert!(result.is_none());
    Ok(())
}
Self::UpdateColloscopeEnabled(v) => {
    let result = data
        .apply(
            collomatique_state_colloscopes::Op::ExportConfig(
                ExportConfigOp::UpdateColloscopeEnabled(*v),
            ),
            self.get_desc(),
        )
        .expect("ExportConfigOp::UpdateColloscopeEnabled should never fail");
    assert!(result.is_none());
    Ok(())
}
```

New — one patch step building the whole struct, then one shared apply. The eleven
user-facing variants (and `get_desc` with its French descriptions) stay exactly as they
are:

```rust
pub(crate) fn apply_no_cleaning<
    T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
>(
    &self,
    data: &mut T,
) -> Result<(), ExportConfigUpdateError> {
    use collomatique_state_colloscopes::ExportConfigOp;

    let mut new_config = data.get_data().get_inner_data().export_config.clone();
    match self {
        Self::UpdateGlobalConfig(v) => new_config.global = v.clone(),
        Self::UpdateColloscopeEnabled(v) => new_config.colloscope_enabled = *v,
        Self::UpdateAllGroupsEnabled(v) => new_config.all_groups_enabled = *v,
        Self::UpdatePrefilledGroupsEnabled(v) => new_config.prefilled_groups_enabled = *v,
        Self::UpdateAutomaticGroupsEnabled(v) => new_config.automatic_groups_enabled = *v,
        Self::UpdatePerGroupListEnabled(v) => new_config.per_group_list_enabled = *v,
        Self::UpdateColloscopeConfig(v) => new_config.colloscope_config = v.clone(),
        Self::UpdateAllGroupsConfig(v) => new_config.all_groups_config = v.clone(),
        Self::UpdatePrefilledGroupsConfig(v) => new_config.prefilled_groups_config = v.clone(),
        Self::UpdateAutomaticGroupsConfig(v) => new_config.automatic_groups_config = v.clone(),
        Self::UpdatePerGroupListConfig(v) => new_config.per_group_list_config = v.clone(),
    }

    let result = data
        .apply(
            collomatique_state_colloscopes::Op::ExportConfig(ExportConfigOp::Update(
                new_config,
            )),
            self.get_desc(),
        )
        .expect("ExportConfigOp::Update should never fail");
    assert!(result.is_none());
    Ok(())
}
```

Note the file shrinks from ~190 lines of apply code to ~30. `get_next_cleaning_op`
(returns `None`), `ExportConfigUpdateError` (empty), `ExportConfigUpdateWarning`
(empty) and `get_desc` are untouched.

### Site B — `testgen-colloscopes/`

`src/synth.rs` gains a composer next to the four existing export-config synths
(`global_config`, `colloscope_config`, `per_student_groups_config`,
`per_group_list_config`):

```rust
pub fn export_config(rng: &mut ChaCha8Rng) -> export_config::ExportConfig {
    export_config::ExportConfig {
        global: global_config(rng),
        colloscope_enabled: rng.random_bool(0.5),
        all_groups_enabled: rng.random_bool(0.5),
        automatic_groups_enabled: rng.random_bool(0.5),
        prefilled_groups_enabled: rng.random_bool(0.5),
        per_group_list_enabled: rng.random_bool(0.5),
        colloscope_config: colloscope_config(rng),
        all_groups_config: per_student_groups_config(rng),
        automatic_groups_config: per_student_groups_config(rng),
        prefilled_groups_config: per_student_groups_config(rng),
        per_group_list_config: per_group_list_config(rng),
    }
}
```

`src/generator.rs` — old (`generator.rs:1116`):

```rust
fn gen_export_config(rng: &mut ChaCha8Rng) -> Op {
    let op = match rng.random_range(0..11) {
        0 => ExportConfigOp::UpdateGlobalConfig(synth::global_config(rng)),
        1 => ExportConfigOp::UpdateColloscopeEnabled(rng.random_bool(0.5)),
        // ... nine more arms ...
    };
    Op::ExportConfig(op)
}
```

New:

```rust
fn gen_export_config(rng: &mut ChaCha8Rng) -> Op {
    Op::ExportConfig(ExportConfigOp::Update(synth::export_config(rng)))
}
```

Seed walks shift downstream of any export-config op (decision D5 — accepted, the
harness is self-checking, not walk-pinned).

### Site C — `storage/tests/populated_round_trip/builder.rs`

The six per-field ops at `builder.rs:760-850` collapse into **one** `Update` whose
struct literal reproduces the identical final state (decision D4). The fields the old
script never touched must be spelled out with the same defaults the document had
(`Default for ExportConfig` seeds them at `Data` creation):

```rust
// Export configuration: one whole-struct op, values away from defaults
apply(
    &mut state,
    Op::ExportConfig(ExportConfigOp::Update(export_config::ExportConfig {
        global: export_config::GlobalConfig {
            background_color: export_config::Color { red: 240, green: 240, blue: 255 },
            stripes_color_enabled: false,
            stripes_color: export_config::Color { red: 200, green: 200, blue: 200 },
        },
        colloscope_enabled: false,
        all_groups_enabled: true,
        automatic_groups_enabled: false,
        prefilled_groups_enabled: true,
        per_group_list_enabled: true,
        colloscope_config: export_config::ColloscopeConfig {
            sheet_name: "Colloscope 2026".to_string(),
            // ... identical to the old UpdateColloscopeConfig literal,
            //     including the "Vacances" extra color ...
        },
        all_groups_config: export_config::PerStudentGroupsConfig {
            sheet_name: "Groupes".to_string(),
            orientation: Some(export_config::PageOrientation::Portrait),
            show_emails: true,
            show_tel: false,
        },
        automatic_groups_config:
            export_config::PerStudentGroupsConfig::default_automatic_groups(),
        prefilled_groups_config:
            export_config::PerStudentGroupsConfig::default_prefilled_groups(),
        per_group_list_config: export_config::PerGroupListConfig {
            orientation: export_config::PageOrientation::Landscape,
            show_emails: false,
            show_tel: true,
            center_vertically: true,
        },
    })),
    "export config",
);
```

The correspondence with the old script, field by field: `global`,
`colloscope_config`, `all_groups_config`, `per_group_list_config` carry the old ops'
literals verbatim; `colloscope_enabled: false` and `prefilled_groups_enabled: true`
were the two boolean ops; `all_groups_enabled: true`, `automatic_groups_enabled: false`,
`per_group_list_enabled: true`, `automatic_groups_config` and `prefilled_groups_config`
were never touched by the old script and take the exact `Default` values. The
byte-stability test (`reserialize` golden) is the proof this reproduction is exact — if
any field is wrong, it fails.

### Verification for commit 2

Full workspace suite (background, captured once to the scratchpad, then grepped) — in
particular `populated_round_trip` (byte-stability) and the property harness. This is a
normal standing-gate commit; the user runs tests regularly.

---

## 3. Commit 3 — remove the old variants

Pure deletion, compiler-driven. Sites, all in `state-colloscopes/`:

- `src/ops.rs`: delete the ten old variants from `ExportConfigOp` and
  `AnnotatedExportConfigOp` (leaving the single `Update`), and their ten identity arms
  in `AnnotatedExportConfigOp::annotate` (the function becomes a one-arm match; keep it
  as a function for uniformity with the other families' annotate dispatch).
- `src/export_config.rs`: delete the ten old arms of `force_apply_export_config`; the
  match keeps only the `Update` arm.

Nothing else can reference them — commit 2 removed every constructor. A
`grep -rn "UpdateGlobalConfig\|UpdateColloscopeEnabled\|UpdateAllGroupsEnabled\|UpdatePrefilledGroupsEnabled\|UpdateAutomaticGroupsEnabled\|UpdatePerGroupListEnabled\|UpdateColloscopeConfig\|UpdateAllGroupsConfig\|UpdatePrefilledGroupsConfig\|UpdateAutomaticGroupsConfig\|UpdatePerGroupListConfig"`
after the deletion must hit **only** `ops/src/export_config.rs` (the kept user-facing
`ExportConfigUpdateOp` variants and their `get_desc` arms) and gtk4's
`export_panel.rs` (which constructs those ops-level variants). Full workspace suite
green.

---

## 4. Commit 4 — amend `docs/plans/state_consolidation_plan.md`

Documentation only. Three edits, matching the document's established annotation style:

- **§6 item 4 (uniform op granularity)** — mark **DONE (July 2026)**: the survey found
  every family already uniform after the step-1 re-cuts and earlier reshapes
  (`AssignmentOp::SetRow` had long replaced the single-bool op); the last residue —
  `ExportConfigOp`'s 11 per-field variants — collapsed into one whole-struct
  `Update(ExportConfig)` in this sidework (commits 1–3). User-facing per-field
  granularity lives on in `ops/`' `ExportConfigUpdateOp` descriptions, exactly as the
  item prescribed.
- **§6 item 7 (Python glue)** — mark **DONE / closed as a non-item**: the write path
  was and remains insulated behind `ops::UpdateOp`; the read-path pyclass mirrors have
  been regenerated mechanically in the same change every time a struct moved
  (steps 1a/1c/1d glue notes); no Python API redesign happens in this phase (still
  expected later with the MVVM UI work). The export config is not exposed in Python at
  all, so this sidework has zero Python surface. The three contract scripts remain the
  acceptance oracle for future changes.
- **§1 problem 5 (inconsistent op granularity)** — annotate *(resolved — phase 2
  item 4)*, same style as problems 1/3/4/6.

---

## 5. Commit 5 — amend `docs/plans/plan_step_7.md`

The new op changes exactly one section of the step-7 draft; nothing else in the plan or
in `invariant_cascade_design.md` pins the 11-variant elementary shape
(`ExportConfigPrecheckError` stays, still empty; the ContentOrd atom ruling D6 stays).

- **§3.1 `export_config.rs`** — rewrite the family description. Old text: "Eleven
  variants, all 1:1 elementary passthroughs, error enum **empty**, every apply
  `.expect("… should never fail")`. Change: `data.apply` → `session.apply`, expects
  kept." New text (to the same effect): eleven **ops-level** variants, each a
  read-patch-replace onto the single whole-struct elementary
  `ExportConfigOp::Update` (pre-step-7 sidework, this plan); error enum still empty,
  the one apply still `.expect`s. Change for step 7: the current-config read moves from
  `data.get_data().get_inner_data()` to the session's read surface (which other
  families — assignments, colloscope — already require), and `data.apply` →
  `session.apply`. Still the trivial family; fixture unchanged (one op round-trips,
  zero warnings).
- Sweep the rest of the draft for stale mentions while editing (§2.4's translation
  doctrine and the §3 family table don't enumerate export-config variants; the
  commit-plan table row "3.1" needs no change).

---

## 6. Gates

- Every commit: full workspace suite in the background, captured once, grepped.
- Commit 2 specifically proves D4 via the byte-stability golden.
- No `Cargo.lock` change anywhere (no new dependencies) — no cargoHash refresh.
- No Python, no gtk4, no storage-format change; no contract-script run required by this
  sidework (zero Python surface), the user's regular testing cadence covers the rest.
