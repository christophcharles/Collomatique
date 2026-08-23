import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the five labels are the
# french names `ops` gives the five operations of this family — handed in from
# rust so that this pins the operations' own labels and not merely some strings.
#
# Everything else this script leans on it finds for itself, off the document;
# what rust asserts on its own side is that the example really holds those
# shapes.
doc = collomatique.load(source)
slots = doc.slots

before = len(slots)
rules_before = len(doc.slot_pairings)

# A slot only exists in a subject that runs colles — there is no interrogation
# for it to carry otherwise. The document also holds subjects that run none,
# and one of those is what the model's first refusal is asserted on.
with_colles = [subject for subject in doc.subjects if subject.interrogation is not None]
maths, physics = with_colles[0], with_colles[1]
without_colles = next(
    subject for subject in doc.subjects if subject.interrogation is None
)

# A teacher may only be given a slot in a subject they are declared in, so both
# kinds are needed: the one this script builds its slot around, and a stranger
# to that subject for the refusal.
teaches = next(teacher for teacher in doc.teachers if maths in teacher.subjects)
stranger = next(teacher for teacher in doc.teachers if maths not in teacher.subjects)

pattern = list(doc.week_patterns)[0]
maths_slots = [slot for slot in slots if slot.subject == maths]
assert len(maths_slots) >= 2

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = slots.add(
    collomatique.SlotData(
        maths, teaches, collomatique.Weekday.THURSDAY, datetime.time(14, 0)
    )
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# A new slot holds no colle and is related to nothing, so there is nothing for
# the cascade to repair. The result says so rather than the call saying nothing.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.Slot)
assert isinstance(created.id, collomatique.SlotId)
assert created in slots
assert slots[created.id] == created
assert len(slots) == before + 1

assert created.subject == maths
assert created.teacher == teaches
assert created.weekday == collomatique.Weekday.THURSDAY
assert created.start_time == datetime.time(14, 0)
assert created.extra_info == ""
assert created.week_pattern is None
assert created.cost == 0

# A new slot lands last among its subject's slots, which is where the
# application puts one too — and the position is the one inside the subject.
assert created.index == len(maths_slots)
assert [slot for slot in slots if slot.subject == maths] == maths_slots + [created]

assert added.created == created
assert repr(added).startswith("AddResult(created=<Slot #")
assert "warnings=[]" in repr(added)

# Rewriting replaces every field but the subject at once, and the id stays, so
# the handle a script is holding reads the new state.
result = slots.update(
    created,
    collomatique.SlotData(
        maths,
        teaches,
        collomatique.Weekday.FRIDAY,
        datetime.time(9, 30),
        extra_info="Salle 12",
        week_pattern=pattern,
        cost=3,
    ),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")
# The document holds no colle at this point, so putting the slot on a pattern
# repairs nothing.
assert result.warnings == []

assert created.weekday == collomatique.Weekday.FRIDAY
assert created.start_time == datetime.time(9, 30)
assert created.extra_info == "Salle 12"
assert created.week_pattern == pattern
assert created.cost == 3
# The position did not move: an `update` rewrites the slot where it stands.
assert created.index == len(maths_slots)

# The slot is named by an id or by a handle, interchangeably — this is the write
# half of the argument convention — and so are the entities a value names.
slots.update(
    created.id,
    collomatique.SlotData(
        maths.id,
        teaches.id,
        collomatique.Weekday.MONDAY,
        datetime.time(8, 0),
        week_pattern=pattern.id,
        cost=-2,
    ),
)
assert created.weekday == collomatique.Weekday.MONDAY
assert created.start_time == datetime.time(8, 0)
assert created.extra_info == ""
assert created.week_pattern == pattern
assert created.cost == -2

# The family's own pair, and the only way a slot moves: the position is the one
# inside the subject, so `move_up` swaps the slot with the one before it there.
# Nothing else moves — a position is display order, and no colle or rule reads it.
moved = slots.move_up(created)
assert isinstance(moved, collomatique.OpResult)
assert moved.warnings == []
assert created.index == len(maths_slots) - 1
assert [slot for slot in slots if slot.subject == maths] == (
    maths_slots[:-1] + [created, maths_slots[-1]]
)

slots.move_down(created.id)
assert created.index == len(maths_slots)
assert [slot for slot in slots if slot.subject == maths] == maths_slots + [created]

# This is what rust reads back off the disk.
doc.save(target)

# The four refusals this family keeps for the model. Each of them is a statement
# about the document rather than about the value, so each arrives as the
# family's own exception rather than from the value boundary.
too_late = datetime.time(23, 59)
for call, op, case, details in (
    (
        lambda: slots.add(
            collomatique.SlotData(
                without_colles,
                teaches,
                collomatique.Weekday.MONDAY,
                datetime.time(10, 0),
            )
        ),
        "AddNewSlot",
        "SubjectHasNoInterrogation",
        (without_colles.id,),
    ),
    (
        lambda: slots.add(
            collomatique.SlotData(
                maths, stranger, collomatique.Weekday.MONDAY, datetime.time(10, 0)
            )
        ),
        "AddNewSlot",
        "TeacherDoesNotTeachInSubject",
        (stranger.id, maths.id),
    ),
    (
        lambda: slots.add(
            collomatique.SlotData(
                maths, teaches, collomatique.Weekday.MONDAY, too_late
            )
        ),
        "AddNewSlot",
        "SlotOverlapsWithNextDay",
        (),
    ),
    (
        lambda: slots.update(
            created,
            collomatique.SlotData(
                maths, stranger, collomatique.Weekday.MONDAY, datetime.time(10, 0)
            ),
        ),
        "UpdateSlot",
        "TeacherDoesNotTeachInSubject",
        (stranger.id, maths.id),
    ),
    (
        lambda: slots.update(
            created,
            collomatique.SlotData(
                maths, teaches, collomatique.Weekday.MONDAY, too_late
            ),
        ),
        "UpdateSlot",
        "SlotOverlapsWithNextDay",
        (),
    ),
    # A slot at either end of its subject's list has nowhere left to go, and
    # that is a refusal rather than a call that quietly did nothing.
    (
        lambda: slots.move_up(maths_slots[0]),
        "MoveSlotUp",
        "NoUpperPosition",
        (),
    ),
    (
        lambda: slots.move_down(created),
        "MoveSlotDown",
        "NoLowerPosition",
        (),
    ),
):
    try:
        call()
    except collomatique.SlotsError as error:
        assert isinstance(error, collomatique.UpdateError)
        assert isinstance(error, collomatique.Error)
        assert str(error)
        assert error.op == op
        assert error.case == case
        # The entities the model named, as the id classes — the very ones this
        # script is holding. A case naming nothing carries the empty tuple.
        assert error.details == details
    else:
        raise AssertionError(f"{op}/{case} must refuse")

# Nothing of that was written: the slot is what the last accepted write left.
assert len(slots) == before + 1
assert created.teacher == teaches
assert created.start_time == datetime.time(8, 0)
assert created.index == len(maths_slots)

# The one field the ops cannot carry, and the mirror says so loudly rather than
# dropping it: a slot cannot change subject, because the model files it under
# that subject in the very list that gives it its position.
elsewhere = created.to_data()
assert elsewhere.subject == maths.id
elsewhere.subject = physics
try:
    slots.update(created, elsewhere)
except ValueError as error:
    assert not isinstance(error, collomatique.Error)
    assert str(error)
else:
    raise AssertionError("a value naming another subject must be refused")
assert created.subject == maths

# A read-modify-write never meets that refusal: `to_data()` fills the field with
# the slot's own subject, so the value is accepted as it comes.
same = created.to_data()
same.extra_info = "Salle 8"
slots.update(created, same)
assert created.extra_info == "Salle 8"
assert created.subject == maths

# A slot nothing names takes nothing with it: the removal is the same op the
# cascading one is, and the answer is empty because this one held nothing.
gone = slots.remove(created)
assert isinstance(gone, collomatique.OpResult)
assert not hasattr(gone, "created")
assert gone.warnings == []
assert created not in slots
assert len(slots) == before
assert len(doc.slot_pairings) == rules_before

try:
    created.teacher
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed slot must raise")

bare = collomatique.SlotData(
    maths, teaches, collomatique.Weekday.MONDAY, datetime.time(10, 0)
)

# A dead slot is refused by the argument convention, and not by the model: the
# four ops that name one can object to an id the document does not hold, and
# that is caught here, where the message can say which argument was wrong.
for call in (
    lambda: slots.remove(created),
    lambda: slots.update(created, bare),
    lambda: slots.update(created.id, bare),
    lambda: slots.move_up(created),
    lambda: slots.move_down(created.id),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead slot must not be written through")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    slots.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# `other` is this same file loaded twice, so its slots, subjects and teachers
# carry the very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_slot = list(other.slots)[0]
foreign_teacher = list(other.teachers)[0]

try:
    slots.move_up(foreign_slot)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a slot of another document must not resolve")

# A value is refused before any op is built, and nothing is written: a field
# naming another document's entity …
try:
    slots.add(
        collomatique.SlotData(
            maths, foreign_teacher, collomatique.Weekday.MONDAY, datetime.time(10, 0)
        )
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a teacher of another document names nothing here")

# … and a field that was never of the right shape.
try:
    slots.add(
        collomatique.SlotData(maths, teaches, collomatique.Weekday.MONDAY, "midi")
    )
except TypeError:
    pass
else:
    raise AssertionError("a start time that is not a time must be refused")
assert len(slots) == before

# A call that is wrong about both names the *slot*: a value meant for nothing is
# moot, so the addressee is resolved first.
try:
    slots.update(
        created,
        collomatique.SlotData(maths, teaches, collomatique.Weekday.MONDAY, "midi"),
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Each accepted call was its own undo slot, named by the operation itself.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert doc.undo_name == update_label
assert created in slots
assert created.extra_info == "Salle 8"

doc.undo()
assert created.extra_info == ""

assert doc.undo_name == move_down_label
doc.undo()
assert created.index == len(maths_slots) - 1

assert doc.undo_name == move_up_label
doc.undo()
assert created.index == len(maths_slots)

assert doc.undo_name == update_label
doc.undo()
assert created.weekday == collomatique.Weekday.FRIDAY
assert created.cost == 3
doc.undo()
assert created.weekday == collomatique.Weekday.THURSDAY
assert created.week_pattern is None

assert doc.undo_name == add_label
doc.undo()
assert created not in slots
assert len(slots) == before
assert doc.can_undo is False
