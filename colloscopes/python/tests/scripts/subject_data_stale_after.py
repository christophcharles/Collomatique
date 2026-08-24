import collomatique

# `to_data()` is a read, so a dead handle refuses it the way every other read
# does — it does not hand back a value describing a subject that is gone.
try:
    doomed.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale handle must raise")

# Its sub-view says which of its two deaths this was: the subject is gone, not
# merely its colles.
try:
    doomed_view.to_data()
except collomatique.StaleHandleError as error:
    assert "no longer in the document" in str(error)
else:
    raise AssertionError("to_data() through a stale sub-view must raise")

# The other death, on a subject that is perfectly alive: its colles were
# switched off. The handle keeps working and its value says so with a `None`,
# while the view handed out before the change has nothing left to read.
assert switched_off.name == switched_off_name
assert switched_off.interrogation is None
assert switched_off.to_data().interrogation is None
try:
    switched_off_view.to_data()
except collomatique.StaleHandleError as error:
    assert "no longer holds interrogations" in str(error)
else:
    raise AssertionError("a view of colles that were switched off must raise")

# The values built before the removal are untouched objects — a dataclass knows
# nothing about a document, and nothing reached in to edit them. What has
# changed is that two of them no longer name anything, which is rust's half of
# this test.
assert len(naming_the_dead_by_handle.excluded_periods) == 1
assert len(naming_the_living.excluded_periods) == 1
