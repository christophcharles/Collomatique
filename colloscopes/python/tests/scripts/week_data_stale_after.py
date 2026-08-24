import collomatique

# `to_data()` is a read, so a dead handle refuses it the way every other read
# does — it does not hand back a value describing a week that is gone.
try:
    doomed_week.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale week must raise")

# The week that survived reads as it did, and its value still names its own
# period.
assert living_week.interrogations == living_week_interrogations
assert living_week.to_data().period == living_week.period.id

# The values built before the removal are untouched objects: a dataclass knows
# nothing about a document, and nothing reached in to edit them. What has
# changed is that one of them no longer names anything, which is rust's half of
# this test.
assert naming_the_dead_period.period is not None
assert naming_the_living_period.period is not None
