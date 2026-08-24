import collomatique

# The first half of a two-stage script. The write surface publishes no op the
# model can refuse yet, so rust applies four refused writes between this stage
# and the next, and leaves what they raised in the globals. What this stage
# keeps is what the next one compares those exceptions against.
doc = collomatique.load(source)

periods = list(doc.periods)
first = periods[0]
doomed = periods[-1]

# The period rust is about to delete: after that, naming it is naming nothing.
doomed_id = doomed.id
# The first period's weeks, so the next stage can say what "too long" was
# measured against.
first_week_count = len(list(first.weeks))
# The subject rust is about to delete, for the same reason as the period — and
# it is a *second* family, so the next stage can see the class change with it.
doomed_subject_id = list(doc.subjects)[0].id
