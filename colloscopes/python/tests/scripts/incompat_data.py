import dataclasses
import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope. Its incompatibilities all
# apply on every week — none carries a pattern of its own — and its windows
# are busy ones: several windows on one of them, none crossing midnight.
doc = collomatique.load(source)

incompat_list = list(doc.incompats)

# What a handle hands back detached, in the collection's order, which is the
# order rust compares them in.
incompat_values = [incompat.to_data() for incompat in incompat_list]

assert all(isinstance(d, collomatique.IncompatData) for d in incompat_values)

# The fields as python sees them, so that a conversion wrong in both directions
# at once — the day and the time swapped, say — cannot pass rust's round-trip
# comparison by cancelling itself out. The entities are named by their place in
# the walk they belong to, since an id is opaque and means nothing written down.
subject_positions = {
    subject.id: position for position, subject in enumerate(doc.subjects)
}
pattern_positions = {
    pattern.id: position for position, pattern in enumerate(doc.week_patterns)
}

# The day is one of the seven members, which are class attributes rather than an
# `enum.Enum`, so the script names them itself and rust holds the other half of
# this dictionary.
weekday_names = {
    collomatique.Weekday.MONDAY: "monday",
    collomatique.Weekday.TUESDAY: "tuesday",
    collomatique.Weekday.WEDNESDAY: "wednesday",
    collomatique.Weekday.THURSDAY: "thursday",
    collomatique.Weekday.FRIDAY: "friday",
    collomatique.Weekday.SATURDAY: "saturday",
    collomatique.Weekday.SUNDAY: "sunday",
}

incompat_names = [d.name for d in incompat_values]
incompat_subject_indices = [subject_positions[d.subject] for d in incompat_values]
incompat_slots = [
    [
        (weekday_names[slot.weekday], slot.start_time, slot.duration)
        for slot in d.slots
    ]
    for d in incompat_values
]
minimum_free_slots = [d.minimum_free_slots for d in incompat_values]
week_pattern_positions = [
    None if d.week_pattern is None else pattern_positions[d.week_pattern]
    for d in incompat_values
]

# A value holds ids, never handles: it is detached, and a handle would carry the
# document with it.
assert all(isinstance(d.subject, collomatique.SubjectId) for d in incompat_values)
assert all(
    d.week_pattern is None or isinstance(d.week_pattern, collomatique.WeekPatternId)
    for d in incompat_values
)

# The subject in the value is the incompatibility's own.
assert [d.subject for d in incompat_values] == [
    incompat.subject.id for incompat in incompat_list
]

# The windows come out as the read surface's own vocabulary — the same leaf
# value, not a second spelling of it — and as the mutable container a value is
# for, not the read surface's tuple: `d.slots.append(...)` writes where it
# looks like it writes.
assert all(isinstance(d.slots, list) for d in incompat_values)
assert all(
    isinstance(slot, collomatique.TimeSlot)
    for d in incompat_values
    for slot in d.slots
)
assert [d.week_pattern for d in incompat_values] == [
    None if incompat.week_pattern is None else incompat.week_pattern.id
    for incompat in incompat_list
]

# Every incompatibility of the example applies on every week — `None` means no
# pattern, not a missing one — and the minimums are all at least one.
assert not any(d.week_pattern is not None for d in incompat_values)
assert all(count >= 1 for count in minimum_free_slots)

# A fresh object every call. Two of them are equal and share nothing, so writing
# to one is invisible to the other and to the document.
first = incompat_list[0]
again = first.to_data()
assert again == incompat_values[0]
assert again is not incompat_values[0]
again.slots.append(
    collomatique.TimeSlot(collomatique.Weekday.TUESDAY, datetime.time(12, 0), 60)
)
assert incompat_values[0].slots != again.slots
assert list(first.slots) != again.slots

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
scratch = collomatique.IncompatData(3, "personne")
scratch.slots = None
scratch.minimum_free_slots = "beaucoup"

# And a value has no identity: an id names a place in a document, and a value
# has none. Updating an existing incompatibility will pass the id as the
# method's argument.
assert not hasattr(incompat_values[0], "id")

# The field order of each class, which is what a positional call depends on:
# required first, then the defaulted ones in the order the handle shows them.
assert [f.name for f in dataclasses.fields(collomatique.IncompatData)] == [
    "name",
    "subject",
    "slots",
    "minimum_free_slots",
    "week_pattern",
]

# The class is the module's, not a private submodule's, whichever door a script
# comes in through.
from collomatique._data import IncompatData as _same_class  # noqa: E402

assert _same_class is collomatique.IncompatData
assert collomatique.IncompatData.__module__ == "collomatique"

# A field that names an entity takes a handle or an id, interchangeably. The
# values below extract to the same incompatibility and — this is the wart — do
# not compare equal, because a dataclass stores what it was given.
subject = list(doc.subjects)[0]
pattern = list(doc.week_patterns)[0]
noon = collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(12, 0), 60)

incompat_by_handle = collomatique.IncompatData(
    "Mercredi après-midi",
    subject,
    slots=[noon],
    minimum_free_slots=2,
    week_pattern=pattern,
)
incompat_by_id = collomatique.IncompatData(
    "Mercredi après-midi",
    subject.id,
    slots=[noon],
    minimum_free_slots=2,
    week_pattern=pattern.id,
)
assert incompat_by_handle != incompat_by_id

# Nothing but the two fields an incompatibility cannot do without, so rust can
# pin what the defaulted three come out as: no windows, one window free at
# least, and every week.
bare_incompat = collomatique.IncompatData("", subject)
assert bare_incompat.slots == []
assert bare_incompat.minimum_free_slots == 1
assert bare_incompat.week_pattern is None

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
not_a_name = collomatique.IncompatData(3, subject)
not_a_subject = collomatique.IncompatData("Lundi Midi", 3)
not_a_slots = collomatique.IncompatData("Lundi Midi", subject, slots=3)
not_a_time_slot = collomatique.IncompatData("Lundi Midi", subject, slots=["jeudi"])
not_a_minimum = collomatique.IncompatData("Lundi Midi", subject, minimum_free_slots=0)
not_a_minimum_count = collomatique.IncompatData(
    "Lundi Midi", subject, minimum_free_slots="beaucoup"
)
not_a_pattern = collomatique.IncompatData(
    "Lundi Midi", subject, week_pattern="Lundi Midi"
)

# A window that crosses midnight cannot exist in the model, so the leaf value
# refuses it when it is built — a list in a dataclass only ever holds windows
# the document could hold.
try:
    collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(23, 30), 60)
except ValueError:
    pass
else:
    raise AssertionError("a window crossing midnight must refuse to exist")

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_subject = collomatique.IncompatData(
    "Ailleurs", list(other.subjects)[0]
)
