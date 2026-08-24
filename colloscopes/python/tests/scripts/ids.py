import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

period_list = list(doc.periods)
week_list = list(doc.weeks)

period_id = period_list[0].id
other_period_id = period_list[1].id
week_id = week_list[0].id

assert isinstance(period_id, collomatique.PeriodId)
assert isinstance(week_id, collomatique.WeekId)

# Two ids of the same period are equal and hash the same, so an id is a usable
# dict key and set element.
assert period_id == period_list[0].id
assert period_id != other_period_id
assert hash(period_id) == hash(period_list[0].id)
assert len({period_id, period_list[0].id}) == 1
assert len({period_id, other_period_id}) == 2

# They order against their own kind, which is what makes a sorted output stable.
assert (period_id < other_period_id) != (other_period_id < period_id)
assert sorted([other_period_id, period_id]) == sorted([period_id, other_period_id])

# Against another kind, equality is `False` and ordering raises: an id of one
# kind is not a number that happens to be labelled.
assert period_id != week_id
assert not period_id == week_id
try:
    period_id < week_id
except TypeError:
    pass
else:
    raise AssertionError("ordering two kinds of id must raise")

# And against anything else at all.
assert period_id != 3
assert period_id != "period"
assert period_id is not None

# The repr says the kind and the number, in brackets: `PeriodId(3)` would read
# as an expression a script could paste back, and there is no such constructor.
period_id_repr = repr(period_id)
week_id_repr = repr(week_id)

# The eleven kinds are all in the module from the start, and not one of them can
# be built, turned into a number, or written down.
id_classes = [
    collomatique.PeriodId,
    collomatique.WeekId,
    collomatique.SubjectId,
    collomatique.TeacherId,
    collomatique.StudentId,
    collomatique.WeekPatternId,
    collomatique.SlotId,
    collomatique.IncompatId,
    collomatique.GroupListId,
    collomatique.PairingRuleId,
    collomatique.SlotPairingRuleId,
]
id_class_names = [cls.__name__ for cls in id_classes]

for cls in id_classes:
    try:
        cls()
    except TypeError:
        pass
    else:
        raise AssertionError(f"{cls.__name__} must not be constructible")
    try:
        cls(3)
    except TypeError:
        pass
    else:
        raise AssertionError(f"{cls.__name__} must not be constructible")

try:
    int(period_id)
except TypeError:
    pass
else:
    raise AssertionError("an id must not convert to a number")

# An id does not read the document, so it is not affected by anything that
# happens to it — that is the whole of what an id is.
assert period_id == period_list[0].id
