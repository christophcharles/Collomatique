# New Python API — design

This document is the design of the replacement Python API, worked out in discussion
(August 2026). It supersedes the direction of the current `colloscopes/python/` crate
(previously `python/`), which was a
hack job built for testing purposes. The old API remains untouched during the
transition (see §13) and is retired at the end.

The requirements come from `docs/todos/todo_python_api.md`, since retired now that
they are met and pinned at `git show 5a0a3c10:docs/todos/todo_python_api.md`:

- **Mirror the database-like structure** of the data.
- **Clear, predictable value vs. reference behaviour.**
- **Completeness**: everything a user can do in the GUI must be doable from Python —
  including launching the solver.

---

## 1. Architecture: a library first

`collomatique` becomes a real importable Python module. The script API does not
depend on a running GUI. There are three contexts, one API:

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

- **GUI-hosted**: the GUI's "run script" feature keeps today's boundary — hand a copy
  of the state to a worker subprocess, let the script work on it, review the result,
  commit as **one undo slot**. The script sees the same `Document` object, obtained
  differently:

  ```python
  doc = clm.current_document()
  ```

  `current_document()` returns `None` when running standalone. `load()` works in
  **both** contexts: the hosted document is not the only document a script may open.
  Importing last year's students from another file is an ordinary thing to write.
  What makes the hosted document special is only *where its changes go*, and that is
  readable on the document itself:

  ```python
  doc.is_hosted        # bool
  doc.source_path      # Path, or None
  ```

  Getting the right document in both contexts is §9.1; writing it back is §9.2.

- **From the command line**: the collomatique binary runs a script itself, with no
  window and no GUI initialization at all.

  ```
  collomatique --python-file import.py
  collomatique --python 'import collomatique as clm; print(clm.load("2026.collomatique"))'
  ```

  This is the standalone context, reached without packaging the wheel: nothing is
  hosted, so `current_document()` is `None` and a script works on the files it names
  itself. What it adds is that the interpreter is running *inside* a collomatique
  binary, which is therefore an engine a solve can re-execute — the second rung of
  §10.3. `--python-no-engine` withholds it, which is what lets a script, or a test,
  reach the rungs below.

The embedded interpreter stops being the API's foundation and becomes *a runner*.

**Packaging.** Done. The Rust crate builds both as an rlib (linked into the
collomatique binary, module registered via `append_to_inittab` — the hosted path
needed **no** packaging or nix changes) and as a cdylib for a wheel, which maturin
builds from `colloscopes/python/pyproject.toml` and which a Python environment must have on
`sys.path` for standalone use. On the nix side `pkgs/nix/collomatique-python.nix` is
the module as a member of an interpreter's package set, and `pkgs/nix/python-env.nix`
an interpreter with it already in; the flake exposes both. The wheel is the `.so` and
nothing beside it — the value dataclasses travel inside it (§12) — and it is built
against a particular collomatique, which it remembers as the last engine rung of
§10.3.

**No UI framework.** The five RPC dialog primitives of the old API are not part of the
new one: nothing in the API talks to the GUI over RPC to draw something. Scripts that
want prompts use `tkinter` (stdlib) directly, so the script-running environment needs
tkinter available (nix side). The one exception is file selection, which every script
needs and which the module provides itself through `rfd` — a deliberate design change,
argued in §9.3.

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
module, converted at the Rust boundary). It mirrors the **entity**, flattened the
way the matching handle shows it: `SubjectData`, `TeacherData`, `LimitsData`,
`GroupListData`, … It has no `.id`, is fully mutable, and is connected to nothing.

The entity, not the op payload — the two differ in exactly two places, and the
value is the larger one both times: `SubjectData` carries the excluded periods no
subject op takes, and `SlotData` names its subject though no slot op really
carries it. The snapshot of §8 is why: a tree built from these classes must hold
a whole document, and dropping either field would lose it information. The ops
mirror handles the mismatch loudly — an `add` or `update` that cannot carry a
field raises, naming the op that moves it, rather than discarding it silently.

Values are chosen to be Python dataclasses rather than Rust pyclasses deliberately:
pyo3 getters clone nested structs, which would reintroduce the temporary trap
*inside* values (`teacher_value.person.email = ...` mutating a clone). As plain
Python objects, values get real nested mutation, `==`, `repr`, defaults and type
hints for free. The flat immutable leaf values (`Weekday`, `TimeSlot`, `Limit`,
the periodicity family, the two group-list fillings) stay frozen Rust pyclasses —
the clone trap only bites what nests — and a `*Data` field holds them, so the two
kinds meet without defining anything twice. The Rust boundary extracts a
dataclass by attribute access and validates on the way in — through a `Value`
trait rather than a pyo3 `FromPyObject` impl, because a field that names an
entity is resolved against the document, and `extract_bound` has nowhere to put
one. The dataclass definitions must be kept in sync with the Rust structs;
round-trip tests pin the correspondence, and the Python-side defaults are pinned
against the model's own.

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
handle or an id interchangeably, and so does a value field that names an entity:
`subjects={maths}` and `subjects={maths.id}` extract to the same payload.
`to_data()` always *produces* ids, because a value holding handles would carry
its document around and keep it alive. (The wart, stated plainly: a dataclass
stores what it is given, so those two spellings compare unequal — a handle and
an id hash differently. The extraction also refuses, loudly, a mapping that
names one entity twice through the two spellings.)

### Ids — fully opaque

Ids support `==`, hashing, ordering, and a readable `repr` for logging. Nothing
else: no `int()`, no constructors, no serialization. Ids are meaningless outside the
run that produced them — `WeekId` is renumbered on every file load, and compaction
(§9.5) renumbers every id kind. Scripts that span runs re-find entities by content
(name, matching), which is more robust anyway.

## 3. Naming and conventions

The public API does not follow internal Rust naming — those names are historical.
Consistency *of the public API* is the rule:

- Handle classes: the entity name — `Subject`, `Teacher`, `Student`, `Period`,
  `Week`, `WeekPattern`, `Slot`, `Incompat`, `GroupList`, `PairingRule`,
  `SlotPairingRule`.
- Value classes: uniformly `*Data` — `SubjectData`, `TeacherData`, `StudentData`,
  `WeekData`, `WeekPatternData`, `SlotData`, `IncompatData`, `GroupListData`,
  `PairingRuleData`, `SlotPairingRuleData`, `LimitsData`, `BalancingData` (the
  `BalancingOptions` sub-view's value — the one place `to_data()`'s naming is
  not mechanical, and this list settles it),
  `ExportConfigData` (and its sub-configs), `ColloscopeData`, `DocumentData`.
- Copy-out: `handle.to_data()` returns the matching `*Data`.
- Collection methods: `add(data)`, `update(id_or_handle, data)`,
  `remove(id_or_handle)` for entities; `set_*` for cells, toggles and singleton
  configs (`set_period_status`, `set_interrogation`, `set_global_limits`);
  `move_up`/`move_down` where user-visible order exists. Every mutator answers
  an `OpResult`; an `add` answers an `AddResult`, the subclass that also
  carries the created handle on `.created` (§5).
- Absent optional text is `None`, never `""`. The boundary rejects empty strings
  where the model requires non-empty (`tel`, `email`, week annotations, group
  names) instead of silently conflating them. A *boundary* rule, not a dataclass
  rule: a dataclass is dumb and stores what it is given, and the refusal happens
  on extraction — the last moment the failure can still name the field.
- Enumerations (`Weekday`, periodicity kinds, …) are Python `enum`/dataclass unions
  with working `==` (the old API's `Weekday == Weekday.Monday` identity-comparison
  trap is gone).
- **Rendered text is French.** Everything the module writes for a human to
  read is a French sentence or word: the warning texts, `str()` on a caveat
  (the same sentence the GUI's caveat dialog shows), the `group_name`
  fallback « Groupe N », and the words inside `repr`s — a dead handle prints
  `(périmé)`, a slot names its day « Jeudi », a pairing side is
  `(antécédent)`. A `repr` exists to be read in a log, and the log's reader
  speaks the application's language. Identifiers are the exception and stay
  English: class and attribute names (`Subject`, `weekday`, `MONDAY`), and
  repr shapes that only echo them (`<Subject #3>`, `count=4`, `index=0`).
  Exception messages are English — they are for the script author, not for
  the end user.

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
  `doc.assignments[pid, sid]` returns a (possibly empty) frozenset of `Student`
  handles, never a `KeyError` for a valid address. The canonical-absent
  property of the model is preserved automatically on write because everything
  goes through ops.
- **Reads name entities with handles, uniformly.** A read that names an entity
  hands back a handle, never an id: the students of a row, the association of a
  `(period, subject)` pair (`doc.group_lists.association_for(period, subject)`
  → a `GroupList` handle or `None`), the members of every set. A handle is
  strictly more useful — the id is one attribute away — and membership tests
  still work, since handles hash and compare by `(document, id)`.
- **Two lookup conventions.** A mapping position answers in python's mapping
  vocabulary: `collection[x]` raises `KeyError` when `x` names nothing,
  `collection.get(x)` returns `None`, `x in collection` returns `False` —
  asking a lookup is legitimate. Everywhere else — an id-or-handle *argument*
  to a method (`doc.is_week_active(week)`, `doc.assignments[p, s]`'s address,
  `doc.colloscope.interrogation(slot, week)`) — a dead reference raises
  `StaleHandleError`; the model's own forgiving answers are not mirrored,
  because the question was malformed before it had an answer. (One wrinkle:
  `doc.assignments[p, s]` is spelled as an indexing, but its address follows
  the *argument* convention — its reads are total, so `KeyError` could never
  mean "no row".)
- The colloscope reads mirror its two sparse tables:
  `doc.colloscope.interrogation(slot, week)` → frozenset of group indices or
  `None`; iteration over existing cells; `doc.colloscope.group_list(gl)` → mapping
  student → group index. Group numbers are indices into the associated group
  list's `group_names` — the `(period, subject) → group_list` hop is exposed as a
  helper (`doc.group_lists.association_for(period, subject)`). Naming a group is
  `gl.group_name(i)`, which always returns a string: the stored name, or the
  GUI's own fallback « Groupe n » (1-based) for an unnamed group; the raw names,
  `None` included, are on `.group_names`.
- Derived predicates the model already provides are exposed:
  `doc.is_week_active(week, pattern)`, `doc.is_interrogation_possible(slot, week)`,
  `doc.settings.limits_for(student)`, `doc.balancing.options_for(subject)` (the
  whole-entry override semantics stay in Rust). Their entity arguments follow
  the argument convention above: a dead `week`, `pattern` or `slot` raises
  `StaleHandleError` rather than echoing the model's forgiving `false`.
- Reverse lookups ride the existing reference registry:
  `handle.referenced_by()` returns the sites that point at the entity
  (`InnerData::references_to_*`).

## 5. The write surface

The write API is the `colloscopes/ops/` layer (previously `ops/`), grouped by collection — the collection object
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
| Slots (5) | `doc.slots.add(SlotData)`, `.update(slot, SlotData)`, `.remove(slot)`, `.move_up/.move_down` — the value names its subject (§2), which is fixed at creation |
| Incompatibilities (3) | `doc.incompats.add/update/remove` |
| Pairings (3) | `doc.pairings.add/update/remove` |
| SlotPairings (3) | `doc.slot_pairings.add/update/remove` |
| GroupLists (5) | `doc.group_lists.add/update/remove`, `.set_association(p, subj, gl_or_None)`, `.duplicate_previous_period(p)` |
| Settings (3) | `doc.settings.set_global_limits(LimitsData)`, `.set_student_limits(student, LimitsData)`, `.remove_student_limits(student)` |
| Balancing (3) | `doc.balancing.set_global(BalancingData)`, `.set_subject(subj, BalancingData)`, `.remove_subject(subj)` |
| Colloscope (4 + 1 new) | `doc.colloscope.set_group_list(gl, {student: group})`, `.set_interrogation(slot, week, groups)`, `.erase()`, `.erase_group_lists()`, `.install(ColloscopeData)` (`InstallColloscope`, §11.1) |
| ExportConfig (11) | `doc.export_config.set_global(...)`, `.set_colloscope_enabled(bool)` / `_config(...)`, `.set_all_groups_enabled/_config`, `.set_prefilled_groups_enabled/_config`, `.set_automatic_groups_enabled/_config`, `.set_per_group_list_enabled/_config` |

> **Added since**, on the `greedy_group_lists` branch (`e99f861b`, `42fd5a74`):
> `group_lists.remove_all()` and `group_lists.clear_associations(period)`,
> mirroring the two new ops `DeleteAllGroupLists` and `ClearPeriodAssociations`.
> The GroupLists row counts neither, and neither does the total above it.
>
> And `group_lists.add_generated(entries)`, mirroring
> `AddGeneratedGroupLists` — the landing door of the generation §10 describes,
> fed by `doc.generate_group_lists` and `doc.default_generation_request`. It
> mints a list per entry and reports no id back, so it answers a plain
> `OpResult` rather than the `AddResult` of `add`.
>
> **Added since**, on the `anonymity` branch: a sixteenth family, Anonymize (1),
> with one door — `doc.anonymize_names(seed=None)`, mirroring `AnonymizeNames`.
> It sits on `Document` rather than on a collection, since it renames the
> students and the teachers at once. Left out, the seed comes from python's
> `random`.

No elementary `Op` is exposed. The cascade architecture makes raw elementary access
unsafe to hand out (`force_apply` fixes nothing by design), and ops + the coarse
door (§8) already cover everything.

### Warnings

Every mutator returns an `OpResult`; an `add` returns an `AddResult`, the
subclass that also carries the created entity's **handle**. Different answers
are different types, structured the same: a write that creates nothing has no
id field holding `None`, and `isinstance(r, OpResult)` holds for both, so code
that only reads warnings treats them alike. A handle and not an id, for §4's
own reason — it is strictly more useful, and the id is one attribute away.

```python
r = doc.teachers.remove(tid)     # OpResult
r.warnings      # list[Warning] — the cascade repairs that were applied

r = doc.students.add(d)          # AddResult
r.created       # the new Student handle
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

Backed by a stack of `AppSession`s in `generic/state/` (previously `state/`) (`SessionStack`), so blocks really
nest: an inner block that rolls back takes only its own writes, and an outer block
that catches the exception keeps everything it did before. Writes, `undo()` and
`redo()` land in the innermost open block, and committing it folds it into the one
below as a single slot. `with doc.transaction(...) as t:` binds the transaction, and
`t.cancel()` rolls back at once and makes leaving the block do nothing — that is the
preview §5 mentions above. Outside a transaction, each op is its own undo
slot with an auto-generated label. Exposed: `doc.undo()`, `doc.redo()`,
`doc.can_undo`, `doc.can_redo`, `doc.undo_name`, `doc.redo_name`.

Why the stack is in the value and not in the type — `AppSession` nests only in the
type, so the depth would have to be known at compile time, and `dyn Manager` cannot
stand in because `ManagerInternal` is `pub(crate)` *and* requires `Clone` — is
written up in `git show 6a377893:docs/python/transactions.md`, the note this design
was chosen from. It also records the alternative that was dropped, one session plus
a counter, and the corner it could not close: an inner block rolling back would take
the outer block's earlier writes with it.

A document's undo history is its own and never leaves the script. In hosted mode the
script works on a copy in the worker process, so `doc.undo()` and a rolled-back
transaction are invisible to the GUI — only an explicit send (§9.2) crosses. On the
host side each send lands in one `AppSession` wrapping the whole run, and the user's
validation commits it as a single undo slot in the real document.

### Arguments are resolved before the borrow, not inside it

Every mutator here takes entities as arguments — `doc.subjects.update(s, data)`,
`doc.settings.remove_student_limits(student)` — and each of those goes through the
read surface's argument check (`handles::argument`, §4's argument convention),
which refuses a wrong kind, a handle of another document, and a reference this
document no longer holds. That check borrows the document to ask.

A mutator borrows it too, mutably: either by hand (`self.doc.borrow_mut(py)`) or by
being a `&mut self` method, in which case pyo3 holds the `PyRefMut` for the whole
call. `Py<Document>` is a `RefCell`, so the two borrows cannot overlap — and
`Py::borrow` has no error path for that, it *panics*. pyo3 turns the panic into a
`PanicException` at the boundary, which is exactly the worker-killing panic §6
exists to eliminate, and it would replace the clean `StaleHandleError` the argument
check was built to raise.

So a mutator resolves all of its arguments first, and only then takes the mutable
borrow. That is the order the work wants anyway: an op built from a dead id would be
refused further down, by a layer that knows nothing about handles and cannot say
which argument was wrong.

## 6. Errors

A typed exception hierarchy replaces the old mix of `PyValueError` strings and
worker-killing `panic!`s:

- `collomatique.Error` — base class.
- One subclass per `UpdateError` family (`SubjectsError`, `AssignmentsError`, …),
  carrying the structured error data. The mapping is generated structurally from
  the (serde-able) `UpdateError` type, not matched arm by arm — a new Rust error
  variant must become a new Python-visible case, never a panic.
- `UpdateError` itself, unparameterized, for the coarse door: what
  `replace_all` was refused over is the whole document and not one family's
  business, so it raises the base class, carrying the model's own sentence —
  which names every invariant the tree broke, not just the first (§8). Message
  only, and deliberately: a structured payload like the families' can be added
  if scripts turn out to want one, and an alpha scripting surface is a place to
  find that out.
- `StaleHandleError` for access through a dead handle, `ValueError`-family
  conversion errors for invalid value contents (empty strings, bad ranges, sealed
  constructor violations such as a pairing rule whose antecedent equals its
  consequent).
- Document-plumbing errors (§9): `NoDocument` (nothing to open), `Cancelled` (the
  user dismissed a dialog), `DialogUnavailable` (a dialog asked for on a machine that
  cannot show one, §9.3), `NotHosted` (a host-only call made standalone),
  `NoOrigin` (`save()` with nowhere to write), `ExportError` (an export could not be
  produced or written — a workbook that could not be built and a file that could not
  be written arrive alike, since to a script they mean the same thing, §9.4),
  `IdCeilingExceeded` (a save the file format cannot represent), `CaveatedOverwrite`
  (a bare `save()` back over a file that was loaded with caveats). Both of the last two carry an instruction rather than just
  a diagnosis: `IdCeilingExceeded` names `compacted()` as the way out (§9.5), and
  `CaveatedOverwrite` lists what was lost and names `ignore_caveats=True` (§9.2). Both
  are `SaveError`s, so a script that only cares that the write failed catches one
  thing. `DocumentChanged` is a third: the application declined the document
  `send_to_host` offered it. Only the interactive console meets it — there the user
  goes on editing while the console is open, so the document may have moved since it
  was read, and the application asks before overwriting. `NoOrigin` stays generic — a document has an origin or it has not, and nothing
  tracks how it was produced.
- `ModelBuildError` for a colloscope model the constraint builder refuses to
  build (`doc.build_colloscope_model`, §10.2), carrying the builder's own
  sentence. It is a build failure and not an export failure, which is what lets
  `model.export_mps` raise `ExportError` over the file alone.
- `NothingToUndo` for `doc.undo()` or `doc.redo()` with nothing left in that
  direction (§5). One class for both, because it is one question — the history has
  another step that way, or it has not — and a script that wants to ask rather than
  catch has `can_undo` / `can_redo`. Raising rather than doing nothing: a script that
  undoes more than it wrote is mistaken about its own document, and a quiet no-op
  hides that.

## 7. Quality floor

- `.pyi` type stubs for the whole module; `collomatique.__version__`.
- `eq`/`repr` on everything user-visible — the reprs' rendered words are French,
  per §3.
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

The surface it landed with (`4c3ba5eb`): `label` is optional and defaults to
« Mise à jour globale », the name the application's own global updates carry;
the answer is an `OpResult` whose `warnings` is always empty, because a global
update lands as given or is refused whole and so has nothing to repair; and,
being an ordinary write, it folds into an open `doc.transaction(...)` like
everything else. A refused tree changes nothing at all — the document is left
bit-identical — and raises the base `UpdateError`, whose message itemizes
*every* invariant the tree broke rather than stopping at the first, so a script
fixing its tree does not do it one round trip at a time (§6).

`snapshot()` is built — it landed with the values (§13.3), because it is a pure
read and because it is what forces the values to be entity-complete (§2). The
orders are carried by the containers themselves (dicts keep insertion order),
and the sparse sections hold the stored rows only. What `replace_all` inherits
is a question the snapshot never has to answer: a tree names its entities by id,
and ids have no constructor (§2), so a script can rename, delete and rewire a
snapshot, but cannot *add* an entity to one. The three ways out were a
document-scoped minting call on ids, a tree keying a new entry by something that
means "give it a fresh id", and `replace_all` simply being the door for
transformations that add nothing. The third is the one that landed: creating an
entity is the incremental ops' business and stays theirs. Nothing enforces that
on top of what is already there — every id in a tree is resolved against the
receiving document by the argument convention (§5), and one that names nothing
in it is refused like any other dead reference, which is exactly the rule. The
other two remain addable later; choosing this forecloses neither.

## 9. Documents, dialogs and maintenance operations

### 9.1 Getting a document

Three primitives, all usable in either context:

- `clm.new_document()` — an empty document, no origin.
- `clm.load(path)` — open a file. Surfaces the storage caveats (foreign version,
  unknown entries) on the document.
- `clm.current_document()` — the hosted document, or `None` when standalone.

The caveats land on `doc.caveats`, a `frozenset` of `Caveat` values — one class per
kind (`CreatedWithNewerVersion`, `UnknownEntry`), all under a `Caveat` base so
`isinstance` catches them without listing the kinds. They are values with `==`, a
`repr` and a `str` — the `str` being the same French sentence the GUI's caveat
dialog writes (§3) — so a script names the one it knows how to handle:

```python
if clm.UnknownEntry("colloscope", 3) in doc.caveats:
    ...
```

`doc.caveats` is empty for a clean file and for `new_document()`, and it is part of
the origin: it is fixed at load and no save changes it. Loading prints nothing and
raises no `warnings.warn` — the GUI shows a modal because a human is there, a script
has nobody, and a library writing to stderr is a nuisance in a cron job. What was
skipped is by construction something this build cannot use, so a read-only script
loses nothing by the silence; the loss happens on rewrite, which is where §9.2 puts
the guard.

Most scripts want the same resolution chain, so it gets a name of its own:

```python
doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)
```

`clm.default_document(path=None, *, dialog=True)` tries, in order: the hosted
document, then `path`, then a file-open dialog. Host first is the safe order — a
script run inside the GUI must never quietly start editing a file on disk because a
stale argument was lying around.

It takes a path, not `sys.argv`, so scripts that use `argparse` keep control of their
own command line. It *raises* rather than returning `None` when nothing is found:
cancelling the dialog raises `Cancelled`, and `dialog=False` with no other source
raises `NoDocument`. Returning `None` would make every script write an `if doc is
None` check, and forgetting it gives an obscure `AttributeError` twenty lines later.
`dialog=False` is what a cron job passes, where a dialog would hang forever.

### 9.2 Writing a document out

**The hosted document is not sent back automatically.** Today's engine does that (one
`SetData` at script exit, when the state was modified — code since removed, see below), but
only because the old API gives the script no way to say it. Once there is a call,
automatic becomes harmful: a script that raises halfway pushes its half-finished
state, and a script that sends deliberately gets a second, unwanted send at exit. The
cost is that a hosted script which edits and forgets the call does nothing — but that
failure is visible (the GUI says "Aucune modification effectuée"), whereas an implicit
send fails by pushing something the author did not mean to push.

The engine's automatic send went with `python-old/` (§13, step 6). It fired on the old
module's own shared `AppState` having been modified, which a script using this module
never touches, so it could never fire for a new-API script — §11 works this through —
and the two lived side by side, each serving its own module's scripts, until the old
crate left.

Sending is a **module-level function taking any document**, because its subject is the
host slot, not the document:

```python
clm.send_to_host(doc)          # raises NotHosted when standalone
```

Restricting it to the hosted document would buy nothing and would block ordinary
scripts: building next year's file from a template plus a CSV and dropping it into the
open GUI, or loading a backup to repair a broken document. No protocol change is
needed — `SetData` is already accepted at any moment, any number of times, and the
host applies each one onto the same `AppSession`.

Two semantics, both loud in the docstring:

- **A send replaces the host's whole state.** It is not a merge. Pushing a different
  file wipes the user's document; the GUI's validation step is the only safety net.
- **Sending twice is allowed and the last one wins.** That is what makes the
  send-a-different-document case composable.

Saving is then one method with the ordinary Save / Save As meaning —
`doc.save(path=None, *, ignore_caveats=False)`:

```python
doc.save(path)     # write that file, whatever the origin
doc.save()         # write back to the origin
```

With no argument it dispatches on the origin: hosted → `send_to_host(doc)`; loaded
from a file → that path; no origin → raises `NoOrigin`. It is never a silent no-op,
and it never opens a dialog of its own: a save that is silent on one document and puts
a chooser up on the next is a call a script cannot reason about, and the one it would
block on is the one where the whole run's work is already in memory. A script that
does want to be asked has `clm.dialogs.save_file()` and a path to hand to `save`.
Together with
`default_document()` it is the symmetric pair a script needs to work in both contexts:

```python
doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)
...
doc.save()
```

The origin is immutable: `doc.save("other.collomatique")` does not re-target a later
`doc.save()`. Silent re-targeting of the hosted document would be nasty.

**A caveated file is not overwritten behind the script's back.** When `doc.caveats`
(§9.1) is non-empty, the file held something this build could not read, and the format
spec is explicit that rewriting drops it. So the no-argument form raises
`CaveatedOverwrite` — a `SaveError` — and the ways out are named in its message:

```python
doc.save()                      # raises
doc.save("copy.collomatique")   # writes; the suspect original survives
doc.save(doc.source_path)       # writes; the script named the target
doc.save(ignore_caveats=True)   # writes; deliberate
```

The rule keys on *no path argument*, not on whether the path equals the origin. Path
equality is fragile (symlinks, relative paths, hard links) and the GUI does not test it
either: "Enregistrer" on a caveat-loaded file opens a Save-As dialog defaulting to that
same file, and a user who picks it overwrites it, because they chose. Naming the path
from python is the same choice. `save()` with no argument is the one form that writes
somewhere the script never named, so it is the one that is loud. `ignore_caveats` is
keyword-only, defaults to `False`, and does nothing when a path is given — accepted
there so a script can pass it uniformly.

`send_to_host(doc)` is not guarded: it already says it replaces the host's state
wholesale, and the GUI's validation step is the safety net. A **hosted document carries
no caveats today** — the handoff carries the `Data`, not the host's caveat set — so a
script cannot see that the GUI opened its file with caveats. Fixing that needs a
protocol change and belongs with the hosted milestone.

### 9.3 Dialogs

The module ships native dialogs, routed through `rfd`:

```python
clm.dialogs.open_file(title=..., filters=...)   # Path, or None on cancel
clm.dialogs.save_file(...)
clm.dialogs.pick_folder(...)
```

This is a **design change and a new dependency for the `colloscopes/python/` crate** — `rfd` is
today a `gtk4` dependency only. It is recorded here as a decision, not smuggled in.
Being already in the lockfile buys only a vetted version and an unsurprised nix side.

The case for it: file selection is the one dialog every script needs; `rfd` is small
next to a UI framework; on Linux it goes through the XDG portal, so the dialogs are
native and work inside a sandbox; and it needs no GTK, which is what a plain Python
interpreter wants. So §1's rule narrows from "no GUI API" to "no UI framework".

What it buys is **files and folders, and nothing else**. Message boxes looked free —
the crate has a `MessageDialog` — and they are not: under the portal backend `rfd`
draws one by spawning `zenity`, an external binary a sandboxed run has no reason to
hold, and when it cannot be spawned `rfd` logs and answers `Cancel`. A `confirm()`
would then quietly say no and a `message()` would quietly show nothing, and two calls
that lie are worse than two calls that are not there. So message boxes join text entry
and list choice as `tkinter`'s job. The file dialogs are what the dependency is for
anyway, and they are clean: they go through the portal proper (`ashpd`), and touch
`zenity` only as a fallback for a portal request that itself errored — a session with
a portal never gets there.

The downsides, stated plainly:

- A library that can open windows can hang a cron job forever. Dialogs are therefore
  always an explicit call, never something the API does on its own — which is also
  why `default_document(dialog=False)` exists.
- On a headless machine with no portal, the call must surface as an exception rather
  than block — `DialogUnavailable`. It cannot be done by reading what `rfd` gives back:
  a portal that is not there comes back as `None`, which is exactly what a user
  pressing Cancel looks like. So the question is asked *before* the dialog, off the
  session's environment — `WAYLAND_DISPLAY`, `DISPLAY`, `DBUS_SESSION_BUS_ADDRESS`, any
  one of them being enough, since a desktop that autolaunches its bus sets no address
  while the cron job this guards against has none of the three.
- Dialogs must be called from the main thread (a macOS requirement), and the Rust
  side must release the GIL while one is open, or the script's other threads freeze.
- Hosted scripts run in a separate process, so a dialog is a top-level window of its
  own: not parented to the GUI window, and it may appear behind it. Routing it
  through RPC would fix that, but would resurrect the RPC dialogs this design
  deletes. The cosmetic cost is worth paying.

### 9.4 Export

- `doc.export_xlsx(path, config=None)` — `None` uses the document's export config.
  The `ExportConfig → xlsx::Config` conversion lives in the `xlsx` crate (§11.2);
  no shared crate is needed, since `xlsx` already depends on `state-colloscopes`.
- MPS — the GUI's advanced-tools export, written through the model object of §10:
  `model = doc.build_colloscope_model(config)`, then `model.export_mps(path)`.
  It writes the full problem, objective included; `checker=True` writes the
  constraints-only checker problem the same build already carries (§10.2). The
  GUI writes the *base* model, with no config at all; the Python export writes
  the configured one, so the file matches the problem a solve would actually
  attack. Failures raise `ExportError`, as for xlsx — and only file failures
  reach it, since a model that cannot be built already failed at
  `build_colloscope_model` with `ModelBuildError` (§6). There is no
  `doc.export_mps` sugar: an export needs a config, a config produces a model,
  and one door is enough.

### 9.5 Compaction and the id ceiling

`storage::serialize_data` fails when a document holds an id above the spec-2 format's
ceiling. Dense renumbering (`InnerData::compact_ids`) is the only way out, and the
storage crate never renumbers by itself. So `doc.save(path)` can fail for this one
reason, and it raises `IdCeilingExceeded` naming the way out. The GUI asks the
question in a dialog; a script cannot be asked, so the error has to carry the
instruction.

Compaction is **functional**: it returns a new document instead of mutating one.

```python
doc = clm.load("big_ids.collomatique")
doc.compacted().save()
```

`doc.compacted()` returns a fresh `Document` with dense ids and no undo history. The
original is untouched and stays valid, which is the reason for this shape: the ids and
handles a script holds are not invalidated — they still belong to the old document —
and "clears the undo history" stops being a warning, because a new document simply has
none. The GUI mutates in place instead, but it has a confirmation dialog to carry
those warnings and a script has nothing.

**Compaction cannot travel to the host.** It would have to arrive as an
`Op::GlobalUpdate`, whose annotate arm only pushes the id issuer *forward*
(`IdIssuer::skip_to_id`, `colloscopes/state-colloscopes/src/ops.rs`, previously `state-colloscopes/src/ops.rs`). It must: an undone
`GlobalUpdate` restores the old, bigger ids, so the issuer has to stay above
everything the redo stack still holds. The host would end up with dense ids in its
current data, an issuer still at its old high-water mark — the next entity added gets
a big id again — and a history that brings the big ids back on Ctrl-Z. For exactly
this reason the GUI does not compact through an op at all: it replaces the whole
`AppState` and destroys the history (`EditorInput::CompactIds` in
`colloscopes/gtk4/src/editor.rs`, previously `gtk4/src/editor.rs`).

The API settles this with the origin rule, not with a special case:

- `compacted()` **inherits the file path**, and with it **the caveats** (§9.1). So the
  rescue script above overwrites the file it read, which is what the GUI's "compact and
  save" does — and a caveated file still refuses the bare `.save()`, instead of
  `clm.load(f).compacted().save()` becoming a laundering route around §9.2's guard.
- `compacted()` **never inherits the hosted-ness**. A compacted copy of the hosted
  document has `is_hosted == False` and `source_path == None`, so `.save()` raises
  the ordinary `NoOrigin`: this document has nowhere to write. Nothing tracks where a
  document came from beyond its origin, and the error says no more than that.
  Compacting the open document is still an ordinary script:

  ```python
  clm.current_document().compacted().save("clean.collomatique")
  ```

Inheriting the hosted-ness would make `.save()` send to the host, and that send would
look like it worked while the issuer and the history stayed untouched.

`clm.send_to_host(doc.compacted())` stays callable and does what §9.2 says a send
does: it replaces the host's state wholesale. The compaction simply does not stick.
Nothing structural distinguishes a compacted payload from an ordinary one — deleting
the last-added subject also lowers the maximum id, and sending that is entirely
normal — so there is no check to add, only this note. `send_to_host` is the explicit
"replace the host's state, I mean it" door, and the accidental path is already closed
by the origin rule.

## 10. The solver

Designed first, implemented last (§13.5). All configuration types are value
dataclasses mirroring the (serde) Rust structs, with the GUI's presets exposed:

- `ConductorStrategy` — sub-configs `DefaultConfig`, `WarmStartConfig`,
  `IncrementalConfig`, `FuzzyConfig`; classmethods `ConductorStrategy.search()`
  and `.optimize()` (the two GUI presets); `.warnings()` returns the preflight
  `ConductorWarning`s as a tuple, in the model's own order — sorted by
  declaration, which is the order the solve dialog lists them in. Their French
  sentences are `collomatique_ui_text::solver::conductor_warning_text`, the same
  function the solve dialog renders its list from, so `str(warning)` is what the
  user would have read.

  Each `*_config` field both **enables and tunes** its substrategy: `None`
  switches it off, an object switches it on. So `ConductorStrategy()` — warm
  start alone, one worker — is the application's « Recherche simple », and the
  two classmethods answer plain instances built on the Rust side, which is what
  makes drift against the application's own presets impossible.

  This is the one value family with no document behind it: a strategy names no
  entity, so it is read through a marker struct with inherent methods rather than
  through the `Value` trait of §2, whose `from_py` wants a document to resolve
  against. Its boundary checks are its own two: a time limit is a whole number of
  seconds and at least one, `None` being how "no limit" is said — zero is refused
  rather than read as no limit — and a sigma or a tolerance must be finite and
  not negative, zero allowed.
- Colloscope solve: `ColloscopeSolveConfig` and its two sub-configs, mirroring
  `constraints_colloscopes::SolveConfig` — settled in §10.1 below.

**Group-list generation is in**, settled once the generator was. It stayed out
while there was no shape to mirror — the feature was still an ILP with objective
weights of its own — and it is mirrored now that the greedy answers in
milliseconds and the GUI drives it directly. Three doors, and the document is
touched only by the last of them:

- `doc.default_generation_request()` hands back a `GroupListsGenerationRequest`
  — `rebuild`, a set of `(period, subject)` pairs, and `kept_lists`, the
  prefilled lists the generator must respect. It is the selection the
  application's own generate dialog opens with, built by the very function that
  dialog calls (`greedy_groups::default_generation_request`), so the two cannot
  drift. It says nothing about what will work: a pair whose students the group
  sizes cannot split is offered here exactly as the dialog offers it.
- `doc.generate_group_lists(request, *, on_log=None)` builds the lists and
  writes nothing. Synchronous — the greedy is milliseconds on a whole class, so
  there is no run to wait on and nothing to stop — and `on_log` follows
  `build_colloscope_model`'s contract, one line at a time, the first raising
  callback winning with no result handed back. What comes back is a
  `GroupListsGenerationResult`: `entries`, the lists paired with the
  coordinates each must serve, and `skipped`, the requested pairs nobody is
  registered for, which is a report and not a refusal.
- `doc.group_lists.add_generated(entries)` lands them, one op and one undo slot.
  It reads its argument structurally, so entries a script built by hand land
  through the same door.

There is **no `names` parameter**, and there cannot be one: pairs sharing a
student set and a size range share a single list, so how many names a caller
would owe is not knowable until the plan exists. The lists come out carrying the
coverage labels the application's naming dialog seeds its own rows with —
« Sortilèges (période 1) », « Sortilèges et Métamorphose (périodes 1 et 2) » —
which live in `ui-text` because two front ends now print them. Renaming is
editing `.name` on the returned values before landing them.

`GroupListsGenerationError` is what a request the plan will not build raises,
carrying the generator's own sentence: a subject that runs no interrogations, a
kept list that is not prefilled, a class the group sizes cannot split. It is not
under `UpdateError` — nothing was written and no op was refused, the request
itself is what could not be made sense of. A reference the document does not
hold is refused earlier still, by the argument convention of §2.3.

There are no objective weights to expose. The greedy maximizes one fixed
objective, and the retired ILP that had tunable ones is not coming back.

### 10.1 The colloscope solve config

`ColloscopeSolveConfig` says which periods and which group lists a solve
recomputes, and what a dropped constraint or a previous value is worth as an
objective term. It mirrors `constraints_colloscopes::SolveConfig`, which is what
the GUI's own solve dialog fills in.

```python
@dataclass
class PeriodSolveConfig:
    recompute: bool = True             # solve this period again
    use_current_values: bool = False   # start from what the document holds

@dataclass
class GroupListSolveConfig:
    recompute: bool = True
    previous_values_as_objective: bool = False   # stay close to the current list

@dataclass
class ColloscopeSolveConfig:
    periods: dict[Period | PeriodId, PeriodSolveConfig] = field(default_factory=dict)
    group_lists: dict[GroupList | GroupListId, GroupListSolveConfig] = field(default_factory=dict)
    objectify_cross_fixed_period: float | None = 1000.0
    l1_anchor_weight: float = 1000.0
```

- **Names.** This family does not take the `*Data` suffix of §3. That suffix
  means "the detached value of an entity", and these are call arguments. Rust's
  internal `PeriodSolveData` / `GroupListSolveData` are not public-API precedent
  either (§3).
- **The group-list config is flat.** Rust nests it
  (`Option<GroupListRecompute>`); Python gets the two booleans the GUI's own
  dialog row holds. The one combination the nesting cannot express —
  `recompute=False, previous_values_as_objective=True` — is refused at
  extraction, naming the list: nothing is recomputed, so there is nothing for
  the anchor to hold on to.
- **A missing entry means the default.** A period or a group list absent from
  the dicts is recomputed from scratch, so `ColloscopeSolveConfig()` is the
  "recompute everything" config — what a script that does not care wants. The
  dict keys are handles or ids like every other mapping argument (§2), with the
  same refusal of dead, foreign and twice-named keys.
- **A prefilled group list in `group_lists` is refused**, loudly, naming the
  list: it has no solve to configure. Rust's `sanitize` drops such an entry
  instead, but that runs before the GUI shows its dialog, to carry an earlier
  choice forward sensibly. A Python config is written at the moment of the call,
  so an entry like that is a mistake in the script and is said out loud.
- **The weights are validated at the boundary**: non-finite or negative is
  refused, zero is allowed. `objectify_cross_fixed_period=None` means the cross
  constraints of a fixed period are dropped rather than paid for.
- **The config is never stored on the document.** It is an argument, like the
  GUI's dialog result, and every call takes its own.

### 10.2 The model object

The config is the common gate to the remaining doors — writing the problem out,
solving it, and asking what a colloscope breaks. They all take the same road:
build the model once, then use it.

```python
model = doc.build_colloscope_model(config, on_log=...)   # config is required
model.export_mps("problem.mps")                          # §9.4
model.export_mps("checker.mps", checker=True)
run = model.solve(strategy, on_progress=..., on_log=...)
violations = model.blame(colloscope)                     # §10.4
```

- `config` is **required**. The GUI never solves without passing its dialog, and
  a `None` here would read like `export_xlsx`'s `None`, which means something
  else entirely (the document's own stored export config, §9.4). A script that
  wants everything recomputed writes `ColloscopeSolveConfig()` and says so.
- `ColloscopeModel` is **opaque**: no accessors, nothing to walk. Its `repr`
  carries the two counts the GUI's advanced-tools panel shows
  (`<ColloscopeModel: 12345 variables, 6789 constraints>`) and no names.
- It is **detached**, like a value (§2): a snapshot taken at build time. Editing
  the document afterwards neither changes it nor invalidates it — there is no
  staleness question to ask. It is not a handle.
- A model that cannot be built raises `ModelBuildError` (§6). That failure
  belongs to the build, which leaves `export_mps` with nothing to fail over but
  the file itself.
- `on_log` is keyword-only, takes one `str` per line, and `None` discards: the
  log the GUI's loading dialog shows while it builds. There is no `on_progress`
  on a build — a build has lines, not a proportion; progress is the solver's
  (below). A callback that raises does not tear the build in half: the build
  runs to its end, the callback is not called again, and the exception
  propagates once it returns, with no model produced.
- **Checker or full is an export-time choice**, not a build-time one. One built
  model already carries both problems: the real one, and the constraints-only
  checker with a trivial objective (`ilp-modeler`'s `problem()` and
  `checker_problem()`). There is no cheaper checker-only build to ask for, so
  `checker=True` is a flag on the export rather than a second build or a
  `model.checker()` object.

**The ILP problem is never introspectable.** `ColloscopeModel` is a token for a
built problem, not a view of one: no variables, no constraints, no accessors.
Exposing the problem would pin the internal variable and constraint naming of
`constraints-colloscopes` as public API, so every rename downstream would break
scripts. The only thing that crosses is the file `export_mps` writes (§9.4). The
names inside it are already a de-facto contract through the GUI's own export, but
that is a diagnostic file for a solver, not an API.

### 10.3 The run, its outcome, and the engine

Execution is subprocess-based, reusing `StrategySubprocess::spawn` — the
battle-tested path, with hard cancellation and no GIL contention. The API is a run
handle:

```python
run = model.solve(strategy, engine=..., on_progress=..., on_log=...)   # non-blocking
run.progress()          # last known progress (async by design — no waiting)
run.stop()              # cooperative stop → run finishes with best-so-far
run.kill()              # hard kill
outcome = run.wait()    # SolveOutcome: status, objective, bound, colloscope
colloscope = outcome.colloscope     # a ColloscopeData value, NOT yet applied
doc.colloscope.install(colloscope)  # lands through ops, one undo slot
```

Mid-solve incumbent accept (the GUI's "take best so far") is `run.stop()` + using
the outcome.

- **The run owns the engine process.** Dropping the handle kills it, so a script
  holds it for as long as the solve should live. Two solves on one model are fine
  and independent — each starts its own engine.
- **`wait()` answers once and keeps answering.** A second wait hands back the very
  same `SolveOutcome` object: a finished run has one outcome, not two. A run that
  was `kill()`ed has none at all and raises `SolveError`, and so does an engine that
  exits without reporting one. An exception raised by `on_progress` or `on_log`
  comes out of `wait()` in place of the outcome, every time it is asked — the rule
  `build_colloscope_model` already follows for its own log. Both callbacks run on
  another thread, and neither may call `wait()`.
- **`stop()` and `kill()` are safe to call late.** Stopping a run that has already
  finished does nothing, which is the honest answer since the two race by design;
  killing one twice, or killing one that finished, is fine, so a `finally:` that
  tidies up need not ask first. Stopping a *killed* run raises: there is no longer
  an engine to ask.
- **Nothing is written to the document**, so a solve takes no undo slot. What it
  produces is a value, and `doc.colloscope.install(...)` is what lands it.
- **`progress()` answers a `SolveProgress`**, or `None` before the run's first
  report: the best incumbent's cost and the best proven bound, either of which may
  be `None` on its own. It is the same pair `SolveOutcome` carries as `objective`
  and `bound` once the run has finished.

**One verdict, and the interface shows the same one.** `outcome.status` is a
`SolveStatus` with exactly four values — `OPTIMAL`, `FEASIBLE`, `NO_SOLUTION`,
`ERROR` — and it is deliberately *not* the status the solver reports about the
problem it was handed. `collomatique_strategies::SolveStatus` calls a run optimal
as soon as the conductor holds any incumbent, and never reports `Infeasible` or
`Error` at all, so a script given the raw word would read a promise nobody made.

The answer to « how did it go » is `collomatique_strategies::verdict`, which reads
one finished outcome whole: an incumbent with a closed optimality gap is `Optimal`,
an incumbent alone is `Feasible`, nothing in hand is `NoSolution` — whether the run
was stopped before it found anything or the problem has no answer, which the
conductor cannot tell apart either — and a run that broke down is `Error`.
`NO_SOLUTION` therefore states the fact and not the cause. `ERROR` stays a status
rather than an exception because such a run may still carry the best colloscope it
had found by then, and raising would throw it away.

That verdict lives in `strategies` rather than in either front end, and its four
French sentences live in `collomatique_ui_text::solver::solve_verdict_text` beside
the warnings'. The solve dialog and `str(outcome.status)` print the same sentence
about the same run, which is the completeness requirement at the head of this
document applied to what a user is *told* and not only to what they can do.

**Engine location.** The subprocess mechanism re-executes a collomatique binary with
`--rpc-engine`, so a solve has to name one. Four rungs, tried in order, and nothing
found is a loud `NoEngine` — a subclass of `SolveError` — rather than a guess:

1. the call's own `engine=`, being the most local thing said about this solve;
2. the engine the runner **injected**, being about this run rather than about the
   machine;
3. the `COLLOMATIQUE_ENGINE` environment variable, where an empty value counts as
   unset;
4. a default baked in at build time — `COLLOMATIQUE_DEFAULT_ENGINE`, read with
   `option_env!` — being about the build, the least local thing there is. An empty
   value counts as unset here too, and a build that named none leaves the rung
   simply absent.

The second rung is a parameter of `run_python_script`, decided by whoever starts the
interpreter rather than by the module: a hosted script and a `--python-file` script
both run inside a collomatique binary, which is therefore an engine, so both call
sites pass `EngineExe::Current` — `rpc-engine`, which is what the GUI's script
runner spawns, and the command-line branch of the gtk4 binary itself. A bare
`python` importing the wheel injects nothing, so a standalone script falls through
to what its environment or its build says. The last rung is what makes that
painless where it can be: `pkgs/nix/collomatique-python.nix` sets
`COLLOMATIQUE_DEFAULT_ENGINE` to the store path of the collomatique the wheel was
built against, which ends up inside the compiled module — so nix keeps that binary
alive, and a script run from `pkgs/nix/python-env.nix` solves without naming
anything. A wheel built anywhere else simply has no fourth rung unless its builder
sets the variable. `--python-no-engine` (§1) withholds the injection, which is how a
script — and `colloscopes/gtk4/tests/e2e/` — reaches the rungs below it.

### 10.4 The blame: what a colloscope breaks

The third door on a model answers the other question — not « what is the best
colloscope » but « what is wrong with this one », which is what the GUI shows
under « Vérification du colloscope ».

```python
for violation in model.blame(doc.colloscope.to_data()):
    print(violation.severity, "-", violation)
```

- **It takes a `ColloscopeData`** — `doc.colloscope.to_data()`, or a solve's
  `outcome.colloscope`. The model is detached (§10.2), so there is no document
  to resolve a handle against: the keys are **ids alone**, which is what
  `to_data()` hands back, and a handle is a `TypeError` saying so.
- **It blocks**, unlike `solve()`. Filling in what a colloscope does not say —
  the helper variables the constraints are really written against — takes a
  solver, so an engine runs in its own process here too; it is a quick
  feasibility problem with nothing to optimize, and there is no progress to
  report, so a run handle would be an empty ceremony. `engine=` and `on_log=`
  mean what they mean for `solve()`. Ctrl-C interrupts the wait and kills the
  engine with it.
- **A violation is a severity and a sentence**, and nothing else:
  `ConstraintViolation` carries a `SeverityLevel` and the French text
  `ConstraintDesc::user_readable` writes, which is what `str()` gives. A
  structured mirror of the model's constraint descriptions would pin the
  internal vocabulary of `constraints-colloscopes` as public API — the same rule
  that keeps the model itself opaque (§10.2).
- **`SeverityLevel` has six members where the model has five.** `FIXED <
  INFEASIBILITY < STRUCTURAL < QUALITY < PROGRESSIVE < PREFERENCE`, worst first,
  so `sorted()` and `min()` do the obvious thing. `FIXED` is not one of the
  model's tiers: it marks a broken **pin** of the solve configuration — a
  variable the config said not to recompute, which the colloscope contradicts —
  and that outranks anything the model says, being the one thing the person
  driving the solve asked for by hand. Its sentence lives in
  `collomatique_ui_text::solver::fixed_pin_violation_text`, beside the solve
  dialog's own. The Rust `SeverityLevel` keeps its five tiers.
- **The list is the *minimal* blame**, sorted worst first: a violation another
  reported one already implies is left out. Every constraint of the model is
  hard, so a `PREFERENCE` violation is a real violation — the tiers say what a
  relaxation would give up first, not what the colloscope is allowed to break.
  An empty list is a colloscope the model has nothing against.
- **Refusal order, as `solve()`'s** (§5): the value is read, then judged against
  the model's parameters, and only then is an engine looked for. A colloscope
  this model cannot read — an unknown slot or week, a group number past the
  list's last group, a student it does not place — is a `ValueError` raised
  before any machine is asked for; an engine that cannot verify it is a
  `SolveError`. Nothing is written to the document, which the model is not
  attached to anyway.

## 11. Rust-side prerequisites

Work outside the `colloscopes/python/` crate that this design requires. All four have landed;
nothing here blocks the new crate any more.

1. **Whole-colloscope install op** — done: the op in `colloscopes/ops/` (5ee7ec05), the GUI
   adopting it (dfe62270). `ColloscopeUpdateOp::InstallColloscope` writes a whole
   colloscope through the cascade: afterwards the document holds the payload's rows
   and no others. It is the solver's landing door and the scripting API's
   `colloscope.install`, so neither reaches for the forced `Op::GlobalUpdate`.

   Two decisions worth remembering. The payload is `ColloscopeContents`, a plain-map
   twin of the state's `Colloscope` — the state type is two `Table`s and carries no
   serde, while an `UpdateOp` payload must serialize — with the `From<&Colloscope>`
   callers that already hold one need; a value built by hand need not be canonical, an
   empty group set or an empty placement map just means "no row". And the op *carries*
   a whole colloscope but *lands* as a diff: clears for the dropped rows, writes for
   the added and changed ones, nothing at all for a row the document already holds.
   Every elementary op costs a document clone and a whole-model invariant scan, and
   the case the op is really for is "read a colloscope, change a handful of cells,
   install it back". `InstallColloscopeError` is its own vocabulary rather than a reuse
   of the two single-row ones, every variant carrying the ids that locate the offending
   row.

   On the GUI side this retired `EditorInput::UpdateFullColloscope` and its ad-hoc
   handler: the generic `UpdateOp` arm now does the dry-apply, the warning dialog and
   the error dialog. Two user-visible consequences — the undo entry reads « Mettre à
   jour le colloscope » rather than « Résolution du colloscope » (the op is not
   solver-specific), and its category is `OpCategory::Colloscope`, so undoing a solve
   result brings the colloscope panel forward.

2. **`to_xlsx_config` in the `xlsx` crate** — done (273f3e28). It moved out of
   `colloscopes/gtk4/src/editor/export.rs` into `colloscopes/xlsx/src/config_conversion.rs`
   (previously `xlsx/…`) as `From` impls;
   `gtk4`'s `export.rs` keeps only `export_to_xlsx` and its anyhow wrapper. No new
   dependency was needed: `xlsx` already depends on `state-colloscopes`, since
   `write_xlsx` takes an `&InnerData`. The two config types stay separate on purpose:
   `ExportConfig` keeps its `*_enabled` flags beside the values they gate, which is
   the interface's memory of what was chosen before a section was switched off, while
   `xlsx::Config` is the resolved form with `Option<T>`, so a sheet builder cannot
   read a disabled value by accident.

3. **Crate split** (§12) — done (c1a4ce18), with the rename following (fd343b5e).
   `colloscopes/python-runner/` (previously `python-runner/`) took `initialize()` and `run_python_script()` unchanged, `colloscopes/python/`
   kept `glue` and nothing else, and `rpc-engine` now depends on the runner only — it
   never used anything else from the python crate.

   The seam is the shared file state. The static stays in the module crate, because
   `glue::current_session()` reads it and the library cannot depend on the runner; the
   library gained a public setter and the runner drives it around a script run. pyo3
   moved to the workspace dependencies so the two crates cannot drift onto versions
   that would only disagree at link time. Then `colloscopes/python/` became `python-old/` (crate
   `collomatique-python-old`, module `collomatique_old`), freeing the name for the new
   crate; the three contract scripts import it under an alias, so their bodies keep
   calling `collomatique.X` unchanged.

4. **Engine spawn from a non-collomatique host** — done (a07e32b1). `Worker::spawn`
   now names its engine, `EngineExe::Current` or `EngineExe::Explicit(path)`, resolved
   inside `spawn` so existing call sites grow no error path. It is threaded through
   `SolverSubprocess::spawn`, `StrategySubprocess::spawn` and `spawn_raw`, and
   `SubprocessSolveBackend` stores one.

   The decision that keeps this small: nested workers spawned from inside an engine
   process stay `Current`, because even when that engine was launched by explicit path,
   `current_exe()` there is the very binary that was named. So the path never travels
   over RPC and `InitMsg` is unchanged. `Process::spawn_pty` also takes an `&OsStr`
   command now, which retired `WorkerSpawnError::NonUtf8ExePath` — a path coming from a
   user's environment is not the same thing as `current_exe()`, and refusing a
   non-UTF-8 one buys nothing. The `Explicit` arm's caller is the module's own engine
   resolution, which landed with the solver (§10.3, §13.5).

**No hosted-handoff prerequisite.** An earlier draft listed a fifth item: make the
`RunPythonScript` path of `rpc-engine` stop sending `SetData` at script exit, and have
the runner expose the send to the module instead. It is not needed. That send is
conditioned on the old module's shared `AppState` having been modified (`state.can_undo()`
in the engine's `RunPythonScript` path — code since removed), and that
`Arc<Mutex<AppState>>` is `python-old`'s own
structure, handed to `run_python_script` as its file state. The new crate has its own
document and never touches it, so the automatic send could not fire for a new-API
script; it went with `python-old/`, together with the `AppState` the `RunPythonScript`
glue built to feed it (§13.6). The explicit `send_to_host` / `doc.save()` of §9.2
therefore needed nothing removed first.

## 12. Crate layout

- `python-old/` — the old `colloscopes/python/` crate, renamed; Python module name
  `collomatique_old`. Frozen for the transition, then **removed** once the three
  contract scripts had been ported (§13, step 6).
- `colloscopes/python/` — the new API crate, module name `collomatique`. Builds as rlib (for
  embedding) and cdylib (for the wheel, whose maturin manifest is
  `colloscopes/python/pyproject.toml`). Ships the value-dataclass `.py` source, baked in with
  `include_str!` and materialized by the crate's own `data::register`
  (`PyModule::from_code`) during module init — which is why neither a hosted script
  nor a wheel needs a filesystem package. Takes a new `rfd` dependency for §9.3.
  It is also the one crate that does not inherit the workspace version: maturin
  reads its `[package] version` and wants PEP 440, which allows nothing numeric
  after a prerelease and so refuses `0.1.0-alpha.1.99`. It therefore writes that
  version out truncated, `0.1.0-alpha.1`, which means an alpha bump touches two
  files — the pre-commit hook truncates the workspace version itself and refuses a
  commit where the two disagree. Nothing user-facing is affected: `__version__` and
  every document header still come from `collomatique_settings::current_version()`.
- `colloscopes/python-runner/` — the executor: interpreter lifecycle, inittab registration (of
  *both* modules during the transition, of `collomatique` alone since), the document
  handoff to hosted scripts. `rpc-engine` depends on this crate only.

## 13. Migration

1. Crate split and rename: `colloscopes/python/` → `python-old/` (`collomatique_old`), new
   empty `colloscopes/python/`, `colloscopes/python-runner/`. The three contract scripts
   (`extra-scripts/import.py`, `scripts/import_pronote_web_2026_05_06.py`,
   `scripts/examples/custom_export_xlsx.py`) get their one-line import change in
   the same change; the user runs them as the acceptance test (the §7 contract of
   the state-consolidation record).
2. Read surface: document, handles, collections, ids — **done**, in thirteen
   commits, the last being the reference registry (`04888a59`). The design it
   was built from, collection by collection, is in
   `git show 04888a59:docs/python/handle_api.md`; the refinements it recorded
   over this document's §2 and §4 are folded in above.
3. Write surface: ops mirror, `OpResult` warnings, transactions, undo. Value
   dataclasses land here — **done**: transactions and undo came first (§5's
   stack), then the value dataclasses in eleven commits plus two review
   follow-ups, the last being the double-naming refusal (`3ae29f8d`). The design
   the values were built from, class by class, is in
   `git show 3ae29f8d:docs/python/values.md`; the refinements it recorded over
   this document's §2, §3 and §5 are folded in above, and `doc.snapshot()` came
   with it (§8). The ops mirror followed in eighteen commits — the `OpResult`
   true-up, the typed errors of §6, the structured warnings, then the fifteen
   families leaves inward, the last being the period and week mutators
   (`c8133fa9`). The split it was built from is
   `git show c8133fa9:docs/python/ops_migration.md`; of the two solver landing
   doors it gates out, `colloscope.install` lands with step 5 below and
   `group_lists.add_generated` stays out of the API for as long as group-list
   generation itself is unsettled (§10).
4. Coarse door (`replace_all` — `snapshot` landed with the values, §8), then the
   document plumbing of §9 — **done**. The plumbing came
   early rather than last: `load`/`save` with the caveat guard, the `Origin`
   rule and `compacted()` landed right after the crate split
   (`12f9d959`…`20e4a7ca`), the hosted handoff in `8fd457f8`, the dialogs in
   `6bc64975` and `f5ddc152`, and `default_document` in `8138f50d`. The rest
   waited for the ops mirror: `replace_all` landed with the §8 decision it was
   holding open (`4c3ba5eb`), and `doc.export_xlsx` with the `ExportError` of §6
   (`b9dcd6a7`). The MPS export came last of all, since it waited for the
   `ColloscopeSolveConfig` of §10 rather than fronting it: the config value
   itself (`a0330d84`, §10.1), then the build door
   `doc.build_colloscope_model` with its opaque `ColloscopeModel` and
   `ModelBuildError` (`c17507ed`, §10.2), and `model.export_mps` on top of the
   two of them (§9.4). The build door is shared with step 5, which hangs
   `model.solve` on the same object.
5. Solver (last), including the engine-location mechanism — **done**, in eleven
   commits, the last being the end-to-end tests (`8f3ff6f4`). The design it was
   built from, door by door, is
   `git show 5d19b15a:docs/python/solver.md`; the refinements it settled over
   this document's §1 and §10 are folded in above.

   It added `model.solve` to the `ColloscopeModel` of step 4 — the config and the
   build were already there — plus the run handle and the `SolveOutcome` of §10.3,
   and with them the landing door the ops mirror had gated out,
   `colloscope.install`, whose payload only exists once a solve has produced one.
   Group-list generation stayed out, as §10 says.

   The order was: the conductor warnings' French sentences into a new `ui-text`
   crate, shared with the solve dialog (`d6d80d5c`); `colloscope.install`
   (`b32621b8`); the strategy value family with its two presets (`38666fd8`) and
   its preflight `warnings()` (`d880ae70`); the engine resolution with its
   `SolveError`/`NoEngine` vocabulary (`717680df`); the parameters a built model
   has to keep for a solution to be read back out of it (`8fd7b9d3`); then
   `model.solve` itself, with the run handle and the outcome (`6ea8a6d5`).

   Three things the plan had not foreseen. The gtk4 binary grew `--python`,
   `--python-file` and `--python-no-engine` (`efb576ab`, §1): without them nothing
   could run a new-API script outside the GUI, and the engine rungs could not be
   told apart. The verdict a finished solve earns moved out of the two front ends
   into `strategies`, and its sentences into `ui-text` (`1f4d9aae`, `a8678c4a`,
   §10.3) — the solve dialog and the module had each been computing one, and the
   module's was three states too wide. And the tests became a new end-to-end
   target, `colloscopes/gtk4/tests/e2e.rs` (`8f3ff6f4`), which spawns the built binary once
   per test: which engine a solve re-executes is a property of the *process* a
   script runs in, and one interpreter cannot be in three of them at once.
6. Migrate the three contract scripts to the new API (user-validated) — they gain an
   explicit `doc.save()`. The three ports are **done**: the old-API versions moved
   to `scripts/old_api/` (`e304580b`), and the new ones landed beside them in
   `scripts/` — the Pronote web import (`aee006b1`), the custom xlsx export
   (`0479324e`) and the full draft import (`321acd1c`). The user ran all three on
   real documents: the web import and the full import write byte-identical files
   on the old and the new API, and the xlsx export was checked visually. The
   step is closed: `python-old/` and its registration are gone — the crate, its
   workspace entry, its inittab line and the `file_state` argument of
   `run_python_script` — and with them the old-API copies under `scripts/old_api/`
   and the engine's automatic send-back (§11).

Standalone packaging (wheel + nix environment) depended on no step and landed after
this one: the baked engine rung (`a7a15da5`), the cdylib and the maturin manifest
(`22b1f79f`), and the nix packaging (`1a0c4bc6`). The user built both nix entry
points and imported the module out of the resulting interpreter; a solve through the
baked rung has not been run.

## 14. Examples

Import-style (write-heavy):

```python
import sys
import collomatique as clm

doc = clm.default_document(sys.argv[1] if len(sys.argv) > 1 else None)

with doc.transaction("Import CSV"):
    maths = doc.subjects.add(clm.SubjectData(
        "Maths",
        interrogation=clm.InterrogationData(
            students_per_group=(2, 3),
            duration=60,
            periodicity=clm.EveryNWeeks(2),
        ),
    )).created
    t = doc.teachers.add(clm.TeacherData("Emmy", "Noether",
                                         subjects={maths.id})).created
    for row in rows:
        s = doc.students.add(clm.StudentData(row.firstname, row.surname,
                                             email=row.email or None)).created
        doc.assignments.set(period, maths, s, True)

doc.save()      # back to the origin: the host, or the file it came from
```

Building a document and pushing it into the open GUI:

```python
doc = clm.load("template.collomatique")
with doc.transaction("Rentrée 2027"):
    ...
clm.send_to_host(doc)      # replaces the GUI's document wholesale
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
