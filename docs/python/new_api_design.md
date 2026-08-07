# New Python API — design

This document is the design of the replacement Python API, worked out in discussion
(August 2026). It supersedes the direction of the current `python/` crate, which was a
hack job built for testing purposes. The old API remains untouched during the
transition (see §13) and is retired at the end.

The requirements come from `docs/todos/todo_python_api.md`:

- **Mirror the database-like structure** of the data.
- **Clear, predictable value vs. reference behaviour.**
- **Completeness**: everything a user can do in the GUI must be doable from Python —
  including launching the solver.

---

## 1. Architecture: a library first

`collomatique` becomes a real importable Python module. The script API does not
depend on a running GUI. There are two contexts, one API:

- **Standalone**: a plain Python interpreter imports the module.

  ```python
  import collomatique as clm

  doc = clm.load("2026.collomatique")
  # ... read, edit ...
  doc.save("2026.collomatique")
  ```

  `clm.new_document()` creates an empty document. This context enables headless
  batch jobs, cron-style automation, and interactive exploration in a Python REPL
  or a Jupyter notebook.

- **GUI-hosted**: the GUI's "run script" feature keeps today's boundary — clone the
  state, hand it to a worker subprocess, review the result, commit as **one undo
  slot**. The script sees the same `Document` object, obtained differently:

  ```python
  doc = clm.current_document()
  ```

  `current_document()` returns `None` when running standalone; `load()` raises when
  running hosted (the hosted document is the only one). A script that wants to work
  in both contexts does:

  ```python
  doc = clm.current_document() or clm.load("2026.collomatique")
  ```

The embedded interpreter stops being the API's foundation and becomes *a runner*.

**Packaging.** The Rust crate builds both as an rlib (linked into the collomatique
binary, module registered via `append_to_inittab` — the hosted path needs **no**
packaging or nix changes) and as a cdylib for a wheel (maturin), which a Python
environment must have on `sys.path` for standalone use. The nix wiring for the
standalone environment (a `python3.withPackages` with the built module, available in
the dev shell) is separate work, and only gates standalone use — the whole API can be
implemented and used hosted-first.

**No GUI API.** The five RPC dialog primitives of the old API are not part of the new
one. Scripts that want prompts use `tkinter` (stdlib) directly. This removes the last
GUI coupling from the API; the script-running environment just needs tkinter
available (nix side).

## 2. The object model: document, handles, values

The old API's central defect is the *middle ground*: mutable deep copies that look
live, so `params.subjects[0].parameters.name = "X"` runs fine and silently does
nothing. The new model has exactly three kinds of objects, and no object is ever two
of them at once.

### The `Document`

The single mutable object. It wraps an `AppState` over
`collomatique_state_colloscopes::Data`, so undo/redo and transactions come from the
existing machinery. Everything reachable from it is either a handle or a value.

### Handles — live, read-only, frozen

A handle is a view bound to `(document, id)`. It holds no data of its own.

```python
subj = doc.subjects[sid]     # Subject handle
subj.name                    # reads the CURRENT state
subj.name = "X"              # AttributeError — handles have no setters
```

- Handle classes are frozen Rust pyclasses. Attribute *assignment raises*; the old
  API's silent lost write becomes a loud error.
- A handle attribute never returns anything mutable: only scalars, ids, other
  handles, and immutable containers (tuples, frozensets). This kills the
  chained-temporary trap (`subj.parameters.name = ...`) structurally.
- A handle to a deleted entity raises on access (`StaleHandleError`). Stale is loud,
  never silent.
- Handles are hashable and comparable by `(document, id)`.
- Navigation mints handles on demand: `slot.subject`, `slot.teacher`,
  `slot.week_pattern`, `week.period`, `subject.slots`, `period.weeks`, … Since no
  object graph is ever materialized, the cycle question (the id graph is a DAG;
  parent→child conveniences would create object cycles) dissolves — there is no
  graph, only lazily-created views.

### Values — detached, mutable, dumb

A value is a plain **Python dataclass** (defined in a `.py` file shipped with the
module, converted at the Rust boundary). It mirrors an op payload: `SubjectData`,
`TeacherData`, `LimitsData`, `GroupListData`, … It has no `.id`, is fully mutable,
and is connected to nothing.

Values are chosen to be Python dataclasses rather than Rust pyclasses deliberately:
pyo3 getters clone nested structs, which would reintroduce the temporary trap
*inside* values (`teacher_value.person.email = ...` mutating a clone). As plain
Python objects, values get real nested mutation, `==`, `repr`, defaults and type
hints for free. The Rust boundary extracts them by attribute access (pyo3
`FromPyObject`) and validates on the way in. The dataclass definitions must be kept
in sync with the Rust payload structs; round-trip tests pin the correspondence.

You get a value in exactly two ways:

```python
d = clm.SubjectData("Maths")           # build from scratch
d = doc.subjects[sid].to_data()        # explicit copy out of the document
d.name = "Mathématiques"               # real mutation of a detached builder
doc.subjects.update(sid, d)            # the only way anything reaches the document
```

`to_data()` is a *method* with a conversion name, never an attribute — an attribute
returning a copy is how the old API laid its trap.

**Rule of thumb: handles have `.id`, values don't.** Every write method accepts a
handle or an id interchangeably.

### Ids — fully opaque

Ids support `==`, hashing, ordering, and a readable `repr` for logging. Nothing
else: no `int()`, no constructors, no serialization. Ids are meaningless outside the
run that produced them — `WeekId` is renumbered on every file load, and
`compact_ids` renumbers every id kind. Scripts that span runs re-find entities by
content (name, matching), which is more robust anyway.

## 3. Naming and conventions

The public API does not follow internal Rust naming — those names are historical.
Consistency *of the public API* is the rule:

- Handle classes: the entity name — `Subject`, `Teacher`, `Student`, `Period`,
  `Week`, `WeekPattern`, `Slot`, `Incompat`, `GroupList`, `PairingRule`,
  `SlotPairingRule`.
- Value classes: uniformly `*Data` — `SubjectData`, `TeacherData`, `StudentData`,
  `WeekPatternData`, `SlotData`, `IncompatData`, `GroupListData`,
  `PairingRuleData`, `SlotPairingRuleData`, `LimitsData`, `BalancingData`,
  `ExportConfigData` (and its sub-configs), `ColloscopeData`, `DocumentData`.
- Copy-out: `handle.to_data()` returns the matching `*Data`.
- Collection methods: `add(data) -> handle`, `update(id_or_handle, data)`,
  `remove(id_or_handle)` for entities; `set_*` for cells, toggles and singleton
  configs (`set_period_status`, `set_interrogation`, `set_global_limits`);
  `move_up`/`move_down` where user-visible order exists.
- Absent optional text is `None`, never `""`. The boundary rejects empty strings
  where the model requires non-empty (`tel`, `email`, week annotations, group
  names) instead of silently conflating them.
- Enumerations (`Weekday`, periodicity kinds, …) are Python `enum`/dataclass unions
  with working `==` (the old API's `Weekday == Weekday.Monday` identity-comparison
  trap is gone).

## 4. The read surface

The document exposes the data model's tables as named collections, mirroring
`InnerData` section by section:

| Collection | Backing | Notes |
|---|---|---|
| `doc.periods` | `params.periods` | user order; also `first_week` |
| `doc.weeks` | `params.weeks` | via period walk; `week.index` = global index |
| `doc.subjects` | `params.subjects` | user order |
| `doc.teachers` | `params.teachers` | id order |
| `doc.students` | `params.students` | id order |
| `doc.assignments` | `params.assignments` | keyed `(period, subject)` |
| `doc.week_patterns` | `params.week_patterns` | id order |
| `doc.slots` | `params.slots` | per-subject user order |
| `doc.incompats` | `params.incompats` | id order |
| `doc.group_lists` | `params.group_lists` | + `associations` keyed `(period, subject)` |
| `doc.pairings` | `params.pairings` | id order |
| `doc.slot_pairings` | `params.slot_pairings` | id order |
| `doc.settings` | `params.settings` | global + per-student overrides |
| `doc.balancing` | `params.balancing` | global + per-subject overrides |
| `doc.colloscope` | `colloscope` | sparse cells, see below |
| `doc.export_config` | `export_config` | config values |

Conventions:

- Ordered collections (`subjects`, `periods`, per-subject `slots`, per-period
  `weeks`) iterate in **user order**; keyed collections iterate in id order.
  Indexing is by id/handle: `doc.subjects[sid]`; `.get(id)` returns `None` instead
  of raising.
- **Sparse reads are total.** A missing junction row reads as empty:
  `doc.assignments[pid, sid]` returns a (possibly empty) frozenset of `StudentId`,
  never a `KeyError` for a valid address. The canonical-absent property of the
  model is preserved automatically on write because everything goes through ops.
- The colloscope reads mirror its two sparse tables:
  `doc.colloscope.interrogation(slot, week)` → frozenset of group indices or
  `None`; iteration over existing cells; `doc.colloscope.group_list(gl)` → mapping
  student → group index. Group numbers are indices into the associated group
  list's `group_names` — the `(period, subject) → group_list` hop is exposed as a
  helper (`doc.group_lists.association_for(period, subject)`).
- Derived predicates the model already provides are exposed:
  `doc.is_week_active(week, pattern)`, `doc.is_interrogation_possible(slot, week)`,
  `doc.settings.limits_for(student)`, `doc.balancing.options_for(subject)` (the
  whole-entry override semantics stay in Rust).
- Reverse lookups ride the existing reference registry:
  `handle.referenced_by()` returns the sites that point at the entity
  (`InnerData::references_to_*`).

## 5. The write surface

The write API is the `ops/` layer, grouped by collection — the collection object
*is* the module grouping. One method call = one `UpdateOp` = one undo slot. This is
the completeness guarantee: the mapping below covers all 68 composite ops, including
the five families the old API never bound (pairings, slot pairings, balancing,
colloscope, export config).

| `UpdateOp` family (ops) | Python surface |
|---|---|
| GeneralPlanning (9) | `doc.periods.add(week_count)`, `.set_week_count(p, n)`, `.remove_with_weeks(p)`, `.cut(p, remaining)`, `.merge_with_previous(p)`, `.set_first_week(date)`, `.clear_first_week()`; `doc.weeks.set_status(week, active)`, `.set_annotation(week, text_or_None)` |
| Subjects (6) | `doc.subjects.add(SubjectData)`, `.update(s, SubjectData)`, `.remove(s)`, `.move_up(s)`, `.move_down(s)`, `.set_period_status(s, p, active)` |
| Teachers (3) | `doc.teachers.add/update/remove` |
| Students (3) | `doc.students.add/update/remove` |
| Assignments (3) | `doc.assignments.set(p, subj, student, assigned)`, `.set_all(p, subj, assigned)`, `.duplicate_previous_period(p)` |
| WeekPatterns (3) | `doc.week_patterns.add/update/remove` |
| Slots (5) | `doc.slots.add(subject, SlotData)`, `.update(slot, SlotData)`, `.remove(slot)`, `.move_up/.move_down` — the subject is fixed at creation |
| Incompatibilities (3) | `doc.incompats.add/update/remove` |
| Pairings (3) | `doc.pairings.add/update/remove` |
| SlotPairings (3) | `doc.slot_pairings.add/update/remove` |
| GroupLists (6) | `doc.group_lists.add/update/remove`, `.set_association(p, subj, gl_or_None)`, `.duplicate_previous_period(p)`, `.add_generated(entries)` (solver landing) |
| Settings (3) | `doc.settings.set_global_limits(LimitsData)`, `.set_student_limits(student, LimitsData)`, `.remove_student_limits(student)` |
| Balancing (3) | `doc.balancing.set_global(BalancingData)`, `.set_subject(subj, BalancingData)`, `.remove_subject(subj)` |
| Colloscope (4 + 1 new) | `doc.colloscope.set_group_list(gl, {student: group})`, `.set_interrogation(slot, week, groups)`, `.erase()`, `.erase_group_lists()`, `.install(ColloscopeData)` (**new Rust op**, §11) |
| ExportConfig (11) | `doc.export_config.set_global(...)`, `.set_colloscope_enabled(bool)` / `_config(...)`, `.set_all_groups_enabled/_config`, `.set_prefilled_groups_enabled/_config`, `.set_automatic_groups_enabled/_config`, `.set_per_group_list_enabled/_config` |

No elementary `Op` is exposed. The cascade architecture makes raw elementary access
unsafe to hand out (`force_apply` fixes nothing by design), and ops + the coarse
door (§8) already cover everything.

### Warnings

Every mutator returns an `OpResult`:

```python
r = doc.teachers.remove(tid)
r.new_id        # for add-type ops, else None
r.warnings      # list[Warning] — the cascade repairs that were applied
```

A `Warning` carries the structured `Fix` information and the rendered French text
(rendered against the pre-state, like the GUI's confirmation dialog). The old API
silently dropped these; the new one always returns them. There is no separate
dry-run flag: a script that wants a preview uses a transaction and cancels.

### Transactions and undo

```python
with doc.transaction("Import Pronote"):
    ...                    # any number of ops → ONE undo slot
                           # exception → everything rolled back
```

Backed by `AppSession` (nestable). Outside a transaction, each op is its own undo
slot with an auto-generated label. Exposed: `doc.undo()`, `doc.redo()`,
`doc.can_undo`, `doc.can_redo`, `doc.undo_name`, `doc.redo_name`. In hosted mode the
host additionally wraps the whole run in one session, exactly like today.

## 6. Errors

A typed exception hierarchy replaces the old mix of `PyValueError` strings and
worker-killing `panic!`s:

- `collomatique.Error` — base class.
- One subclass per `UpdateError` family (`SubjectsError`, `AssignmentsError`, …),
  carrying the structured error data. The mapping is generated structurally from
  the (serde-able) `UpdateError` type, not matched arm by arm — a new Rust error
  variant must become a new Python-visible case, never a panic.
- `StaleHandleError` for access through a dead handle, `ValueError`-family
  conversion errors for invalid value contents (empty strings, bad ranges, sealed
  constructor violations such as a pairing rule whose antecedent equals its
  consequent).

## 7. Quality floor

- `.pyi` type stubs for the whole module; `collomatique.__version__`.
- `eq`/`repr` on everything user-visible.
- Round-trip tests pinning dataclass ↔ Rust payload correspondence (this also
  retires the old API's read-back corruption bug, where
  `SubjectInterrogationParameters` filled `groups_per_interrogation` from
  `students_per_group`).
- The three legacy contract scripts remain the acceptance oracle during migration
  (§13).

## 8. The coarse door: snapshot / replace

Alongside incremental ops, one blessed low-level interface for wholesale
transforms — the same mechanism the script boundary already uses (`GlobalUpdate`):

```python
tree = doc.snapshot()          # DocumentData — a detached value tree of the whole
                               # document (params sections, colloscope, export config)
...                            # arbitrary functional transformation
doc.replace_all(tree, "Rebuilt from scratch")   # one GlobalUpdate, one undo slot
```

`replace_all` validates at the existing trust boundary (`Data::from_inner_data`);
an invalid tree raises with the invariant diagnostics. `DocumentData` is built from
the same `*Data` dataclasses, so the two interfaces share one vocabulary.

## 9. File, export and maintenance operations

Everything the GUI can do outside the op pipeline:

- `clm.load(path)` / `doc.save(path)` / `clm.new_document()`. Load surfaces the
  storage caveats (foreign version, unknown entries) on the document.
- `doc.export_xlsx(path, config=None)` — `None` uses the document's export config.
  Requires moving the `ExportConfig → xlsx::Config` conversion out of `gtk4`
  (§11).
- MPS export of the built ILP problem (diagnostics) — lives with the solver API.
- `doc.compact_ids()` — renumbers every id densely. **This invalidates every id and
  handle the script holds** (they raise on next use) and clears the undo history,
  matching the GUI's behaviour. The docstring says so loudly.

## 10. The solver

Designed now, implemented as the last milestone. All configuration types are value
dataclasses mirroring the (serde) Rust structs, with the GUI's presets exposed:

- `ConductorStrategy` — sub-configs `DefaultConfig`, `WarmStartConfig`,
  `IncrementalConfig`, `FuzzyConfig`; classmethods `ConductorStrategy.search()`
  and `.optimize()` (the two GUI presets); `.warnings()` returns the preflight
  `ConductorWarning`s.
- Colloscope solve: `ColloscopeSolveConfig` (per-period recompute flags, per-group-
  list recompute, anchor weights) mirroring `constraints_colloscopes::SolveConfig`.
- Group-list generation: `GenerationRequest` (rebuild set, kept lists, canonical
  range) and `ObjectiveWeights`.

Execution is subprocess-based, reusing `StrategySubprocess::spawn` — the
battle-tested path, with hard cancellation and no GIL contention. The API is a run
handle:

```python
run = doc.solve_colloscope(config, strategy,
                           on_progress=..., on_log=...)   # non-blocking
run.progress()          # last known progress (async by design — no waiting)
run.stop()              # cooperative stop → run finishes with best-so-far
run.kill()              # hard kill
outcome = run.wait()    # SolveOutcome: status, objective, bound, result
colloscope = outcome.colloscope     # a ColloscopeData value, NOT yet applied
doc.colloscope.install(colloscope)  # lands through ops, one undo slot
```

Group lists analogously: `doc.generate_group_lists(request, weights, strategy, ...)`
→ outcome → `doc.group_lists.add_generated(outcome.entries)`. Mid-solve incumbent
accept (the GUI's "take best so far") is `run.stop()` + using the outcome.

**Engine location.** The subprocess mechanism re-executes a collomatique binary with
`--rpc-engine`. Standalone, the interpreter is `python`, so the engine binary is
located by: explicit `engine=` parameter > `COLLOMATIQUE_ENGINE` environment
variable > error. Hosted, the host injects its own executable path; scripts never
think about it.

## 11. Rust-side prerequisites

Work outside the `python/` crate that this design requires:

1. **Whole-colloscope install op** — a `ColloscopeUpdateOp` variant (or family
   addition) for "install this colloscope", replacing the raw `Op::GlobalUpdate`
   the GUI currently uses for solve results. The GUI should adopt it too.
2. **Move `to_xlsx_config` out of `gtk4`** (currently
   `gtk4/src/editor/export.rs`) into a shared crate so `export_xlsx` is callable
   from the library.
3. **Crate split** (§12): extract the executor from the module glue.
4. **Engine spawn from a non-collomatique host** — `Worker::spawn` must accept an
   explicit executable path instead of always `current_exe()`.

## 12. Crate layout

- `python-old/` — the current `python/` crate, renamed; Python module name
  `collomatique_old`. Frozen, untouched otherwise.
- `python/` — the new API crate, module name `collomatique`. Builds as rlib (for
  embedding) and cdylib (for the wheel). Ships the value-dataclass `.py` source; in
  embedded mode the runner materializes it (`PyModule::from_code`) so hosted
  scripts need no filesystem package.
- `python-runner/` — the executor: interpreter lifecycle, inittab registration of
  *both* modules during the transition, the document handoff to hosted scripts.
  `rpc-engine` depends on this crate only.

## 13. Migration

1. Crate split and rename: `python/` → `python-old/` (`collomatique_old`), new
   empty `python/`, `python-runner/`. The three contract scripts
   (`extra-scripts/import.py`, `scripts/import_pronote_web_2026_05_06.py`,
   `scripts/examples/custom_export_xlsx.py`) get their one-line import change in
   the same change; the user runs them as the acceptance test (the §7 contract of
   the state-consolidation record).
2. Read surface: document, handles, collections, ids.
3. Write surface: ops mirror, `OpResult` warnings, transactions, undo. Value
   dataclasses land here.
4. Coarse door (`snapshot`/`replace_all`) and file/export/maintenance ops (with
   the Rust prerequisites of §11 as they are needed).
5. Solver (last), including the engine-location mechanism.
6. Migrate the three contract scripts to the new API (user-validated), retire
   `python-old/` and its registration.

Standalone packaging (wheel + nix environment) can land any time after step 3; no
step depends on it.

## 14. Examples

Import-style (write-heavy):

```python
import collomatique as clm

doc = clm.current_document() or clm.load("2026.collomatique")

with doc.transaction("Import CSV"):
    maths = doc.subjects.add(clm.SubjectData(
        "Maths",
        interrogation=clm.InterrogationData(
            students_per_group=(2, 3),
            duration=60,
            periodicity=clm.EveryNWeeks(2),
        ),
    ))
    t = doc.teachers.add(clm.TeacherData("Emmy", "Noether",
                                         subjects={maths.id}))
    for row in rows:
        s = doc.students.add(clm.StudentData(row.firstname, row.surname,
                                             email=row.email or None))
        doc.assignments.set(period, maths, s, True)

doc.save("2026.collomatique")
```

Export-style (read-heavy):

```python
for subject in doc.subjects:                      # user order
    for slot in subject.slots:                    # per-subject user order
        for week in doc.weeks:
            groups = doc.colloscope.interrogation(slot, week)
            if groups is None:
                continue
            gl = doc.group_lists.association_for(week.period, subject)
            names = [gl.group_name(g) for g in sorted(groups)]
            print(subject.name, slot.teacher.surname, week.index, names)
```

Read-modify-write:

```python
d = doc.subjects[sid].to_data()      # explicit detached copy
d.name = "Mathématiques"
r = doc.subjects.update(sid, d)
for w in r.warnings:
    print(w.text)
```
