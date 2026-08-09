import datetime

import collomatique

# A TimeSlot is a constructible leaf value: a script names the busy window it
# expects and compares it against what a document hands back. Construction
# validates what the model's own `SlotWithDuration::new` validates — a start
# time with zero seconds and microseconds, a duration of at least one minute,
# and no crossing of midnight into the next day — and raises `ValueError` when
# a window refuses to exist.

monday_noon = collomatique.TimeSlot(
    collomatique.Weekday.MONDAY, datetime.time(12, 0), 60
)
assert isinstance(monday_noon, collomatique.TimeSlot)
assert monday_noon.weekday == collomatique.Weekday.MONDAY
assert monday_noon.start_time == datetime.time(12, 0)
assert monday_noon.duration == 60

# A window ending exactly at midnight ends on the same day: the model accepts
# it, and so does the constructor.
until_midnight = collomatique.TimeSlot(
    collomatique.Weekday.FRIDAY, datetime.time(22, 0), 120
)
assert until_midnight.start_time == datetime.time(22, 0)
assert until_midnight.duration == 120

# A value compares by its fields and hashes accordingly — a window made the
# same way is the same window, however many times it is built.
assert monday_noon == collomatique.TimeSlot(
    collomatique.Weekday.MONDAY, datetime.time(12, 0), 60
)
assert monday_noon != collomatique.TimeSlot(
    collomatique.Weekday.TUESDAY, datetime.time(12, 0), 60
)
assert monday_noon != collomatique.TimeSlot(
    collomatique.Weekday.MONDAY, datetime.time(13, 0), 60
)
assert monday_noon != collomatique.TimeSlot(
    collomatique.Weekday.MONDAY, datetime.time(12, 0), 61
)
assert monday_noon != datetime.time(12, 0)
assert monday_noon != 60
assert hash(monday_noon) == hash(
    collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(12, 0), 60)
)
assert len({monday_noon, monday_noon}) == 1
assert len({monday_noon, until_midnight}) == 2

# It matches, so a script can take one apart by shape rather than by getter.
match monday_noon:
    case collomatique.TimeSlot(weekday, start_time, duration):
        assert weekday == collomatique.Weekday.MONDAY
        assert start_time == datetime.time(12, 0)
        assert duration == 60
    case _:
        raise AssertionError("TimeSlot should match on its three arguments")

# The repr names the fields, english like the rest of the module's reprs.
assert repr(monday_noon) == "TimeSlot(weekday=Monday, start_time=12:00, duration=60)"

# Construction validates what the model validates, and says so with `ValueError`
# rather than building a window the document could never hold: a zero-minute
# duration, a time that is not a whole minute, and a crossing of midnight.
refused = [
    lambda: collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(12, 0), 0),
    lambda: collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(12, 0, 30), 60),
    lambda: collomatique.TimeSlot(
        collomatique.Weekday.MONDAY, datetime.time(12, 0, 0, 500), 60
    ),
    lambda: collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(23, 30), 60),
    lambda: collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(23, 0), 120),
]
for build in refused:
    try:
        build()
    except ValueError:
        pass
    else:
        raise AssertionError("a leaf value must refuse what the model refuses")

# A weekday that is not a Weekday member was never a window.
for bad_weekday in (0, "Monday", datetime.time(12, 0)):
    try:
        collomatique.TimeSlot(bad_weekday, datetime.time(12, 0), 60)
    except TypeError:
        pass
    else:
        raise AssertionError("a weekday must be a Weekday member")
