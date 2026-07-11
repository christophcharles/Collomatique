# The Collomatique file format (spec 2)

This document is the reference specification of the Collomatique colloscope file.
It describes the file **entirely**, field by field. The phase-1 implementation of
`docs/state_consolidation_plan.md` is checked against this document; when the two
disagree, one of them has a bug and the discrepancy must be resolved explicitly.

## 1. Overview

A Collomatique file is a single UTF-8 JSON document. It is a **snapshot**: it stores
the complete state of one colloscope project (parameters, the colloscope itself, and
the export configuration) and nothing else — no undo history, no oplog, no caches.

The file stores **free semantic state only**. Anything that is fully determined by
other stored data (derived week activity, id counters, positional skeletons) is *not*
written; readers reconstruct it. This is the format's guiding principle: the file is a
*document* describing the user's choices, not a dump of in-memory representation.

### Versioning model

Two version mechanisms coexist:

- `produced_with_version` in the header is the **application** semver that wrote the
  file. It is informational: a file produced by a newer application version can still
  be opened (with a caveat reported to the user) as long as its entries are readable.
- Each entry carries a `minimum_spec_version`, an integer identifying the **format
  spec** needed to understand that entry, together with a `needed_entry` flag saying
  whether the file makes sense without it. This is what actually gates readability
  (see §2).

The current spec version is **2**.

### Spec 1 is dead

Spec 1 had a single entry, `InnerDataDump`, whose payload was a raw serde dump of the
in-memory `InnerData` type. It only ever existed during pre-alpha development. All
spec-1 files are bulk-converted to spec 2 once, after which the spec-1 decoder is
deleted. A spec-2 reader recognises the `InnerDataDump` entry name **only as a
tombstone**: encountering it produces the dedicated error "unsupported pre-alpha
development format" (rather than a generic parse failure). Spec 2 is *not* renumbered
as 1; the number 1 stays burned.

## 2. Envelope

The top-level document is:

```json
{
  "header": {
    "file_type": "Collomatique",
    "produced_with_version": { "major": 0, "minor": 1, "patch": 0 },
    "file_content": "Colloscope"
  },
  "entries": [
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": { "<SectionName>": <section payload> }
    }
  ]
}
```

### Header

| Field | Type | Meaning |
|---|---|---|
| `file_type` | string | Always `"Collomatique"`. Anything else fails JSON-structure parsing. |
| `produced_with_version` | object `{major, minor, patch}` (u32 each) | Application version that wrote the file. Informational. |
| `file_content` | string | Kind of document. Currently only `"Colloscope"`. An unrecognised value is a decode error (`UnknownFileType`) carrying `produced_with_version`, so the user learns a newer application is needed. |

If `produced_with_version` is greater than the running application's version, the file
is still decoded but the caveat `CreatedWithNewerVersion` is reported.

### Entries

Each entry is `{minimum_spec_version, needed_entry, content}`. `content` is an object
with exactly one key — the entry name (**externally tagged**) — whose value is the
entry's payload.

Decoding rules, applied per entry:

1. **Recognised entry name.** The declared `minimum_spec_version` and `needed_entry`
   must equal the canonical values for that entry name (for every spec-2 section:
   `2` and `true`). A mismatch is the decode error `MismatchedSpecRequirementInEntry`.
   If the payload then fails to parse, the decode error **must surface the underlying
   serde error** (field name, expected type, position). Spec 1 swallowed these into
   "unknown entry" and misreported them; spec 2 makes good diagnostics part of the
   format contract.
2. **Unrecognised entry name, `minimum_spec_version > 2`.** The entry comes from a
   future spec. If `needed_entry` is `true`, decoding fails (`UnknownNeededEntry`,
   carrying the producing version). If `false`, the entry is skipped and the caveat
   `UnknownEntries` is reported.
3. **Unrecognised entry name, `minimum_spec_version <= 2`.** The entry claims to be
   understandable by this reader but isn't: the file is ill-formed. Decoding fails,
   again surfacing the underlying parse diagnostics.

**Skipped entries are not preserved.** Saving a file that was loaded with skipped
unknown entries writes them out of existence. Opening a newer file with an older
application and re-saving is lossy by design; the `UnknownEntries` caveat is the
user's warning.

### Section entries: mandatory, exactly once

A spec-2 file contains **exactly these 15 entries**, each with
`minimum_spec_version: 2` and `needed_entry: true`:

`GeneralPlanning`, `Subjects`, `Teachers`, `Students`, `Assignments`, `WeekPatterns`,
`Slots`, `Incompatibilities`, `GroupLists`, `Pairings`, `SlotPairings`, `Settings`,
`Balancing`, `Colloscope`, `ExportConfig`.

A missing section is a decode error. A duplicated section is a decode error. There
are **no defaults**: an empty section is written explicitly (e.g. the `Teachers`
payload is `[]`). Defaults may later appear inside decoders for *older* specs (to
fill in data a newer spec added), never silently within a spec.

Entry order in `entries` is the canonical order listed above; readers accept any
order (each section is identified by name, and exactly-once is enforced regardless).

## 3. Conventions

### Identifiers

All persistent objects (periods, subjects, teachers, students, week patterns, slots,
incompatibilities, group lists, pairing rules, slot pairing rules) draw their ids from
a **single global id space**: a bare JSON number holding a `u64`. An id value appears
at most once across the *whole file*, regardless of object kind — a subject and a
teacher can never share id `3`. Duplicate ids anywhere are a load error.

No id counter is stored. Readers rebuild the issuer from the maximum id in use; a
file whose ids exhaust the usable space fails to load (`EndOfTheUniverse`).

### Scalar encodings

| In-memory type | JSON encoding | Constraints |
|---|---|---|
| id newtypes | number (u64) | globally unique, see above |
| `WeekStart` | string `"YYYY-MM-DD"` | must be a Monday; anything else is a decode error |
| `WholeMinuteTime` | string `"HH:MM"` | 24-hour, zero-padded, minute precision; `"9:00"`, `"09:00:00"`, seconds — all decode errors |
| `Weekday` | string | lowercase English: `"monday"` … `"sunday"` |
| durations (`NonZeroMinutes`) | number | integer minutes, ≥ 1 |
| `RangeInclusive<T>` | object `{"min": n, "max": n}` | `min <= max` (a decode error otherwise) |
| `NonZeroU32` | number | ≥ 1 |
| `NonEmptyString` | string | empty string is a decode error |
| plain `String` | string | may be empty (e.g. `extra_info`, names, surnames) |
| `Color` | object `{"red": n, "green": n, "blue": n}` | each 0–255 |

### Optionality

Optional values are encoded as `null`, **never omitted**. Every field of every
structure is mandatory in the file; a missing field is a decode error, and an unknown
field is a decode error. (This is the no-defaults policy applied at field level: the
byte sequence for a given state is unique, and typos in hand-edited files fail loudly
instead of silently becoming defaults.)

### Enums

Enums use serde's default **externally tagged** representation:

- data-carrying variants: `{"ExactlyPeriodic": {"periodicity_in_weeks": 2}}`
- unit variants: bare string, e.g. `"Portrait"`, `"Landscape"`

Soft parameters (`SoftParam<T>`) are objects `{"soft": bool, "value": T}`. When the
parameter carries no value (`SoftParam<()>`), the `value` field is dropped and the
encoding is `{"soft": bool}`. An *optional* soft parameter is therefore
`null | {"soft": ..., "value": ...}` or `null | {"soft": ...}`.

### Rows, not maps

Keyed collections are **arrays of objects with an explicit `"id"` (or key) field** —
the flat-row shape of a relational schema — never JSON objects keyed by stringified
ids. Association data (assignments, group-list associations, colloscope rows) is
likewise flat rows carrying their full key.

### Ordering and determinism

Two kinds of arrays:

- **Order-significant arrays** (user-visible order is state): `periods`, `subjects`,
  the per-subject `slots` array, `group_names`, prefilled `groups`, incompatibility
  `slots`, periodicity `blocks`, and all `weeks` arrays (positional). Readers preserve
  their order; reordering them changes the document's meaning.
- **Sorted arrays** (order carries no meaning): everything else. Writers **must** emit
  them sorted — by id for object rows; by full key ascending for association rows
  (e.g. `(period_id, subject_id)`, `(slot_id, week)`); ascending for id sets and
  group-number sets; lexicographically (by Unicode code point) for `extra_colors`
  names. Readers accept any order but reject duplicate keys as decode errors.

Serialization uses pretty-printed JSON (`serde_json::to_string_pretty`, 2-space
indent). **Byte stability is a format guarantee**: encoding the same state always
produces the same bytes, pinned by a golden-fixture test
(`storage/tests/populated_round_trip.rs::reserialize_is_stable`). Together with
canonical ordering and the null-not-omitted rule, one state has exactly one canonical
byte sequence.

### Week coordinates

The schedule is a concatenation of periods; each period is a list of weeks. The
**global week index** is the 0-based position of a week in that concatenation (period
order is significant, so global week numbering is well defined). Wherever the format
stores a per-week value outside `GeneralPlanning` — week-pattern `weeks` arrays,
colloscope `week` fields — it uses global week indices.

## 4. Section entries

Each subsection below gives the payload shape, the fields, and the constraints
checked at load. §5 explains *where* each kind of constraint is enforced.

### 4.1 `GeneralPlanning`

The period structure and start date. Payload: object.

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
| `first_week` | date string or `null` | Monday of global week 0. `null` if not set (hinders pretty output only). |
| `periods` | array, **order-significant** | The periods in user order. Their concatenated `weeks` arrays define global week numbering. |
| `periods[].id` | id | Period id. |
| `periods[].weeks` | array, positional | One element per week of the period. |
| `weeks[].interrogations` | bool | `false` marks a week with no interrogations at all (holidays, exams). |
| `weeks[].annotation` | non-empty string or `null` | Free label displayed on exports. |

Constraints: `first_week` must be a Monday. A period may have zero weeks. The file
may have zero periods.

### 4.2 `Subjects`

Payload: array, **order-significant** (user order).

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
| `interrogation_parameters` | object or `null` | `null` means the subject has **no interrogations** (it still exists for assignments). |
| `excluded_periods` | sorted array of period ids | Periods the subject does not run on. By default a subject runs on every period. |

`interrogation_parameters` fields:

| Field | Type | Meaning |
|---|---|---|
| `students_per_group` | range of integers ≥ 1 | Students per group for this subject (can differ from the group list's own range). |
| `groups_per_interrogation` | range of integers ≥ 1 | How many groups can share one interrogation slot. |
| `duration_minutes` | integer ≥ 1 | Duration of one interrogation. |
| `take_duration_into_account` | bool | Whether this duration counts toward per-week/day time limits. |
| `periodicity` | tagged enum | See below. |

`periodicity` variants (externally tagged):

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

Referential constraints: every id in `excluded_periods` is an existing period; all
ranges satisfy `min <= max`.

### 4.3 `Teachers`

Payload: array, sorted by `id`.

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
| `subjects` | sorted array of subject ids | Subjects the teacher can interrogate in. |

Referential constraints: every id in `subjects` is an existing subject **and** that
subject has interrogations (`interrogation_parameters` is not `null`).

### 4.4 `Students`

Payload: array, sorted by `id`.

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

Same person fields as teachers. `excluded_periods` (sorted array of period ids) lists
periods the student does not attend at all.

Referential constraints: every id in `excluded_periods` is an existing period.

### 4.5 `Assignments`

Which students take which subject on which period. Payload: array of association
rows, sorted by `(period_id, subject_id)`.

```json
[
  { "period_id": 1, "subject_id": 2, "students": [5, 6] },
  { "period_id": 1, "subject_id": 3, "students": [] }
]
```

**Row-set is exact**: there is exactly one row for every (period × subject not
excluded from that period) pair — including subjects without interrogations — even
when no student is assigned (`"students": []`). A missing row, an extra row (unknown
ids, subject excluded from the period), or a duplicated `(period_id, subject_id)` key
is a load error. This explicit presence is deliberate: the row-set mirrors the
model's shape, so absence never has to be interpreted.

`students` is a sorted array of student ids. Referential constraints: each student
exists and is not excluded from the row's period.

### 4.6 `WeekPatterns`

Named week masks used by slots and incompatibilities. Payload: array, sorted by `id`.

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

Referential constraints: `weeks.length` must equal the **total week count** (the sum
of all period lengths in `GeneralPlanning`) — no shorter, no longer.

### 4.7 `Slots`

Interrogation slots, grouped by subject. Payload: array of per-subject rows, sorted
by `subject_id`; each inner `slots` array is **order-significant** (user order).

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

**Row-set is exact**: there is exactly one per-subject row for every subject **with
interrogations**, even when it has no slots (`"slots": []`); no row for subjects
without interrogations. Duplicated `subject_id` is a load error.

Slot fields:

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Slot id. |
| `teacher_id` | id | The interrogating teacher. |
| `start` | `{"day", "time"}` | Weekday + start time. The duration comes from the subject's `duration_minutes`. |
| `extra_info` | string | Free info for exports (room number…). May be empty. |
| `week_pattern_id` | id or `null` | `null` = the slot exists every week. |
| `cost` | integer (i32) | Solver preference: positive avoids the slot, negative favours it, 0 neutral. |

Referential constraints: `teacher_id` exists and that teacher's `subjects` contains
this subject; `week_pattern_id` (when non-null) exists; `start` plus the subject's
duration must not cross midnight.

### 4.8 `Incompatibilities`

Recurring external commitments (e.g. an optional course) that make students
unavailable. Payload: array, sorted by `id`.

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
| `slots` | array, **order-significant** | Time slots when students may be unavailable; each `{"day", "time", "duration_minutes"}`. |
| `minimum_free_slots` | integer ≥ 1 | How many of `slots` must be kept free. |
| `week_pattern_id` | id or `null` | `null` = applies every week. |

Referential constraints: `subject_id` exists; `week_pattern_id` (when non-null)
exists; each slot must not cross midnight.

### 4.9 `GroupLists`

Group lists and their per-(period, subject) associations. Payload: object.

```json
{
  "group_lists": [
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
  ],
  "associations": [
    { "period_id": 1, "subject_id": 2, "group_list_id": 10 }
  ]
}
```

`group_lists` is sorted by `id`. Group list fields:

| Field | Type | Meaning |
|---|---|---|
| `id` | id | Group list id. |
| `name` | string | Display name. |
| `students_per_group` | range of integers ≥ 1 | Allowed group size. |
| `group_names` | array of (non-empty string or `null`), **order-significant** | One element per group; its length **is** the group count. `null` = unnamed group. Group numbers used elsewhere are 0-based indices into this array. |
| `filling` | tagged enum | `{"Prefilled": {"groups": [...]}}` — groups are fixed by hand; `groups` is order-significant, aligned with `group_names`, each element `{"students": [sorted student ids]}`. Or `{"Automatic": {"excluded_students": [sorted student ids]}}` — the solver fills groups, skipping the excluded students. |

`associations` is a flat array of rows, sorted by `(period_id, subject_id)`. A row
means "on this period, this subject uses this group list". **Absence means no
association** — this is one of the places where absence carries the actual semantics,
so there is no exact row-set requirement. Duplicated `(period_id, subject_id)` is a
load error (a subject has at most one group list per period).

Referential constraints: for prefilled lists, `groups.length == group_names.length`,
no student appears in two groups, and all students exist; for automatic lists, all
excluded students exist. For associations: all three ids exist; the subject has
interrogations; the subject runs on that period (is not excluded from it).

### 4.10 `Pairings`

Implication rules between subjects: "if a student has an interrogation in the
antecedent subject on some week, then the consequent condition must hold that week."
Payload: array, sorted by `id`.

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
| `antecedent`, `consequent` | `{"subject_id", "should_have"}` | `should_have`: `true` = "has an interrogation that week", `false` = "has none". |
| `excluded_periods` | sorted array of period ids | Periods where the rule does not apply. |
| `soft` | bool | `true` = best-effort (optimized), `false` = hard constraint. |

Referential constraints: both subjects exist; antecedent and consequent subjects
differ; excluded periods exist. Rules apply only to students enrolled in both
subjects (a solver semantic, not a file constraint).

### 4.11 `SlotPairings`

The same implication shape between two **slots of the same subject**: "if the
antecedent slot is used on some week, the consequent condition must hold that week."
Payload: array, sorted by `id`.

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

Referential constraints: both slots exist; they differ; they belong to the **same
subject**; excluded periods exist.

### 4.12 `Settings`

Global and per-student interrogation-load limits. Payload: object.

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

A `Limits` object has exactly the three fields shown; each is `null` (no limit) or a
soft parameter `{"soft": bool, "value": n}`. `value` is an integer ≥ 0 for the
per-week limits and ≥ 1 for `max_interrogations_per_day`.

`students` is sorted by `student_id`; each row **overrides** the global limits for
that student. **Absence means no override** — no exact row-set requirement.
Duplicated `student_id` is a load error.

Referential constraints: every `student_id` exists.

### 4.13 `Balancing`

Global and per-subject balancing options for the solver. Payload: object.

```json
{
  "global": {
    "teacher_rotation": { "soft": true },
    "slot_rotation": null,
    "avoid_twice_in_a_row": true,
    "year_teacher_rotation": false,
    "period_teacher_rotation": true
  },
  "subjects": [
    {
      "subject_id": 2,
      "options": {
        "teacher_rotation": null,
        "slot_rotation": { "soft": false },
        "avoid_twice_in_a_row": false,
        "year_teacher_rotation": true,
        "period_teacher_rotation": false
      }
    }
  ]
}
```

A `BalancingOptions` object has exactly the five fields shown:

| Field | Type | Meaning |
|---|---|---|
| `teacher_rotation` | `null` or `{"soft": bool}` | Rotate teachers across groups (`null` = off). |
| `slot_rotation` | `null` or `{"soft": bool}` | Rotate time slots across groups (`null` = off). |
| `avoid_twice_in_a_row` | bool | Avoid the same teacher twice in a row for a group. |
| `year_teacher_rotation` | bool | Fair teacher distribution over the whole year. |
| `period_teacher_rotation` | bool | Fair teacher distribution within each period. |

`subjects` is sorted by `subject_id`; each row overrides the global options for that
subject. Absence means no override. Duplicated `subject_id` is a load error.

Referential constraints: every `subject_id` exists **and** has interrogations.

### 4.14 `Colloscope`

The colloscope itself: which groups sit which interrogation, and how automatic group
lists were filled. Payload: object.

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

#### `interrogations` — sparse, global-week keyed

One row `{"slot_id", "week", "assigned_groups"}` for each interrogation that has at
least one group assigned. `week` is a **global week index** (§3); there is no
`period_id` in the row — the week determines the period. `assigned_groups` is a
sorted array of 0-based group numbers. Rows are sorted by `(slot_id, week)`.

The file stores free state only. In memory, every slot carries a full per-week
skeleton distinguishing "no interrogation possible this week" from "interrogation
possible but no group assigned"; that skeleton has **zero degrees of freedom** — it
is entirely determined by the parameters (the merged pattern: the period's per-week
`interrogations` flags AND the slot's week pattern, with the slot's subject running
on the period). Readers rebuild it from `GeneralPlanning`, `WeekPatterns`, `Subjects`
and `Slots`; writers never store it.

Hard decode errors:

- a row whose `slot_id` is unknown;
- a row whose `week` is out of range (≥ total week count);
- a row on a week where the merged pattern is inactive, or where the slot's subject
  does not run (its period is excluded, so the slot has no cell there);
- a row with **empty** `assigned_groups` — canonicity: an unsolved interrogation is
  encoded by the row's absence, so writers never emit empty rows and readers reject
  them (one state, one byte sequence);
- a duplicated `(slot_id, week)` key.

An entirely unsolved colloscope is `"interrogations": []`.

Referential constraints: every group number in `assigned_groups` is `<` the group
count (`group_names.length`) of the group list associated to the slot's subject on
the row's period; if no group list is associated there, no group number is valid.

#### `group_lists` — exact row-set

One row per **non-prefilled** (automatic) group list — exactly those, even when
empty (`"students": []`). A row for a prefilled group list, a missing automatic one,
or a duplicated `group_list_id` is a load error. (Prefilled lists carry their
composition in `GroupLists`; repeating it here would be stored representation.)

`students` is sorted by `student_id`; each element assigns the student a 0-based
`group` number.

Referential constraints: every student exists, is not in the group list's
`excluded_students`, and `group` is `<` the list's group count.

### 4.15 `ExportConfig`

Presentation settings for spreadsheet export. Payload: object, a faithful flat
mirror of the in-memory configuration. No field references ids; everything is local.

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
    "extra_colors": [
      { "name": "Vacances", "color": { "red": 0, "green": 128, "blue": 0 } }
    ]
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
    "center_vertically": true
  }
}
```

Notes:

- `orientation` is `"Portrait"` or `"Landscape"`; in the three per-student-groups
  configs it may be `null`, meaning auto-detect from the group count.
- `extra_colors` maps annotation names to colors: an array of `{"name", "color"}`
  rows sorted by `name` (duplicated names are a load error).
- All strings are plain strings and may be empty. Every `..._enabled` flag is a bool.

## 5. Validation at load

Loading has three layers with a clear trust boundary:

1. **JSON structure** (serde): the envelope shape, the exactly-once section rule,
   every field's presence and type, and all *local* scalar constraints — time and
   date syntax, Monday-ness, non-empty strings, `min <= max`, integers ≥ 1 where
   specified, unknown fields, duplicated keys in row arrays, empty
   `assigned_groups` rows. Failures here report the underlying serde diagnostics
   (§2 rule 1).
2. **Reconstruction** (format → in-memory `InnerData`): the decoder rebuilds derived
   structure the file deliberately omits — per-period maps, the colloscope
   interrogation skeleton (§4.14), the id issuer. Sparse-row placement errors
   (unknown slot, out-of-range or inactive week) surface here.
3. **Invariants** (`Data::from_inner_data`, in `state-colloscopes`): global id
   uniqueness and every cross-entry referential constraint listed in §4 — dangling
   ids, exact row-sets (assignments, slots, colloscope group lists), week-pattern
   lengths, teacher/subject compatibility, group-number bounds. This layer is the
   single trust boundary: it validates *any* `InnerData` regardless of provenance,
   so the decoder does not need to be trusted for semantic integrity, and
   constraints listed in §4 as "referential" are enforced here even when the
   decoder also happens to catch them earlier.

Nothing in the file is trusted without being checked; a file that decodes
successfully is fully valid state.

## 6. Evolution rules

- **Adding optional data** (new feature whose absence is tolerable): a **new entry
  name** with `needed_entry: false` and `minimum_spec_version` = the spec that
  introduces it. Older applications skip it with the `UnknownEntries` caveat (and
  drop it on re-save); newer ones read it. The existing 15 sections are untouched.
- **Changing the shape of a section** (or adding data the file must not be read
  without): a **new entry name** — by convention the section name with a version
  suffix, e.g. `SubjectsV3` — with `needed_entry: true` and the new spec number,
  plus a spec bump of `CURRENT_SPEC_VERSION`. Writers emit only the new entry.
  Older applications fail cleanly with `UnknownNeededEntry` ("this file needs a
  newer version"). The decoder for the old entry name is **frozen forever**: newer
  applications keep reading old files by decoding the old entry into the current
  in-memory model (this is where defaults are allowed — filling in what the old
  spec could not express).
- **Never** change the meaning or shape of an existing entry name in place, and
  never delete a frozen decoder (the spec-1 tombstone is the unique, deliberate
  exception from the pre-alpha era).
- Recent applications must always open every file produced since spec 2.

## 7. Complete minimal example

A small but fully populated, internally consistent spec-2 file: one period of two
weeks, one subject, one teacher, two students, one week pattern, one slot, one
automatic group list, and a partially filled colloscope. (Application version and
non-essential `ExportConfig` values are illustrative.)

```json
{
  "header": {
    "file_type": "Collomatique",
    "produced_with_version": { "major": 0, "minor": 1, "patch": 0 },
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
      "content": { "Incompatibilities": [] }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "GroupLists": {
          "group_lists": [
            {
              "id": 8,
              "name": "Groupes de maths",
              "students_per_group": { "min": 1, "max": 2 },
              "group_names": ["Groupe 1", null],
              "filling": { "Automatic": { "excluded_students": [] } }
            }
          ],
          "associations": [
            { "period_id": 1, "subject_id": 2, "group_list_id": 8 }
          ]
        }
      }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": { "Pairings": [] }
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": { "SlotPairings": [] }
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
        "Balancing": {
          "global": {
            "teacher_rotation": { "soft": true },
            "slot_rotation": null,
            "avoid_twice_in_a_row": true,
            "year_teacher_rotation": false,
            "period_teacher_rotation": false
          },
          "subjects": []
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
    },
    {
      "minimum_spec_version": 2,
      "needed_entry": true,
      "content": {
        "ExportConfig": {
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
      }
    }
  ]
}
```

A corpus of larger example files is planned as part of phase 1.5 of
`docs/state_consolidation_plan.md` (golden fixtures under `examples/`).
