import collomatique

# `to_data()` is a read, so a dead handle refuses it the way every other read
# does — it does not hand back a value describing a group list that is gone.
try:
    doomed.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale group list must raise")

# The group list that survived reads as it did.
assert living.name == living_name

# The value built before the removal is untouched: a dataclass knows nothing
# about a document, and nothing reached in to edit it.
assert doomed_value.name == doomed_name
