import collomatique

# The first half of a two-stage script: rust deletes the last period and all its
# weeks between this stage and the next, because the read surface ships no
# removes of its own yet. Everything this stage leaves in the globals is what
# the next one asks questions about.
doc = collomatique.load(source)

period_count_before = len(doc.periods)
week_count_before = len(doc.weeks)

doomed = list(doc.periods)[-1]
doomed_weeks = list(doomed.weeks)
doomed_id = doomed.id
doomed_week_id = doomed_weeks[0].id

survivor = list(doc.periods)[0]
survivor_weeks = list(survivor.weeks)

# Handles as dict keys: this is the thing that must not blow up when the entity
# they name dies.
by_handle = {doomed: "period", doomed_weeks[0]: "week"}

# A walk that is under way when the removal happens. Iteration takes its
# snapshot of the ids here, so the loop still meets the doomed weeks.
walk = iter(doc.weeks)
first_seen = next(walk)
