import collomatique

# The stage before this one undid every write it made, so the document is the
# example again — and rust has since written one colle, on a `(slot, week)` cell
# the write surface cannot reach yet: the colloscope family is a later piece of
# the mirror. `prepared` is the `OpResult` that write answered, and
# `cell_week_index` is the place, in the weeks' own order, of the week it is on.
# The slot it is on is the antecedent of the document's first slot pairing rule,
# which is how this stage gets a cell and a rule on one slot at once.
assert prepared.warnings == []

rule = list(doc.slot_pairings)[0]
slot = rule.antecedent.slot
week = list(doc.weeks)[cell_week_index]

slots_before = len(doc.slots)
rules_before = len(doc.slot_pairings)

# The cell is on a week the slot really runs on, which is why the colle could be
# written there at all.
assert slot.week_pattern is None
assert doc.is_interrogation_possible(slot, week) is True
assert doc.colloscope.interrogation(slot, week) == frozenset({0})

# The `update` cascade: putting the slot on a pattern that switches that week
# off means the slot no longer runs there, so the colle already written in the
# cell contradicts the new pattern and goes. A read-modify-write is what a
# script does here, and it never trips over the subject the value carries, since
# `to_data()` fills that field with the slot's own subject.
narrowed = slot.to_data()
narrowed.week_pattern = doc.week_patterns.add(
    collomatique.WeekPatternData("Sans cette semaine", excluded_weeks={week})
).created
result = doc.slots.update(slot, narrowed)

assert [w.kind for w in result.warnings] == ["ClearInterrogationCell"]
cleared = result.warnings[0]
assert cleared.details == {"slot": slot.id, "week": week.id}
assert cleared.parent is None
assert str(cleared)

assert doc.is_interrogation_possible(slot, week) is False
assert doc.colloscope.interrogation(slot, week) is None
assert slot.week_pattern == narrowed.week_pattern
assert slot in doc.slots

# And the colle comes back when the write is undone, since undoing a write
# undoes what it had to repair on the way.
doc.undo()
assert slot.week_pattern is None
assert doc.colloscope.interrogation(slot, week) == frozenset({0})

# The removal cascade, and the whole of what a slot drags along: the colles
# written on it, since there is no slot left to hold them, and the slot pairing
# rule that related it to another slot, since a rule with one end missing
# relates nothing.
removed = doc.slots.remove(slot)
warnings = removed.warnings

for w in warnings:
    assert isinstance(w, collomatique.Warning)
    assert str(w)
    assert isinstance(w.details, dict)

# The order is the reference sites' own: the rule, then the cells in week order.
assert [w.kind for w in warnings] == [
    "DeleteSlotPairingRule",
    "ClearInterrogationCell",
]
assert warnings[0].details == {"rule": rule.id}
assert warnings[1].details == {"slot": slot.id, "week": week.id}
# The write asked for the slot's removal itself, so neither repair hangs off
# another: this cascade is flat where a removal that takes slots with it is a
# tree.
assert all(w.parent is None for w in warnings)

assert slot not in doc.slots
assert len(doc.slots) == slots_before - 1
assert rule not in doc.slot_pairings
assert len(doc.slot_pairings) == rules_before - 1

# The rule the cascade took stales its handle, exactly as the slot's own does.
for read in (lambda: slot.teacher, lambda: rule.antecedent):
    try:
        read()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("what the cascade removed must read as gone")

# Undoing the removal puts back everything it took: the slot, the rule, and the
# colle that stood in the cell.
doc.undo()
assert slot in doc.slots
assert rule in doc.slot_pairings
assert len(doc.slots) == slots_before
assert doc.colloscope.interrogation(slot, week) == frozenset({0})
