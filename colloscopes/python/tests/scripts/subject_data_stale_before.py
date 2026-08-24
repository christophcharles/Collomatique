import collomatique

doc = collomatique.load(source)

# Rust removes one subject, switches another one's colles off and removes the
# last period between this stage and the next — the read surface ships no
# writes of its own. `doomed_index` and `switched_off_index` count among the
# subjects that hold colles, which is the list this stage builds.
with_colles = [subject for subject in doc.subjects if subject.interrogation is not None]

doomed = with_colles[doomed_index]
doomed_view = doomed.interrogation
switched_off = with_colles[switched_off_index]
switched_off_view = switched_off.interrogation

# Written down while everything is alive, and read afterwards.
switched_off_name = switched_off.name
switched_off_duration = switched_off_view.duration

# Two values naming the period that is about to go, one by handle and one by id,
# and one naming a period that will survive.
periods = list(doc.periods)
doomed_period = periods[-1]
living_period = periods[0]
assert doomed_period != living_period

naming_the_dead_by_handle = collomatique.SubjectData(
    "Sortilèges", excluded_periods={doomed_period}
)
naming_the_dead_by_id = collomatique.SubjectData(
    "Sortilèges", excluded_periods={doomed_period.id}
)
naming_the_living = collomatique.SubjectData(
    "Sortilèges", excluded_periods={living_period}
)
