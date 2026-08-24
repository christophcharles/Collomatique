import collomatique

# A document written for this: among its four patterns there is one that
# excludes nothing, one that excludes every week, and one the user never named.
# The example has none of the three — all of its patterns carry a name and all
# of them switch something off.
doc = collomatique.load(source)

pattern_list = list(doc.week_patterns)
pattern_values = [pattern.to_data() for pattern in pattern_list]

weeks = list(doc.weeks)
week_positions = {week.id: position for position, week in enumerate(weeks)}

value_names = [d.name for d in pattern_values]
value_excluded_week_indices = [
    sorted(week_positions[week_id] for week_id in d.excluded_weeks)
    for d in pattern_values
]

# The three ends, read off the values rather than off the handles: an empty set
# is a value and not an absence, a whole one is not a special case, and a
# nameless pattern reads as `""` because the model types the field as a plain
# string.
assert any(d.excluded_weeks == set() for d in pattern_values)
assert any(len(d.excluded_weeks) == len(weeks) for d in pattern_values)
assert any(d.name == "" for d in pattern_values)

# And the same two ends written out by hand, which is the direction rust drives
# through the boundary.
excluding_nothing = collomatique.WeekPatternData("Toutes les semaines")
excluding_everything = collomatique.WeekPatternData(
    "Aucune semaine", excluded_weeks=set(weeks)
)
