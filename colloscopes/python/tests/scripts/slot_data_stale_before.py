import datetime

import collomatique

doc = collomatique.load(source)

# Rust removes one slot, one week pattern and the last period with all its weeks
# between this stage and the next — the read surface ships no writes of its own.
slot_list = list(doc.slots)
doomed_slot = slot_list[-1]
living_slot = slot_list[0]

patterns = list(doc.week_patterns)
doomed_pattern = patterns[-1]

# Written down while everything is alive, and read again afterwards.
living_slot_extra_info = living_slot.extra_info

weeks = list(doc.weeks)
doomed_week = weeks[-1]
living_week = weeks[0]
assert doomed_week != living_week

subject = list(doc.subjects)[0]
teacher = list(doc.teachers)[0]

# Two slot values naming the pattern that is about to go, one by handle and one
# by id, and one naming no pattern at all — which is a slot that runs every
# week, and has nothing to lose here.
naming_the_dead_pattern_by_handle = collomatique.SlotData(
    subject,
    teacher,
    collomatique.Weekday.MONDAY,
    datetime.time(8, 0),
    week_pattern=doomed_pattern,
)
naming_the_dead_pattern_by_id = collomatique.SlotData(
    subject,
    teacher,
    collomatique.Weekday.MONDAY,
    datetime.time(8, 0),
    week_pattern=doomed_pattern.id,
)
naming_no_pattern = collomatique.SlotData(
    subject, teacher, collomatique.Weekday.MONDAY, datetime.time(8, 0)
)

# And a pattern value naming the week that is about to go, beside one naming a
# week that survives.
naming_the_dead_week = collomatique.WeekPatternData(
    "Sortilèges", excluded_weeks={doomed_week}
)
naming_the_living_week = collomatique.WeekPatternData(
    "Sortilèges", excluded_weeks={living_week}
)
