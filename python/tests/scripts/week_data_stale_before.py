import collomatique

doc = collomatique.load(source)

# Rust removes the last period with all its weeks between this stage and the
# next — the read surface ships no writes of its own.
week_list = list(doc.weeks)
doomed_week = week_list[-1]
living_week = week_list[0]
assert doomed_week != living_week

# Written down while everything is alive, and read again afterwards.
living_week_interrogations = living_week.interrogations

# Two week values naming the period that is about to go, and one naming a
# period that survives.
naming_the_dead_period = collomatique.WeekData(doomed_week.period)
naming_the_living_period = collomatique.WeekData(living_week.period)
