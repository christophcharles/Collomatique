import collomatique

# The second of three stages: Rust has just installed a whole-entry override
# for Harry — a minimum of four interrogations a week, objective rather than
# strict, and the two other fields unset. The `None` fields must *disable* the
# corresponding global limits, not inherit them: the verbatim whole-entry rule
# the model's own tests pin.
assert doc.settings.override_for(harry) is not None

# The same view from the first stage re-resolves on every read: it now reads
# the override, whose unset fields mask the set global limits.
assert resolved.interrogations_per_week_min == collomatique.Limit(4, collomatique.Enforcement.OBJECTIVE)
assert resolved.interrogations_per_week_max is None
assert resolved.max_interrogations_per_day is None

# The raw view is bound to the entry itself and reads the same whole entry.
override_view = doc.settings.override_for(harry)
assert override_view is not None
assert override_view.interrogations_per_week_min == resolved.interrogations_per_week_min
assert override_view.interrogations_per_week_max is None
assert override_view != resolved  # same reading, different binding
