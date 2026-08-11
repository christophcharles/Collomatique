# Value dataclasses — design and implementation plan

This document is the complete design of the value layer — the first half of step 3
of the migration plan in `docs/python/new_api_design.md` §13: "Write surface: ops
mirror, `OpResult` warnings, transactions, undo. Value dataclasses land here." It
sharpens the design document's §2 ("Values — detached, mutable, dumb") down to
every class, field and default, and ends with the implementation plan, split into
commits.

It stands to the value layer as `git show 04888a59:docs/python/handle_api.md`
stands to the read surface. Where this document names a field, that is the whole
value surface: a field not named here does not exist. Completeness is checked
twice: against the entities of `state-colloscopes/src/`, since a value is the
entity (§2.0) and `doc.snapshot()` must be able to hold a whole document; and
against the payloads of the 68 composite ops in `ops/src/`, since every payload
must be buildable from a value. The two checks disagree in exactly two places,
which is what §2.0 is about.

**Why the values come before the ops mirror.** They can be built and pinned on
their own: `handle.to_data()` exercises the outbound half, `Value::from_py`
exercises the inbound half, and a round-trip test compares the result
against the model struct read straight out of `InnerData`. Nothing about that
needs a write to exist. The ops mirror then adds no vocabulary at all — it only
spends what this milestone builds, which is what makes it a short milestone
rather than a second large one.

**The coarse door is half in scope.** §8 of the design gives two calls,
`doc.snapshot()` and `doc.replace_all(tree, label)`, and they are not the same
kind of thing. **`snapshot()` is in this milestone** (§3.12): it is a read, it
writes nothing, it needs no op — and it is the reason the values are shaped the
way §2.0 shapes them, so building it here is what proves the rule rather than
merely asserting it. **`replace_all` stays in step 4**: it is a write, it goes
through `Op::GlobalUpdate` and `Data::from_inner_data`'s invariant check, and it
raises a question this milestone does not have to answer — how a tree names an
entity that does not exist yet, when ids have no constructor (§3.12 records what
step 4 inherits).

**What is out of scope.** The ops mirror itself (`add` / `update` / `remove` /
`set_*` / `move_*`), the structured `Fix` payload on `Warning`, and
`OpResult.new_id` carrying a real id — all the sibling half of step 3.
`replace_all`, per above. The solver's configuration dataclasses (§10) are step
5: they mirror `constraints_colloscopes`, not the document. The `.pyi` stubs of §7
cover the whole module and are written once the module has its whole shape, after
the ops mirror.

**Refinements to `new_api_design.md`.** Seven places where this document sharpens
the design's letter, recorded so the authority chain stays clear:

- §2 says a value "mirrors an op payload". **It mirrors the entity**, which is
  sometimes larger. §2.0 gives the rule and the reason — the snapshot of §8 needs
  values that hold a whole document, and two of the op payloads hold less than
  their entity does.
- §5's write table spells the slot add `doc.slots.add(subject, SlotData)`. It
  becomes `doc.slots.add(SlotData)`: once the value carries the subject, the
  separate argument is the same fact written twice (§3.4).
- §14's `TeacherData("Emmy", "Noether", …)` fixes the person field order as
  **firstname then surname**, which is the reverse of the handle table's listing
  order in `handle_api.md` §3.4. The attribute *names* are the same; only the
  constructor order differs, and this document follows §14 (§3.1).
- §14's `subjects={maths.id}` is not the only accepted spelling: a field that
  names an entity takes a handle or an id interchangeably, like every other
  argument of the API (§2.3).
- §3's value list names `BalancingData`, while the handle sub-view is
  `BalancingOptions`. The list wins; the asymmetry is recorded in §3.8.
- §3's "Absent optional text is `None`, never `""`" is a *boundary* rule, not a
  dataclass rule: a dataclass is dumb and stores what it is given, and the
  refusal happens on extraction (§2.4).
- §2's "converted at the Rust boundary (pyo3 `FromPyObject`)" cannot be a
  `FromPyObject` impl: the conversion needs the document, to resolve the entity
  a field names, and `extract_bound` has nowhere to put one. It is a `Value`
  trait instead, `pub` so the round-trip test can drive it before the ops mirror
  lands (§2.2).

---

## 1. Why dataclasses, and where the line with the leaf values falls

The module already has two kinds of value-like object, and this milestone adds a
third. The line between them is not a matter of taste, so it is worth writing
down once:

- **Leaf values** — `Weekday`, `Enforcement`, `Orientation`, `Limit`, `Color`,
  `TimeSlot`, `WeekBlock`, the periodicity family — are frozen Rust pyclasses,
  in `values.rs`. The reason is recorded there and in `caveats.rs`: §2's
  argument for dataclasses is that a pyo3 getter *clones* the struct it hands
  back, so `value.nested.field = x` mutates a temporary. That argument does not
  apply to a flat immutable value that only travels out of Rust. A leaf value is
  built whole, compared whole and replaced whole.
- **`*Data` values** — everything this document defines — are Python
  dataclasses, in a `.py` file the module ships. They are exactly the case §2's
  argument is about: they nest (`SubjectData.interrogation.duration`,
  `ExportConfigData.colloscope_config.sheet_name`) and they hold real mutable
  containers (`d.excluded_periods.add(period)`). A Rust pyclass could offer
  neither without the temporary trap coming back.

The two meet without friction: a `*Data` field *holds* leaf values —
`InterrogationData.periodicity` is a `Periodicity`, `LimitsData`'s three fields
are `Limit`s, `IncompatData.slots` is a list of `TimeSlot`s. Nothing is defined
twice, and the read surface's vocabulary is the write surface's vocabulary.

Validation follows the same line, and for the same reason (§2.4): a leaf value
can refuse at birth because it is born whole; a dataclass cannot, because it is
filled in over several statements and the empty builder must be legal.

## 2. Common machinery

### 2.0 A value is the entity, flattened

**A `*Data` holds every field of its entity, in the shape the matching handle
shows.** Not the shape of the op payload, and not the shape of the model struct.

For eleven of the thirteen entity values these are all the same thing, and the
rule only says something in two places:

- The subject ops take a `SubjectParameters`, which holds a name and the
  interrogation parameters. The subject's `excluded_periods` sit one level up, on
  `subjects::Subject`, and no subject op carries them.
- The slot ops take a `slots::Slot`, which does hold a `subject_id` — but the add
  op overwrites it with its own separate argument (`ops/src/slots.rs`,
  `AddNewSlot`) and the update op refuses a slot whose subject changed.

An earlier draft of this document let the values follow the payloads there, and
dropped both fields. **That is wrong, and `doc.snapshot()` is what shows it.**
§8's `DocumentData` is "a detached value tree of the whole document", "built from
the same `*Data` dataclasses". A snapshot whose `SubjectData` has no exclusions
loses which subjects skip which periods, and one whose `SlotData` has no subject
loses which subject each slot belongs to. To keep them, `DocumentData` would have
to carry side tables of its own — and then there are two subject vocabularies,
one for the ops and one for the snapshot, which is exactly what §8's "the two
interfaces share one vocabulary" forbids.

`snapshot()` is built in this milestone (§3.12), so this is not an argument from
a future that might never arrive: the last commit assembles a whole document out
of these classes, and the two fields are load-bearing there.

So the values are entity-complete, and the ops mirror is where the mismatch is
handled — **loudly, never by ignoring a field**:

| call | the field it cannot carry | what it does |
|---|---|---|
| `doc.subjects.add(d)` | `d.excluded_periods` non-empty | raises `ValueError`, naming `set_period_status` |
| `doc.subjects.update(s, d)` | `d.excluded_periods` differs from the document's | raises `ValueError`, naming `set_period_status` |
| `doc.slots.update(slot, d)` | `d.subject` differs from the slot's | raises `ValueError`, saying the subject is fixed at creation |

Both common paths still work without a thought: `doc.subjects.update(s,
s.to_data())` is a no-op because the exclusions match, and a script that reads a
subject, renames it and writes it back never touches the field. What raises is
what would otherwise have been silently thrown away.

These three checks belong to the **ops mirror**, not to this milestone: each one
compares a value against a target entity, and a value does not know its target.
This milestone's job is to make the fields exist and round-trip. It is recorded
here because it is the reason they exist.

**The flattening.** The model nests where its reference machinery needs it to:
`Subject` splits an id-free `SubjectParameters` from an `#[fk] excluded_periods`,
`GroupList` splits id-free parameters from an `#[fk] filling`, `Teacher` and
`Student` share a `PersonWithContact`, `Slot` packs a `SlotStart`. Every one of
those splits separates the part that holds ids from the part that does not, so
that the `References` derive and the dense renumbering walk only visit the first.
Python has no such machinery, so it flattens all of them — which is what the
handles already do (`teacher.surname`, not `teacher.desc.surname`;
`slot.weekday`, not `slot.start_time.weekday`). What stays nested in a value is
only what has a life of its own: an optional sub-record (`InterrogationData`), a
sum (the fillings), a repeated shape (the rule sides, the export sub-configs).

### 2.1 The `.py` module, and how it is shipped

The dataclasses live in one file, `python/src/data.py`, `include_str!`'d into the
crate and materialized at module initialization:

```rust
let data = PyModule::from_code(py, DATA_PY, "collomatique/_data.py", "collomatique._data")?;
```

The module is registered in `sys.modules` under `collomatique._data` — the
`dialogs` submodule already sets that precedent (`dialogs.rs`: a submodule hung
off its parent is not one Python can `import`) — and every class it defines is
re-exported into `collomatique` itself, so a script writes `clm.SubjectData` and
never names the private module.

Compiling it from a string rather than shipping a package is what makes the
hosted path need no filesystem at all (`new_api_design.md` §12), and the same
code runs for the wheel, so there is exactly one mechanism rather than one per
build shape.

The file opens with `from __future__ import annotations`, so its type hints are
strings and it needs no runtime import of the Rust classes it mentions — which it
could not have anyway, since it is created *during* `collomatique`'s own
initialization.

### 2.2 The Rust boundary

`python/src/data.rs` holds one marker type per value class and one trait they all
implement — the same shape `handles.rs` gives the handle classes, for the same
reason: the classes are uniform, and a trait is what stops them drifting.

```rust
/// One python value class, and the model type behind it
pub trait Value: Sized {
    /// The model type this value converts to — the **entity**, per §2.0, not
    /// the op payload. They are the same type for eleven of the thirteen;
    /// `SubjectData` converts to a `subjects::Subject` and `SlotData` to a
    /// `slots::Slot`, and it is the ops mirror that takes the `parameters` half
    /// out of the first and checks what it cannot carry.
    type Model;

    /// The python class name — `SubjectData`
    const CLASS: &'static str;

    /// The entity one python value names
    fn from_py(doc: &Py<Document>, obj: &Bound<'_, PyAny>) -> PyResult<Self::Model>;

    /// The python value for one entity
    fn to_py<'py>(py: Python<'py>, model: &Self::Model) -> PyResult<Bound<'py, PyAny>>;
}
```

Two directions, both explicit:

- **In** — `from_py`, reading the dataclass by attribute access
  (`obj.getattr("name")?`), converting each field and validating (§2.4). Never
  by `cast::<T>()`: a value is a *Python* object, so anything with the right
  attributes extracts. That is deliberate — a script may subclass a dataclass,
  and duck typing is the language's own convention for this shape.
- **Out** — `to_py`, calling the dataclass class object with keyword arguments,
  fetched from the `collomatique._data` module. `to_data()` is its only caller in
  this milestone.

**`from_py` takes the document, and that is why it is not a `FromPyObject`
impl.** A field that names an entity has to be resolved against *this* document
— a handle of another one names nothing here, and a dead id has to be refused —
and that is `handles::argument`, which takes a `&Py<Document>`.
`FromPyObject::extract_bound` has nowhere to put one. Half the classes could
implement it (the export configs name no entity) and half could not, and two
shapes for one boundary would be worse than one shape that carries an argument
it sometimes ignores. Going out is *not* symmetric, which the implementation
proved against this document's first draft: `to_py` needs no document at all — a
value holds ids, and an id is minted detached (`SubjectId::wrap` and its kin),
resolving nothing and keeping nothing alive — so the outbound half takes only
the model, and `from_py` alone carries the argument.

The check is done where §5 of the design says all argument checks are done —
**before** anything takes the mutable borrow, never inside it. `from_py` borrows
the document to ask about liveness, so the ops mirror will call it first and take
its `borrow_mut` after, and the `PanicException` that a nested borrow would raise
never comes up.

The trait and the marker types are `pub`. That is not a concession to the tests:
the extraction's real caller is the ops mirror, in `collections/*.rs`, and until
it lands the integration test drives the same public door — the same way the
read surface's staleness tests already reach `Document::update`, rather than a
test-only back door.

Where the model type is sealed, the boundary calls the model's own constructor
and turns its error into a `ValueError`: `PairingRule::new`, `GroupList::new`,
`SlotWithDuration::new`, `NonEmptyRangeInclusive::new`. The API never
re-implements a rule the model already states.

### 2.3 What a value holds when it names an entity

A field that names an entity — `TeacherData.subjects`, `SlotData.teacher`,
`StudentData.excluded_periods` — **accepts a handle or an id interchangeably**,
and `to_data()` always **produces ids**.

Accepting both is the rule the rest of the API already follows: every method that
takes an entity takes either. Making the value classes the one place where a
handle is refused would be a bigger inconsistency than what it buys, and it would
show up on every line a script writes — `subjects={maths}` is the natural
spelling, and `subjects={maths.id}` should not be the only legal one.

Producing ids is what keeps a value detached, which is its whole point: a handle
carries a `Py<Document>`, so a value holding handles would be connected to the
document `to_data()` copied it out of, and would keep it alive.

**The wart this leaves, stated plainly:** a dataclass is dumb, so it stores what
it was given, and

```python
clm.TeacherData("Emmy", "Noether", subjects={maths})
    == clm.TeacherData("Emmy", "Noether", subjects={maths.id})   # False
```

Two values that extract to the same payload can compare unequal, because a
handle and an id hash differently. The docstring of every such field says so, and
`repr` shows the difference plainly (`<Subject #3 'Maths'>` against
`<SubjectId 3>`). The alternative — refusing handles in fields so that `==` is
exact — was weighed and dropped: it trades a rare confusion between two
hand-built values for friction on every ordinary line.

A value built for one document and extracted against another is refused by the
extraction, through the same `handles::argument` check every method uses: a
foreign handle raises `StaleHandleError`, and an id this document does not hold
raises it too.

The two spellings leave a mapping key one more way to go wrong: a dict that
names one entity twice — once by handle and once by id — holds two keys python
cannot merge, since they hash differently. Extraction refuses that loudly, as a
`ValueError` naming the section and the id both spellings resolve to, rather
than letting one entry silently overwrite the other. The rule holds everywhere
a value is keyed by entities: the sections of `DocumentData`, its two junction
tables, and both tables of `ColloscopeData` — the placements inside one row
included. A *set* field needs no such refusal: two spellings of one student
collapse into one member on extraction, which is exactly what a set means.

### 2.4 Validation: where it happens, and what it raises

**The dataclasses are dumb.** No `__post_init__`, no property setters, no checks.
`clm.SubjectData("Maths")` and `d.duration = -1` both simply work; the second one
is caught when the value is used.

**The boundary validates.** Extraction is where a value stops being a script's
scratchpad and becomes an op payload, and it is the last moment at which the
failure can still name the field that caused it. It performs, in this order per
field:

1. **Type conversion** — `int` → `NonZeroU32`, `(min, max)` → `NonEmptyRangeInclusive`,
   `datetime.time` → `WholeMinuteTime`, `str | None` → `Option<NonEmptyString>`.
2. **The model's non-empty rule** — a field the model types
   `Option<NonEmptyString>` (`tel`, `email`, a week annotation, a group name)
   refuses `""`: absent is `None`. A field the model types `String`
   (`SubjectData.name`, `SlotData.extra_info`, the export sheet names) accepts
   `""`, mirroring rather than editorializing — the same rule the read surface
   follows.
3. **The model's sealed constructors** — `GroupList::new`'s group-count match and
   duplicate-student check, `PairingRule::new`'s "antecedent is not the
   consequent", `SlotWithDuration::new`'s midnight rule.
4. **Entity resolution** — `handles::argument`, per §2.3.

Failures 1–3 raise `ValueError`, which is what the leaf values already raise and
what §6 of the design calls for ("`ValueError`-family conversion errors for
invalid value contents"). No new exception class: a script that mistyped a field
has not met a *document* error, it has met a bad value, and Python already has
the word for that. Failure 4 raises `StaleHandleError`, because it is the same
failure `doc.is_week_active(dead_week)` is.

Messages name the class, the field and what was wrong, in the style `values.rs`
already uses:

```
a TeacherData's tel is a non-empty string or None, and '' is neither
a SubjectData's interrogation.duration is at least 1, and 0 was given
a GroupListData has 4 groups and 3 group names
```

### 2.5 Defaults, and how they are kept honest

A field takes the **model's own default** wherever the model has one, so that
`clm.InterrogationData()` is the same thing the application creates when a user
switches interrogations on. A field with no honest default is required.

Required, everywhere: **a name** (`SubjectData.name`, `TeacherData`'s two person
fields, `WeekPatternData.name`, `IncompatData.name`, `GroupListData.name`) and
**an entity reference that must point somewhere** (`IncompatData.subject`,
`SlotData.teacher`). Nothing else. A nameless entity and a slot with nobody to
hold it are not things to create by accident.

Python requires defaulted fields to come last, so the field order of each class is
**required fields first, then defaulted ones, each group in the order the
handle's attribute table lists them**. The order is only ever load-bearing for the
one or two required fields; everything else is passed by keyword, as §14 does.

**The drift risk is real and is closed by a test, not by discipline.** A default
written in Python is a copy of a value defined in Rust, and copies drift. So
every class whose model type has a `Default` gets one assertion in the round-trip
test:

```rust
assert_eq!(extract::<InterrogationData>(py_default)?.0, SubjectInterrogationParameters::default());
```

Where the model has three constructors rather than one `Default` —
`PerStudentGroupsConfig::default_all_groups` and its two siblings — the dataclass
mirrors them as three classmethods, and all three are pinned the same way
(§3.9).

### 2.6 `to_data()`

Every handle and every sub-view that has a matching value class grows one method:

```python
d = doc.subjects[sid].to_data()     # SubjectData
d = subject.interrogation.to_data() # InterrogationData
d = doc.settings.global_limits.to_data()   # LimitsData
```

- It is a **method with a conversion name**, never an attribute. An attribute
  returning a copy is precisely how the old API laid its trap
  (`new_api_design.md` §2).
- It returns a **fresh object every call**. Two calls give two values that compare
  equal and share nothing.
- It reads through the handle, so a **stale handle raises `StaleHandleError`**
  like every other read, and an `Interrogation` sub-view whose subject stopped
  holding colles raises the sub-view's own message.
- It is **the handle, detached, with no exceptions**: every field the handle shows
  comes out, entity references turning into ids (§2.3) and the model's inner
  grouping structs staying flat (§2.0). That is what makes `doc.snapshot()`
  buildable out of these classes, and it is why the rule of §2.0 is worth the
  three checks it costs the ops mirror.

`to_data()` is *not* added to `Period`, `Week`, or the export-config views'
parents that have no payload of their own; §4 lists what does not exist and why.

### 2.7 Conventions

- **Naming** is `handle_api.md`'s, with `Data` appended: the value matching the
  `Subject` handle is `SubjectData`, the one matching the `PairingRuleSide`
  sub-view is `PairingRuleSideData`. The two exceptions are recorded where they
  occur (§3.6, §3.8).
- **No `.id`.** The design's rule of thumb — handles have `.id`, values don't —
  is structural here: a value has no identity to carry.
- **Ranges are `(min, max)` tuples**, as on the read surface. The model's
  `NonEmptyRangeInclusive` never leaks as a class.
- **Times and durations** as on the read surface: `datetime.time` for times of
  day, plain int minutes for durations, `datetime.date` for dates.
- **Containers are the mutable ones** — `list` for ordered content, `set` for
  sets, `dict` for mappings — which is the whole point of a value. The read
  surface's tuples and frozensets are a *handle* rule; a value that could not be
  appended to would be a worse leaf value.
- **Mutable defaults** use `field(default_factory=…)`, per Python's own rule.
- **`==` and `repr`** come from `@dataclass`. The rendered-text rule of §3 of the
  design does not bite here: a dataclass repr echoes identifiers only, and the
  French words appear inside the leaf values it prints
  (`TimeSlot(weekday=Jeudi, …)`), which already render themselves.

## 3. The vocabulary, class by class

### 3.1 `TeacherData` and `StudentData`

Payloads: `teachers::Teacher { desc, subjects }`, `students::Student { desc,
excluded_periods }`. The model's shared `PersonWithContact` card is flattened, as
it is on the handles.

`TeacherData`:

| field | type | default | notes |
|---|---|---|---|
| `firstname` | `str` | — | `""` allowed; the model types it `String` |
| `surname` | `str` | — | idem |
| `tel` | `str \| None` | `None` | `""` refused |
| `email` | `str \| None` | `None` | `""` refused |
| `subjects` | `set` | empty | `Subject` handles or `SubjectId`s |

`StudentData` is the same card with a different second field:

| field | type | default | notes |
|---|---|---|---|
| `firstname` | `str` | — | |
| `surname` | `str` | — | |
| `tel` | `str \| None` | `None` | |
| `email` | `str \| None` | `None` | |
| `excluded_periods` | `set` | empty | `Period` handles or `PeriodId`s |

Firstname before surname, per §14 — and per `collections::person_name`, which is
the order every screen of the application shows.

A teacher's `subjects` may only name subjects that hold interrogations: the op
refuses the rest (`AddNewTeacherError::SubjectHasNoInterrogation`). That check
stays where it is, in `ops/`, and reaches Python as an `UpdateError` when the
write runs — it is a statement about the document, not about the value.

### 3.2 `SubjectData` and `InterrogationData`

Entity: `subjects::Subject { parameters: { name, interrogation_parameters },
excluded_periods }`, flattened. The op payload is the `parameters` half alone.

| field | type | default | notes |
|---|---|---|---|
| `name` | `str` | — | `""` allowed |
| `interrogation` | `InterrogationData \| None` | `InterrogationData()` | `None` = the subject holds no colles |
| `excluded_periods` | `set` | empty | `Period` handles or `PeriodId`s |

`excluded_periods` is the field §2.0 is about. It is here because
`doc.snapshot()` needs it, it round-trips through `subject.to_data()`, and the
two subject mutators refuse to silently discard it: `add` raises when it is
non-empty, `update` raises when it differs from what the document holds, and both
messages name `doc.subjects.set_period_status(s, p, active)` — the one op that
moves it. Adding a subject that skips a period is therefore two calls, and a
transaction makes them one undo slot:

```python
with doc.transaction("Ajouter la spécialité"):
    s = doc.subjects.add(clm.SubjectData("Spé maths"))
    doc.subjects.set_period_status(s, first_period, False)
```

The default deserves its own note: **`clm.SubjectData("Maths")` creates a subject
that holds colles**, with the application's own default parameters — because
`SubjectParameters::default()` does, because the GUI's "add subject" does, and
because a subject exists to be interrogated in. The subject that holds none is
the exception, and it is spelled out: `clm.SubjectData("Quidditch",
interrogation=None)`.

`InterrogationData` — payload `SubjectInterrogationParameters`, every field
defaulted from the model:

| field | type | default | notes |
|---|---|---|---|
| `students_per_group` | `(int, int)` | `(2, 3)` | both ends ≥ 1 |
| `groups_per_interrogation` | `(int, int)` | `(1, 1)` | both ends ≥ 1 |
| `duration` | `int` | `60` | minutes, ≥ 1 |
| `take_duration_into_account` | `bool` | `True` | |
| `periodicity` | `Periodicity` | `EveryNWeeks(2)` | one of the four leaf values |

### 3.3 `WeekPatternData`

Payload: `week_patterns::WeekPattern`.

| field | type | default | notes |
|---|---|---|---|
| `name` | `str` | — | `""` allowed |
| `excluded_weeks` | `set` | empty | `Week` handles or `WeekId`s |

### 3.4 `SlotData`

Entity: `slots::Slot { subject_id, teacher_id, start_time, extra_info,
week_pattern, cost }`, with `start_time` — a `SlotStart` — flattened into two
fields.

| field | type | default | notes |
|---|---|---|---|
| `subject` | `Subject \| SubjectId` | — | fixed once the slot exists |
| `teacher` | `Teacher \| TeacherId` | — | |
| `weekday` | `Weekday` | — | |
| `start_time` | `datetime.time` | — | whole minutes |
| `extra_info` | `str` | `""` | the model's plain `String` |
| `week_pattern` | `WeekPattern \| WeekPatternId \| None` | `None` | `None` = every week |
| `cost` | `int` | `0` | > 0 avoid, < 0 favour |

`subject` is the second field §2.0 is about, and it is the reason **`doc.slots.add`
loses its separate subject argument**: §5's table spells it `add(subject,
SlotData)`, which made sense while the value had no subject of its own. Now it
would be the same fact written twice, with a mismatch to arbitrate. `add(data)`
is also what every other collection's add looks like — slots were the only
exception, and the exception was an artifact of the payload.

```python
doc.slots.add(clm.SlotData(maths, snape, clm.Weekday.THURSDAY, datetime.time(14, 0)))
```

The subject cannot be *changed*: `SlotOp::Update` refuses it
(`SlotPrecheckError::CannotChangeSubject`), because the model files a slot under
its subject in the list that gives it its position. So `doc.slots.update(slot,
d)` raises when `d.subject` names a different subject, rather than discarding the
field — and the message says the subject is fixed at creation. A read-modify-write
never meets it: `slot.to_data()` fills the field with the slot's own subject.

`weekday` and `start_time` have no model default and no honest one, so both are
required — a slot that does not say when it happens is not a slot.

### 3.5 `IncompatData`

Payload: `incompats::Incompatibility`.

| field | type | default | notes |
|---|---|---|---|
| `name` | `str` | — | `""` allowed |
| `subject` | `Subject \| SubjectId` | — | not required to hold interrogations |
| `slots` | `list[TimeSlot]` | empty | the busy windows |
| `minimum_free_slots` | `int` | `1` | ≥ 1; no model default, and 1 is the neutral one |
| `week_pattern` | `WeekPattern \| WeekPatternId \| None` | `None` | |

### 3.6 `GroupListData`, and the two fillings

The payload is sealed and two-part: `group_lists::GroupList` holds a
`GroupListParameters { name, students_per_group, group_names }` and a
`GroupListFilling`, which is `Prefilled { groups }` or `Automatic {
excluded_students }`.

**Why the model nests it, since the question came up.** Not history. The split is
the same one `subjects::Subject` makes, and for the same reason: the id-free
parameters on one side, the `#[fk]` part that holds `StudentId`s on the other, so
that the `References` derive and the dense renumbering walk visit only the second
(`GroupLists::collect_ids` calls `group_list.filling.collect_ids`, and never
looks at `params`). On top of that, `GroupList` is *sealed* — private fields, a
validating `GroupList::new` — because one of its invariants spans the two halves:
`filling`'s prefilled groups must number exactly as many as `params.group_names`.
The pair is the unit that gets checked, so the pair is the unit that gets a type.

Python has neither a reference walk nor a renumbering pass, so the parameters
half flattens (§2.0). The seal is not lost: the boundary calls `GroupList::new`,
which is where the group-count and duplicate-student checks stay.

The handle flattens the sum too, into `.is_prefilled`, `.groups` and
`.excluded_students`, with the `None`-for-inapplicable rule. **The value does
not**: it keeps the sum, because a value is written as well as read, and a flat
encoding of a sum type has two states that mean nothing (both set, neither set)
and can only be refused late.

| field | type | default | notes |
|---|---|---|---|
| `name` | `str` | — | the model's own default is `"Liste"` |
| `students_per_group` | `(int, int)` | `(2, 3)` | both ends ≥ 1 |
| `group_names` | `list[str \| None]` | `[None] * 16` | length = the maximum group count; `None` = unnamed; `""` refused |
| `filling` | `Filling` | `AutomaticGroups()` | one of the two leaf values below |

Two new frozen leaf values in `values.rs`, under an abstract base `Filling` so
`isinstance(f, clm.Filling)` works — exactly the shape `Periodicity` and its four
subclasses already have:

- `PrefilledGroups(groups)` — `.groups` is a tuple of frozensets of `Student`
  handles or `StudentId`s, group *i* being entry *i*. It must have exactly
  `len(group_names)` entries and no student twice, which is `GroupList::new`'s
  own check, raised as `ValueError`.
- `AutomaticGroups(excluded_students=())` — `.excluded_students` is a frozenset;
  the students the automatic filling must skip.

`gl.to_data()` builds the matching filling from the handle's flat reads. The
asymmetry between the two surfaces is deliberate and is documented on both sides.

### 3.7 `PairingRuleData` and `SlotPairingRuleData`

Payloads: `pairings::PairingRule` and `slot_pairings::SlotPairingRule`, both
sealed by a `new` that refuses a rule whose two sides name the same entity.

`PairingRuleData`:

| field | type | default | notes |
|---|---|---|---|
| `antecedent` | `PairingRuleSideData` | — | |
| `consequent` | `PairingRuleSideData` | — | |
| `excluded_periods` | `set` | empty | |
| `soft` | `bool` | `False` | objective rather than hard constraint |

`PairingRuleSideData` is a dataclass, not a leaf value — it nests inside a value
and `d.antecedent.should_have = False` should be a real mutation:

| field | type | default |
|---|---|---|
| `subject` | `Subject \| SubjectId` | — |
| `should_have` | `bool` | `True` |

`SlotPairingRuleData` and `SlotPairingRuleSideData` are the same two classes with
`slot` in place of `subject`.

A rule whose antecedent and consequent name the same entity raises `ValueError`
at extraction, carrying the model's own message — the "sealed constructor
violations" §6 of the design names.

### 3.8 `LimitsData` and `BalancingData`

Payloads: `settings::Limits` and `balancing::BalancingOptions`. Both are
whole-entry override records: a `None` field *disables* the corresponding global
limit rather than inheriting it, and that semantic stays in Rust. A value here is
one raw entry, never a merge.

`LimitsData` — the model's `Default` is all-`None`:

| field | type | default | notes |
|---|---|---|---|
| `interrogations_per_week_min` | `Limit \| None` | `None` | count ≥ 0 |
| `interrogations_per_week_max` | `Limit \| None` | `None` | count ≥ 0 |
| `max_interrogations_per_day` | `Limit \| None` | `None` | count ≥ **1** — the model types this one non-zero, and the extraction is where a `0` is refused, as `Limit`'s docstring already promises |

`BalancingData` — the model's `Default` is teacher rotation as an objective and
nothing else:

| field | type | default | notes |
|---|---|---|---|
| `teacher_rotation` | `Enforcement \| None` | `Enforcement.OBJECTIVE` | `None` = not pursued |
| `slot_rotation` | `Enforcement \| None` | `None` | |
| `avoid_twice_in_a_row` | `Enforcement \| None` | `None` | |
| `year_teacher_rotation` | `bool` | `False` | |
| `period_teacher_rotation` | `bool` | `False` | |

The name is `BalancingData`, from the design's §3 list, while the handle sub-view
is `BalancingOptions`. It is the one place the "`to_data()` returns the matching
`*Data`" rule reads oddly — `BalancingOptionsData` would be uniform and clumsy —
and the design's own list settles it.

### 3.9 `ExportConfigData` and its four sub-configs

Payloads: the eleven export-config ops take one `GlobalConfig`, one
`ColloscopeConfig`, three `PerStudentGroupsConfig`s, one `PerGroupListConfig` and
five bools.

`ExportGlobalConfigData` — mirrors `GlobalConfig::default()`:

| field | type | default |
|---|---|---|
| `background_color` | `Color` | `Color(255, 255, 255)` |
| `stripes_color_enabled` | `bool` | `True` |
| `stripes_color` | `Color` | `Color(220, 220, 230)` |

`ExportColloscopeConfigData` — mirrors `ColloscopeConfig::default()`:

| field | type | default |
|---|---|---|
| `sheet_name` | `str` | `"Colloscope"` |
| `extra_info_column_enabled` | `bool` | `True` |
| `extra_info_column_name` | `str` | `"Info"` |
| `teacher_email_enabled` | `bool` | `True` |
| `teacher_email` | `str` | `"Contact"` |
| `teacher_tel_enabled` | `bool` | `False` |
| `teacher_tel` | `str` | `""` |
| `orientation` | `Orientation` | `LANDSCAPE` |
| `display_week_dates` | `bool` | `True` |
| `display_annotations` | `bool` | `True` |
| `no_interrogation_color` | `Color` | `Color(140, 140, 140)` |
| `annotation_color_enabled` | `bool` | `True` |
| `annotation_color` | `Color` | `Color(255, 255, 0)` |
| `extra_colors` | `dict[str, Color]` | empty |

`ExportStudentGroupsConfigData` — the model has three constructors rather than a
`Default`, so the dataclass has three classmethods and a required `sheet_name`:

| field | type | default |
|---|---|---|
| `sheet_name` | `str` | — |
| `orientation` | `Orientation \| None` | `None` (auto-detect from the group count) |
| `show_emails` | `bool` | `True` |
| `show_tel` | `bool` | `False` |

```python
clm.ExportStudentGroupsConfigData.all_groups()          # « Tous les groupes »
clm.ExportStudentGroupsConfigData.automatic_groups()    # « Groupes automatiques »
clm.ExportStudentGroupsConfigData.prefilled_groups()    # « Groupes préremplis »
```

`ExportGroupListConfigData` — mirrors `PerGroupListConfig::default()`:

| field | type | default |
|---|---|---|
| `orientation` | `Orientation` | `PORTRAIT` |
| `show_emails` | `bool` | `True` |
| `show_tel` | `bool` | `False` |
| `center_vertically` | `bool` | `False` |

`ExportConfigData` — the whole tree, mirroring `ExportConfig::default()`. No op
takes it: it exists because `doc.export_config.to_data()` is the natural copy-out
of the view, and because §8's `DocumentData` will hold one. Its extraction has no
caller until then, and that is said in its docstring rather than hidden.

| field | type | default |
|---|---|---|
| `global_config` | `ExportGlobalConfigData` | the default above |
| `colloscope_enabled` | `bool` | `True` |
| `all_groups_enabled` | `bool` | `True` |
| `automatic_groups_enabled` | `bool` | `False` |
| `prefilled_groups_enabled` | `bool` | `False` |
| `per_group_list_enabled` | `bool` | `True` |
| `colloscope_config` | `ExportColloscopeConfigData` | the default above |
| `all_groups_config` | `ExportStudentGroupsConfigData` | `.all_groups()` |
| `automatic_groups_config` | `ExportStudentGroupsConfigData` | `.automatic_groups()` |
| `prefilled_groups_config` | `ExportStudentGroupsConfigData` | `.prefilled_groups()` |
| `per_group_list_config` | `ExportGroupListConfigData` | the default above |

The enabled flags sit beside the configs they gate, not inside them — the model's
own shape, and the interface's memory of what was chosen before a section was
switched off (`new_api_design.md` §11.2).

### 3.10 `ColloscopeData`

Payload: `ops::ColloscopeContents`, the plain-map twin of the state's
`Colloscope` that `InstallColloscope` takes.

| field | type | default | notes |
|---|---|---|---|
| `interrogations` | `dict[(slot, week), set[int]]` | empty | the assigned group numbers per cell |
| `group_lists` | `dict[group_list, dict[student, int]]` | empty | the placements of each automatic list |

The keys follow §2.3 like every other entity reference: handles or ids, in the
`(slot, week)` pairs and in both mapping keys alike. Group numbers are indices
into the associated group list's names, as everywhere in this API.

A hand-built value need not be canonical: an empty group set or an empty
placement map just means "no row", which is what `ColloscopeContents` already
promises its callers. `doc.colloscope.to_data()` copies the whole colloscope out;
`doc.colloscope.install(d)` — the ops mirror's job — puts one back.

### 3.11 `WeekData`

Entity: `weeks::Week`.

| field | type | default | notes |
|---|---|---|---|
| `period` | `Period \| PeriodId` | — | the owning period; authoritative in the model |
| `interrogations` | `bool` | `True` | whether colles happen on this week at all |
| `annotation` | `str \| None` | `None` | « Rentrée », « Vacances »; `""` refused |

**No op takes a `WeekData`.** The two week ops address a week by `(period,
index)` and each carries one value: `UpdateWeekStatus` a bool,
`UpdateWeekAnnotation` an annotation. So this class exists for §3.12 and for
`week.to_data()`, and nothing else — the second value class in that position,
beside `ExportConfigData`. Its docstring says so rather than leaving a script to
look for a `doc.weeks.update` that is not there; §5 of the design maps the two
ops to `doc.weeks.set_status(week, active)` and `.set_annotation(week, text)`.

The handle's `.index` and `.monday` are not fields: both are derived — the index
from the week's place in the walk, the monday from the index and the document's
start date. A snapshot that stored them could contradict itself.

There is still **no `PeriodData`**, and §3.12 shows why it would have nothing in
it: `periods::Periods` keeps `first_week` and an `OrderedTable<PeriodId, ()>` —
"existence and display order only", as its own comment says.

### 3.12 `DocumentData`, and `doc.snapshot()`

```python
tree = doc.snapshot()      # DocumentData — the whole document, detached
```

`DocumentData` mirrors `InnerData` section by section: `params`, the colloscope,
the export configuration. It is a dataclass like the others, every field
defaulted to an empty document, so `clm.DocumentData()` is what
`clm.new_document()` holds.

| field | type | mirrors |
|---|---|---|
| `first_week` | `datetime.date \| None` | `params.periods.first_week` |
| `periods` | `list[PeriodId]` | `params.periods.ordered_period_list`, in display order |
| `weeks` | `dict[WeekId, WeekData]` | `params.weeks`, in global week order |
| `subjects` | `dict[SubjectId, SubjectData]` | `params.subjects`, in user order |
| `teachers` | `dict[TeacherId, TeacherData]` | `params.teachers`, id order |
| `students` | `dict[StudentId, StudentData]` | `params.students`, id order |
| `assignments` | `dict[(PeriodId, SubjectId), set[StudentId]]` | `params.assignments`, stored rows only |
| `week_patterns` | `dict[WeekPatternId, WeekPatternData]` | `params.week_patterns`, id order |
| `slots` | `dict[SlotId, SlotData]` | `params.slots`, in subject-then-position order |
| `incompats` | `dict[IncompatId, IncompatData]` | `params.incompats`, id order |
| `group_lists` | `dict[GroupListId, GroupListData]` | `params.group_lists.group_list_map`, id order |
| `group_list_associations` | `dict[(PeriodId, SubjectId), GroupListId]` | `params.group_lists.subjects_associations` |
| `pairings` | `dict[PairingRuleId, PairingRuleData]` | `params.pairings`, id order |
| `slot_pairings` | `dict[SlotPairingRuleId, SlotPairingRuleData]` | `params.slot_pairings`, id order |
| `global_limits` | `LimitsData` | `params.settings.global` |
| `student_limits` | `dict[StudentId, LimitsData]` | `params.settings.students` |
| `global_balancing` | `BalancingData` | `params.balancing.global` |
| `subject_balancing` | `dict[SubjectId, BalancingData]` | `params.balancing.subjects` |
| `colloscope` | `ColloscopeData` | `colloscope` |
| `export_config` | `ExportConfigData` | `export_config` |

Three things the shape settles:

- **Order lives in the containers.** A Python dict has preserved insertion order
  since 3.7, by language guarantee, so `subjects` in user order and `slots` in
  the `doc.slots` walk order carry the model's two `OrderedTable`s and two
  `ordering` sidecars without a single index field. `periods` is a plain list
  because a period has nothing but its identity and its place.
- **The two `ordering` sidecars are not stored twice.** The model keeps slots as
  a `slot_map` plus a `Table<SubjectId, Vec<SlotId>>`; the snapshot keeps one
  ordered dict, because each `SlotData` names its own subject (§3.4) and the
  subject order is in `subjects`. The weeks are the same: one ordered dict, each
  `WeekData` naming its period (§3.11). This is where §2.0's two fields earn
  their place.
- **Sparse stays sparse.** `assignments` and `group_list_associations` hold the
  stored rows only, which is what the read surface's iterators already yield.

`snapshot()` is a **method on the document**, not on a collection, and it is a
pure read: it borrows the document, walks `InnerData`, and builds the tree. It
cannot fail on a document that exists. It is the same conversion `to_data()` is,
run over everything at once — and a script that wants one section still calls the
handle's own `to_data()`.

**What step 4 inherits.** `replace_all` will take a tree back, and a tree names
its entities by id — but ids have no constructor (`new_api_design.md` §2), so a
script can rename, delete and rewire a snapshot, and cannot *add* an entity to
one. That is a real gap and it is a question about a write: whether ids gain a
document-scoped minting call, or a tree may key a new entry by something that
means "give it a fresh id", or `replace_all` is simply the door for
transformations that add nothing. Nothing here forecloses any of the three, and
this milestone does not have to choose, because it only ever hands trees out.

## 4. What deliberately does not exist

- **No `PeriodData`.** A period has no data at all: `periods::Periods` keeps a
  start date and an `OrderedTable<PeriodId, ()>`, "existence and display order
  only". `WeekData` exists, though no op takes one either — the snapshot needs
  it, and a period's would have been an empty class (§3.11).
- **No `replace_all`.** The snapshot's other half is a write, and step 4's
  (§3.12). What this milestone does *not* leave it is a vocabulary problem: the
  tree it will take is the tree `snapshot()` already hands out.
- **No solver configuration classes.** Step 5, and they mirror
  `constraints_colloscopes` rather than `ops/`.
- **No `to_data()` on `Period`, `Week` or the collection views.** The first two
  have no value class; a collection is not an entity.
- **No validation inside the dataclasses.** §2.4 gives the reason: a builder that
  refuses its own empty state is not a builder.
- **No `.id` on any value, and no way to give one.** An id names a place in a
  document, and a value has none. Updating an existing entity is
  `collection.update(id_or_handle, value)` — the id is the method's argument,
  never the value's field.
- **No serialization** (`to_dict`, `from_json`, pickle support). A value holds
  ids, and ids are meaningless outside the run that produced them
  (`new_api_design.md` §2). A script that wants to carry data between runs
  carries content.

---

## 5. Implementation plan

### 5.1 Crate layout

Two new files in `python/src/` (the `foo.rs` + `foo/` convention, never
`mod.rs`), plus growth in files that already exist:

- `data.py` — every dataclass of §3, in one file, `include_str!`'d.
- `data.rs` — the boundary of §2.2: the `Value` trait, one marker type per value
  class with its `from_py` and `to_py`, and the module materialization and
  re-export of §2.1.
- `values.rs` — grows the `Filling` base class and its two subclasses (§3.6).
  Nothing else: the leaf-value vocabulary the read surface landed is complete.
- `collections/*.rs` — each handle and sub-view grows its `to_data()`.
- `lib.rs` — registers `Filling`, `PrefilledGroups`, `AutomaticGroups`, and calls
  the `data` module's own registration, which re-exports the dataclasses.

### 5.2 Testing approach

The existing harness carries this milestone as it carried the last one
(`python/tests/module.rs` + `tests/scripts/*.py`): a script reads a document and
leaves what it built in globals, Rust asserts against the same `InnerData` read
directly. Two additions:

- **The inbound direction** needs Rust to reach a Python object the script built.
  It already can — the harness reads the script's globals, and it already pulls
  the `Py<Document>` out of them for the staleness tests — so a round-trip test
  is: the script leaves a value in a global, Rust pulls it and the document out
  and calls `Value::from_py`, then compares against the payload read from
  `InnerData`. No new harness machinery.
- **The default pins of §2.5** ride the same door: the script leaves
  `clm.InterrogationData()` in a global, Rust extracts it against the document it
  already has and compares with `SubjectInterrogationParameters::default()`.
  These are the assertions that stop the Python-side defaults drifting from the
  Rust ones, so every class that has a model default gets one.

Each commit's tests cover, for its classes:

1. **Round trip out and back** — `handle.to_data()` extracts to exactly the
   payload the document holds, field by field, over the hogwarts fixture and over
   the synthetic documents the read-surface commits already build.
2. **Build from scratch** — a value written out in Python extracts to the payload
   the test expects, including every optional field left at its default.
3. **The defaults** — §2.5, where the model has one.
4. **The refusals** — `""` where the model wants `Option<NonEmptyString>`, a zero
   where it wants non-zero, an inverted range, a sealed-constructor violation:
   each raises `ValueError`, and the message names the field.
5. **The entity fields** — handles and ids accepted interchangeably (§2.3), a
   foreign handle and a dead id both raising `StaleHandleError`.
6. **Staleness** — `to_data()` through a dead handle raises, using the
   `run_stages` helper the read surface already built.

Every commit lands with its tests, compiles, and passes the suite on its own.
Test-first commit splitting does not apply — these are new features, not bug
fixes.

### 5.3 The commits

Dependency order: the first commit builds the mechanism on the simplest pair of
classes, and each later one is independent of the others. Where a class holds
another (`SubjectData.interrogation`), they land together.

1. **`python: the value dataclasses, and the people`** — `data.py` with its
   header and `TeacherData` / `StudentData`; `data.rs` with the `Value` trait,
   the module materialization, the re-export, and the two value types;
   `to_data()` on `Teacher` and `Student`. This commit is where the mechanism is
   settled
   and reviewed — the `from_code` materialization, the `sys.modules` entry, the
   attribute-access extraction, the `""`-refusal, the handle-or-id fields — on
   two classes small enough to read at a glance. Tests: the six kinds of §5.2
   against hogwarts, plus a `tel=""` and an `email=""` refusal.

2. **`python: the subject and interrogation values`** — `SubjectData`,
   `InterrogationData`; `to_data()` on `Subject` and the `Interrogation`
   sub-view. The periodicity leaf values travel back in for the first time, so
   this commit adds their extraction (the four subclasses, told apart by
   `isinstance`, each read through its own getters). Tests: the four periodicity
   kinds round-tripping through the synthetic fixture commit 2 of the read
   surface already builds; the model-default pin; `subject.to_data()` carrying
   the excluded periods, over a hogwarts subject that has some — the §2.0 field,
   pinned where it lands rather than where it is later checked; the
   `interrogation` default creating a colle-bearing subject.

3. **`python: the week pattern and slot values`** — `WeekPatternData`,
   `SlotData`; `to_data()` on `WeekPattern` and `Slot`. Tests: `Weekday` and
   `datetime.time` travelling back in; a non-whole-minute time refused;
   `slot.to_data().subject` naming the slot's own subject, and agreeing with
   `slot.subject.id`.

4. **`python: the incompatibility values`** — `IncompatData`; `to_data()` on
   `Incompat`. `TimeSlot` travels back in as a list element. Tests: hogwarts's
   six incompatibilities out and back; a window crossing midnight refused;
   `minimum_free_slots=0` refused.

5. **`python: the group list values, and the two fillings`** — `Filling`,
   `PrefilledGroups` and `AutomaticGroups` in `values.rs`; `GroupListData`;
   `to_data()` on `GroupList`. Tests: the prefilled and the automatic list side
   by side, from the fixture the read surface's commit 8 built; the group-count
   mismatch and the duplicated student both raising `ValueError` with
   `GroupList::new`'s own message; `""` refused as a group name.

6. **`python: the pairing rule values`** — `PairingRuleData`,
   `PairingRuleSideData`, `SlotPairingRuleData`, `SlotPairingRuleSideData`;
   `to_data()` on the two rule handles and their four sub-views. Tests: both
   `should_have` polarities and both `soft` values; a rule naming the same
   subject on both sides refused, and the same for slots.

7. **`python: the settings and balancing values`** — `LimitsData`,
   `BalancingData`; `to_data()` on the `Limits` and `BalancingOptions` sub-views.
   Tests: the whole-entry semantics preserved through a round trip (an override
   with a `None` field stays a `None` field); `max_interrogations_per_day` of 0
   refused while `interrogations_per_week_min` of 0 is accepted; both model
   defaults pinned.

8. **`python: the export configuration values`** — the four sub-config
   dataclasses and `ExportConfigData`; `to_data()` on the five export views and
   on `ExportConfig` itself. Tests: the non-default synthetic config of the read
   surface's commit 12, out and back; all six model defaults pinned, including
   the three `PerStudentGroupsConfig` constructors; `extra_colors` surviving as a
   plain dict.

9. **`python: the colloscope value`** — `ColloscopeData`; `to_data()` on the
   `Colloscope` view. Tests: the filled synthetic colloscope out and back; keys
   accepted as handles and as ids; an empty group set and an empty placement map
   accepted as "no row".

10. **`python: the week values`** — `WeekData`; `to_data()` on `Week`. The last
    entity value, and the only one no op consumes: it lands here because commit
    11 needs it. Tests: a hogwarts week with an annotation and one without;
    `""` refused as an annotation; the derived `.index` and `.monday` absent
    from the value and still on the handle.

11. **`python: the whole-document snapshot`** — `DocumentData` and
    `doc.snapshot()`. No new entity vocabulary: the tree is the ten commits
    above, assembled. Tests: a hogwarts snapshot compared against `InnerData`
    section by section — the completeness check for the whole milestone in one
    test, and the one that would catch a field this document forgot; the four
    orders (periods, weeks, subjects, slots) surviving as list and dict order,
    checked against the model's own walks; `clm.DocumentData()` equal to the
    snapshot of `clm.new_document()`; the sparse sections holding the stored
    rows only. (As landed, the section-by-section comparison runs against a
    purpose-built fixture rather than hogwarts: the completeness check wants
    something in every section — both shapes of every optional field, rows in
    both junction tables, a filled colloscope, non-default settings, balancing
    and export configuration — and the fixture guarantees that by
    construction.)

Eleven commits; each leaves the suite green and none changes a surface a previous
commit published. After commit 11 every op payload in `ops/` has a Python value
that converts to it and a `to_data()` that produces it, every entity of
`state-colloscopes/` has a value that holds all of it, and `doc.snapshot()`
proves the two claims at once. What is left of step 3 — the ops mirror — is a
mapping exercise with no vocabulary to invent, and step 4 starts from a
`DocumentData` it only has to learn to read back.
