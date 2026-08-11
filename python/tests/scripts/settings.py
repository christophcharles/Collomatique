import collomatique

# `source` is a copy of the hogwarts example: global limits on everyone — two
# strict per-week limits and a strict per-day one — and a single student
# (Hermione) whose override sets all three fields, everyone else inheriting
# the global entry. The numeric values are compared against the model on the
# rust side; what this script pins is the api's own structure.
doc = collomatique.load(source)

settings = doc.settings
assert isinstance(settings, collomatique.Settings)
assert repr(settings) == "<collomatique.Settings overrides=%d>" % len(settings.overrides())

global_limits = settings.global_limits
assert isinstance(global_limits, collomatique.Limits)
assert global_limits.interrogations_per_week_min is not None
assert global_limits.interrogations_per_week_max is not None
assert global_limits.max_interrogations_per_day is not None
for limit in (global_limits.interrogations_per_week_min,
              global_limits.interrogations_per_week_max,
              global_limits.max_interrogations_per_day):
    assert isinstance(limit, collomatique.Limit)
    assert isinstance(limit.enforcement, collomatique.Enforcement)

# The global view is bound to the entry itself, so it never goes stale, and
# every ask is an equal view of the same thing.
assert settings.global_limits == global_limits
assert repr(settings.global_limits) == repr(global_limits)

# Hermione is the one student with an override; Harry has none.
hermione = [student for student in doc.students if student.surname == "Granger"][0]
harry = [student for student in doc.students if student.surname == "Potter"][0]

hermione_limits = settings.limits_for(hermione)
hermione_override = settings.override_for(hermione)
assert hermione_override is not None
assert hermione_limits == settings.limits_for(hermione.id)
assert hermione_override == settings.override_for(hermione.id)

# The kind is part of a view's identity: the resolved and the raw views of one
# student are different views with different lives, even while they read the
# same entry — like the two ends of a pairing rule.
assert hermione_limits != hermione_override
assert len({hermione_limits, hermione_override}) == 2

# The stored overrides come back as live `(Student, Limits)` pairs, the view
# bound to the entry it describes.
overrides = settings.overrides()
assert len(overrides) == 1
override_student, override_limits = overrides[0]
assert override_student == hermione
assert override_limits == hermione_override
assert override_limits == settings.override_for(hermione)

# Harry inherits the global entry — the resolution is the model's, not a merge.
harry_limits = settings.limits_for(harry)
assert settings.override_for(harry) is None
assert harry_limits != global_limits  # same reading, different binding

# The leaf values: named, compared, hashed, matched — a script says which
# limit it expects and compares.
assert collomatique.Limit(2, collomatique.Enforcement.STRICT) == global_limits.interrogations_per_week_min
assert collomatique.Limit(0, collomatique.Enforcement.OBJECTIVE) == collomatique.Limit(0, collomatique.Enforcement.OBJECTIVE)
assert collomatique.Limit(2, collomatique.Enforcement.STRICT) != collomatique.Limit(3, collomatique.Enforcement.STRICT)
assert collomatique.Limit(2, collomatique.Enforcement.STRICT) != collomatique.Limit(2, collomatique.Enforcement.OBJECTIVE)
assert hash(collomatique.Limit(2, collomatique.Enforcement.STRICT)) == hash(global_limits.interrogations_per_week_min)
assert repr(collomatique.Limit(2, collomatique.Enforcement.STRICT)) == "Limit(value=2, enforcement=Enforcement.STRICT)"
assert repr(collomatique.Enforcement.OBJECTIVE) == "Enforcement.OBJECTIVE"
assert collomatique.Enforcement.OBJECTIVE != collomatique.Enforcement.STRICT
assert collomatique.Enforcement.OBJECTIVE == collomatique.Enforcement.OBJECTIVE
match global_limits.interrogations_per_week_min:
    case collomatique.Limit(value, enforcement):
        assert value == global_limits.interrogations_per_week_min.value
        assert enforcement == global_limits.interrogations_per_week_min.enforcement
    case _:
        raise AssertionError("a Limit must match by (value, enforcement)")
try:
    collomatique.Limit(2, "strict")
except TypeError:
    pass
else:
    raise AssertionError("a Limit's enforcement must be an Enforcement")

# What the rust half compares against the same document read from the model.
global_values = [global_limits.interrogations_per_week_min.value,
                 global_limits.interrogations_per_week_max.value,
                 global_limits.max_interrogations_per_day.value]
global_strict = [global_limits.interrogations_per_week_min.enforcement == collomatique.Enforcement.STRICT,
                 global_limits.interrogations_per_week_max.enforcement == collomatique.Enforcement.STRICT,
                 global_limits.max_interrogations_per_day.enforcement == collomatique.Enforcement.STRICT]
hermione_values = [hermione_limits.interrogations_per_week_min.value,
                   hermione_limits.interrogations_per_week_max.value,
                   hermione_limits.max_interrogations_per_day.value]
hermione_strict = [hermione_limits.interrogations_per_week_min.enforcement == collomatique.Enforcement.STRICT,
                   hermione_limits.interrogations_per_week_max.enforcement == collomatique.Enforcement.STRICT,
                   hermione_limits.max_interrogations_per_day.enforcement == collomatique.Enforcement.STRICT]
harry_values = [harry_limits.interrogations_per_week_min.value,
                harry_limits.interrogations_per_week_max.value,
                harry_limits.max_interrogations_per_day.value]
override_values = [override_limits.interrogations_per_week_min.value,
                   override_limits.interrogations_per_week_max.value,
                   override_limits.max_interrogations_per_day.value]
global_repr = repr(global_limits)
hermione_repr = repr(hermione_limits)
