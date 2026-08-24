import collomatique

# The first half of a two-stage script: rust removes one week pattern and one
# whole period between this stage and the next, because the read surface ships no
# removes of its own yet. Everything this stage leaves in the globals is what the
# next one asks questions about.
doc = collomatique.load(source)

pattern_count_before = len(doc.week_patterns)
week_count_before = len(doc.weeks)

every, doomed, unnamed, all_off = list(doc.week_patterns)
doomed_id = doomed.id

# The first period stays and the second one goes, weeks and all.
weeks = list(doc.weeks)
live_week = weeks[0]
doomed_week = weeks[3]
doomed_week_id = doomed_week.id

# What the predicate answers while everything is alive, for the very pairs the
# next stage asks about once they are not.
assert doc.is_week_active(live_week, every)
assert doc.is_week_active(live_week, doomed)
assert doc.is_week_active(doomed_week, every)
assert not doc.is_week_active(doomed_week, doomed)

# Handles as dict keys: this is the thing that must not blow up when the entity
# they name dies.
by_handle = {doomed: "pattern", doomed_week: "week"}
