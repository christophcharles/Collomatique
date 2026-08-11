import collomatique

# The first of three stages. Rust installs a limits override for Harry between
# this stage and the next, and removes it before the third — the read surface
# ships no writes of its own yet. Everything this stage leaves in the globals
# is what the later ones ask questions about.
doc = collomatique.load(source)

harry = [student for student in doc.students if student.surname == "Potter"][0]

# Harry has no override: the resolved view reads the global entry, and there
# is no raw view to be had.
resolved = doc.settings.limits_for(harry)
assert doc.settings.override_for(harry) is None
assert resolved.interrogations_per_week_min is not None
assert resolved.interrogations_per_week_max is not None
assert resolved.max_interrogations_per_day is not None

# The limit value read at this stage, for the last one to compare against: a
# value is detached content, so the same reading after the override is gone
# must compare equal.
global_week_max = resolved.interrogations_per_week_max
