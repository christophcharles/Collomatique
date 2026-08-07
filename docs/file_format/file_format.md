# The Collomatique file format

This document specifies the Collomatique colloscope file format, **spec version 2**.
It is self-contained: it describes the format in the abstract, so that anyone can
write an independent loader, exporter or tool from this document alone.

## 1. Overview

A Collomatique file is a single JSON document encoded in UTF-8. It is a
**snapshot**: it stores the complete state of one colloscope project — planning
parameters, the colloscope itself, and the export configuration — and nothing else.
There is no undo history and no derived data; anything fully determined by other
stored data is reconstructed by readers rather than stored.

The document has a small **envelope** (§2) wrapping a list of independent
**blocks** (§4). Each block describes one aspect of the project and carries its own
versioning information, which is what makes the format evolvable (§5).

### Versioning

Two version numbers coexist, with different roles:

- `produced_with_version`, in the header, is the version of the application that
  wrote the file, as a Semantic Versioning 2.0.0 string. It is **informational
  only**: it never decides whether a file can be read.
- `minimum_spec_version`, on each block, is the format spec revision needed to
  understand that block, together with a `needed_entry` flag saying whether the
  document is meaningful without it. These two fields are what actually gate
  readability (§2, §5).

The spec version described by this document is **2**. The value **1 is permanently
retired**: it belonged to a pre-release format, and no valid file contains a block
declaring `minimum_spec_version: 1`. Spec numbering for this format effectively
starts at 2.

## 2. Envelope

The top-level document is:

```json
{
  "header": {
    "file_type": "Collomatique",
    "produced_with_version": "0.1.0-alpha.0.99",
    "file_content": "Colloscope"
  },
  "entries": [
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": { "<BlockName>": <block payload> }
    }
  ]
}
```

### Header

| Field | Type | Meaning |
|---|---|---|
| `file_type` | string | Always `"Collomatique"`. |
| `produced_with_version` | version string | Version of the writing application. Informational. |
| `file_content` | string | Kind of document. This document specifies the `"Colloscope"` kind. A reader that does not recognise the value must refuse the file. |

A reader may open a file whose `produced_with_version` is newer than itself, and
should let the user know it was produced by a newer application. Versions are
compared with semver precedence, so a prerelease sorts **below** its own release:
`0.1.0-alpha.0.99` is older than `0.1.0`.

### Blocks

Each element of `entries` is `{minimum_spec_version, needed_entry, content}`, where
`content` is an object with **exactly one key** — the block name — whose value is
the block's payload.

Spec 2 defines sixteen block names (§4):

`GeneralPlanning`, `Subjects`, `Teachers`, `Students`, `Assignments`,
`WeekPatterns`, `Slots`, `Incompatibilities`, `GroupLists`,
`GroupListAssociations`, `Pairings`, `SlotPairings`, `Settings`, `Balancing`,
`Colloscope`, `ExportConfig`.

For each of them the canonical envelope values are `minimum_spec_version: 2` and
`needed_entry: true`; a spec-2 block declaring different values is invalid.

Rules for the block list:

- **A block name appears at most once.** A duplicated block name is invalid.
- **A block may be absent.** An absent block means the block's **default state**,
  which is specified explicitly for every block in §4. Consequently,
  `"entries": []` is a valid document: a blank project, with every block in its
  default state.
- **Block order is not significant.** Readers identify blocks by name. (The
  canonical order is the list above; see §3, *Canonical form*.)
- **Unrecognised blocks** are handled by the forward-compatibility rules of §5.

## 3. Conventions

### Identifiers

Persistent objects — periods, subjects, teachers, students, week patterns, slots,
incompatibilities, group lists, pairing rules, slot pairing rules — are identified
by a bare JSON number holding a non-negative integer **at most 2⁶³ − 1**; larger
values make the file invalid. Ids are drawn from a **single
global id space**: an id value appears at most once across the whole file,
regardless of object kind. A subject and a teacher can never share id `3`.
Duplicate ids anywhere make the file invalid.

Nothing else is significant about id values: they need not be dense, ordered, or
small. No id counter is stored. Writers should nevertheless strive to keep ids
within the 32-bit range: small ids are easier on human readers, and we avoid
potential bugs if the IDs exceed 2^63 while the application is running.

Readers may reserve id headroom above the largest defining id (for synthesized
ids); writers should stay far below the 2⁶³ − 1 ceiling — the 32-bit guidance
above makes the reservation invisible in practice.

### Scalar encodings

| Value | JSON encoding | Constraints |
|---|---|---|
| id | number | non-negative integer ≤ 2⁶³ − 1, globally unique (see above) |
| week start | string `"YYYY-MM-DD"` | an ISO date that must be a Monday; anything else is invalid |
| time of day | string `"HH:MM"` | 24-hour, zero-padded, minute precision; `"9:00"` or `"09:00:00"` are invalid |
| weekday | string | lowercase English: `"monday"` … `"sunday"` |
| duration | number | integer minutes, 1 to 2³² − 1 |
| integer range | record `{"min": n, "max": n}` | `min <= max` |
| color | record `{"red": n, "green": n, "blue": n}` | each 0–255 |
| non-empty string | string | the empty string is invalid where "non-empty" is stated |
| version | string | Semantic Versioning 2.0.0: `MAJOR.MINOR.PATCH`, with an optional `-prerelease` and an optional `+build`; anything else is invalid |
| string | string | may be empty unless stated otherwise |

### Integer widths

Every integer field has a fixed width, and a value outside it makes the file
invalid. Ids have their own ceiling of 2⁶³ − 1 (above). Apart from ids, every
unsigned integer field must fit in 32 bits: 0 to 2³² − 1, or 1 to 2³² − 1 where
a minimum of 1 is stated. The single signed integer field — a slot's `cost`
(§4.7) — must fit in a signed 32-bit integer: −2³¹ to 2³¹ − 1. This covers,
among others, durations, week indices, group numbers, limit values, periodicity
parameters, and the envelope's `minimum_spec_version`.

### Records and keyed collections

The format distinguishes two composite shapes, with opposite strictness rules:

- A **record** is an object with a fixed set of fields (an assignment row, a slot,
  a limits object…). **Every field is always present.** A record with a missing
  field or an unknown field is invalid. Optional values are written as `null`,
  never omitted.
- A **keyed collection** is an array of items identified by a key — an id, or a
  combination like `(period_id, subject_id)`. Keyed collections are **sparse**: any
  subset of keys may be present, and an absent key means the neutral state for that
  key (stated per block in §4). A duplicated key is invalid. The block list itself
  (§2) is the outermost keyed collection, with block names as keys and the blocks'
  default states as neutral states.

Some keyed collections have a **derived key set**: the set of meaningful keys is
fully determined by other data (for instance, assignments have exactly one
meaningful key per period × subject pair). Keys outside that set are invalid. In
such collections, presence of a key carries no information by itself, so an entry
whose content equals the neutral state (an assignment row with no students, a slot
row with no slots…) encodes exactly the same state as its absence: it is **valid
but redundant**, and the canonical form omits it. §4 marks these collections
explicitly. In all other keyed collections the key set is free — an entry existing
*is* information — and entries are written exactly as they exist, whatever their
content.

### Enums

Values that are one of several named variants are encoded as:

- a bare string when the variant carries no data: `"Portrait"`, `"Landscape"`;
- an object with exactly one key — the variant name — otherwise:
  `{"ExactlyPeriodic": {"periodicity_in_weeks": 2}}`.

Soft parameters — values that can be enforced strictly or used as an optimisation
goal — are records `{"soft": bool, "value": ...}`; when the parameter carries no
value, the `value` field is dropped and the record is `{"soft": bool}`. An optional
soft parameter is `null` or that record.

### Ordering

Two kinds of arrays:

- **Order-significant arrays**, whose order is part of the state: `periods`,
  `Subjects`, the per-subject `slots` arrays, `group_names`, prefilled `groups`,
  incompatibility `slots`, and periodicity `blocks`, as well as all positional
  `weeks` arrays. Reordering them changes the document's meaning.
- **Unordered collections** (every keyed collection): readers must accept any
  order; only key uniqueness matters.

### Canonical form

Many equivalent serialisations of the same state are valid: blocks in default state
may be written or omitted, unordered collections may appear in any order, and JSON
whitespace is free. The **canonical form** of a document — recommended for writers,
and what the Collomatique application produces — is:

- blocks in default state are **omitted**; a document only contains the features
  actually used (this also maximises how far back the file remains readable, §5);
- in collections with a derived key set, entries in neutral state are **omitted**;
- present blocks appear in the canonical name order of §2;
- unordered collections are sorted: object rows by `id`, association rows by their
  full key ascending (e.g. `(period_id, subject_id)`, `(slot_id, week)`), id sets
  and group-number sets ascending, named entries (`extra_colors`) by name
  (by Unicode code point);
- the JSON is pretty-printed with 2-space indentation.

In canonical form, one state has exactly one byte sequence.

### Week coordinates

The schedule is a concatenation of periods; each period is a list of weeks. The
**global week index** is the 0-based position of a week in that concatenation
(period order is significant, so global week numbering is well defined). Wherever
the format stores a per-week value outside `GeneralPlanning` — week-pattern `weeks`
arrays, colloscope `week` fields — it uses global week indices.

## 4. Blocks

For each block: its purpose, its **default state** (the meaning of the block's
absence), its payload shape, and its validity constraints. A file violating any
constraint is invalid.

### 4.1 `GeneralPlanning`

The period structure and start date. Payload: record.

**Default:** `{"first_week": null, "periods": []}` — no start date, no periods.

```json
{
  "first_week": "2026-08-31",
  "periods": [
    {
      "id": 1,
      "weeks": [
        { "interrogations": true, "annotation": "Rentrée" },
        { "interrogations": false, "annotation": null }
      ]
    }
  ]
}
```

| Field | Type | Meaning |
|---|---|---|
| `first_week` | week start or `null` | Monday of global week 0. `null` if not set. |
| `periods` | array, **order-significant** | The periods in user order. Their concatenated `weeks` arrays define global week numbering. |
| `periods[].id` | id | Period id. |
| `periods[].weeks` | array, positional | One record per week of the period. |
| `weeks[].interrogations` | bool | `false` marks a week with no interrogations at all (holidays, exams). |
| `weeks[].annotation` | non-empty string or `null` | Free label displayed on exports. |

Constraints: a period may have zero weeks.

### 4.2 `Subjects`

Payload: array, **order-significant** (user order).

**Default:** `[]` — no subjects.

```json
[
  {
    "id": 2,
    "name": "Mathématiques",
    "interrogation_parameters": {
      "students_per_group": { "min": 2, "max": 3 },
      "groups_per_interrogation": { "min": 1, "max": 1 },
      "duration_minutes": 60,
      "take_duration_into_account": true,
      "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
    },
    "excluded_periods": []
  },
  {
    "id": 3,
    "name": "Sport",
    "interrogation_parameters": null,
    "excluded_periods": [1]
  }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Subject id. |
| `name` | string | Descriptive name (may be empty). |
| `interrogation_parameters` | record or `null` | `null` means the subject has **no interrogations** (it still exists for assignments). |
| `excluded_periods` | array of period ids | Periods the subject does not run on. By default a subject runs on every period. |

`interrogation_parameters` fields:

| Field | Type | Meaning |
|---|---|---|
| `students_per_group` | range of integers ≥ 1 | Students per group for this subject (can differ from the group list's own range). |
| `groups_per_interrogation` | range of integers ≥ 1 | How many groups can share one interrogation slot. |
| `duration_minutes` | duration | Duration of one interrogation. |
| `take_duration_into_account` | bool | Whether this duration counts toward per-week/day time limits. |
| `periodicity` | enum | See below. |

`periodicity` variants:

- `{"OnceForEveryBlockOfWeeks": {"weeks_per_block": ≥1, "minimum_week_separation": ≥1}}`
  — one interrogation per fixed-size block; blocks tile the schedule from week 0.
  `minimum_week_separation` cannot be 0 (at most one interrogation per block already
  forbids two in the same week).
- `{"ExactlyPeriodic": {"periodicity_in_weeks": ≥1}}` — strict periodicity: an
  interrogation on week *w* forces the next on week *w + p*.
- `{"AmountInYear": {"interrogation_count_in_year": {"min": ≥0, "max": n}, "minimum_week_separation": ≥0}}`
  — fixes only the yearly total (as a range); `0` separation allows two in one week.
- `{"AmountForEveryArbitraryBlock": {"blocks": [...], "minimum_week_separation": ≥0}}`
  — generalisation of `OnceForEveryBlockOfWeeks` with explicit, possibly irregular
  blocks. `blocks` is order-significant; each block is
  `{"delay_in_weeks": ≥0, "size_in_weeks": ≥1, "interrogation_count_in_block": {"min": ≥0, "max": n}}`
  where `delay_in_weeks` counts weeks since the end of the previous block (or since
  week 0 for the first). Zero blocks is representable (and means no interrogations
  can be scheduled). Blocks may extend past the end of the schedule.

Constraints: every id in `excluded_periods` is an existing period; all ranges
satisfy `min <= max`.

### 4.3 `Teachers`

Payload: keyed collection (by `id`).

**Default:** `[]` — no teachers.

```json
[
  {
    "id": 4,
    "surname": "Rogue",
    "firstname": "Severus",
    "tel": "0605060708",
    "email": null,
    "subjects": [2]
  }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Teacher id. |
| `surname`, `firstname` | string | May be empty. |
| `tel`, `email` | non-empty string or `null` | Contact info, used only for exports. |
| `subjects` | array of subject ids | Subjects the teacher can interrogate in. |

Constraints: every id in `subjects` is an existing subject **and** that subject has
interrogations (`interrogation_parameters` is not `null`).

### 4.4 `Students`

Payload: keyed collection (by `id`).

**Default:** `[]` — no students.

```json
[
  {
    "id": 5,
    "surname": "Granger",
    "firstname": "Hermione",
    "tel": null,
    "email": "hermione@poudlard.fr",
    "excluded_periods": [1]
  }
]
```

Same person fields as teachers. `excluded_periods` (array of period ids) lists
periods the student does not attend at all.

Constraints: every id in `excluded_periods` is an existing period.

### 4.5 `Assignments`

Which students take which subject on which period. Payload: keyed collection of
association rows, keyed by `(period_id, subject_id)`, with a **derived key set**:
the meaningful keys are exactly the (period × subject not excluded from that
period) pairs — including subjects without interrogations.

**Default:** `[]` — no student is assigned to anything.

```json
[
  { "period_id": 1, "subject_id": 2, "students": [5, 6] }
]
```

An absent row means no students are assigned for that pair. A row whose `students`
array is empty encodes the same thing — it is valid but redundant, and omitted in
canonical form.

Constraints: `period_id` is an existing period; `subject_id` is an existing subject
not excluded from that period; each student in `students` exists and is not
excluded from the period.

### 4.6 `WeekPatterns`

Named week masks used by slots and incompatibilities. Payload: keyed collection
(by `id`).

**Default:** `[]` — no week patterns.

```json
[
  { "id": 7, "name": "Quinzaine A", "weeks": [true, false, true, false, true, false, true] }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Week pattern id. |
| `name` | string | Display name. |
| `weeks` | array of bool, positional | `weeks[w]` = pattern active on **global week** `w`. |

Constraints: `weeks` has exactly one element per week of the schedule (the sum of
all period lengths in `GeneralPlanning`) — no shorter, no longer.

### 4.7 `Slots`

Interrogation slots, grouped by subject. Payload: keyed collection of per-subject
rows, keyed by `subject_id`, with a **derived key set**: the meaningful keys are
exactly the subjects with interrogations. Each row's inner `slots` array is
**order-significant** (user order).

**Default:** `[]` — no subject has any slots.

```json
[
  {
    "subject_id": 2,
    "slots": [
      {
        "id": 8,
        "teacher_id": 4,
        "start": { "day": "monday", "time": "14:00" },
        "extra_info": "Salle 101",
        "week_pattern_id": 7,
        "cost": 0
      }
    ]
  }
]
```

An absent row means the subject has no slots. A row whose `slots` array is empty
encodes the same thing — valid but redundant, omitted in canonical form.

Slot fields:

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Slot id. |
| `teacher_id` | id | The interrogating teacher. |
| `start` | record `{"day", "time"}` | Weekday + start time. The duration comes from the subject's `duration_minutes`. |
| `extra_info` | string | Free info for exports (room number…). May be empty. |
| `week_pattern_id` | id or `null` | `null` = the slot exists every week. |
| `cost` | signed 32-bit integer | Solver preference: positive avoids the slot, negative favours it, 0 neutral. |

Constraints: `subject_id` is an existing subject with interrogations; `teacher_id`
exists and that teacher's `subjects` contains this subject; `week_pattern_id` (when
non-null) exists; `start` plus the subject's duration must not cross midnight.

### 4.8 `Incompatibilities`

Recurring external commitments (e.g. an optional course) that make students
unavailable. Payload: keyed collection (by `id`).

**Default:** `[]` — no incompatibilities.

```json
[
  {
    "id": 9,
    "subject_id": 2,
    "name": "Option latin",
    "slots": [
      { "day": "monday", "time": "08:00", "duration_minutes": 60 },
      { "day": "thursday", "time": "10:00", "duration_minutes": 90 }
    ],
    "minimum_free_slots": 1,
    "week_pattern_id": null
  }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Incompatibility id. |
| `subject_id` | id | The subject this incompatibility is linked to. |
| `name` | string | Display name. |
| `slots` | array, **order-significant** | Time slots when students may be unavailable; each a record `{"day", "time", "duration_minutes"}`. |
| `minimum_free_slots` | integer ≥ 1 | How many of `slots` must be kept free. |
| `week_pattern_id` | id or `null` | `null` = applies every week. |

Constraints: `subject_id` exists; `week_pattern_id` (when non-null) exists; each
slot must not cross midnight.

### 4.9 `GroupLists`

The group lists themselves (their association to subjects is the separate
`GroupListAssociations` block). Payload: keyed collection (by `id`).

**Default:** `[]` — no group lists.

```json
[
  {
    "id": 10,
    "name": "Groupes de maths",
    "students_per_group": { "min": 2, "max": 3 },
    "group_names": ["Gryffondor", null],
    "filling": {
      "Prefilled": {
        "groups": [
          { "students": [5, 6] },
          { "students": [] }
        ]
      }
    }
  },
  {
    "id": 11,
    "name": "Groupes de physique",
    "students_per_group": { "min": 1, "max": 2 },
    "group_names": [null, null, null],
    "filling": { "Automatic": { "excluded_students": [6] } }
  }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Group list id. |
| `name` | string | Display name. |
| `students_per_group` | range of integers ≥ 1 | Allowed group size. |
| `group_names` | array of (non-empty string or `null`), **order-significant** | One element per group; its length **is** the group count. `null` = unnamed group. Group numbers used elsewhere are 0-based indices into this array. |
| `filling` | enum | See below. |

`filling` variants:

- `{"Prefilled": {"groups": [...]}}` — groups are fixed by hand. `groups` is
  order-significant and aligned with `group_names` (same length); each element is a
  record `{"students": [student ids]}`, possibly empty.
- `{"Automatic": {"excluded_students": [student ids]}}` — the solver fills the
  groups, skipping the excluded students.

Constraints: for prefilled lists, `groups` has the same length as `group_names`, no
student appears in two groups, and all students exist; for automatic lists, all
excluded students exist.

### 4.10 `GroupListAssociations`

Which group list a subject uses on a period. Payload: keyed collection of
association rows, keyed by `(period_id, subject_id)`.

**Default:** `[]` — no associations.

```json
[
  { "period_id": 1, "subject_id": 2, "group_list_id": 10 }
]
```

An absent row means the subject has no group list on that period. The key set is
free: every present row carries real state (`group_list_id`), so there is no
neutral-content rule here.

Constraints: all three ids exist; the subject has interrogations; the subject runs
on that period (is not excluded from it).

### 4.11 `Pairings`

Implication rules between subjects: "if a student has an interrogation in the
antecedent subject on some week, then the consequent condition must hold that
week." Payload: keyed collection (by `id`).

**Default:** `[]` — no pairing rules.

```json
[
  {
    "id": 12,
    "antecedent": { "subject_id": 2, "should_have": true },
    "consequent": { "subject_id": 13, "should_have": false },
    "excluded_periods": [1],
    "soft": true
  }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Pairing rule id. |
| `antecedent`, `consequent` | record `{"subject_id", "should_have"}` | `should_have`: `true` = "has an interrogation that week", `false` = "has none". |
| `excluded_periods` | array of period ids | Periods where the rule does not apply. |
| `soft` | bool | `true` = best-effort (optimised), `false` = hard constraint. |

Constraints: both subjects exist **and** have interrogations
(`interrogation_parameters` is not `null`); antecedent and consequent subjects
differ; excluded periods exist. A rule naming a subject without interrogations
is vacuous or impossible, never meaningful. (Rules apply only to students
enrolled in both subjects — a solver semantic, not a file constraint.)

### 4.12 `SlotPairings`

The same implication shape between two **slots of the same subject**: "if the
antecedent slot is used on some week, the consequent condition must hold that
week." Payload: keyed collection (by `id`).

**Default:** `[]` — no slot pairing rules.

```json
[
  {
    "id": 14,
    "antecedent": { "slot_id": 8, "should_have": true },
    "consequent": { "slot_id": 15, "should_have": true },
    "excluded_periods": [],
    "soft": false
  }
]
```

Fields as in `Pairings` with `slot_id` in the rule parts.

Constraints: both slots exist; they differ; they belong to the same subject;
excluded periods exist.

### 4.13 `Settings`

Global and per-student interrogation-load limits. Payload: record.

**Default:** no limits at all —

```json
{
  "global": {
    "interrogations_per_week_min": null,
    "interrogations_per_week_max": null,
    "max_interrogations_per_day": null
  },
  "students": []
}
```

Populated example:

```json
{
  "global": {
    "interrogations_per_week_min": { "soft": false, "value": 1 },
    "interrogations_per_week_max": { "soft": true, "value": 4 },
    "max_interrogations_per_day": { "soft": false, "value": 2 }
  },
  "students": [
    {
      "student_id": 5,
      "limits": {
        "interrogations_per_week_min": null,
        "interrogations_per_week_max": { "soft": true, "value": 3 },
        "max_interrogations_per_day": null
      }
    }
  ]
}
```

A limits record has exactly the three fields shown; each is `null` (no limit) or a
soft parameter `{"soft": bool, "value": n}`. `value` is an integer ≥ 0 for the
per-week limits and ≥ 1 for `max_interrogations_per_day`.

`students` is a keyed collection (by `student_id`); each row **overrides** the
global limits for that student. The key set is free: a row existing is itself state
(an override exists), whatever its values.

Constraints: every `student_id` exists.

### 4.14 `Balancing`

Global and per-subject balancing options for the solver. Payload: record.

**Default:** teacher rotation as a soft goal, nothing else —

```json
{
  "global": {
    "teacher_rotation": { "soft": true },
    "slot_rotation": null,
    "avoid_twice_in_a_row": null,
    "year_teacher_rotation": false,
    "period_teacher_rotation": false
  },
  "subjects": []
}
```

A balancing-options record has exactly these five fields. The first three are
three-state: `null` means the goal is not pursued at all (no constraint and no
optimisation term), `{"soft": true}` makes it an optimisation goal and
`{"soft": false}` a strict constraint.

| Field | Type | Meaning |
|---|---|---|
| `teacher_rotation` | `null` or `{"soft": bool}` | Rotate teachers across groups. |
| `slot_rotation` | `null` or `{"soft": bool}` | Rotate time slots across groups. |
| `avoid_twice_in_a_row` | `null` or `{"soft": bool}` | Avoid the same teacher twice in a row for a group. |
| `year_teacher_rotation` | bool | Fair teacher distribution over the whole year. |
| `period_teacher_rotation` | bool | Fair teacher distribution within each period. |

`subjects` is a keyed collection (by `subject_id`) of records
`{"subject_id", "options"}`; each row overrides the global options for that
subject. The key set is free: a row is an override, whatever its values.

Constraints: every `subject_id` exists **and** has interrogations.

### 4.15 `Colloscope`

The colloscope itself: which groups sit which interrogation, and how automatic
group lists were filled. Payload: record.

**Default:** `{"interrogations": [], "group_lists": []}` — an unsolved colloscope.

```json
{
  "interrogations": [
    { "slot_id": 8, "week": 0, "assigned_groups": [0] },
    { "slot_id": 8, "week": 2, "assigned_groups": [0, 1] }
  ],
  "group_lists": [
    {
      "group_list_id": 11,
      "students": [
        { "student_id": 5, "group": 0 },
        { "student_id": 6, "group": 1 }
      ]
    }
  ]
}
```

#### `interrogations`

A keyed collection of rows `{"slot_id", "week", "assigned_groups"}`, keyed by
`(slot_id, week)`. `week` is a global week index (§3); the week determines the
period, so no period appears in the row. `assigned_groups` is an array of 0-based
group numbers.

The key set is derived: the (slot, week) cells that can host an interrogation are
fully determined by the other blocks. A cell exists exactly when

1. the slot's subject runs on the period containing that week (the period is not in
   the subject's `excluded_periods`), and
2. the week's `interrogations` flag (§4.1) is `true`, and
3. the slot's week pattern (when it has one) is `true` on that week.

A row on a non-existent cell — unknown slot, week out of range, or conditions 1–3
not met — is invalid, whatever its content. An absent row means no groups are
assigned to that cell; a row with an empty `assigned_groups` encodes the same
thing — valid but redundant, omitted in canonical form.

Constraints (in addition to cell existence): every group number in
`assigned_groups` is `<` the group count (`group_names` length) of the group list
associated to the slot's subject on the row's period; if no group list is
associated there, no group number is valid.

#### `group_lists`

How each **automatic** group list was filled: a keyed collection of rows
`{"group_list_id", "students"}`, keyed by `group_list_id`, with a **derived key
set**: the meaningful keys are exactly the automatic (non-prefilled) group lists.
(Prefilled lists carry their composition in `GroupLists` and never appear here.)

An absent row means the list is unfilled. A row whose `students` array is empty
encodes the same thing — valid but redundant, omitted in canonical form.

`students` is a keyed collection (by `student_id`) of records
`{"student_id", "group"}` assigning the student a 0-based group number.

Constraints: `group_list_id` is an existing automatic group list; every student
exists and is not in the list's `excluded_students`; `group` is `<` the list's
group count.

### 4.16 `ExportConfig`

Presentation settings for spreadsheet export. Payload: record. No field references
ids; everything is local.

**Default:**

```json
{
  "global": {
    "background_color": { "red": 255, "green": 255, "blue": 255 },
    "stripes_color_enabled": true,
    "stripes_color": { "red": 220, "green": 220, "blue": 230 }
  },
  "colloscope_enabled": true,
  "all_groups_enabled": true,
  "automatic_groups_enabled": false,
  "prefilled_groups_enabled": false,
  "per_group_list_enabled": true,
  "colloscope_config": {
    "sheet_name": "Colloscope",
    "extra_info_column_enabled": true,
    "extra_info_column_name": "Info",
    "teacher_email_enabled": true,
    "teacher_email": "Contact",
    "teacher_tel_enabled": false,
    "teacher_tel": "",
    "orientation": "Landscape",
    "display_week_dates": true,
    "display_annotations": true,
    "no_interrogation_color": { "red": 140, "green": 140, "blue": 140 },
    "annotation_color_enabled": true,
    "annotation_color": { "red": 255, "green": 255, "blue": 0 },
    "extra_colors": []
  },
  "all_groups_config": {
    "sheet_name": "Tous les groupes",
    "orientation": null,
    "show_emails": true,
    "show_tel": false
  },
  "automatic_groups_config": {
    "sheet_name": "Groupes automatiques",
    "orientation": null,
    "show_emails": true,
    "show_tel": false
  },
  "prefilled_groups_config": {
    "sheet_name": "Groupes préremplis",
    "orientation": null,
    "show_emails": true,
    "show_tel": false
  },
  "per_group_list_config": {
    "orientation": "Portrait",
    "show_emails": true,
    "show_tel": false,
    "center_vertically": false
  }
}
```

The shape is exactly the default above; notes:

- `orientation` is `"Portrait"` or `"Landscape"`; in the three per-student-groups
  configs it may be `null`, meaning auto-detect from the group count.
- `extra_colors` maps annotation names to colors: a keyed collection of records
  `{"name", "color"}` keyed by `name`.
- All strings are plain strings and may be empty. Every `..._enabled` field is a
  bool.

## 5. Forward compatibility

The format evolves by these rules, which a robust reader or writer can rely on:

- **Block names are permanent.** The meaning and shape of a block name never
  changes once published. When a block's shape must change, a **new block name** is
  introduced with a higher `minimum_spec_version`. Likewise, a block's **default
  state is frozen forever**: changing a default also requires a new block name.
- **New aspects of the state get new blocks**, with `minimum_spec_version` set to
  the spec revision that introduces them.

How future spec revisions use this mechanism — which names they introduce, whether
they keep emitting superseded names alongside new ones for the benefit of older
readers, which blocks are needed — is decided by those revisions; this document
only fixes the mechanism itself.

A conforming reader therefore behaves as follows:

- A block whose name it recognises is read per its spec; the block must declare
  that name's canonical `minimum_spec_version` and `needed_entry` values.
- The **absence** of any block it recognises means that block's default state.
  This is what keeps older files readable by newer applications: they simply lack
  the newer blocks.
- A block whose name it does **not** recognise: if `needed_entry` is `true`, the
  reader must refuse the file (it cannot faithfully represent the document); if
  `false`, the reader may skip the block and proceed, and should inform the user —
  in particular, rewriting the file will drop the skipped block.

Writers get the mirror-image benefit from the canonical form's omit-defaults rule
(§3): a document only carries blocks for the features actually used, so a file
demands the spec level of its *content*, not of the application that wrote it. A
newer application that never uses a newer feature keeps producing files that older
readers open cleanly.

## 6. Complete example

A small, internally consistent document in canonical form: one period of two weeks,
one subject, one teacher, two students, one week pattern, one slot, one automatic
group list, and a partially filled colloscope. `Incompatibilities`, `Pairings`,
`SlotPairings`, `Balancing` and `ExportConfig` are in their default state and
therefore omitted.

```json
{
  "header": {
    "file_type": "Collomatique",
    "produced_with_version": "0.1.0-alpha.0.99",
    "file_content": "Colloscope"
  },
  "entries": [
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "GeneralPlanning": {
          "first_week": "2026-08-31",
          "periods": [
            {
              "id": 1,
              "weeks": [
                { "interrogations": true, "annotation": "Rentrée" },
                { "interrogations": true, "annotation": null }
              ]
            }
          ]
        }
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Subjects": [
          {
            "id": 2,
            "name": "Mathématiques",
            "interrogation_parameters": {
              "students_per_group": { "min": 1, "max": 2 },
              "groups_per_interrogation": { "min": 1, "max": 1 },
              "duration_minutes": 60,
              "take_duration_into_account": true,
              "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
            },
            "excluded_periods": []
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Teachers": [
          {
            "id": 3,
            "surname": "Rogue",
            "firstname": "Severus",
            "tel": null,
            "email": "rogue@poudlard.fr",
            "subjects": [2]
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Students": [
          {
            "id": 4,
            "surname": "Potter",
            "firstname": "Harry",
            "tel": "0601020304",
            "email": null,
            "excluded_periods": []
          },
          {
            "id": 5,
            "surname": "Granger",
            "firstname": "Hermione",
            "tel": null,
            "email": "hermione@poudlard.fr",
            "excluded_periods": []
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Assignments": [
          { "period_id": 1, "subject_id": 2, "students": [4, 5] }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "WeekPatterns": [
          { "id": 6, "name": "Toutes les semaines", "weeks": [true, true] }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Slots": [
          {
            "subject_id": 2,
            "slots": [
              {
                "id": 7,
                "teacher_id": 3,
                "start": { "day": "monday", "time": "14:00" },
                "extra_info": "Salle 101",
                "week_pattern_id": 6,
                "cost": 0
              }
            ]
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "GroupLists": [
          {
            "id": 8,
            "name": "Groupes de maths",
            "students_per_group": { "min": 1, "max": 2 },
            "group_names": ["Groupe 1", null],
            "filling": { "Automatic": { "excluded_students": [] } }
          }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "GroupListAssociations": [
          { "period_id": 1, "subject_id": 2, "group_list_id": 8 }
        ]
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Settings": {
          "global": {
            "interrogations_per_week_min": null,
            "interrogations_per_week_max": { "soft": true, "value": 3 },
            "max_interrogations_per_day": null
          },
          "students": []
        }
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "Colloscope": {
          "interrogations": [
            { "slot_id": 7, "week": 0, "assigned_groups": [0] }
          ],
          "group_lists": [
            {
              "group_list_id": 8,
              "students": [
                { "student_id": 4, "group": 0 },
                { "student_id": 5, "group": 0 }
              ]
            }
          ]
        }
      }
    }
  ]
}
```
