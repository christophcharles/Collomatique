import collomatique

# `to_data()` is a read, so a dead handle refuses it the way every other read
# does — it does not hand back a value describing a rule that is gone. The
# sides go with their rule, like every other read through them.
try:
    doomed.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale pairing rule must raise")
try:
    doomed_antecedent.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale side view must raise")
try:
    doomed_consequent.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale side view must raise")

# The rule that survived reads as it did.
assert living.soft == living_soft

# The value built before the removal is untouched: a dataclass knows nothing
# about a document, and nothing reached in to edit it.
assert doomed_value.soft == doomed_soft
