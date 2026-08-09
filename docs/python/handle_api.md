# Handle API — the read surface: design and implementation plan

This document is the complete design of the handle API — step 2 of the migration
plan in `docs/python/new_api_design.md` §13: "Read surface: document, handles,
collections, ids." It sharpens the design document's §2 and §4 down to every
class, attribute and method, and ends with the implementation plan, split into
commits.

Where this document names an attribute, that is the whole read surface: an
attribute not named here does not exist. Completeness is checked against the
field inventory of `state-colloscopes/src/` — every public field of every
entity has exactly one place below.

**What is out of scope.** Everything that mutates: the ops mirror, `OpResult.new_id`
carrying real ids, and the value dataclasses — including `handle.to_data()` — are
step 3. The coarse door, document plumbing beyond what already exists, and the
solver are steps 4 and 5. The `.pyi` stubs of §7 cover the whole module and are
written once the module has its whole shape, after step 3. The *vocabulary*
defined here (ids, enums, leaf value classes) is shared with step 3 unchanged:
`SlotData` will hold the same `Weekday`, `SubjectData` the same periodicity
values, so nothing lands twice.

**Refinements to `new_api_design.md`.** Four places where this document sharpens
the design's letter, recorded so the authority chain stays clear:

- §4 says `doc.assignments[pid, sid]` returns a frozenset of `StudentId`. It
  returns a frozenset of `Student` *handles* (§3.7 below): every collection read
  that names an entity hands back a handle, uniformly. A handle is strictly more
  useful (the id is one attribute away) and membership tests still work, since
  handles hash and compare by `(document, id)`.
- §4's `doc.group_lists.association_for(period, subject)` returns a `GroupList`
  handle or `None`, for the same reason.
- `gl.group_name(g)` (used in §14's export example) always returns a string:
  the stored name, or the GUI's own fallback « Groupe n » (1-based) for an
  unnamed group. The raw names, `None` included, are on `.group_names`.
- §4's derived predicates `is_week_active` and `is_interrogation_possible` keep
  their place on the document, with the argument rule of §2.4 below: a dead
  argument raises instead of echoing the model's forgiving `false`.

---

## 1. The three layers, restated for this milestone

`docs/python/new_api_design.md` §2 gives the object model. This milestone builds
the first two layers of it, plus the leaf values both layers share:

- **Ids** — eleven opaque classes, one per id kind.
- **Handles** — live, read-only, frozen views bound to `(document, id)`, one
  class per entity, reached through collections on the document.
- **Leaf values** — small immutable, constructible classes for data that is not
  an entity: `Weekday`, `Enforcement`, `Orientation`, the periodicity family,
  `WeekBlock`, `TimeSlot`, `Limit`, `Color`. These are frozen Rust pyclasses,
  not the mutable `.py` dataclasses of step 3 — the same distinction
  `caveats.rs` already draws and documents: §2's argument for dataclasses is
  about *nested mutable* copies, and does not apply to a flat immutable value
  that only travels out of Rust. Step 3's dataclasses will hold these values in
  their fields and replace them wholesale to change them.

There is a fourth, minor kind: **live sub-views** — `Interrogation`,
`Limits`, `BalancingOptions`, the rule sides, the export sub-configs. A
sub-view is a handle in everything but the `.id`: it is bound to its parent
(`(document, subject_id)` for `subject.interrogation`, `(document,)` for
`doc.settings.global_limits`), reads the current state on every access, and
goes stale loudly with its parent. Sub-views exist where the model nests a
struct that step 3 will mirror with a nested dataclass, so the two surfaces
stay parallel: `subject.interrogation.duration` on the handle side,
`SubjectData.interrogation.duration` on the value side.

## 2. Common machinery

### 2.1 Ids

One class per id kind: `PeriodId`, `WeekId`, `SubjectId`, `TeacherId`,
`StudentId`, `WeekPatternId`, `SlotId`, `IncompatId`, `GroupListId`,
`PairingRuleId`, `SlotPairingRuleId`.

Each supports `==`/`!=`, hashing, ordering against its own kind, and a readable
`repr` — `<SubjectId 3>` — and nothing else. No constructor (`TypeError`), no
`int()`, no serialization; ordering against another kind raises `TypeError`,
equality against anything that is not the same kind is `False`. The
angle-bracket repr is deliberate: `SubjectId(3)` would read as an expression a
script could paste back, and there is no such constructor.

Ids do not know their document. Two documents open in one script can hand back
ids that compare equal while naming unrelated entities — the design already
says ids are meaningless outside the run that produced them, and the same
warning applies across documents within a run. The safe currency between
documents is content (names, matching); the safe currency within one document
is the handle, which *is* bound to its document. The docstring on every id
class says so.

### 2.2 Handles

A handle holds `(Py<Document>, id)` and nothing else. Every attribute access
borrows the document, resolves the id, reads, and lets go — a handle read
always sees the current state, through undo, redo and transactions alike.

- Handle classes are frozen pyclasses with no constructor. Attribute
  *assignment* raises `AttributeError` (pyo3 does this for getter-only frozen
  classes); there are no setters anywhere on the read surface.
- `==`/`!=` and `hash` work by `(document identity, id)` and **never touch the
  state**, so they keep working on a stale handle — a dict holding handles must
  not blow up when an entity dies.
- Ordering handles raises `TypeError`. Ids order; handles identify.
- `repr` reads the current state when it can and never raises:
  `<Subject #3 'Maths'>` alive, `<Subject #3 (stale)>` dead. Reprs exist for
  logging, and logging a dead handle is exactly when it matters.
- `.id` returns the id — the one attribute that works on a stale handle, since
  it does not read the state.

### 2.3 `StaleHandleError`

New exception, subclassing `Error`. Raised by any state-reading access through
a handle or sub-view whose entity is gone. The message names the kind and id
(`"this Subject handle is stale: subject <SubjectId 3> is no longer in the
document"`). Stale is always loud, never silent — the design's core repair of
the old API.

### 2.4 The two lookup conventions

There are exactly two ways an id-or-handle can fail to resolve, and each place
belongs to one convention:

- **Mapping positions** follow Python's mapping protocol: `collection[x]`
  raises `KeyError` when `x` names nothing in the document, `collection.get(x)`
  returns `None`, `x in collection` returns `False`. Asking a lookup is
  legitimate; the mapping vocabulary is the right answer.
- **Everywhere else** — attribute access through a handle, and an id-or-handle
  *argument* to a method (`doc.is_week_active(week)`,
  `doc.assignments[p, s]`'s address, `doc.group_lists.association_for(p, s)`,
  `doc.colloscope.interrogation(slot, week)`) — a dead reference raises
  `StaleHandleError`. A script passing a dead argument is mistaken about its
  own document; the model's own forgiving answers (`is_week_active` returns
  `false` for a dangling week) are not mirrored, because the question was
  malformed before it had an answer.

One deliberate wrinkle: `doc.assignments[p, s]` is spelled as an indexing but
its address arguments follow the *argument* convention. §3.7 explains why: its
reads are total, so `KeyError` can never mean "no row" — the only failure is a
bad address, and a bad address is a stale reference.

Every method that takes an entity takes **a handle or an id interchangeably**,
the design's rule of thumb applied to reads as well as the future writes.

### 2.5 Immutable containers, and only immutable containers

A read never returns anything mutable: tuples for ordered content, frozensets
for sets, and `types.MappingProxyType` over a fresh dict for the two mappings
the model has (colloscope placements, export extra colors). The proxy is
read-only and the dict under it is unreachable, so there is nothing to mutate
by accident — the structural kill of the chained-temporary trap, extended to
mappings.

Containers are **snapshots built at call time**: `subject.excluded_periods` is
the frozenset of that moment, and does not grow when the document changes. The
*elements* are handles, which stay live. This is the only coherent choice — a
frozenset cannot re-resolve — and it is also why containers are cheap to hand
back: they hold handles, never entity data.

Iteration over a collection snapshots the membership (the ids, in the
collection's order) when iteration starts, then mints handles lazily. Removing
an entity mid-iteration is therefore safe and loud: the loop still sees the id,
and touching the minted handle raises `StaleHandleError`.

### 2.6 Enums and leaf values

- `Weekday` — `MONDAY` … `SUNDAY`. A pyo3 fieldless enum: real `==`, `hash`,
  `repr`, members as class attributes. The old API's identity-comparison trap
  is structurally gone.
- `Enforcement` — `OBJECTIVE` (the model's `soft: true`: the solver optimizes
  for it) and `STRICT` (`soft: false`: a hard constraint). One vocabulary for
  every `SoftParam` in the model, so limits and balancing read the same way.
- `Orientation` — `PORTRAIT`, `LANDSCAPE`.
- `Limit(value, enforcement)` — a settings limit: `.value` (int),
  `.enforcement` (`Enforcement`). `Limit(3, clm.Enforcement.STRICT)`.
- `Color(red, green, blue)` — three ints 0–255, validated on construction.
- `TimeSlot(weekday, start_time, duration)` — an incompatibility slot:
  `.weekday` (`Weekday`), `.start_time` (`datetime.time`, whole minutes),
  `.duration` (int, minutes ≥ 1). Construction validates what
  `SlotWithDuration::new` validates and raises `ValueError` when it refuses.
- `WeekBlock(delay_in_weeks, size_in_weeks, count)` — one block of a custom
  periodicity: `.delay_in_weeks` (int ≥ 0, from the previous block, or from the
  start for the first), `.size_in_weeks` (int ≥ 1), `.count` (an `(int, int)`
  range, see below).
- The periodicity family, under an abstract base `Periodicity` so
  `isinstance(p, clm.Periodicity)` works:
  - `EveryNWeeks(n)` — the model's `ExactlyPeriodic` (§14 of the design already
    uses this name).
  - `OncePerBlock(weeks_per_block, minimum_week_separation)` — the model's
    `OnceForEveryBlockOfWeeks`; `minimum_week_separation` ≥ 1.
  - `CountInYear(count, minimum_week_separation)` — the model's
    `AmountInYear`; `count` is an `(int, int)` range,
    `minimum_week_separation` ≥ 0.
  - `CustomBlocks(blocks, minimum_week_separation)` — the model's
    `AmountForEveryArbitraryBlock`; `blocks` is a tuple of `WeekBlock`,
    `minimum_week_separation` ≥ 0.

All leaf values are frozen, constructible, with `==`/`hash`/`repr` and
`__match_args__`, like the caveat classes: a script names the value it expects
and compares. Construction validates ranges and rejects nonsense with
`ValueError` — the boundary of §6 applied one milestone early, so step 3 can
accept these objects as-is inside its dataclasses.

**Ranges are `(min, max)` tuples**, inclusive on both ends, min ≤ max, both
ints — `students_per_group=(2, 3)` is the design's own spelling in §14. The
model's `NonEmptyRangeInclusive` never leaks as a class.

**Optional text follows the model exactly**: a field the model types
`Option<NonEmptyString>` (tel, email, week annotation, group name) reads as
`str` or `None`, never `""`. A field the model types `String` (`Subject.name`,
`WeekPattern.name`, `Incompatibility.name`, `GroupList.name`,
`Slot.extra_info`, the export sheet names) reads as `str`, `""` allowed —
mirroring, not editorializing. The empty-string *rejections* of §3 are a write
boundary and belong to step 3.

**Durations and times**: `datetime.time` for times of day (whole minutes —
seconds and microseconds are always zero), plain int minutes for durations,
`datetime.date` for dates (the existing `first_week` precedent).

## 3. The read surface, collection by collection

Every collection is a frozen view holding only the document, like the existing
`Periods`: `doc.subjects` twice gives two interchangeable objects reading the
same document. None has a constructor. Unless said otherwise below, a
collection supports:

```python
len(c)              # how many entities
iter(c)             # handles, in the collection's order
c[id_or_handle]     # the handle; KeyError when it names nothing
c.get(id_or_handle) # the handle, or None
x in c              # membership, by (document, id); False for foreign ids
```

`c[handle]` and `handle in c` accept a handle for uniformity; a handle from
*another* document is simply not in this collection (`False` / `KeyError`),
whatever its id says.

Iteration orders mirror `docs/python/new_api_design.md` §4: **user order**
where the model keeps one (`periods`, `subjects`, weeks within a period, slots
within a subject), **id order** everywhere else.

### 3.1 `doc.periods` — and the `Period` handle

The existing `Periods` view keeps `first_week`, `set_first_week`,
`clear_first_week`, and grows the collection protocol above, iterating
`Period` handles in display order.

A period carries no data of its own in the model — the handle is pure
navigation:

| attribute | type | reads |
|---|---|---|
| `.id` | `PeriodId` | |
| `.index` | `int` | display position, 0-based |
| `.weeks` | `tuple[Week, ...]` | the period's weeks, in order |

### 3.2 `doc.weeks` — and the `Week` handle

Iterates in **global week order**: period display order, then position within
the period (the model's `walk_weeks`). `len` is the total week count.

| attribute | type | reads |
|---|---|---|
| `.id` | `WeekId` | |
| `.period` | `Period` | the owning period |
| `.index` | `int` | global index, 0-based, across all periods |
| `.interrogations` | `bool` | whether the week holds interrogations at all |
| `.annotation` | `str \| None` | the week's annotation |
| `.monday` | `datetime.date \| None` | `first_week + 7 × index` days; `None` when the document has no start date |

`.monday` is derived the way the xlsx export derives its week dates
(`xlsx/src/lib.rs`, `generate_week_dates_title`): weeks are consecutive from
the start date, in global order.

### 3.3 `doc.subjects` — and the `Subject` handle

Iterates in user order.

| attribute | type | reads |
|---|---|---|
| `.id` | `SubjectId` | |
| `.index` | `int` | display position, 0-based |
| `.name` | `str` | |
| `.interrogation` | `Interrogation \| None` | `None` when the subject holds no interrogations |
| `.excluded_periods` | `frozenset[Period]` | periods the subject does not run in |
| `.slots` | `tuple[Slot, ...]` | the subject's slots, in per-subject order |

`Interrogation` is a live sub-view bound to `(document, subject_id)`:

| attribute | type | reads |
|---|---|---|
| `.students_per_group` | `(int, int)` | |
| `.groups_per_interrogation` | `(int, int)` | |
| `.duration` | `int` | minutes |
| `.take_duration_into_account` | `bool` | |
| `.periodicity` | `Periodicity` | one of the four §2.6 values |

Accessing an `Interrogation` view after the subject was deleted, *or* after its
interrogations were switched off, raises `StaleHandleError`: in both cases the
thing the view was viewing is gone. `subject.interrogation` re-asked answers
the current truth.

### 3.4 `doc.teachers` — and the `Teacher` handle

Iterates in id order. `PersonWithContact` is flattened onto the handle — §14's
`TeacherData("Emmy", "Noether", subjects=...)` already commits the values side
to the flat shape:

| attribute | type | reads |
|---|---|---|
| `.id` | `TeacherId` | |
| `.surname` | `str` | |
| `.firstname` | `str` | |
| `.tel` | `str \| None` | |
| `.email` | `str \| None` | |
| `.subjects` | `frozenset[Subject]` | the subjects this teacher interrogates in |

### 3.5 `doc.students` — and the `Student` handle

Iterates in id order.

| attribute | type | reads |
|---|---|---|
| `.id` | `StudentId` | |
| `.surname` | `str` | |
| `.firstname` | `str` | |
| `.tel` | `str \| None` | |
| `.email` | `str \| None` | |
| `.excluded_periods` | `frozenset[Period]` | periods the student is absent from |

### 3.6 `doc.week_patterns` — and the `WeekPattern` handle

Iterates in id order.

| attribute | type | reads |
|---|---|---|
| `.id` | `WeekPatternId` | |
| `.name` | `str` | |
| `.excluded_weeks` | `frozenset[Week]` | the exception set: weeks the pattern switches off |

The merged answer — "is this week active under this pattern" — is the
document-level predicate, because it combines a week's own flag with a
pattern's exceptions:

```python
doc.is_week_active(week, pattern=None)   # -> bool
```

`pattern=None` asks about no pattern at all: only the week's own
`interrogations` flag counts (the meaning `week_pattern=None` has on a slot). A
dead `week` or `pattern` argument raises `StaleHandleError` (§2.4).

### 3.7 `doc.assignments`

The junction table: which students take which subject in which period. Its
reads are **total** over valid addresses — the model's canonical-sparse row is
invisible from Python:

```python
doc.assignments[period, subject]   # frozenset[Student], possibly empty
for period, subject, students in doc.assignments:   # stored rows, key order
    ...
```

The indexing never raises `KeyError` for a valid address: an absent row *is*
the empty frozenset. A `period` or `subject` that names nothing raises
`StaleHandleError` — the address was malformed, not empty. There is no `len`,
no `in`, no `.get`: over a total mapping, row count and row membership are
statements about the model's storage, not about the data, and a script has no
use for them. Iteration yields the stored (non-empty) rows as
`(Period, Subject, frozenset[Student])` triples, which is the whole content.

Whether a subject *runs* in a period is not this table's question —
`subject.excluded_periods` answers it.

### 3.8 `doc.slots` — and the `Slot` handle

Iterates all slots in subject-then-position **user order** — the model keeps no
single global slots order, so the walk composes the two user orders it does
keep: the subjects in `doc.subjects`' order, each followed by its own slots in
theirs. Per-subject access is `subject.slots`.

| attribute | type | reads |
|---|---|---|
| `.id` | `SlotId` | |
| `.index` | `int` | position within its subject's slots, 0-based |
| `.subject` | `Subject` | fixed at creation, per the design's write table |
| `.teacher` | `Teacher` | |
| `.weekday` | `Weekday` | |
| `.start_time` | `datetime.time` | whole minutes |
| `.extra_info` | `str` | possibly `""` — the model's plain `String` |
| `.week_pattern` | `WeekPattern \| None` | `None` = every week |
| `.cost` | `int` | > 0 avoid, < 0 favour |

A slot has no duration of its own — `slot.subject.interrogation.duration` is
where the model keeps it, and inventing a convenience alias would paper over
that.

The possibility oracle joins slots, weeks, subjects and patterns, so it lives
on the document like `is_week_active`:

```python
doc.is_interrogation_possible(slot, week)   # -> bool
```

True exactly when the GUI would draw that cell: the subject holds
interrogations, does not exclude the week's period, and the week is active
under the slot's pattern. Dead arguments raise `StaleHandleError`.

### 3.9 `doc.incompats` — and the `Incompat` handle

Iterates in id order.

| attribute | type | reads |
|---|---|---|
| `.id` | `IncompatId` | |
| `.name` | `str` | |
| `.subject` | `Subject` | deliberately not required to hold interrogations |
| `.slots` | `tuple[TimeSlot, ...]` | the busy slots, as §2.6 leaf values |
| `.minimum_free_slots` | `int` | ≥ 1 |
| `.week_pattern` | `WeekPattern \| None` | `None` = every week |

### 3.10 `doc.group_lists` — and the `GroupList` handle

Iterates in id order.

| attribute | type | reads |
|---|---|---|
| `.id` | `GroupListId` | |
| `.name` | `str` | |
| `.students_per_group` | `(int, int)` | |
| `.group_count` | `int` | `len(.group_names)` — the maximum number of groups |
| `.group_names` | `tuple[str \| None, ...]` | raw names; `None` = unnamed |
| `.is_prefilled` | `bool` | |
| `.groups` | `tuple[frozenset[Student], ...] \| None` | the prefilled groups; `None` for an automatic list |
| `.excluded_students` | `frozenset[Student] \| None` | students the automatic filling must skip; `None` for a prefilled list |

and one method:

```python
gl.group_name(i)    # str: the stored name, or "Groupe {i+1}" when unnamed
```

The fallback is the GUI's own (`gtk4/src/editor/colloscope.rs`: « Groupe 3 » or
« Groupe 3 : B2 » — the number always shows), so a script's output names groups
the way the application does. An index out of `range(group_count)` raises
`IndexError`.

`.groups` and `.excluded_students` answer `None` — not empty — when the
question does not apply to the list's filling kind: an automatic list *has* no
prefilled groups (its placements live in the colloscope, §3.14), and a
prefilled list has no exclusion set. Group *number* is the index into
`.group_names` / `.groups`, everywhere in this API and in the model.

The `(period, subject) → group list` associations live here, as the design
says:

```python
doc.group_lists.association_for(period, subject)   # GroupList | None
for period, subject, gl in doc.group_lists.associations():  # stored rows, key order
    ...
```

`association_for` is total over valid addresses like assignments reads: no
association is `None`, a dead address raises `StaleHandleError`.

### 3.11 `doc.pairings` — and the `PairingRule` handle

Iterates in id order. A pairing rule says: a student who `should_have` the
antecedent subject should (or should not) have the consequent one.

| attribute | type | reads |
|---|---|---|
| `.id` | `PairingRuleId` | |
| `.antecedent` | `PairingRuleSide` | |
| `.consequent` | `PairingRuleSide` | |
| `.excluded_periods` | `frozenset[Period]` | periods the rule does not apply to |
| `.soft` | `bool` | objective rather than hard constraint |

`PairingRuleSide` is a live sub-view bound to `(document, rule_id, side)`:
`.subject` (a `Subject` handle) and `.should_have` (`bool`). It goes stale with
its rule.

### 3.12 `doc.slot_pairings` — and the `SlotPairingRule` handle

Structurally §3.11 with slots: `.id`, `.antecedent` / `.consequent`
(`SlotPairingRuleSide` sub-views with `.slot` and `.should_have`),
`.excluded_periods`, `.soft`.

### 3.13 `doc.settings` and `doc.balancing`

Two singleton views, each a global entry plus sparse per-entity overrides, with
the same whole-entry resolution: an override replaces the global **verbatim** —
a `None` field in an override *disables* the corresponding global limit, it
does not inherit it. That semantic stays in Rust (`Settings::limits_for`,
`Balancing::options_for`); Python only ever sees resolved or raw entries,
never a merge it could get wrong.

```python
doc.settings.global_limits           # Limits — the global entry
doc.settings.limits_for(student)     # Limits — what applies to this student (resolved)
doc.settings.override_for(student)   # Limits | None — the raw override, if one is set
doc.settings.overrides()             # tuple[(Student, Limits), ...] in id order
```

`Limits` is a live sub-view: `.interrogations_per_week_min`,
`.interrogations_per_week_max`, `.max_interrogations_per_day`, each
`Limit | None` (§2.6) — `None` meaning the limit is not set at all.
`limits_for` is live in the strong sense: the view re-resolves on every read,
so it tracks an override appearing or vanishing. `override_for`'s view is bound
to the override entry and goes stale when the override is removed; both go
stale when the student does. `global_limits` can never go stale.

```python
doc.balancing.global_options         # BalancingOptions
doc.balancing.options_for(subject)   # BalancingOptions — resolved
doc.balancing.override_for(subject)  # BalancingOptions | None
doc.balancing.overrides()            # tuple[(Subject, BalancingOptions), ...] in id order
```

`BalancingOptions` is a live sub-view: `.teacher_rotation`, `.slot_rotation`,
`.avoid_twice_in_a_row`, each `Enforcement | None` (`None` = not pursued,
`OBJECTIVE` = optimize for it, `STRICT` = hard constraint — the model's
three-state `Option<SoftParam<()>>`), plus `.year_teacher_rotation` and
`.period_teacher_rotation`, both `bool`.

### 3.14 `doc.colloscope`

The result data: two sparse tables, read exactly as the design's §4 sketches.

```python
doc.colloscope.interrogation(slot, week)   # frozenset[int] | None
for slot, week, groups in doc.colloscope.interrogations():   # stored cells, key order
    ...
doc.colloscope.group_list(gl)              # Mapping[Student, int] | None (read-only)
for gl, placements in doc.colloscope.group_lists():
    ...
```

- `interrogation` returns the assigned group numbers — indices into the
  associated group list, the `(period, subject) → group list` hop being
  `doc.group_lists.association_for` — or `None` when nothing is scheduled
  there. (The model's canonical form makes "empty cell" unrepresentable, so
  `None` is the single absent answer.) Dead `slot`/`week` arguments raise
  `StaleHandleError`; pair reads with `doc.is_interrogation_possible` to know
  whether a cell *could* hold anything.
- `group_list` returns the student → group-number placements the solver (or a
  script) chose for an **automatic** group list, as a read-only mapping
  (`MappingProxyType` with `Student` handle keys), or `None` when the document
  holds no placements for that list. Prefilled lists never appear here — their
  groups are `gl.groups`.
- The two iterators yield the stored rows in the model's key order.

### 3.15 `doc.export_config`

A singleton tree of live sub-views mirroring `ExportConfig` field by field.
Nothing here can go stale; it is pure value data with the whole-struct write
landing in step 3.

```python
doc.export_config.colloscope_enabled          # bool — and the four other *_enabled flags:
                                              # all_groups, automatic_groups,
                                              # prefilled_groups, per_group_list
doc.export_config.global_config               # ExportGlobalConfig
doc.export_config.colloscope_config           # ExportColloscopeConfig
doc.export_config.all_groups_config           # ExportStudentGroupsConfig
doc.export_config.automatic_groups_config     # ExportStudentGroupsConfig
doc.export_config.prefilled_groups_config     # ExportStudentGroupsConfig
doc.export_config.per_group_list_config       # ExportGroupListConfig
```

- `ExportGlobalConfig`: `.background_color` (`Color`),
  `.stripes_color_enabled` (`bool`), `.stripes_color` (`Color`).
- `ExportColloscopeConfig`: `.sheet_name` (`str`),
  `.extra_info_column_enabled` / `.extra_info_column_name`,
  `.teacher_email_enabled` / `.teacher_email`,
  `.teacher_tel_enabled` / `.teacher_tel`, `.orientation` (`Orientation`),
  `.display_week_dates`, `.display_annotations`,
  `.no_interrogation_color` (`Color`),
  `.annotation_color_enabled` / `.annotation_color` (`Color`),
  `.extra_colors` (read-only `Mapping[str, Color]`).
- `ExportStudentGroupsConfig`: `.sheet_name` (`str`),
  `.orientation` (`Orientation | None` — `None` = auto-detect from group
  count), `.show_emails`, `.show_tel`.
- `ExportGroupListConfig`: `.orientation` (`Orientation`), `.show_emails`,
  `.show_tel`, `.center_vertically`.

The enabled flags sit *beside* the configs they gate, not inside them —
mirroring the model, whose flags are the interface's memory of what was chosen
before a section was switched off (`docs/python/new_api_design.md` §11.2).

## 4. `referenced_by()` — what points at an entity

Every handle has it:

```python
sites = subject.referenced_by()   # tuple[RefSite, ...], in the registry's walk order
```

It rides `InnerData::references_to_*` and answers the question a script asks
before a remove: *what will a cascade touch?* An empty tuple means nothing
points here. `Incompat`, `PairingRule` and `SlotPairingRule` are never the
target of a reference (the registry has no site vocabulary for them), so their
`referenced_by()` is always `()` — present for uniformity, and documented as
constantly empty.

A site is a frozen value class under an abstract base `RefSite`, carrying **the
full coordinates of the referring place** as handle attributes — target
included, unlike the Rust site enums, which omit the target because their
context implies it. Full coordinates cost nothing and let one class serve every
target kind that can appear in the same place: `AssignmentRow(period, subject)`
is a site for a period, for a subject, and for each student in the row.

The vocabulary, one class per referring place:

| class | attributes | referring place |
|---|---|---|
| `WeekPeriod` | `.week` | a week's owning period |
| `SubjectExcludedPeriod` | `.subject` | a subject's exclusion set |
| `StudentExcludedPeriod` | `.student` | a student's exclusion set |
| `PairingRuleExcludedPeriod` | `.rule` | a pairing rule's exclusion set |
| `SlotPairingRuleExcludedPeriod` | `.rule` | a slot pairing rule's exclusion set |
| `AssignmentRow` | `.period`, `.subject` | an assignments row (key or student member) |
| `GroupListAssociation` | `.period`, `.subject` | an association entry (key or its group list) |
| `TeacherSubject` | `.teacher` | a teacher's subject set |
| `SlotSubject` | `.slot` | a slot's subject |
| `SlotTeacher` | `.slot` | a slot's teacher |
| `SlotWeekPattern` | `.slot` | a slot's week pattern |
| `IncompatSubject` | `.incompat` | an incompatibility's subject |
| `IncompatWeekPattern` | `.incompat` | an incompatibility's week pattern |
| `PairingRuleAntecedent` | `.rule` | a pairing rule's antecedent subject |
| `PairingRuleConsequent` | `.rule` | a pairing rule's consequent subject |
| `SlotPairingRuleAntecedent` | `.rule` | a slot pairing rule's antecedent slot |
| `SlotPairingRuleConsequent` | `.rule` | a slot pairing rule's consequent slot |
| `SettingsOverride` | `.student` | a per-student limits override entry |
| `BalancingOverride` | `.subject` | a per-subject balancing override entry |
| `WeekPatternExcludedWeek` | `.week_pattern` | a pattern's excluded-week set |
| `GroupListPrefilledStudent` | `.group_list` | a student inside a prefilled group |
| `GroupListExcludedStudent` | `.group_list` | an automatic list's exclusion set |
| `ColloscopeInterrogation` | `.slot`, `.week` | a colloscope interrogation cell |
| `ColloscopeGroupListRow` | `.group_list` | a colloscope placements row (key or a placed student) |

Each is constructible from handles (or ids), with `==`/`hash`/`repr` and
`__match_args__`, so a script writes
`clm.SlotTeacher(slot) in teacher.referenced_by()` or matches on the class.
Like the Rust registry, the unit is the id *occurrence*: a subject referenced
by an assignments row yields one `AssignmentRow` site, and each student in that
row yields the same-shaped site from *their* `referenced_by()`.

## 5. What deliberately does not exist

- **No `to_data()` yet** — it returns a value dataclass, and those are step 3.
  Adding it here would mean landing the dataclasses without the writes they
  exist for.
- **No positional indexing** (`doc.subjects[0]`): indexing is by id/handle
  only, per the design. Order is reachable through iteration and `.index`.
- **No reverse-navigation conveniences** beyond what the model derives
  (`teacher.slots`, `period.subjects`, …): `referenced_by()` is the one
  reverse door, and it is exact. Conveniences can be argued for one by one
  later; none is load-bearing for the contract scripts.
- **No `doc.colloscope` emptiness flags** (`are_interrogations_empty`, …):
  derivable from the iterators in one line.
- **No exposure of the undo category, the id issuer, or any storage detail** —
  same reasoning as `undo_name`'s docstring.

---

## 6. Implementation plan

### 6.1 Crate layout

New files in `python/src/` (the `foo.rs` + `foo/` convention, never `mod.rs`):

- `ids.rs` — the eleven id classes, generated by one `macro_rules!` (they are
  uniform by design; hand-writing eleven copies invites drift).
- `handles.rs` — the shared plumbing: the borrow-resolve-read helper the handle
  classes use, the `StaleHandleError` constructors that name kind and id, and
  the id-or-handle argument extraction implementing §2.4.
- `values.rs` — the leaf values and enums of §2.6 (`Weekday`, `Enforcement`,
  `Orientation`, `Limit`, `Color`, `TimeSlot`, `WeekBlock`, `Periodicity` and
  its four subclasses).
- `refs.rs` — `RefSite` and the 24 site classes of §4.
- `collections/` — one file per collection, each holding the collection view,
  its handle class(es) and their sub-views: `periods.rs` (existing, grows
  `Period`), `weeks.rs`, `subjects.rs`, `teachers.rs`, `students.rs`,
  `week_patterns.rs`, `assignments.rs`, `slots.rs`, `incompats.rs`,
  `group_lists.rs`, `pairings.rs`, `slot_pairings.rs`, `settings.rs`,
  `balancing.rs`, `colloscope.rs`, `export_config.rs`.

Every class is registered in `lib.rs` the way the existing ones are, so
`isinstance` and `repr` work; none of the new classes has a constructor except
the §2.6 leaf values and the §4 sites.

### 6.2 Testing approach

The existing harness (`python/tests/module.rs` + `tests/scripts/*.py`) carries
the whole milestone: scripts read documents and leave what they saw in globals;
Rust asserts against the same `InnerData` read directly.

Two fixtures:

- **`examples/hogwarts.collomatique`** (copied per test, as today) for
  everything it covers — periods, weeks, subjects, teachers, students,
  patterns, slots, incompatibilities, group lists and their associations,
  assignments, slot pairing rules, and settings and balancing overrides.
- **Synthetic documents built in Rust** for what it lacks (the four
  periodicity kinds, subject pairing rules, a filled colloscope, a
  non-default export config, a prefilled group list, …): construct an `InnerData` directly — the sealed types through
  their constructors — pass it through `Data::from_inner_data` (so a fixture
  that breaks an invariant fails loudly in the test, not in the API), write it
  with `serialize_data`, and let the script `clm.load` it. The fixture builder
  is a test-side helper in `module.rs`, not crate API.

**Staleness needs a mutation the script cannot make yet** — the read milestone
ships no removes. The ops layer has them all, so the harness grows one helper:
`run_stages(&[script1, script2, …], between)`, which runs the stage scripts in
one namespace and calls a Rust closure between stages. Stage 1 leaves a handle
in the globals; the closure extracts the `Py<Document>` and applies a real
`UpdateOp` remove through `Document::update`; stage 2 asserts
`StaleHandleError` on attribute access, `repr` saying `(stale)`, `==`/`hash`
still working, and the mapping conventions (`get` → `None`, `in` → `False`,
`[...]` → `KeyError`). This is the mechanism's proper pin, and each later
commit reuses it for its own entity kind where the staleness has structure
(the `Interrogation` view going stale on switch-off, an `override_for` view on
override removal).

Every commit below lands with its tests, compiles, and passes the suite on its
own. Test-first commit splitting (regression test before fix) does not apply —
these are new features, not bug fixes.

### 6.3 The commits

Dependency order: a handle class only lands once every class its attributes
return already exists. Where a class gains an attribute in a *later* commit
(`Subject.slots` cannot exist before `Slot`), the earlier commit's docstring
does not mention it and the design stays this document — the intermediate
states are construction stages, not published shapes.

1. **`python: opaque ids, stale handles, and the periods and weeks read
   surface`** — `ids.rs` (all eleven classes: the macro is uniform, so they
   cost nothing to land together and every later commit can hand them out),
   `StaleHandleError` in `errors.rs`, `handles.rs` plumbing. `Periods` grows
   the collection protocol and the `Period` handle (`.id`, `.index`,
   `.weeks`); new `collections/weeks.rs` with `Weeks` (`doc.weeks`) and the
   `Week` handle, `.monday` included. Tests: iteration orders and every
   attribute against hogwarts; id semantics (eq, hash, ordering, cross-kind
   `TypeError`, no constructor, no `int()`); the first `run_stages` staleness
   test, using `RemoveWithWeeks` to kill a period and its weeks in one blow.

2. **`python: the subjects read surface`** — `values.rs` opens with the
   periodicity family, `WeekBlock`, and range-tuple conversion;
   `collections/subjects.rs` with `Subjects`, `Subject` (without `.slots`,
   which waits for commit 5) and the `Interrogation` sub-view. Tests: hogwarts
   subjects in user order; a synthetic document holding all four periodicity
   kinds, read back and compared value by value (also pinning leaf-value
   construction and `==`); staleness of both the handle and the sub-view,
   including the switch-off case via a subject `Update` op.

3. **`python: the teachers and students read surface`** —
   `collections/teachers.rs`, `collections/students.rs`. Tests: flattened
   person fields with `None` (not `""`) for absent tel/email pinned against a
   synthetic document that has both shapes; `.subjects` / `.excluded_periods`
   as frozensets of live handles; membership tests with handles from another
   document answering `False`.

4. **`python: the week patterns read surface, and which weeks are active`** —
   `collections/week_patterns.rs`, `doc.is_week_active`. Tests: a synthetic
   pattern with exclusions; the predicate against the model's own answers over
   every (week, pattern) pair including `pattern=None`; a dead argument
   raising `StaleHandleError` where the model would have shrugged `false`.

5. **`python: the slots read surface, and when interrogations are
   possible`** — `Weekday` in `values.rs`, `collections/slots.rs` (`Slots` in
   subject-then-position order, the `Slot` handle, `datetime.time`
   conversion), `Subject.slots`, `doc.is_interrogation_possible`. Tests:
   hogwarts slots; per-subject order agreeing between `doc.slots`,
   `subject.slots` and `.index`; the oracle compared exhaustively against
   `Parameters::is_interrogation_possible` over hogwarts.

6. **`python: the assignments read surface`** — `collections/assignments.rs`.
   Tests: a stored row and an absent row both reading as frozensets (empty for
   absent); row iteration matching the model's key order; dead-address
   `StaleHandleError`; handle-vs-id argument interchangeability.

7. **`python: the incompatibilities read surface`** — `TimeSlot` in
   `values.rs`, `collections/incompats.rs`. Tests: hogwarts's six
   incompatibilities read back; `TimeSlot` construction validating what
   `SlotWithDuration::new` validates, `ValueError` on refusal.

8. **`python: the group lists read surface`** —
   `collections/group_lists.rs`: `GroupLists`, `GroupList`, `group_name`,
   `association_for`, `associations()`. Tests: a prefilled and an automatic
   list side by side (the `None`-for-inapplicable rule); `group_name`'s
   fallback matching the GUI's wording; associations total reads and row
   iteration.

9. **`python: the pairing rules read surface`** — `collections/pairings.rs`
   and `collections/slot_pairings.rs`, with the two side sub-views. Tests:
   hogwarts's two slot pairing rules; synthetic subject pairing rules (the
   example has none), both `should_have` polarities, `.soft` both ways;
   sub-views staling with their rule.

10. **`python: the settings and balancing read surface`** — `Enforcement` and
    `Limit` in `values.rs`, `collections/settings.rs`,
    `collections/balancing.rs`. Tests: the verbatim whole-entry override
    semantics pinned from Python (an override with a `None` field masking a
    set global — the exact case the Rust tests pin); `limits_for` tracking an
    override appearing and vanishing across `run_stages`; `override_for`'s
    view going stale on override removal; the `Enforcement` three-state on
    balancing.

11. **`python: the colloscope read surface`** — `collections/colloscope.rs`.
    Tests: a synthetic document with a filled colloscope (the example ships
    none), cells compared against the model; `None` for an empty cell;
    placements as a read-only mapping (mutating it raises `TypeError`);
    every stored cell agreeing with `is_interrogation_possible`.

12. **`python: the export config read surface`** — `Color` and `Orientation`
    in `values.rs`, `collections/export_config.rs` with the four sub-config
    views. Tests: a synthetic non-default config read back field by field,
    `extra_colors` as a read-only mapping, the `orientation=None` auto-detect
    case.

13. **`python: what points at an entity`** — `refs.rs` with `RefSite` and the
    site classes, `referenced_by()` on all eleven handles. Tests: one
    synthetic document holding at least one edge of every site class; each
    kind's `referenced_by()` compared against `references_to_*` mapped through
    the site conversion; the three never-referenced kinds answering `()`;
    site `==` and matching from Python.

Thirteen commits; each leaves the suite green and none changes a surface a
previous commit published. After commit 13 the read surface is complete, and
step 3 (the write surface and the value dataclasses) starts from a document
whose every entity can already be reached, inspected and cross-referenced from
Python.
