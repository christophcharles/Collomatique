# Global GroupList update + unified GTK dialog — session plan

Branch: `global_grouplist_update` (branched off `consolidate_state` at `ee95fe44`).
Status: DRAFT for user sign-off.

## 1. Context — why this change

The GTK application currently edits a group list through **two separate dialogs**:
`gtk4/src/editor/group_lists/params_dialog.rs` edits the general parameters (name,
students-per-group range, group count and group names) and
`gtk4/src/editor/group_lists/prefill_dialog.rs` edits the filling (either the automatic
mode with its excluded students, or the prefilled mode with explicit per-group student
lists). The user wants **one dialog**: a window twice as wide, containing a `gtk::Paned`
with the params editor on the left and the prefill editor on the right. Because the group
count becomes editable *while* the prefill pane is visible, the dialog must handle a
dynamic group count, and must keep the data of groups that disappear when the count is
reduced, so that raising the count again restores them.

The split is not only a UI fact. The high-level `ops/` layer still exposes two separate
update operations — `GroupListsUpdateOp::UpdateGroupList(id, GroupListParameters)` and
`GroupListsUpdateOp::SetFilling(id, GroupListFilling)` — even though the low-level state
layer already speaks whole sealed `GroupList` values (`GroupListOp::Update(id, GroupList)`,
delivered by the pre-step-5 loose-ends work, design doc Appendix F.2/F.3). The gtk4
dispatch path applies **one op per user action** (dry-apply → warning dialog → commit,
`gtk4/src/editor.rs:1061-1096`); there is no batch mechanism, so a merged dialog cannot
emit two ops on Accept — it needs a single op carrying the whole new `GroupList`.

Consolidating now also removes a real crash: `SetFilling`'s translator cannot see the
params it is being paired with, so it rebuilds via
`GroupList::new(old_params, filling).expect("caller guarantees prefill arity")`
(`ops/src/group_lists.rs:1039-1043`). A Python script calling `group_lists_set_filling`
with a group count different from `len(group_names)` **panics the process** today —
`extra-scripts/import.py:447-450` dances around this by carefully ordering its two calls.
With one op carrying a sealed `GroupList`, the mismatch is unrepresentable.

Finally, this simplifies step 7 (the `ops/` remaster): the group-list family shrinks from
six to five variants, and the awkward reshaping/cleaning round-trip between
`UpdateGroupList` and `SetFilling` disappears before the remaster has to reproduce it.

## 2. Current state — the code being changed

### 2.1 The op enum and its translators (`ops/src/group_lists.rs`, 1243 lines)

```rust
// ops/src/group_lists.rs:263-281 (OLD)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupListsUpdateOp {
    AddNewGroupList(collomatique_state_colloscopes::group_lists::GroupListParameters),
    UpdateGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupListParameters,
    ),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
    SetFilling(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupListFilling,
    ),
    AssignGroupListToSubject(PeriodId, SubjectId, Option<GroupListId>),
    DuplicatePreviousPeriod(PeriodId),
}
```

The two update translators in `apply_no_cleaning` both end in the same low-level
`GroupListOp::Update(id, GroupList)`, but each reconstructs the half it does not carry:

- `UpdateGroupList` (`:853-923`) keeps the old filling and hosts the grow/shrink
  reshaping: grow pads `PrefilledGroup::default()`, shrink truncates with
  `assert!(group.students.is_empty(), "cleaning phase should have emptied the dropped
  groups")` (`:885-888`).
- `SetFilling` (`:973-1059`) keeps the old params, sweeps every student id in the filling
  for existence (`SetFillingError::InvalidStudentId`), then rebuilds with the arity
  `.expect` quoted above.

The cleaning phase (`get_next_cleaning_op`, `:344-821`) returns one `CleaningOp
{ warning, op }` at a time; `UpdateOp::rec_apply_no_session` (`ops/src/lib.rs:415-435`)
loops until `None`. The arms relevant here:

- `UpdateGroupList` arm (`:353-475`), three checks in order:
  1. `:365-392` — if the list is **not** prefilled and a colloscope placement row exists,
     any student placed in a group `>= params.group_names.len()` is removed one at a time
     (`ColloscopeUpdateOp::UpdateColloscopeGroupList`, warning
     `LooseStudentGroupInColloscope`).
  2. `:394-433` — for every subject associated to this list, interrogation cells (on
     weeks of the association's period) containing out-of-range group numbers are
     trimmed (`UpdateColloscopeInterrogation`, warning
     `LooseGroupsInInterrogationsInColloscope`).
  3. `:435-472` — **the shrink pre-empt**: if the old filling is `Prefilled` and the new
     count is smaller and the dropped groups hold students, emit ONE cleaning
     `SetFilling(id, Prefilled { same count, dropped groups emptied })` with warning
     `LooseStudentsInPrefilledGroupList(id, students)`. This is what makes the
     translator's `assert!` hold.
- `SetFilling` arm (`:605-678`), two checks:
  1. `:617-648` — new filling `Automatic` on a currently non-prefilled list: placements
     of newly-excluded students are removed one at a time
     (`LooseStudentGroupInColloscope`).
  2. `:650-675` — transition non-prefilled → prefilled: the colloscope placement row is
     emptied one student at a time (`LooseStudentGroupInColloscope`).

Internal emitters of `SetFilling` as a *cleaning vehicle* (all keep params unchanged —
they only rewrite the filling):

- `ops/src/group_lists.rs:461` (the shrink pre-empt above);
- `ops/src/group_lists.rs:539` (`DeleteGroupList`: clear `excluded_students`, warning
  `LooseExcludedStudents`);
- `ops/src/group_lists.rs:573` (`DeleteGroupList`: reset a prefilled list to
  `GroupListFilling::default()`, warning `LooseWholePrefilledGroupList`);
- `ops/src/students.rs:270` (student deletion: remove the student from prefilled groups,
  warning `LoosePrefilledGroup`);
- `ops/src/students.rs:288` (student deletion: remove the student from
  `excluded_students`, warning `LooseExclusionFromGroupList`).

Error enums today: `UpdateGroupListError { InvalidGroupListId }` (`:297-301`),
`SetFillingError { InvalidGroupListId, InvalidStudentId }` (`:309-315`), folded into
`GroupListsUpdateError` (`:283-295`) which `UpdateError::GroupLists` forwards
(`ops/src/lib.rs:115-116`). `AddNewGroupList` currently has **no** error (its default
`Automatic` filling contains no student ids, so nothing can dangle).

The sealed value (`state-colloscopes/src/group_lists.rs`): `GroupList { params, filling }`
private fields, built only by `GroupList::new(params, filling) -> Result<_,
GroupListBuildError>` (`:266-293`, checks prefilled-count == group_names count and no
duplicated student), serde via a private `RawGroupList` mirror, plus derives
`Clone, Debug, PartialEq, Eq, Serialize, Deserialize` — so it satisfies every bound the
op enum needs and can be an op payload directly.

### 2.2 The gtk4 side

- `params_dialog.rs` (427 lines): relm4 `SimpleComponent`, `adw::Window` 500×500,
  in `Show(GroupListParameters)` / out `Accepted(GroupListParameters)`. Notable: its
  `group_name_data: Vec<String>` **already implements the keep-on-shrink pattern** — the
  vector only ever grows (`update_group_name_entries`, `:314-335`) and `generate_data`
  reads `.take(selected_max_group_count)` (`:305-310`), so lowering then raising the
  count restores previously typed names.
- `prefill_dialog.rs` (979 lines): `adw::Window` 500×700, in
  `Show(GroupList, BTreeMap<StudentId, Student>)` / out `Accepted(GroupListFilling)`.
  Four components: `Dialog`, `StudentExclusionEntry` (one `adw::SwitchRow` per student,
  automatic mode), `GroupEntry` (per-group `adw::PreferencesGroup` with a student-count
  `SpinRow` + `StudentEntry` combo rows), `StudentEntry`. The group count is **fixed at
  Show time** (`self.group_names.len()`). Cross-group dedup lives in
  `Dialog::update_available_students` (`:387-427`): it recomputes the available set and,
  walking groups in order, nulls out any student already used by an earlier group.
  `generate_data` (`:451-480`) emits exactly `group_names.len()` groups for `Prefilled`.
- Parent `group_lists.rs` (382 lines): the params dialog serves BOTH Add
  (`GroupListParamsSelectionReason::New`, default params with a heuristic count
  `student_count / students_per_group_min` at `:224-228`) and Edit
  (`::Edit(id)`); the prefill dialog only opens on an existing list. Handler outputs
  (`:277-299`): New → `AddNewGroupList(params)`, Edit → `UpdateGroupList(id, params)`,
  prefill Accept → `SetFilling(id, filling)`.
- Row buttons (`group_lists_display.rs`): pencil `edit-symbolic` "Modifier les
  paramètres" → `EditClicked`, `view-list-bullet-symbolic` "Préremplir la liste" →
  `PrefillClicked`, `edit-delete-symbolic` "Supprimer la liste".
- Dispatch: panel output is forwarded at `editor.rs:836-840` into
  `EditorInput::UpdateOp(UpdateOp::GroupLists(op))`; the handler dry-applies, shows the
  warning dialog if any warnings, then commits. **One op per Accept.**
- Paned precedent: `editor/colloscope/config_dialog.rs:276-283` (`gtk::Paned`
  horizontal, `set_position: 510`, inside a `gtk::Frame`; that window is 1024×576, the
  largest in the app).

### 2.3 The python side

`python/src/glue/group_lists.rs` (252 lines): pyclasses `GroupListId` (registered
implicitly through fields), `GroupList { parameters, filling }` (**not** registered with
`m.add_class`, `#[new]` takes only `parameters` and forces `Automatic`), `GroupListParameters`
(min/max flattened scalars + `Vec<Option<String>>` names), `GroupListFilling` (pyclass
enum with staticmethods `prefilled(groups)` / `automatic(excluded_students)`),
`PrefilledGroup`. Conversion `state → py::GroupList` exists (`:71-78`); **`py → state`
does not** — that `TryFrom` (through `GroupList::new`, mapping `GroupListBuildError` to
`PyValueError`) is the missing piece.

`python/src/glue.rs` pymethods: `group_lists_add(params)` `:1109`,
`group_lists_update(id, new_params)` `:1125`, `group_lists_delete(id)` `:1150`,
`group_lists_set_filling(id, filling)` `:1168`, plus association methods. All follow the
same shape: build `UpdateOp`, `self_.file.apply_update(op)`, exhaustive match of the
family error into `PyValueError`, `e => panic!("Unexpected result: {:?}", e)`.

### 2.4 The scripts (compatibility contract = exactly three)

- `scripts/import_pronote_web_2026_05_06.py` — zero group-list usage. Unaffected.
- `scripts/examples/custom_export_xlsx.py` — read-only (`params.group_lists[id].parameters`,
  `.group_names`, associations). Unaffected as long as the read shape is stable (it is).
- `extra-scripts/import.py` — the only mutator, and the exact pattern being replaced
  (`update_group_lists`, `:439-450`):

```python
# extra-scripts/import.py:447-450 (OLD)
        params = group_list_current_params[group_list_id].parameters
        params.group_names = group_names
        f.group_lists_update(group_list_id, params)
        f.group_lists_set_filling(group_list_id, collomatique.GroupListFilling.prefilled(prefilled_groups))
```

The ordering of those two calls is load-bearing today (update must resize first or
`SetFilling` panics on arity). It also uses `group_lists_add(new_group_list_params)` at
`:128-133` and `:420-421`.

## 3. Settled design decisions

**D-A — `AddNewGroupList` also becomes global.** The merged dialog serves both Add and
Edit (the old params dialog did, and it is retired at the end of commit 2). If Add kept
its params-only payload, a user configuring the prefill pane while *creating* a list
would have their right-pane edits silently dropped — and emitting a second op is
impossible (one op per Accept, and the panel never learns the new id). So commit 1
changes `AddNewGroupList(GroupListParameters)` → `AddNewGroupList(GroupList)`. Because a
caller-supplied filling can now carry student ids, Add gains an error enum
`AddNewGroupListError { InvalidStudentId }` and the same student-existence sweep as the
update (the state layer validates student existence on `GroupListOp::Add`; without the
precheck the translator's `.expect` would panic). Its cleaning arm stays `None` (a brand
new list has no colloscope rows and no associations). The Python `group_lists_add`
keeps accepting params only and grows an *optional* `filling=None` argument (see
commit 3), so `import.py`'s two `group_lists_add(params)` calls run unchanged.

**D-B — Naming: transitional `ReplaceGroupList`, final rename to `UpdateGroupList` in
commit 5.** During commits 1–4 the old `UpdateGroupList(id, params)` still exists, so
the new op needs a transitional name; `ReplaceGroupList(id, GroupList)` says what it
does. Commit 5 deletes the old variants and renames `ReplaceGroupList` →
`UpdateGroupList` (and `ReplaceGroupListError` → `UpdateGroupListError`), following the
established "rename at the very end so the lasting API carries no migration scars"
doctrine (step-5 R3 precedent). This also keeps `docs/plans/plan_step_7.md`'s vocabulary
(`UpdateGroupList` composite) valid. Same scheme on the Python side:
`group_lists_replace(id, group_list)` in commit 3, renamed to
`group_lists_update(id, group_list)` in commit 5 (matching the sibling convention
`slots_update` / `incompats_update`, which take the whole new value). The rename churns
`import.py` by one line in commit 5; the alternative (a permanently distinct name) would
leave the final API asymmetric, which the codebase consistently refuses.

**D-C — The new op's cleaning arm cleans what hangs off the list, and says nothing about
the payload.** Since the payload carries both halves, every condition that used to read
"new params vs old filling" or "new filling vs old list" now reads "payload vs current
data". Check order (first match returns, mirroring the old arms' order — old
`UpdateGroupList` checks 1–2, then old `SetFilling` checks 3–4):

1. Payload non-prefilled: colloscope placements in groups `>= payload count` → remove
   one student (`LooseStudentGroupInColloscope`).
2. Out-of-range interrogation groups for associated subjects vs `payload count` → trim
   one cell (`LooseGroupsInInterrogationsInColloscope`).
3. Payload `Automatic` and old list non-prefilled: placements of newly-excluded students
   → remove one (`LooseStudentGroupInColloscope`).
4. Old list non-prefilled, payload prefilled: empty the placement row one student at a
   time (`LooseStudentGroupInColloscope`).

**The old shrink pre-empt is deliberately NOT carried over.** The old `UpdateGroupList`
arm had a third check: shrinking a prefilled list emitted a cleaning `SetFilling`
emptying the dropped groups, with `LooseStudentsInPrefilledGroupList`. That check only
existed because a parameters-only op had to guess what became of a filling its caller
could not touch. Unification removes the guess: the payload *is* the caller's complete,
already-validated description of the list, so a group they deleted and a student they
took out of a group are their own edits, not collateral damage this layer discovered for
them. Warning about them tells the user what they just said. Cleaning stays for the data
that hangs off the list — colloscope placements and interrogation cells — which the
caller never saw.

Beyond being redundant, a warning here would be the wrong *mechanism*: a warning in this
layer is inseparable from a cleaning op, so the only way to report the loss is to feed a
`ReplaceGroupList` back into the cascade with a hand-built payload — the sole place in
the family where an op cleans itself. That is a cascade-termination hazard for no gain.

This supersedes the ★ D12 reading that `LooseStudentsInPrefilledGroupList` must survive:
D12 protects the user from *silent* loss, and under one payload the loss is not silent,
it is authored. Consequence for commit 5: once the old `UpdateGroupList` arm is deleted,
nothing emits `LooseStudentsInPrefilledGroupList` any more, so the warning variant and
its `build_desc_from_data` arm are deleted with it.

**D-D — The translator is precheck-style, like the rest of the module.** Existence check
on the id (`ReplaceGroupListError::InvalidGroupListId`), student sweep over the payload
filling (both `Prefilled` groups and `Automatic` excluded set →
`ReplaceGroupListError::InvalidStudentId`), then
`GroupListOp::Update(id, payload.clone())` with the module's usual
`.expect("All data should be valid at this point")`. No reshaping, no rebuild, no arity
assert — the payload is sealed. (The pairings translate-style is noted as an
alternative, but step 7 remasters error translation anyway; parity with the module wins.)

**D-E — Keep-on-shrink in the dialog = the grow-only backing-vector pattern.** Exactly
what `params_dialog` already does for names: the model's `group_data: Vec<GroupEntryData>`
only ever grows; the factory displays the first `count` entries; `generate_data` reads
`.take(count)`. Reducing then raising the count restores the hidden groups' students.
Cross-group dedup runs over the **visible prefix only**, in group order (earlier groups
win) — so when the count grows and a hidden group re-appears holding a student that was
meanwhile placed in a visible group, the existing walk-and-null-duplicates pass in
`update_available_students` resolves the conflict in favour of the lower-index group.

## 4. Commit 1 — `ops/`: the global op (+ the Add payload change)

All in `ops/src/group_lists.rs` unless noted. The workspace must build green at the end
of the commit, so the two external `AddNewGroupList` call sites get mechanical fixes.

1. **New variant + Add payload change** in `GroupListsUpdateOp`:

```rust
// NEW enum shape (transitional; UpdateGroupList/SetFilling still present)
pub enum GroupListsUpdateOp {
    AddNewGroupList(collomatique_state_colloscopes::group_lists::GroupList),   // was: GroupListParameters
    ReplaceGroupList(                                                          // NEW
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupList,
    ),
    UpdateGroupList(GroupListId, GroupListParameters),   // unchanged, dies in commit 5
    DeleteGroupList(GroupListId),
    SetFilling(GroupListId, GroupListFilling),           // unchanged, dies in commit 5
    AssignGroupListToSubject(PeriodId, SubjectId, Option<GroupListId>),
    DuplicatePreviousPeriod(PeriodId),
}
```

2. **New error enums**, folded into `GroupListsUpdateError` with the usual
   `#[error(transparent)] … #[from]` arms:

```rust
#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplaceGroupListError {
    #[error("Group list id ({0:?}) is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewGroupListError {
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}
```

3. **Translator arms** in `apply_no_cleaning`. `AddNewGroupList` loses its
   `GroupList::new(params, default).expect(..)` build (the payload is already sealed)
   and gains the student sweep; return shape (`Ok(Some(new_id))`) unchanged.
   `ReplaceGroupList` (contrast with the OLD `UpdateGroupList` arm quoted in §2.1 — no
   reshaping, no old-value read beyond the existence check):

```rust
Self::ReplaceGroupList(group_list_id, group_list) => {
    if !data.get_data().get_inner_data().params.group_lists.group_list_map
        .contains(group_list_id)
    {
        return Err(ReplaceGroupListError::InvalidGroupListId(*group_list_id).into());
    }
    // Student-existence sweep over the payload filling (both kinds), exactly
    // the old SetFilling sweep, reporting ReplaceGroupListError::InvalidStudentId.
    for student_id in group_list.filling().iter_students() { /* contains check */ }

    let result = data
        .apply(
            collomatique_state_colloscopes::Op::GroupList(
                collomatique_state_colloscopes::GroupListOp::Update(
                    *group_list_id,
                    group_list.clone(),
                ),
            ),
            self.get_desc(),
        )
        .expect("All data should be valid at this point");
    assert!(result.is_none());
    Ok(None)
}
```

   (Check whether `GroupListFilling::iter_students()` — it exists,
   `state-colloscopes/src/group_lists.rs` helper — covers both variants; if it only
   yields prefilled students, sweep the two variants explicitly like the old
   `SetFilling` arm at `:986-1023` does.)

4. **Cleaning arm** for `ReplaceGroupList` per §3 D-C (four checks, first match
   returns; the old shrink pre-empt is dropped, not ported). `AddNewGroupList`'s arm
   stays `=> None`. Every cleaning op the arm emits targets the colloscope, so no
   `GroupListsUpdateOp` ever appears as its own cleaning vehicle.

5. **`get_desc` arm**: `ReplaceGroupList` → `"Modifier une liste de groupes".into()`
   (the final desc; the old params-only desc "Modifier les paramètres d'une liste de
   groupes" dies with its op in commit 5).

6. **Mechanical call-site fixes** for the Add payload change (behaviour identical):
   - `gtk4/src/editor/group_lists.rs:281`:
     `AddNewGroupList(params)` → `AddNewGroupList(GroupList::new(params,
     Default::default()).expect("automatic filling is always consistent"))`.
   - `python/src/glue.rs:1109` (`group_lists_add`): same wrap around
     `new_params.try_into()?`; the new `AddNewGroupListError::InvalidStudentId` arm is
     unreachable from this method until commit 3 (default filling has no students) but
     must be matched (map to `PyValueError` like the others).

7. **Tests** — new integration file `ops/tests/group_lists_global_update.rs`, modelled
   on `ops/tests/assignments_error_surface.rs` (helpers `desc`, `add_period`,
   `add_subject`, `add_student`, `dead_student_id`). Coverage:
   - `ReplaceGroupList` on a dead list id → `InvalidGroupListId`; with a dead student in
     a prefilled group and in an `Automatic` excluded set → `InvalidStudentId` (both
     arms).
   - Shrink of a prefilled list with students in the dropped groups → the payload lands
     verbatim and **no** warning is raised (the deliberate departure from today's
     `UpdateGroupList`-then-cleaning path, per D-C).
   - Non-prefilled → prefilled with an existing colloscope placement row → row emptied +
     `LooseStudentGroupInColloscope` warnings.
   - Payload `Automatic` excluding a placed student → placement removed + warning.
   - Count shrink with out-of-range interrogation groups on an associated subject →
     cells trimmed + `LooseGroupsInInterrogationsInColloscope`.
   - `AddNewGroupList` with a prefilled payload naming a dead student →
     `InvalidStudentId`; with a valid prefilled payload → list created with that filling
     (pins the new Add capability).

## 5. Commit 2 — gtk4: the merged dialog, old dialogs retired

New module `gtk4/src/editor/group_lists/edit_dialog.rs` (plus an `edit_dialog/`
directory for the factory components if the file gets unwieldy — never `mod.rs`, per
project style). It merges the two existing components; most factory code
(`GroupNameEntry`, `StudentExclusionEntry`, `GroupEntry`, `StudentEntry`) moves over
nearly verbatim.

**Window shell** (following the house dialog idiom and the `config_dialog.rs` Paned
precedent):

```rust
adw::Window {
    set_modal: true,
    set_resizable: true,
    #[watch] set_visible: !model.hidden,
    set_title: Some("Configuration de la liste de groupes"),
    set_default_size: (1000, 700),                       // twice as wide; taller pane wins
    adw::ToolbarView {
        add_top_bar = &adw::HeaderBar { /* Annuler / Valider (suggested-action) */ },
        #[wrap(Some)]
        set_content = &gtk::Paned {
            set_orientation: gtk::Orientation::Horizontal,
            set_position: 500,
            #[wrap(Some)] set_start_child = &gtk::ScrolledWindow { /* params pane */ },
            #[wrap(Some)] set_end_child   = &gtk::ScrolledWindow { /* prefill pane */ },
        },
    },
}
```

Each pane keeps its own `ScrolledWindow` (`(Never, Automatic)`) so the two sides scroll
independently. Left pane = the four params blocks from `params_dialog` unchanged. Right
pane = the mode `ComboRow`, the exclusion `PreferencesGroup`, and the prefilled-groups
box from `prefill_dialog`. The bold "Liste concernée : {name}" footer is dropped — the
name field is now visible in the left pane of the same window.

**Model** = union of the two old models (params fields + prefill fields + the shared
`filtered_students`/`available_students`). Messages = union of the two input enums; one
output:

```rust
pub enum DialogInput {
    Show(collomatique_state_colloscopes::group_lists::GroupList,
         BTreeMap<StudentId, students::Student>),
    Cancel,
    Accept,
    UpdateSelectedName(String),
    UpdateStudentsPerGroupMinimum(u32),
    UpdateStudentsPerGroupMaximum(u32),
    UpdateMaxGroupCount(u32),
    UpdateGroupName(usize, String),
    UpdatePrefillMode(PrefillMode),
    UpdateStudentExclusion(usize, bool),
    UpdateGroup(usize, GroupEntryData),
}
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::group_lists::GroupList),
}
```

**The dynamic group count** — the heart of the merge:

- `UpdateMaxGroupCount(n)` now refreshes BOTH sides: the group-name factory (existing
  grow-only `group_name_data` mechanics) AND the prefill group factory. `group_data`
  becomes grow-only exactly like `group_name_data`: growing the count appends fresh
  `GroupEntryData` entries; shrinking only reduces how many entries the factory shows
  (`update_vec_deque` pops the surplus tail widgets; the backing `group_data` entries
  beyond `n` are untouched). Raising the count again re-displays the stored entries —
  the user's requirement.
- `UpdateGroupName(i, name)` also refreshes the prefill factory so the group titles
  ("Groupe {n} : {name}") track the left pane live.
- The dedup pass (`update_available_students`) and `generate_data` iterate the **visible
  prefix** `self.group_data[..count]` only. Hidden entries keep whatever they hold; on
  re-grow they re-enter the dedup walk and lose any student that a lower-index group
  acquired in the meantime (earlier group wins — same rule as today's walk order).

**Accept** builds the sealed value:

```rust
DialogInput::Accept => {
    self.hidden = true;
    let params = self.generate_params();     // ex params_dialog::generate_data
    let filling = self.generate_filling();   // ex prefill_dialog::generate_data, over the visible prefix
    let group_list = collomatique_state_colloscopes::group_lists::GroupList::new(params, filling)
        .expect("dialog maintains group count and student uniqueness by construction");
    sender.output(DialogOutput::Accepted(group_list)).unwrap();
}
```

(`Prefilled` is generated with exactly `count` groups, and the dedup pass guarantees no
duplicated student, so both `GroupListBuildError` cases are unreachable.)

**Parent rewiring** (`group_lists.rs`):

- One controller replaces two; `GroupListParamsSelectionReason` → `GroupListSelectionReason
  { New, Edit(GroupListId) }`; the `prefill_group_list_id` field dies.
- `AddGroupList` builds a default `GroupList` (`GroupList::new(default_params_with_
  heuristic_count, Default::default()).expect(..)` — the heuristic count computation at
  `:224-228` is kept) and shows the dialog with the full student map.
- `EditGroupList(id)` clones the whole stored `GroupList` (not just params) and shows it.
- `PrefillGroupList` input, its handler, and `GroupListsInput::GroupListPrefillSelected`
  are deleted. On `Accepted(group_list)`: New → `GroupListsUpdateOp::AddNewGroupList
  (group_list)`, Edit(id) → `GroupListsUpdateOp::ReplaceGroupList(id, group_list)`.
- `group_lists_display.rs`: the `view-list-bullet-symbolic` "Préremplir la liste" button,
  `EntryInput::PrefillClicked` and `EntryOutput::PrefillGroupList` are deleted; the
  pencil tooltip becomes "Modifier la liste".
- **Delete `params_dialog.rs` and `prefill_dialog.rs`** and their `mod` declarations at
  the end of the commit.

## 6. Commit 3 — python/: expose the global update

All in `python/src/glue/group_lists.rs` + `python/src/glue.rs`.

1. Register the class: `m.add_class::<group_lists::GroupList>()?` next to the other
   group-list registrations (`glue.rs:49-51`).
2. `GroupList::#[new]` grows an optional filling (backwards-compatible — nothing
   constructs it today since it was unregistered):

```rust
#[pymethods]
impl GroupList {
    #[new]
    #[pyo3(signature = (parameters, filling=None))]
    fn new(parameters: GroupListParameters, filling: Option<GroupListFilling>) -> Self {
        GroupList {
            parameters,
            filling: filling.unwrap_or(GroupListFilling::Automatic {
                excluded_students: BTreeSet::new(),
            }),
        }
    }
}
```

3. The missing fallible conversion (mirrors the read-side `From` at `:71-78`):

```rust
impl TryFrom<GroupList> for collomatique_state_colloscopes::group_lists::GroupList {
    type Error = PyErr;
    fn try_from(value: GroupList) -> PyResult<Self> {
        collomatique_state_colloscopes::group_lists::GroupList::new(
            value.parameters.try_into()?,
            value.filling.into(),
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))
        // GroupListBuildError's Display already says "prefilled group count (…) does
        // not match the group name count (…)" / "student … appears in two prefilled
        // groups" — the honest replacement for today's process abort.
    }
}
```

4. New pymethod on `CollomatiqueFile` (transitional name, final name in commit 5),
   following the house shape (`slots_update` / `incompats_update` precedent):

```rust
fn group_lists_replace(
    self_: PyRef<'_, Self>,
    id: group_lists::GroupListId,
    group_list: group_lists::GroupList,
) -> PyResult<()> {
    let result = self_.file.apply_update(collomatique_ops::UpdateOp::GroupLists(
        collomatique_ops::GroupListsUpdateOp::ReplaceGroupList(
            id.into(),
            group_list.try_into()?,
        ),
    ));
    match result {
        Ok(_) => Ok(()),
        Err(UpdateError::GroupLists(GroupListsUpdateError::ReplaceGroupList(e))) => match e {
            ReplaceGroupListError::InvalidGroupListId(id) =>
                Err(PyValueError::new_err(format!("Invalid group list id {:?}", id))),
            ReplaceGroupListError::InvalidStudentId(id) =>
                Err(PyValueError::new_err(format!("Invalid student id {:?}", id))),
        },
        e => panic!("Unexpected result: {:?}", e),
    }
}
```

5. `group_lists_add` gains the optional filling (scripts calling it with params only are
   untouched): `#[pyo3(signature = (params, filling=None))]`, body builds
   `GroupList::new(params.try_into()?, filling.map(Into::into).unwrap_or_default())`
   mapped through `PyValueError`, and matches the new
   `AddNewGroupListError::InvalidStudentId`.

## 7. Commit 4 — migrate the script

`extra-scripts/import.py:439-450` — the two-call dance becomes one honest call:

```python
# NEW
    for (group_list_id, groups) in group_lists.items():
        prefilled_groups = []
        group_names = []
        for (group_name, students) in groups.items():
            new_group = collomatique.PrefilledGroup()
            new_group.students = set(students)
            prefilled_groups.append(new_group)
            group_names.append(group_name)
        params = group_list_current_params[group_list_id].parameters
        params.group_names = group_names
        new_group_list = collomatique.GroupList(
            params, collomatique.GroupListFilling.prefilled(prefilled_groups)
        )
        f.group_lists_replace(group_list_id, new_group_list)
```

No other script changes (`import_pronote_…` has no group-list usage;
`custom_export_xlsx.py` is read-only). The user runs `import.py` as the acceptance test
for this commit (its CSV inputs are private and out-of-repo).

## 8. Commit 5 — remove the split ops, rename to the final vocabulary

1. `ops/src/group_lists.rs`: delete the `UpdateGroupList(id, params)` and
   `SetFilling(id, filling)` variants, their translator arms, their cleaning arms, their
   `get_desc` arms, `UpdateGroupListError` (the old one) and `SetFillingError`, and
   their `GroupListsUpdateError` arms. The old `UpdateGroupList` cleaning arm is the last
   emitter of `GroupListsUpdateWarning::LooseStudentsInPrefilledGroupList` (D-C dropped
   the pre-empt from the global op), so that warning variant and its
   `build_desc_from_data` arm go too — check with a grep before deleting.
2. Reroute the three internal `SetFilling` cleaning emitters onto the global op, keeping
   each warning unchanged — all three keep the params, so the pattern is uniform:

```rust
// e.g. ops/src/students.rs:270 (OLD)
op: UpdateOp::GroupLists(GroupListsUpdateOp::SetFilling(
    *group_list_id,
    collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups: new_groups },
)),
// NEW
op: UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(
    *group_list_id,
    collomatique_state_colloscopes::group_lists::GroupList::new(
        group_list.params().clone(),
        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups: new_groups },
    )
    .expect("same group count as the existing params"),
)),
```

   Sites: `ops/src/students.rs:270` and `:288`; `ops/src/group_lists.rs:539` and `:573`
   (both inside the `DeleteGroupList` cleaning arm). (`:461`, the old shrink pre-empt,
   dies with its arm in point 1.)
3. Rename `ReplaceGroupList` → `UpdateGroupList` and `ReplaceGroupListError` →
   `UpdateGroupListError` (compiler-driven; touches the gtk4 emit site and the python
   glue arm).
4. `python/src/glue.rs`: delete the old `group_lists_update` and
   `group_lists_set_filling` pymethods, prune the now-unused error imports at
   `glue.rs:10-25`, rename `group_lists_replace` → `group_lists_update`.
5. `extra-scripts/import.py`: the one-line call rename
   `group_lists_replace` → `group_lists_update`.

Per the migration-commit workflow, this destructive commit lands separately from the
additive ones, with the usual user testing pause before it (the user runs tests
regularly; the plan does not gate on confirming it).

## 10. Verification

- **Per commit**: `cargo build --workspace --tests` then the full suite via
  `cargo test --workspace` launched in the background from the start, output captured
  once to a scratchpad file and grepped (never run the suite twice; never foreground).
- **Commit 1**: the new `ops/tests/group_lists_global_update.rs` passes; the
  behaviour-parity fixtures (shrink warning, transition cleanups) are the meaningful
  gate.
- **Commit 2**: user-run gtk4 smoke — open the merged dialog on the Hogwarts example
  (`examples/hogwarts.collomatique` has both a prefilled 8-group class list and a
  5-group Divination list): edit params and prefill together, shrink the count and grow
  it back (data must reappear), shrink-and-accept on a prefilled list (no warning dialog
  — the loss was authored in the dialog; a warning dialog appearing here is the D-C
  regression), create a new list with a prefill configured in the same dialog.
- **Commit 3**: `cargo build -p collomatique-python` (or workspace) green; the glue has
  no unit tests — script-level acceptance covers it.
- **Commits 4–5**: user runs the three contract scripts (`extra-scripts/import.py` is
  the real exercise; the other two prove non-regression).
- **Storage untouched**: no format change anywhere in this work (the sealed `GroupList`
  serde and the spec-2 codec are not touched); `populated_round_trip` byte-stability
  stays green as part of the workspace suite.
