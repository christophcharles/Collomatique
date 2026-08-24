import collomatique

# The third of three stages: Rust has just removed Harry's override. The
# resolved view falls back to the global entry — the same view object, still
# alive, because it is bound to the student, not to the override.
assert resolved.interrogations_per_week_min != collomatique.Limit(4, collomatique.Enforcement.OBJECTIVE)
assert resolved.interrogations_per_week_max is not None
assert resolved.interrogations_per_week_max == global_week_max
assert resolved.max_interrogations_per_day is not None

# The raw view is bound to the entry that was removed, so it is dead, loudly.
for attribute in ("interrogations_per_week_min", "interrogations_per_week_max",
                  "max_interrogations_per_day"):
    try:
        getattr(override_view, attribute)
    except collomatique.StaleHandleError as error:
        assert "Limits" in str(error)
        assert repr(harry.id) in str(error)
        assert "no limits override anymore" in str(error)
    else:
        raise AssertionError(f"a stale Limits view must raise on .{attribute}")

# `==` and `hash` never read the state, so they outlive the entry — a dict
# holding the view must not blow up when the entry dies.
assert override_view == override_view
assert hash(override_view) == hash(override_view)

# Neither repr raises, and the dead one says so.
assert repr(override_view).startswith("<Limits #")
assert repr(override_view).endswith("(périmé)>")

# A fresh ask answers the current truth: no override, so no raw view.
assert doc.settings.override_for(harry) is None
assert doc.settings.limits_for(harry) == resolved
