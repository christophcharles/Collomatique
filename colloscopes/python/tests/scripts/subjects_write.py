import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the seven labels are the
# french names `ops` gives this family's operations — six mutators, and
# `set_period_status` twice, since the op names its own direction. They are
# handed in from rust so that this pins the operations' own labels and not
# merely some strings.
#
# Everything else this script leans on it finds for itself, off the document;
# what rust asserts on its own side is that the example really holds those
# shapes.
doc = collomatique.load(source)
subjects = doc.subjects

before = len(subjects)
order = list(subjects)
periods = list(doc.periods)
first_period = periods[0]

# The subject the cascades are about: the first that runs colles, which is the
# one rust asserts holds every reference site a removal has to repair. The
# second is what its pairing rule points at, and the third is a subject that
# runs no colles at all — the shape a `SubjectData` spells out.
with_colles = [subject for subject in subjects if subject.interrogation is not None]
rich, other = with_colles[0], with_colles[1]

# ---------------------------------------------------------------- the new subject

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = subjects.add(collomatique.SubjectData("Sortilèges"))
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# Nothing in the document can name a subject that did not exist a moment ago, so
# there is nothing for the cascade to repair. The result says so rather than the
# call saying nothing.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.Subject)
assert isinstance(created.id, collomatique.SubjectId)
assert created in subjects
assert subjects[created.id] == created
assert len(subjects) == before + 1

assert created.name == "Sortilèges"
# A bare `SubjectData` creates a subject that *holds* colles, with the
# application's own parameters: `interrogation` defaults to a fresh
# `InterrogationData`, and the subject that holds none is the exception.
assert created.interrogation is not None
assert created.interrogation.duration == 60
assert created.interrogation.students_per_group == (2, 3)
assert created.interrogation.periodicity == collomatique.EveryNWeeks(2)
# A new subject runs on every period and holds no slot yet.
assert created.excluded_periods == frozenset()
assert created.slots == ()

# It lands last in the list, which is where the application puts one too.
assert created.index == before
assert list(subjects) == order + [created]

assert added.created == created
assert repr(added).startswith("AddResult(created=<Subject #")
assert "warnings=[]" in repr(added)

# The other shape the value spells out: the subject that never holds a colle,
# the quidditch practice that only takes up room in the timetable.
practice = subjects.add(
    collomatique.SubjectData("Club de Bavboules", interrogation=None)
).created
assert practice.interrogation is None
assert len(subjects) == before + 2
assert practice.index == before + 1

# Rewriting replaces the name and the interrogation parameters at once, and the
# id stays, so the handle a script is holding reads the new state.
result = subjects.update(
    created,
    collomatique.SubjectData(
        "Sortilèges et enchantements",
        interrogation=collomatique.InterrogationData(
            students_per_group=(1, 4),
            groups_per_interrogation=(2, 2),
            duration=90,
            take_duration_into_account=False,
            periodicity=collomatique.EveryNWeeks(3),
        ),
    ),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")
# Nothing points at this subject yet, so the rewrite repairs nothing.
assert result.warnings == []

assert created.name == "Sortilèges et enchantements"
assert created.interrogation.students_per_group == (1, 4)
assert created.interrogation.groups_per_interrogation == (2, 2)
assert created.interrogation.duration == 90
assert created.interrogation.take_duration_into_account is False
assert created.interrogation.periodicity == collomatique.EveryNWeeks(3)
# The position did not move: an `update` rewrites the subject where it stands.
assert created.index == before

# The subject is named by an id or by a handle, interchangeably — this is the
# write half of the argument convention. Switching the colles off is an ordinary
# rewrite here, since this subject holds nothing that needed them.
subjects.update(created.id, collomatique.SubjectData("Sortilèges", interrogation=None))
assert created.name == "Sortilèges"
assert created.interrogation is None

# And back on, with the application's own parameters.
subjects.update(created, collomatique.SubjectData("Sortilèges"))
assert created.interrogation is not None
assert created.interrogation.duration == 60

# The pattern the subject's colles pause on rides in the same value, and the op
# carries it: `update` writes what the value names. The exclusions, which no op
# carries, it refuses to move instead. A new subject is born without a pattern.
pattern = list(doc.week_patterns)[0]
assert created.week_pattern is None

with_pattern = created.to_data()
with_pattern.week_pattern = pattern
subjects.update(created, with_pattern)
assert created.week_pattern == pattern
assert created.to_data().week_pattern == pattern.id

# A value naming no pattern is a subject left with none: this field is written,
# not preserved behind the value's back.
subjects.update(created, collomatique.SubjectData("Sortilèges"))
assert created.week_pattern is None

# And the id spelling names the same pattern the handle does. This is the state
# the save below writes out.
subjects.update(
    created, collomatique.SubjectData("Sortilèges", week_pattern=pattern.id)
)
assert created.week_pattern == pattern

# The family's own pair, and the only way a subject moves: the list order is the
# one `doc.subjects` walks in, so `move_up` swaps the subject with the one before
# it. Nothing else moves — a position is display order, and nothing reads one.
moved = subjects.move_up(created)
assert isinstance(moved, collomatique.OpResult)
assert moved.warnings == []
assert created.index == before - 1
assert list(subjects) == order[:-1] + [created, order[-1], practice]

subjects.move_down(created.id)
assert created.index == before
assert list(subjects) == order + [created, practice]

# The one op that moves `SubjectData.excluded_periods`, and it reads the other
# way round: `active=False` is the exclusion put *in*. This subject holds
# nothing on the period, so there is nothing to repair.
excluded = subjects.set_period_status(created, first_period, False)
assert isinstance(excluded, collomatique.OpResult)
assert excluded.warnings == []
assert created.excluded_periods == frozenset({first_period})
assert created.to_data().excluded_periods == {first_period.id}

# This is what rust reads back off the disk.
doc.save(target)

# ------------------------------------------------------------------ the refusals

# The two refusals this family keeps for the model, and they are the family's own
# shape: a subject at either end of the list has nowhere left to go, and that is
# a refusal rather than a call that quietly did nothing.
for call, op, case, details in (
    (lambda: subjects.move_up(order[0]), "MoveSubjectUp", "NoUpperPosition", ()),
    (lambda: subjects.move_down(practice), "MoveSubjectDown", "NoLowerPosition", ()),
):
    try:
        call()
    except collomatique.SubjectsError as error:
        assert isinstance(error, collomatique.UpdateError)
        assert isinstance(error, collomatique.Error)
        assert str(error)
        assert error.op == op
        assert error.case == case
        # A case naming nothing carries the empty tuple.
        assert error.details == details
    else:
        raise AssertionError(f"{op}/{case} must refuse")

# The one field the ops cannot carry, and the mirror says so loudly rather than
# dropping it: no subject op takes the excluded periods, so `add` refuses a value
# that excludes anything — a new subject runs on every period the document holds.
try:
    subjects.add(
        collomatique.SubjectData("Vol sur balai", excluded_periods={first_period})
    )
except ValueError as error:
    assert not isinstance(error, collomatique.Error)
    assert "set_period_status" in str(error)
else:
    raise AssertionError("a value that excludes a period must be refused")
assert len(subjects) == before + 2

# And `update` refuses a value whose exclusions differ from what the document
# holds for that subject — in either direction, since it is the difference that
# is refused and not the emptiness.
for value in (
    collomatique.SubjectData("Sortilèges"),
    collomatique.SubjectData("Sortilèges", excluded_periods={periods[1]}),
):
    try:
        subjects.update(created, value)
    except ValueError as error:
        assert not isinstance(error, collomatique.Error)
        assert "set_period_status" in str(error)
    else:
        raise AssertionError("a value naming other exclusions must be refused")
assert created.excluded_periods == frozenset({first_period})
assert created.name == "Sortilèges"

# A read-modify-write never meets that refusal: `to_data()` fills the field with
# the subject's own exclusions, so the value is accepted as it comes.
same = created.to_data()
same.name = "Sortilèges et potions"
subjects.update(created, same)
assert created.name == "Sortilèges et potions"
assert created.excluded_periods == frozenset({first_period})

# And the exclusion goes the way it came.
included = subjects.set_period_status(created, first_period.id, True)
assert included.warnings == []
assert created.excluded_periods == frozenset()

# A subject nothing names takes nothing with it: the removal is the same op the
# cascading one is, and the answer is empty because this one held nothing.
gone = subjects.remove(practice)
assert isinstance(gone, collomatique.OpResult)
assert not hasattr(gone, "created")
assert gone.warnings == []
assert practice not in subjects
assert len(subjects) == before + 1

try:
    practice.name
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed subject must raise")

bare = collomatique.SubjectData("Vol sur balai")

# A dead subject is refused by the argument convention, and not by the model: the
# five ops that name one can object to an id the document does not hold, and that
# is caught here, where the message can say which argument was wrong.
for call in (
    lambda: subjects.remove(practice),
    lambda: subjects.update(practice, bare),
    lambda: subjects.update(practice.id, bare),
    lambda: subjects.move_up(practice),
    lambda: subjects.move_down(practice.id),
    lambda: subjects.set_period_status(practice, first_period, False),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead subject must not be written through")

# And so is a dead period, which is the second addressee of the one op that has
# two.
other_doc = collomatique.load(source)
foreign_period = list(other_doc.periods)[0]
foreign_subject = list(other_doc.subjects)[0]

try:
    subjects.set_period_status(created, foreign_period, False)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a period of another document must not resolve")

try:
    subjects.move_up(foreign_subject)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a subject of another document must not resolve")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    subjects.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# A value refused at the boundary is refused before any op is built.
try:
    subjects.add(collomatique.SubjectData("Vol sur balai", excluded_periods={3}))
except TypeError:
    pass
else:
    raise AssertionError("an excluded period that is not a period must be refused")
assert len(subjects) == before + 1

# A call that is wrong about both names the *subject*: a value meant for nothing
# is moot, so the addressee is resolved first.
try:
    subjects.update(
        practice, collomatique.SubjectData("Vol sur balai", interrogation=3)
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# ------------------------------------------------------------------ the cascades

# What the three cascades need beyond what the example carries: an
# incompatibility built on the subject, a pairing rule naming it, and one colle
# standing in one of its slots. All three go through the surface the earlier
# pieces published, and all three are undone again at the end of this section.
cell_slot, cell_week = next(
    (slot, week)
    for slot in rich.slots
    for week in first_period.weeks
    if doc.is_interrogation_possible(slot, week)
)
incompat = doc.incompats.add(
    collomatique.IncompatData(
        "Préparation des chaudrons",
        rich,
        slots=[
            collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(8, 0), 60)
        ],
    )
).created
rule = doc.pairings.add(
    collomatique.PairingRuleData(
        collomatique.PairingRuleSideData(rich),
        collomatique.PairingRuleSideData(other),
    )
).created
doc.colloscope.set_interrogation(cell_slot, cell_week, {0})
assert doc.colloscope.interrogation(cell_slot, cell_week) == frozenset({0})

# What the document holds about that subject now, which is what the cascades
# below take apart. Rust asserts on its own side that the example really carries
# each of these.
rich_slots = list(rich.slots)
rich_teacher = next(teacher for teacher in doc.teachers if rich in teacher.subjects)
rich_association = doc.group_lists.association_for(first_period, rich)
rich_row = doc.assignments[first_period, rich]
assert rich_association is not None
assert rich_row
assert doc.balancing.override_for(rich) is not None

# The colle stands in the first of the subject's slots that is really active on a
# week of the first period, so the two slot orders below differ by exactly that
# one: it is the slot whose cell has to be cleared before it can itself go.
others = [slot for slot in rich_slots if slot != cell_slot]
assert len(others) == len(rich_slots) - 1


def slots_of(warnings):
    """The slot every `DeleteSlot` of a warning list names, in the list's order."""

    return [w.details["slot"] for w in warnings if w.kind == "DeleteSlot"]


# Taking a subject off a period drops the three things it held there, and in the
# model's own order: the enrolments of that row, the colles already written on
# the period's weeks, and the group list it used. The colles sit in the middle of
# that order on purpose — were the association dropped first, the group bound at
# the coordinate would fall to zero, and the cells would die one group at a time
# instead, which is a different sentence about the same document.
off = subjects.set_period_status(rich, first_period, False)
assert [w.kind for w in off.warnings] == [
    "ClearAssignmentRow",
    "ClearInterrogationCell",
    "UnassignGroupList",
]
assert [w.details for w in off.warnings] == [
    {"period": first_period.id, "subject": rich.id},
    {"slot": cell_slot.id, "week": cell_week.id},
    {"period": first_period.id, "subject": rich.id},
]
# The write asked for the exclusion itself, so no repair hangs off another: this
# cascade is flat where the two below are trees.
assert all(w.parent is None for w in off.warnings)
assert all(str(w) for w in off.warnings)

assert rich.excluded_periods == frozenset({first_period})
assert doc.assignments[first_period, rich] == frozenset()
assert doc.colloscope.interrogation(cell_slot, cell_week) is None
assert doc.group_lists.association_for(first_period, rich) is None
# The subject only left that one period: the others are untouched, and so is
# everything the subject holds outside a period — its slots and its teacher.
assert doc.group_lists.association_for(periods[1], rich) is not None
assert doc.assignments[periods[1], rich]
assert list(rich.slots) == rich_slots
assert rich in rich_teacher.subjects

doc.undo()
assert rich.excluded_periods == frozenset()
assert doc.assignments[first_period, rich] == rich_row
assert doc.colloscope.interrogation(cell_slot, cell_week) == frozenset({0})
assert doc.group_lists.association_for(first_period, rich) == rich_association

# Switching the interrogations off is the update that costs the most: a subject
# without colles may not be taught by anybody, may not hold slots, may not use a
# group list and may not carry balancing options of its own, and a pairing rule
# naming it relates nothing.
#
# The order is not the canonical order of those five, and that is the point: the
# engine rolls the failing write back while it looks for a repair, so striking
# the subject off the teacher's list cannot land while they still hold its slots
# — the slots go first, as that repair's own children. And the slot the colle
# stands in waits for its cell, which is its own child in turn.
silenced = rich.to_data()
silenced.interrogation = None
off_colles = subjects.update(rich, silenced)
warnings = off_colles.warnings

assert [w.kind for w in warnings] == (
    ["DeleteSlot"] * len(others)
    + ["ClearInterrogationCell", "DeleteSlot", "RemoveTeacherSubject"]
    + ["UnassignGroupList"] * len(periods)
    + ["ClearSubjectBalancing", "DeletePairingRule"]
)
assert slots_of(warnings) == [slot.id for slot in others] + [cell_slot.id]
cleared, taken, struck = (
    warnings[len(others)],
    warnings[len(others) + 1],
    warnings[len(others) + 2],
)
assert cleared.details == {"slot": cell_slot.id, "week": cell_week.id}
assert struck.details == {"teacher": rich_teacher.id, "subject": rich.id}
assert [w.details for w in warnings[-len(periods) - 2 : -2]] == [
    {"period": period.id, "subject": rich.id} for period in periods
]
assert warnings[-2].details == {"subject": rich.id}
assert warnings[-1].details == {"rule": rule.id}

# The tree the depth-first search left: every slot went so that the teacher
# could be struck off, and the cell was cleared so that its own slot could go.
assert all(w.parent == struck for w in warnings if w.kind == "DeleteSlot")
assert cleared.parent == taken
assert [w for w in warnings if w.parent is None] == (
    [struck] + warnings[-len(periods) - 2 :]
)

assert rich.interrogation is None
assert rich.slots == ()
assert rich not in rich_teacher.subjects
assert doc.group_lists.association_for(first_period, rich) is None
assert doc.balancing.override_for(rich) is None
assert rule not in doc.pairings
# The enrolments deliberately survive: being registered in a subject says
# nothing about having colles in it. So does the incompatibility, which
# constrains the subject's students and never needed a colle either.
assert doc.assignments[first_period, rich] == rich_row
assert incompat in doc.incompats

doc.undo()
assert rich.interrogation is not None
assert list(rich.slots) == rich_slots
assert rich in rich_teacher.subjects
assert rule in doc.pairings
assert doc.colloscope.interrogation(cell_slot, cell_week) == frozenset({0})

# The removal, and the whole of what the most referenced entity in the document
# drags along: its teacher and their slots in it, the colles standing in those
# slots, its incompatibility, the pairing rule naming it, its own balancing
# options, its enrolment rows and its group-list associations — every reference
# site there is, in the sites' own declaration order.
removed = subjects.remove(rich)
warnings = removed.warnings

assert [w.kind for w in warnings] == (
    ["DeleteSlot"] * len(others)
    + [
        "ClearInterrogationCell",
        "DeleteSlot",
        "RemoveTeacherSubject",
        "DeleteIncompat",
        "DeletePairingRule",
        "ClearSubjectBalancing",
    ]
    + ["ClearAssignmentRow"] * len(periods)
    + ["UnassignGroupList"] * len(periods)
)
assert slots_of(warnings) == [slot.id for slot in others] + [cell_slot.id]
cleared, taken, struck = (
    warnings[len(others)],
    warnings[len(others) + 1],
    warnings[len(others) + 2],
)
assert cleared.details == {"slot": cell_slot.id, "week": cell_week.id}
assert struck.details == {"teacher": rich_teacher.id, "subject": rich.id}
assert warnings[len(others) + 3].details == {"incompat": incompat.id}
assert warnings[len(others) + 4].details == {"rule": rule.id}
assert warnings[len(others) + 5].details == {"subject": rich.id}
assert [w.details for w in warnings[-2 * len(periods) :]] == [
    {"period": period.id, "subject": rich.id} for period in periods
] * 2

# The same tree as above, for the same reason, under a write that asked for the
# subject itself.
assert all(w.parent == struck for w in warnings if w.kind == "DeleteSlot")
assert cleared.parent == taken

assert rich not in subjects
assert len(subjects) == before
assert rich not in rich_teacher.subjects
assert incompat not in doc.incompats
assert rule not in doc.pairings

# What the cascade removed reads as gone, exactly as the subject itself does.
for read in (lambda: rich.name, lambda: rich_slots[0].teacher, lambda: incompat.name):
    try:
        read()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("what the cascade removed must read as gone")

# Undoing the removal puts back everything it took, in one step.
doc.undo()
assert rich in subjects
assert list(rich.slots) == rich_slots
assert incompat in doc.incompats
assert rule in doc.pairings
assert doc.assignments[first_period, rich] == rich_row
assert doc.balancing.override_for(rich) is not None
assert doc.colloscope.interrogation(cell_slot, cell_week) == frozenset({0})

# And the three setup writes go the way they came.
doc.undo()
doc.undo()
doc.undo()
assert doc.colloscope.interrogation(cell_slot, cell_week) is None
assert rule not in doc.pairings
assert incompat not in doc.incompats

# ------------------------------------------------------------------- the undoing

# Each accepted call was its own undo slot, named by the operation itself.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert practice in subjects

assert doc.undo_name == include_label
doc.undo()
assert created.excluded_periods == frozenset({first_period})

assert doc.undo_name == update_label
doc.undo()
assert created.name == "Sortilèges"

assert doc.undo_name == exclude_label
doc.undo()
assert created.excluded_periods == frozenset()

assert doc.undo_name == move_down_label
doc.undo()
assert created.index == before - 1

assert doc.undo_name == move_up_label
doc.undo()
assert created.index == before

# The three week-pattern writes go the way they came, and each is its own undo
# slot: the op that carries the pattern is the ordinary update, so its label is
# the ordinary one too.
assert doc.undo_name == update_label
doc.undo()
assert created.week_pattern is None
doc.undo()
assert created.week_pattern == pattern
doc.undo()
assert created.week_pattern is None

assert doc.undo_name == update_label
doc.undo()
assert created.interrogation is None
doc.undo()
assert created.name == "Sortilèges et enchantements"
doc.undo()
assert created.interrogation.duration == 60

assert doc.undo_name == add_label
doc.undo()
assert practice not in subjects
doc.undo()
assert created not in subjects
assert len(subjects) == before
assert list(subjects) == order
assert doc.can_undo is False
