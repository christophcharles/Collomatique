import collomatique

# `to_data()` is a read, so a dead handle refuses it the way every other read
# does — it does not hand back a value describing an incompatibility that is
# gone.
try:
    doomed_incompat.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale incompatibility must raise")

# The incompatibility that survived reads as it did.
assert living_incompat.name == living_name

# The values built before the removals are untouched objects: a dataclass knows
# nothing about a document, and nothing reached in to edit them. What has
# changed is that two of them no longer name anything, which is rust's half of
# this test.
assert naming_the_dead_pattern_by_handle.week_pattern is not None
assert naming_the_dead_pattern_by_id.week_pattern is not None
assert naming_the_living_pattern.week_pattern is not None
assert naming_no_pattern.week_pattern is None
