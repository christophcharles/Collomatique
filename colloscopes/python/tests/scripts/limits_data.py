import dataclasses

import collomatique

# `source` is a copy of the hogwarts example: global limits on everyone — two
# strict per-week limits and a strict per-day one — and a single student
# (Hermione) whose override sets all three fields, everyone else inheriting
# the global entry. The numeric values are compared against the model on the
# rust side; what this script pins is the api's own shape.
doc = collomatique.load(source)

# What the views hand back detached, as the values a script builds.
global_value = doc.settings.global_limits.to_data()
assert isinstance(global_value, collomatique.LimitsData)

# The three fields come out as Limit leaves or None, whole: an entry is one
# record, and a `None` field is a disabled limit — it does not inherit, it
# disables — never an absent one. The global entry sets all three, strictly.
for name in ("interrogations_per_week_min", "interrogations_per_week_max",
             "max_interrogations_per_day"):
    limit = getattr(global_value, name)
    assert isinstance(limit, collomatique.Limit)
    assert limit.enforcement == collomatique.Enforcement.STRICT

# Hermione is the one student with an override; her entry comes back whole,
# the raw entry the document holds, whatever it holds.
hermione = [student for student in doc.students if student.surname == "Granger"][0]
hermione_value = doc.settings.override_for(hermione).to_data()
assert isinstance(hermione_value, collomatique.LimitsData)
assert all(getattr(hermione_value, name) is not None
           for name in ("interrogations_per_week_min", "interrogations_per_week_max",
                        "max_interrogations_per_day"))

# A student without an override resolves to the global entry, and `to_data()`
# hands back what the view reads — the resolved entry.
harry = [student for student in doc.students if student.surname == "Potter"][0]
harry_value = doc.settings.limits_for(harry).to_data()
assert harry_value == global_value

# A fresh object every call. Two of them are equal and share nothing, so
# writing to one is invisible to the other and to the document.
again = doc.settings.global_limits.to_data()
assert again == global_value
assert again is not global_value
again.interrogations_per_week_min = None
assert global_value.interrogations_per_week_min is not None

# A value has no identity: an id names a place in a document, and a value has
# none. Updating an entry will pass the id as the method's argument.
assert not hasattr(global_value, "id")

# The field order, which is what a positional call depends on. Everything is
# defaulted here — an entry with nothing set is an entry that disables every
# inherited limit.
assert [f.name for f in dataclasses.fields(collomatique.LimitsData)] == [
    "interrogations_per_week_min",
    "interrogations_per_week_max",
    "max_interrogations_per_day",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import LimitsData as _same_class  # noqa: E402

assert _same_class is collomatique.LimitsData
assert collomatique.LimitsData.__module__ == "collomatique"

# The defaults: every field `None`, which is the model's own default. Rust
# pins it against `Limits::default()`.
defaults = collomatique.LimitsData()

# Built by hand, a partial entry: one limit set, the two other fields left at
# their `None` default — which is the point, they must *disable* the inherited
# limits rather than inherit them, and the value must carry the `None`s across
# as the model stores them. Rust compares it whole against the entry the
# settings mutators will install.
partial = collomatique.LimitsData(
    interrogations_per_week_min=collomatique.Limit(4, collomatique.Enforcement.OBJECTIVE))

# `interrogations_per_week_min` of zero is a limit the model holds — a week
# with no interrogation at all is a thing to say — so it extracts.
week_min_zero = collomatique.LimitsData(
    interrogations_per_week_min=collomatique.Limit(0, collomatique.Enforcement.OBJECTIVE))

# `max_interrogations_per_day` is the one the model types non-zero: a day in
# which no interrogation may happen at all is not a limit, and 0 is refused
# when the value is used.
day_zero = collomatique.LimitsData(
    max_interrogations_per_day=collomatique.Limit(0, collomatique.Enforcement.STRICT))

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
not_a_limit = collomatique.LimitsData(interrogations_per_week_min=3)

# A Limit is a leaf that refuses at birth — its enforcement must be an
# Enforcement, so this is refused here, by the leaf.
try:
    collomatique.LimitsData(
        interrogations_per_week_min=collomatique.Limit(2, "STRICT"))
except TypeError:
    pass
else:
    raise AssertionError("a Limit's enforcement must be an Enforcement")
