import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

periods = doc.periods
assert isinstance(periods, collomatique.Periods)

period_list = list(periods)
assert len(period_list) == len(periods)
assert all(isinstance(period, collomatique.Period) for period in period_list)

# Iteration is display order, and `.index` is the position in it.
period_indices = [period.index for period in period_list]

# Indexing takes an id or a handle, and hands back an equal handle either way.
for period in period_list:
    assert periods[period.id] == period
    assert periods[period] == period
    assert periods.get(period.id) == period
    assert periods.get(period) == period
    assert period.id in periods
    assert period in periods

# A handle is a view, not the object the collection keeps: two of them for the
# same period are different objects that compare and hash the same, so a set or
# a dict keyed on handles works.
again = periods[period_list[0].id]
assert again is not period_list[0]
assert again == period_list[0]
assert hash(again) == hash(period_list[0])
assert len({again, period_list[0]}) == 1

# Handles identify; they do not order. Ids are the things that order.
try:
    period_list[0] < period_list[1]
except TypeError:
    pass
else:
    raise AssertionError("ordering two handles must raise")

# A handle is something the document hands out.
try:
    collomatique.Period()
except TypeError:
    pass
else:
    raise AssertionError("a handle must not be constructible")

# And there are no setters anywhere on the read surface: the old api's silent
# lost write is a loud error here.
try:
    period_list[0].index = 3
except AttributeError:
    pass
else:
    raise AssertionError("assigning to a handle attribute must raise")

# Anything that is not an id or a handle of this document simply names nothing.
assert 3 not in periods
assert periods.get(3) is None
try:
    periods[3]
except KeyError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

weeks = doc.weeks
assert isinstance(weeks, collomatique.Weeks)

week_list = list(weeks)
assert len(week_list) == len(weeks)
assert all(isinstance(week, collomatique.Week) for week in week_list)

week_indices = [week.index for week in week_list]
week_interrogations = [week.interrogations for week in week_list]
week_annotations = [week.annotation for week in week_list]
week_mondays = [week.monday for week in week_list]
week_period_indices = [week.period.index for week in week_list]

for week in week_list:
    assert weeks[week.id] == week
    assert weeks[week] == week
    assert week.id in weeks
    assert week in weeks

# The global order *is* the concatenation of the periods' own weeks, taken in
# display order — that is what makes `week.index` a global index.
assert [week.id for period in period_list for week in period.weeks] == [
    week.id for week in week_list
]
weeks_per_period = [len(period.weeks) for period in period_list]

# A period's weeks come back as a tuple: a read never returns anything mutable.
assert isinstance(period_list[0].weeks, tuple)

# An annotation is a string or `None`, and never the empty string.
assert all(
    text is None or (isinstance(text, str) and text != "") for text in week_annotations
)
assert any(text is None for text in week_annotations)
assert any(text is not None for text in week_annotations)

# The mondays run one week apart from the start date, in global order.
first_week = periods.first_week
assert isinstance(first_week, datetime.date)
assert all(isinstance(monday, datetime.date) for monday in week_mondays)
assert week_mondays == [
    first_week + datetime.timedelta(days=7 * index) for index in week_indices
]

# A handle is live: clearing the start date takes the dates away from the very
# handles that were minted before it, without invalidating anything else.
periods.clear_first_week()
assert doc.periods.first_week is None
assert [week.monday for week in week_list] == [None] * len(week_list)
assert [week.index for week in week_list] == week_indices

# A handle from another document names nothing here, whatever its id says — and
# the id, which knows no document, names the other document's own period.
other = collomatique.load(source)
other_periods = list(other.periods)
assert other.periods[period_list[0].id] == other_periods[0]
assert period_list[0] != other_periods[0]
assert period_list[0] not in other.periods
assert other.periods.get(period_list[0]) is None
try:
    other.periods[period_list[0]]
except KeyError:
    pass
else:
    raise AssertionError("a handle of another document must not resolve")

# Nor does a handle of another kind, id or no id.
assert period_list[0] != week_list[0]
assert week_list[0] not in periods
