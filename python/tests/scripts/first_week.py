import datetime

import collomatique

# `monday` and `tuesday` are `datetime.date`s the rust side handed in, and
# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

periods = doc.periods
assert isinstance(periods, collomatique.Periods)

# A view rather than a copy: each access builds a new one, and they all read and
# write the same document.
assert doc.periods is not doc.periods

result = periods.set_first_week(monday)
assert isinstance(result, collomatique.OpResult)
# Setting a date breaks nothing, so there is nothing to report — but the result
# says so rather than the call saying nothing at all.
assert result.warnings == []
# The op creates nothing, so the result carries nothing but its warnings.
assert not hasattr(result, "new_id")

got = doc.periods.first_week
assert isinstance(got, datetime.date)
assert got == monday

# The view the script kept sees the write made through the other one.
assert periods.first_week == monday

# The document counts its weeks from a monday. A wednesday is not quietly moved
# back to "its" monday: it is refused, and nothing is written.
try:
    periods.set_first_week(tuesday)
except ValueError as error:
    assert str(tuesday) in str(error)
else:
    raise AssertionError("a date that is not a monday must raise")
assert doc.periods.first_week == monday

doc.save(target)

# Clearing takes the date away and leaves the weeks alone; clearing a document
# that has no date is what was asked for, not an error.
assert periods.clear_first_week().warnings == []
assert doc.periods.first_week is None
periods.clear_first_week()
assert doc.periods.first_week is None

doc.save(cleared_target)

# A refused write is a `collomatique.UpdateError`, which a script that only
# cares that the call failed catches as `collomatique.Error`. Neither first-week
# op can be refused that way, so nothing here raises it.
assert issubclass(collomatique.UpdateError, collomatique.Error)
