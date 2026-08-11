import collomatique

# The last of three stages: the override is gone. The raw view held from the
# middle stage is bound to the entry that was removed, so `to_data()` through
# it dies loudly — a script meeting the error wants to know what happened.
try:
    raw_view.to_data()
except collomatique.StaleHandleError as error:
    assert "Limits" in str(error)
    assert repr(harry.id) in str(error)
    assert "has no limits override anymore" in str(error)
else:
    raise AssertionError("a stale Limits view must raise on to_data()")

# The value written down while the entry stood is a detached object: the
# removal cannot reach it, since nothing in it names the document. Rust reads
# it back whole and compares it with the entry that stood.
assert partial_value.interrogations_per_week_min == collomatique.Limit(
    4, collomatique.Enforcement.OBJECTIVE)

# A fresh ask answers the current truth: no override, so no raw view.
assert doc.settings.override_for(harry) is None
