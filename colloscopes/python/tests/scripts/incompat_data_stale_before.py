import collomatique

doc = collomatique.load(source)

# Rust removes one incompatibility and one week pattern between this stage and
# the next — the read surface ships no writes of its own.
incompat_list = list(doc.incompats)
doomed_incompat = incompat_list[-1]
living_incompat = incompat_list[0]
assert doomed_incompat != living_incompat

patterns = list(doc.week_patterns)
doomed_pattern = patterns[-1]
living_pattern = patterns[0]
assert doomed_pattern != living_pattern

# Written down while everything is alive, and read again afterwards.
living_name = living_incompat.name

subject = list(doc.subjects)[0]

# Two incompatibility values naming the pattern that is about to go, one by
# handle and one by id, and one naming no pattern at all — which applies on
# every week, and has nothing to lose here. Beside them, one naming a pattern
# that survives.
naming_the_dead_pattern_by_handle = collomatique.IncompatData(
    "À venir", subject, week_pattern=doomed_pattern
)
naming_the_dead_pattern_by_id = collomatique.IncompatData(
    "À venir", subject, week_pattern=doomed_pattern.id
)
naming_no_pattern = collomatique.IncompatData("À venir", subject)
naming_the_living_pattern = collomatique.IncompatData(
    "À venir", subject, week_pattern=living_pattern
)
