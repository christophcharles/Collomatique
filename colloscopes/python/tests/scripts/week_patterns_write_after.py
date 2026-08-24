import collomatique

# The stage before this one undid every write it made, so the document is the
# example again — and rust has since written one colle, on a `(slot, week)` cell
# the write surface cannot reach yet: the colloscope family is a later piece of
# the mirror. `prepared` is the `OpResult` that write answered, and
# `cell_pattern_index` and `cell_week_index` are the places, in the patterns' and
# the weeks' own orders, of the pattern and the week it is about.
assert prepared.warnings == []

pattern = list(doc.week_patterns)[cell_pattern_index]
week = list(doc.weeks)[cell_week_index]
slot = next(slot for slot in doc.slots if slot.week_pattern == pattern)

# The cell is on a week the pattern leaves on, which is why the colle could be
# written there at all.
assert doc.is_week_active(week, pattern) is True
assert week not in pattern.excluded_weeks
assert doc.colloscope.interrogation(slot, week) == frozenset({0})

# The cascade of the `update`, and the one the other way about from the removal's:
# excluding a week makes interrogations impossible on it for every slot following
# the pattern, so the colles already written in those cells contradict the new
# pattern and go. A read-modify-write is what a script does here, and `to_data()`
# fills the set with ids, so adding a handle to it is the ordinary mix.
narrowed = pattern.to_data()
narrowed.excluded_weeks = set(narrowed.excluded_weeks) | {week}
result = doc.week_patterns.update(pattern, narrowed)

assert [w.kind for w in result.warnings] == ["ClearInterrogationCell"]
cleared = result.warnings[0]
assert cleared.details == {"slot": slot.id, "week": week.id}
assert cleared.parent is None
assert str(cleared)

assert week in pattern.excluded_weeks
assert doc.is_week_active(week, pattern) is False
assert doc.colloscope.interrogation(slot, week) is None

# The pattern kept its slot throughout: an `update` rewrites the pattern, it
# does not replace it.
assert slot.week_pattern == pattern

# And the colle comes back with the week, since undoing a write undoes what it
# had to repair on the way.
doc.undo()
assert week not in pattern.excluded_weeks
assert doc.colloscope.interrogation(slot, week) == frozenset({0})
