import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

incompats = doc.incompats
assert isinstance(incompats, collomatique.Incompats)
assert repr(incompats) == "<collomatique.Incompats count=%d>" % len(incompats)

incompat_list = list(incompats)
assert len(incompat_list) == len(incompats)
assert all(isinstance(incompat, collomatique.Incompat) for incompat in incompat_list)

# The order is the ids', which is the one order the model keeps.
incompat_names = [incompat.name for incompat in incompat_list]
incompat_subject_names = [incompat.subject.name for incompat in incompat_list]
minimum_free_slots = [incompat.minimum_free_slots for incompat in incompat_list]
assert all(count >= 1 for count in minimum_free_slots)

# The subject is a live handle of this document, whatever it holds.
assert all(incompat.subject in doc.subjects for incompat in incompat_list)

# Every incompatibility of the example applies on every week — `None` means no
# pattern, not a missing one.
week_pattern_present = [incompat.week_pattern is not None for incompat in incompat_list]
assert not any(week_pattern_present)
assert all(
    incompat.week_pattern is None or incompat.week_pattern in doc.week_patterns
    for incompat in incompat_list
)

# The busy windows are TimeSlot values: the day one of the seven members, the
# time a whole minute, the duration an int. The windows are data, not handles —
# a script compares them by value.
assert all(isinstance(incompat.slots, tuple) for incompat in incompat_list)
assert all(
    isinstance(slot, collomatique.TimeSlot)
    for incompat in incompat_list
    for slot in incompat.slots
)
weekday_names = {
    collomatique.Weekday.MONDAY: "monday",
    collomatique.Weekday.TUESDAY: "tuesday",
    collomatique.Weekday.WEDNESDAY: "wednesday",
    collomatique.Weekday.THURSDAY: "thursday",
    collomatique.Weekday.FRIDAY: "friday",
    collomatique.Weekday.SATURDAY: "saturday",
    collomatique.Weekday.SUNDAY: "sunday",
}
assert all(
    isinstance(slot.weekday, collomatique.Weekday)
    and isinstance(slot.start_time, datetime.time)
    and slot.start_time.second == 0
    and slot.start_time.microsecond == 0
    and isinstance(slot.duration, int)
    and slot.duration >= 1
    for incompat in incompat_list
    for slot in incompat.slots
)
incompat_slots = [
    [
        (weekday_names[slot.weekday], slot.start_time, slot.duration)
        for slot in incompat.slots
    ]
    for incompat in incompat_list
]
assert all(len(slots) > 0 for slots in incompat_slots)

# A window read out of the document is the value a script names: the example's
# first incompatibility is « Lundi Midi », whose windows are monday noon and
# one o'clock, one hour each.
first = incompat_list[0]
assert first.name == "Lundi Midi"
assert first.slots == (
    collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(12, 0), 60),
    collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(13, 0), 60),
)
assert first.slots[0] != collomatique.TimeSlot(
    collomatique.Weekday.MONDAY, datetime.time(12, 0), 61
)
assert first.slots[0] != collomatique.TimeSlot(
    collomatique.Weekday.TUESDAY, datetime.time(12, 0), 60
)
assert hash(first.slots[0]) == hash(
    collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(12, 0), 60)
)
assert first.slots[0] != 60
assert first.slots[0] != "monday 12:00"

# Indexing takes an id or a handle, and hands back an equal handle either way.
for incompat in incompat_list:
    assert incompats[incompat.id] == incompat
    assert incompats[incompat] == incompat
    assert incompats.get(incompat.id) == incompat
    assert incompat.id in incompats
    assert incompat in incompats

assert 3 not in incompats
assert incompats.get(3) is None
try:
    incompats[3]
except KeyError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# A handle is a view, not the object the collection keeps: two of them for the
# same incompatibility are different objects that compare and hash the same.
again = incompats[incompat_list[0].id]
assert again is not incompat_list[0]
assert hash(again) == hash(incompat_list[0])
assert len({again, incompat_list[0]}) == 1
assert len(set(incompat_list)) == len(incompat_list)

# An incompatibility is never equal to something that is not one — that is an
# answer and not an error. Handles identify; they do not order, which is what
# ids are for.
assert incompat_list[0] != 3
assert incompat_list[0] != "Lundi Midi"
assert incompat_list[0] != None  # noqa: E711 — `is not` would not call `__eq__`
try:
    incompat_list[0] < incompat_list[1]
except TypeError:
    pass
else:
    raise AssertionError("ordering two handles must raise")

# A handle is something the document hands out, and it has no setters.
try:
    collomatique.Incompat()
except TypeError:
    pass
else:
    raise AssertionError("a handle must not be constructible")
try:
    incompat_list[0].name = "x"
except AttributeError:
    pass
else:
    raise AssertionError("assigning to a handle attribute must raise")

# A handle from another document names nothing here, whatever its id says.
# `other` is this same file loaded twice, so its incompatibilities carry the
# very ids this document uses.
other = collomatique.load(source)
other_incompat = list(other.incompats)[0]
assert other_incompat not in incompats
assert incompats.get(other_incompat) is None
assert other_incompat.id in incompats
assert other.incompats[other_incompat.id] == other_incompat
try:
    incompats[other_incompat]
except KeyError:
    pass
else:
    raise AssertionError("a handle of another document must not resolve")

# The reprs name the incompatibility the way a log wants to read it.
assert repr(incompat_list[0]).startswith("<Incompat #")
assert "Lundi Midi" in repr(incompat_list[0])
