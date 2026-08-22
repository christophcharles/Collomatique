# The ops mirror — split into commits

What remains of `new_api_design.md` §5, cut into pieces small enough that each
becomes one commit. Two of the 69 composite ops are exposed today —
`set_first_week` and `clear_first_week`, the pair that proved the pipeline —
and two are gated out of the mirror: the solver's landing doors,
`group_lists.add_generated` and `colloscope.install`, reach Python with the
solver milestone (§10) if they do at all, since their payloads only exist once
something produces them. The 65 others are this note's scope. It also places
the two loose ends that ride with the mirror: the typed update errors of §6,
and the result types (the structured warnings, and what an `add` hands back).

The machinery comes first — both pieces of it — and the families follow, so no
family's tests are ever written against a provisional shape and revisited
later. Each piece still gets its own session when it is picked up; this note
fixes the scope and the order, not the code. It retires with the last piece,
the way `handle_api.md` and `values.md` did.

## What every piece rides on (already built)

The machinery below is done, and the first-week pair exercises all of it:

- `Document::update` (`document.rs`): `dry_apply` → warnings rendered in French
  against the pre-state → `OpResult`. Every mutator funnels through it.
- `Value::from_py` (`data.rs`): every `*Data` dataclass extracts to its model
  entity, resolving named entities against the document, refusing another
  document's handles, dead ids, empty strings, and the double naming of one
  entity through its two spellings.
- `handles::argument` (`handles.rs`): the argument convention for id-or-handle
  parameters. Resolution happens *before* the mutable borrow (§5 of the design).
- Transactions, undo, and the collection views the mutators will live on.
- `dry_apply` already answers `new_id` in its `CascadeResult`;
  `Document::update` currently discards it.

So a family piece is wiring: resolve the arguments, extract the value, build
the `UpdateOp`, call `update`, and test it. The vocabulary was all invented
elsewhere.

## The result types

Settled in review (August 2026), replacing the `new_id` stub's shape: different
answers are different types, structured the same.

- `OpResult` — what every non-creating mutator returns: `warnings`, and
  nothing else. A write that creates nothing has no id field holding `None`.
- `AddResult` — what an `add` returns: an `OpResult` subclass that also
  carries `created`, the **handle** of the new entity (a `Student` from
  `doc.students.add`, and so on — the handle is the typed part). A handle and
  not an id, for §4's own reason: it is strictly more useful, and the id is
  one attribute away.

`isinstance(r, OpResult)` holds for both, so code that only reads warnings
treats them alike. Piece 0 trues today's `OpResult` up to this shape;
`AddResult` itself debuts with the first creating op (piece 1).

## The shape of a family commit

Stated once here instead of fifteen times below. A family piece brings:

- The mutators on the family's collection view, named as §5's table says,
  each one `UpdateOp` and one undo slot.
- Test scripts in `tests/scripts/`, in the existing style: the happy path per
  op, the typed errors (piece A), the structured warnings where the op
  cascades (piece B — already landed, so the assertions are written once,
  against the final shape), and the argument-convention refusals (dead handle,
  wrong document, wrong kind).
- Where the family has removals, its staleness-setup scripts can drop the raw
  `Document::update` escape hatch the tests use today and go through the
  public surface. The comment on `Document::update` that explains the escape
  hatch retires when the last user does.

Values arrive as the design fixed them: `add(data)` and `update(id_or_handle,
data)` take the `*Data` dataclass, extraction refuses what the op cannot carry
rather than dropping it, and absent optional text is `None`, never `""`.

## Piece 0 — true up `OpResult`

The one piece that is pure correction. Today's `OpResult` still carries the
placeholder `new_id` getter — always `None`, by its own docstring waiting for
the creating ops — and a `__repr__` that prints it. Piece 0 removes both, so
the class matches the result-types decision above before anything builds on
it. Nothing breaks: the only mutators are the two first-week ops, which
create nothing, so no script reads the getter. `AddResult` is *not* built
here — with no creating op to return it, it could not be exercised — and
debuts with piece 1 instead.

## Piece A — the typed update errors (§6)

`Document::update` today flattens every op failure into a generic
`UpdateError` string. This piece replaces that with the §6 hierarchy: one
exception class per `UpdateOp` family, following the Rust family names —
`GeneralPlanningError`, `SubjectsError`, `TeachersError`, `StudentsError`,
`AssignmentsError`, `WeekPatternsError`, `SlotsError`, `IncompatibilitiesError`,
`PairingsError`, `SlotPairingsError`, `GroupListsError`, `SettingsError`,
`BalancingError`, `ColloscopeError`, `ExportConfigError` — all under the
existing `UpdateError` base, carrying the structured error data.

§6's constraint is the work: the mapping is generated structurally from the
(serde-able) `UpdateError` type, not matched arm by arm, so a new Rust variant
becomes a new Python-visible case and never a panic. The crate has no precedent
for that mechanism, which is why this is its own piece and not a rider.

Most of the classes sit unused until their family lands; that is the price of
never asserting a provisional error shape in a family's tests. The first-week
ops and their tests move onto `GeneralPlanningError` in the same commit, which
is also the proof the mechanism works.

## Piece B — the structured warnings

`Warning` gains the structured `Fix` payload §5 promises next to the rendered
sentence, and the `parent` link `CascadeWarning` already carries, so the
warning list reads as the tree it is.

It needs a design note of its own first, the way the values had `values.md`:
the `Fix` vocabulary (`state-colloscopes/src/resolution.rs`) has 25 variants,
several carrying `rebuilt` entity payloads, and their Python shape — which
variants become what, what a `rebuilt` payload is shown as, ids rather than
handles since a warning names pre-state material the op may just have
removed — has to be settled before it is built.

It does **not** wait for any family: the cascades come from the model, not
from the mutators. The tests drive them the way the staleness tests already
do — the Rust side applies a deleting `UpdateOp` through the raw
`Document::update` and hands the `OpResult` to the script — so the full `Fix`
vocabulary is reachable before a single family has landed. With A and B both
in place, every family commit after them asserts errors and warnings once, in
their final shape; nothing is retrofitted.

## The families, leaves inward

The order starts at the entities nothing references — their removals cascade
nothing, their fixtures are small — and works inward to the heavily-referenced
ones, the period surgery last. Fixtures never wait on another family's
mutators: the tests build documents from the Rust side and through the raw
escape hatch, as they do today.

### Piece 1 — incompatibilities

`add(IncompatData)`, `update(i, IncompatData)`, `remove(i)` on `doc.incompats`.
Nothing references an incompatibility, so this is the quietest classic
three-op family — the right place for `AddResult` to debut (the `OpResult`
subclass of the result types above; `OpResult` itself was trued up in
piece 0).

### Pieces 2 and 3 — pairings, then slot pairings

`add`/`update`/`remove` on `doc.pairings`, then on `doc.slot_pairings`. Twins,
also referenced by nothing; the second is mechanical once the first has
landed. The sealed-side rules (an antecedent equal to its consequent, …) are
already the extraction's business and stay there.

### Pieces 4 and 5 — settings, then balancing

`set_global_limits(LimitsData)`, `set_student_limits(student, LimitsData)`,
`remove_student_limits(student)` on `doc.settings`; then `set_global`,
`set_subject`, `remove_subject` with `BalancingData` on `doc.balancing`.
Twins again, and small; two commits all the same, one family each.

### Piece 6 — the export config

The eleven setters: `set_global(ExportGlobalConfigData)`, the five
`set_*_enabled(bool)` toggles, and the five `set_*_config(...)` for the
colloscope, all-groups, prefilled-groups, automatic-groups and per-group-list
sections. Pure config — the last of the leaf pieces.

### Piece 7 — teachers

`add(TeacherData)`, `update(t, TeacherData)`, `remove(t)` on `doc.teachers`.
The first family whose removal cascades — a teacher's slots go, and the cells
those slots held — so the first whose tests assert piece B's structured
warnings from a family's own surface.

### Piece 8 — students

`add(StudentData)`, `update(s, StudentData)`, `remove(s)` on `doc.students`.
The twin of piece 7; the `Student` payload carries its period exclusions, so
the entity is the payload here too.

### Piece 9 — assignments

`set(p, subj, student, assigned)` (`Assign` — the op orders its arguments
`(period, student, subject)`; the surface keeps §5's order),
`set_all(p, subj, assigned)`, `duplicate_previous_period(p)` on
`doc.assignments`. No values — pure argument-convention wiring. Un-assigning
cascades (the student's placements in that subject go), so the warnings
surface stays exercised.

### Piece 10 — week patterns

`add(WeekPatternData)`, `update(wp, WeekPatternData)`, `remove(wp)` on
`doc.week_patterns`.

### Piece 11 — slots

`add(SlotData)`, `update(slot, SlotData)`, `remove(slot)`, `move_up`,
`move_down` on `doc.slots`.

The first entity/payload mismatch (§2): the value names its subject, and the
subject is fixed at creation. `add` reads it from the value (`AddNewSlot`
takes the subject beside the slot payload); `update` refuses a value naming a
different subject than the slot's own.

### Piece 12 — group lists

`add(GroupListData)`, `update(gl, GroupListData)` (parameters *and* filling,
as the op replaces both), `remove(gl)`,
`set_association(p, subj, gl_or_None)`, and `duplicate_previous_period(p)`.
The family's sixth op, `add_generated`, is a solver landing door and gated
out (see the intro).

### Piece 13 — the colloscope

`set_group_list(gl, {student: group})`, `set_interrogation(slot, week,
groups)`, `erase()`, and `erase_group_lists()`. The family's fifth op,
`install` (`InstallColloscope`, §11.1), is the other solver landing door and
gated out (see the intro).

### Piece 14 — subjects

`add(SubjectData)`, `update(s, SubjectData)`, `remove(s)`, `move_up(s)`,
`move_down(s)`, `set_period_status(s, p, active)` on `doc.subjects`. The most
referenced entity, so the heaviest ordinary deletes.

The second mismatch (§2): `SubjectData` carries the excluded periods no
subject op takes. Per the design's rule — raise, naming the op that moves the
field, never discard — `add` refuses a value with a non-empty
`excluded_periods`, and `update` refuses one whose `excluded_periods` differ
from what the document holds for that subject; both name `set_period_status`.

### Piece 15 — the rest of general planning

The seven remaining ops of the family the first-week pair opened:

- `doc.periods.add(week_count)` (`AddNewPeriod` — an `AddResult` carrying the
  new `Period`), `.set_week_count(p, n)`, `.remove_with_weeks(p)`
  (`DeletePeriodAndWeeks`), `.cut(p, remaining)`, `.merge_with_previous(p)`.
- `doc.weeks.set_status(week, active)`, `.set_annotation(week, text_or_None)`.
  The ops address a week as `(period, index within the period)`; the surface
  takes the `Week` handle and the mutator translates. An empty annotation
  string is refused at the boundary, per §3.

The period surgery (`cut`, `merge_with_previous`, `remove_with_weeks`) is the
heaviest cascade source in the model — the natural last piece, when every
assertion it needs has long been routine.

## What this note does not cover

`replace_all`, the exports of §9.4, and the solver (§10) are the design's
steps 4 and 5, not the mirror; they stay in `new_api_design.md` §13. The two
gated landing doors — `group_lists.add_generated` and `colloscope.install` —
belong to that solver step: their Rust ops exist (§11.1 built
`InstallColloscope` for exactly this), but their Python surface waits for the
machinery that produces their payloads. When the last piece lands, every op
short of those two doors is reachable from Python, and this note retires.
