import collomatique

# The second half: the override is gone. The resolved view falls back to the
# global entry — the same view object, still alive, because it is bound to the
# subject, not to the override.
assert resolved.avoid_twice_in_a_row is None
assert resolved.year_teacher_rotation is False
assert resolved.teacher_rotation == collomatique.Enforcement.OBJECTIVE

# The raw view is bound to the entry that was removed, so it is dead, loudly.
for attribute in ("teacher_rotation", "slot_rotation", "avoid_twice_in_a_row",
                  "year_teacher_rotation", "period_teacher_rotation"):
    try:
        getattr(override_view, attribute)
    except collomatique.StaleHandleError as error:
        assert "BalancingOptions" in str(error)
        assert repr(metamorphose.id) in str(error)
        assert "has no balancing override anymore" in str(error)
    else:
        raise AssertionError(f"a stale BalancingOptions view must raise on .{attribute}")

# `==` and `hash` never read the state, so they outlive the entry.
assert override_view == override_view
assert hash(override_view) == hash(override_view)

# Neither repr raises, and the dead one says so.
assert repr(override_view).startswith("<BalancingOptions #")
assert repr(override_view).endswith("(périmé)>")

# A fresh ask answers the current truth: no override, so no raw view, and the
# stored rows are one fewer.
assert doc.balancing.override_for(metamorphose) is None
assert doc.balancing.options_for(metamorphose) == resolved
assert len(doc.balancing.overrides()) == override_count - 1
