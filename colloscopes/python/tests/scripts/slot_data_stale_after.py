import collomatique

# `to_data()` is a read, so a dead handle refuses it the way every other read
# does — it does not hand back a value describing a slot that is gone.
try:
    doomed_slot.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale slot must raise")

try:
    doomed_pattern.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale week pattern must raise")

# The slot that survived reads as it did, and its value still names its own
# subject — the field a read-modify-write must not have to fill in itself.
assert living_slot.extra_info == living_slot_extra_info
assert living_slot.to_data().subject == living_slot.subject.id

# The values built before the removals are untouched objects: a dataclass knows
# nothing about a document, and nothing reached in to edit them. What has
# changed is that three of them no longer name anything, which is rust's half of
# this test.
assert naming_the_dead_pattern_by_handle.week_pattern is not None
assert naming_the_dead_pattern_by_id.week_pattern is not None
assert len(naming_the_dead_week.excluded_weeks) == 1
assert len(naming_the_living_week.excluded_weeks) == 1
