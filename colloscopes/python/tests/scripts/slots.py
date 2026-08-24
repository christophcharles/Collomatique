import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

slots = doc.slots
assert isinstance(slots, collomatique.Slots)

slot_list = list(slots)
assert len(slot_list) == len(slots)
assert all(isinstance(slot, collomatique.Slot) for slot in slot_list)

# The walk is user order: the subjects as `doc.subjects` shows them, each
# followed by its own slots in theirs. `subject.slots` is one of those groups,
# and the walk is the groups laid end to end — the same walk the design's §4
# example makes. The model keeps no single global slots order to mirror.
by_subject = {subject: list(subject.slots) for subject in doc.subjects}
assert [
    slot for subject in doc.subjects for slot in by_subject[subject]
] == slot_list

# `.index` is the position inside the subject, so it restarts at every group
# rather than running along the walk.
assert all(isinstance(subject.slots, tuple) for subject in doc.subjects)
assert all(
    [slot.index for slot in group] == list(range(len(group)))
    for group in by_subject.values()
)
assert all(
    slot.subject == subject for subject, group in by_subject.items() for slot in group
)

# The example holds subjects that carry no slots at all — the ones that hold no
# colles either. An empty tuple is what that reads as: not `None`, not a failure.
assert any(len(group) == 0 for group in by_subject.values())
assert any(len(group) > 0 for group in by_subject.values())

slot_indices = [slot.index for slot in slot_list]
slot_subject_indices = [slot.subject.index for slot in slot_list]
slot_teacher_surnames = [slot.teacher.surname for slot in slot_list]
slot_extra_info = [slot.extra_info for slot in slot_list]
slot_costs = [slot.cost for slot in slot_list]

# The day is one of the seven members, which are class attributes and compare by
# value: two mondays read out of the document are equal, and the same day is the
# same key in a dict.
weekday_names = {
    collomatique.Weekday.MONDAY: "monday",
    collomatique.Weekday.TUESDAY: "tuesday",
    collomatique.Weekday.WEDNESDAY: "wednesday",
    collomatique.Weekday.THURSDAY: "thursday",
    collomatique.Weekday.FRIDAY: "friday",
    collomatique.Weekday.SATURDAY: "saturday",
    collomatique.Weekday.SUNDAY: "sunday",
}
assert len(weekday_names) == 7
assert all(isinstance(day, collomatique.Weekday) for day in weekday_names)
slot_weekdays = [weekday_names[slot.weekday] for slot in slot_list]

# The old api compared days by identity, so a day read twice was two different
# things. This one is a real value.
assert slot_list[0].weekday == slot_list[0].weekday
assert hash(slot_list[0].weekday) == hash(slot_list[0].weekday)
assert collomatique.Weekday.MONDAY != collomatique.Weekday.TUESDAY

# A time of day, with the whole-minute precision the model stores.
slot_start_times = [slot.start_time for slot in slot_list]
assert all(isinstance(time, datetime.time) for time in slot_start_times)
assert all(time.second == 0 and time.microsecond == 0 for time in slot_start_times)

# The pattern is a live handle or `None`, and `None` means every week.
slot_pattern_names = [
    None if slot.week_pattern is None else slot.week_pattern.name for slot in slot_list
]
assert all(
    slot.week_pattern is None or slot.week_pattern in doc.week_patterns
    for slot in slot_list
)

# The example is worth reading here: some slots carry a pattern and some do not.
assert any(name is not None for name in slot_pattern_names)
assert any(name is None for name in slot_pattern_names)

# A slot has no duration of its own — the subject fixes it, and this is the way
# to it.
assert all(
    isinstance(slot.subject.interrogation.duration, int)
    for slot in slot_list
    if slot.subject.interrogation is not None
)
assert not hasattr(slot_list[0], "duration")

# Whether a colle can really happen there, over every (slot, week) pair.
weeks = list(doc.weeks)
possibility = [
    [doc.is_interrogation_possible(slot, week) for week in weeks] for slot in slot_list
]
assert all(isinstance(answer, bool) for row in possibility for answer in row)

# Both arguments take a handle or an id, and answer the same either way.
assert [
    [doc.is_interrogation_possible(slot.id, week.id) for week in weeks]
    for slot in slot_list
] == possibility

# Indexing takes an id or a handle, and hands back an equal handle either way.
for slot in slot_list:
    assert slots[slot.id] == slot
    assert slots[slot] == slot
    assert slots.get(slot.id) == slot
    assert slot.id in slots
    assert slot in slots

assert 3 not in slots
assert slots.get(3) is None
try:
    slots[3]
except KeyError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# A handle is a view, not the object the collection keeps: two of them for the
# same slot are different objects that compare and hash the same. So a dict can
# be keyed on slots, which is what a script writing out a grid does.
again = slots[slot_list[0].id]
assert again is not slot_list[0]
assert hash(again) == hash(slot_list[0])
assert len({again, slot_list[0]}) == 1
assert len(set(slot_list)) == len(slot_list)

# A slot is never equal to something that is not one — that is an answer and not
# an error. Handles identify; they do not order, which is what ids are for.
assert slot_list[0] != 3
assert slot_list[0] != "jeudi"
assert not (slot_list[0] == slot_list[0].subject)
assert slot_list[0] != None  # noqa: E711 — `is not` would not call `__eq__`
try:
    slot_list[0] < slot_list[1]
except TypeError:
    pass
else:
    raise AssertionError("ordering two handles must raise")

# A handle is something the document hands out, and it has no setters.
try:
    collomatique.Slot()
except TypeError:
    pass
else:
    raise AssertionError("a handle must not be constructible")
try:
    slot_list[0].cost = 12
except AttributeError:
    pass
else:
    raise AssertionError("assigning to a handle attribute must raise")

# What is not a reference at all was never a question about this document, so it
# is a `TypeError` rather than a stale anything.
for rubbish in (3, "jeudi 13h00", weeks[0]):
    try:
        doc.is_interrogation_possible(rubbish, weeks[0])
    except TypeError:
        pass
    else:
        raise AssertionError("a slot argument takes a Slot or a SlotId")
for rubbish in (3, "semaine 2", slot_list[0]):
    try:
        doc.is_interrogation_possible(slot_list[0], rubbish)
    except TypeError:
        pass
    else:
        raise AssertionError("a week argument takes a Week or a WeekId")

# A handle from another document names nothing here, whatever its id says — and
# the two conventions of the api part company on it: a lookup answers, an
# argument raises. `other` is this same file loaded twice, so its slots carry the
# very ids this document uses, and the refusal must say « somebody else's »
# rather than « missing »: the id is not missing here, it names another slot.
other = collomatique.load(source)
other_slot = list(other.slots)[0]
assert other_slot not in slots
assert slots.get(other_slot) is None
assert other.slots[other_slot.id] == other_slot
assert other_slot.id in slots
try:
    slots[other_slot]
except KeyError:
    pass
else:
    raise AssertionError("a handle of another document must not resolve")
try:
    doc.is_interrogation_possible(other_slot, weeks[0])
except collomatique.StaleHandleError as error:
    assert "Slot" in str(error)
    assert "another document" in str(error)
    assert "is not in this document" not in str(error)
else:
    raise AssertionError("a slot argument of another document must raise")

# The reprs name the slot the way the application does: the day in french —
# « Jeudi » — and the time.
first_repr = repr(slot_list[0])
assert first_repr.startswith("<Slot #")
assert {
    collomatique.Weekday.MONDAY: "Lundi",
    collomatique.Weekday.TUESDAY: "Mardi",
    collomatique.Weekday.WEDNESDAY: "Mercredi",
    collomatique.Weekday.THURSDAY: "Jeudi",
    collomatique.Weekday.FRIDAY: "Vendredi",
    collomatique.Weekday.SATURDAY: "Samedi",
    collomatique.Weekday.SUNDAY: "Dimanche",
}[slot_list[0].weekday] in first_repr
assert (
    "%02d:%02d" % (slot_start_times[0].hour, slot_start_times[0].minute)
    in first_repr
)
assert repr(slots) == "<collomatique.Slots count=%d>" % len(slot_list)
