import collomatique

# The second of three stages: rust has just installed the partial override for
# Harry, on this very document, and everything is alive. The raw view is held
# so that `to_data()` through it is tried once the entry is gone; the value it
# hands back now is the whole-entry round trip — the two unset fields come out
# as `None`, and must stay `None` across the extraction, because that is what
# disables the inherited limits.
harry = [student for student in doc.students if student.surname == "Potter"][0]

raw_view = doc.settings.override_for(harry)
assert raw_view is not None

partial_value = raw_view.to_data()
assert partial_value.interrogations_per_week_min == collomatique.Limit(
    4, collomatique.Enforcement.OBJECTIVE)
assert partial_value.interrogations_per_week_max is None
assert partial_value.max_interrogations_per_day is None

# A fresh ask answers the current truth.
assert doc.settings.override_for(harry) is not None
