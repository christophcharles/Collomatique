import collomatique

# The second half: the override is gone. The raw view is bound to the entry
# that was removed, so `to_data()` through it dies loudly — a script meeting
# the error wants to know what happened.
try:
    raw_view.to_data()
except collomatique.StaleHandleError as error:
    assert "BalancingOptions" in str(error)
    assert repr(metamorphose.id) in str(error)
    assert "has no balancing override anymore" in str(error)
else:
    raise AssertionError("a stale BalancingOptions view must raise on to_data()")

# The resolved view is bound to the subject, not to the override, so it is
# still alive — and it now reads the global entry, which is what its
# `to_data()` hands back.
assert resolved_view.to_data() == global_value

# The value written down while the entry stood is a detached object: the
# removal cannot reach it, since nothing in it names the document. Rust reads
# it back whole and compares it with the entry the file held.
assert doomed_value.avoid_twice_in_a_row == collomatique.Enforcement.STRICT

# A fresh ask answers the current truth: no override, so no raw view.
assert doc.balancing.override_for(metamorphose) is None
