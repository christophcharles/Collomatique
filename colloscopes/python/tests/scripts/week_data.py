import dataclasses

import collomatique

# `source` is a throwaway copy of a real colloscope. Its weeks carry both
# shapes — some run colles and some do not, and the annotations are sometimes
# present and sometimes not.
doc = collomatique.load(source)

week_list = list(doc.weeks)
week_values = [week.to_data() for week in week_list]

assert all(isinstance(d, collomatique.WeekData) for d in week_values)

# The fields as python sees them, so that a conversion wrong in both directions
# at once cannot pass rust's round-trip comparison by cancelling itself out.
# The periods are named by their place in the walk, since an id is opaque and
# means nothing written down.
period_positions = {
    period.id: position for position, period in enumerate(doc.periods)
}

value_period_indices = [period_positions[d.period] for d in week_values]
value_interrogations = [d.interrogations for d in week_values]
value_annotations = [d.annotation for d in week_values]

# A value holds ids, never handles: it is detached, and a handle would carry
# the document with it.
assert all(isinstance(d.period, collomatique.PeriodId) for d in week_values)
assert all(isinstance(d.interrogations, bool) for d in week_values)
assert all(
    d.annotation is None or isinstance(d.annotation, str) for d in week_values
)

# The period in the value is the week's own. The model files a week under its
# period in the list that gives it its position, so a value naming a different
# period would describe a week that is not there.
assert [d.period for d in week_values] == [week.period.id for week in week_list]

# The example is worth reading here: both shapes of each field are among the
# weeks, so neither branch passes by never being taken.
assert any(d.interrogations for d in week_values)
assert any(not d.interrogations for d in week_values)
assert any(d.annotation is not None for d in week_values)
assert any(d.annotation is None for d in week_values)

# The derived fields are the handle's, and only the handle's: the index comes
# from the week's place in the walk and the monday from the index and the
# document's start date, so a value that stored them could contradict itself.
assert all(not hasattr(d, "index") for d in week_values)
assert all(not hasattr(d, "monday") for d in week_values)
assert all(hasattr(week, "index") for week in week_list)
assert all(hasattr(week, "monday") for week in week_list)

# A fresh object every call. Two of them are equal and share nothing, so
# writing to one is invisible to the other and to the document.
first = week_list[0]
again = first.to_data()
assert again == week_values[0]
assert again is not week_values[0]
again.interrogations = not again.interrogations
assert week_values[0].interrogations != again.interrogations
assert first.interrogations != again.interrogations

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
scratch = collomatique.WeekData("lundi")
scratch.interrogations = 1
scratch.annotation = 3

# And a value has no identity: an id names a place in a document, and a value
# has none. Updating an existing week's fields will pass the week as the
# method's argument.
assert not hasattr(week_values[0], "id")

# The field order of each class, which is what a positional call depends on:
# required first, then the defaulted ones in the order the handle shows them.
assert [f.name for f in dataclasses.fields(collomatique.WeekData)] == [
    "period",
    "interrogations",
    "annotation",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import WeekData as _same_class  # noqa: E402

assert _same_class is collomatique.WeekData
assert collomatique.WeekData.__module__ == "collomatique"

# A field that names an entity takes a handle or an id, interchangeably, and
# the two values below extract to the same week and — this is the wart — do
# not compare equal, because a dataclass stores what it was given.
first_period = first.period
week_by_handle = collomatique.WeekData(first_period)
week_by_id = collomatique.WeekData(first_period.id)
assert week_by_handle != week_by_id
assert week_by_handle.period == first_period
assert week_by_id.period == first_period.id

# Nothing but the one field a week cannot do without, so rust can pin what the
# defaulted two come out as: a week that runs colles, and no annotation.
bare_week = collomatique.WeekData(first_period)
assert bare_week.interrogations is True
assert bare_week.annotation is None

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
not_a_period = collomatique.WeekData(3)
not_a_flag = collomatique.WeekData(first_period, interrogations=1)
not_an_annotation = collomatique.WeekData(first_period, annotation=3)
empty_annotation = collomatique.WeekData(first_period, annotation="")

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_period = collomatique.WeekData(list(other.periods)[0])
