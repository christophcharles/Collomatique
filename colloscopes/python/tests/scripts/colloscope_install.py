import collomatique

# `source` is the two-filling fixture of the read surface's commit: a document
# written by the test, with cells on several (slot, week) pairs and placements
# for one automatic group list — the two shapes the example file never shows,
# since it was never resolved. `target` is where the script leaves the
# document for rust to read back, and `install_label` is the french name `ops`
# gives the operation this script runs — handed in from rust so that the undo
# assertion pins the operation's own label and not merely some string.
doc = collomatique.load(source)
colloscope = doc.colloscope
group_lists = doc.group_lists

# What the document opens on, read out whole. This is the round trip the
# solver makes: a value comes out, something edits it, and it goes back.
before = colloscope.to_data()
assert isinstance(before, collomatique.ColloscopeData)
assert len(before.interrogations) >= 2
assert len(before.group_lists) == 1

# The rows the edit is about, in the value's own key order: the first cell is
# rewritten, the last one is dropped, and the single placements row has one
# student moved.
cells = list(before.interrogations)
first_cell, dropped_cell = cells[0], cells[-1]
(automatic,) = before.group_lists

first_slot, first_week = doc.slots[first_cell[0]], doc.weeks[first_cell[1]]
bound = group_lists.association_for(first_week.period, first_slot.subject).group_count
assert bound >= 3

interrogations = dict(before.interrogations)
interrogations[first_cell] = {0, 1}
del interrogations[dropped_cell]

placed = dict(before.group_lists[automatic])
first_student = next(iter(placed))
placed[first_student] = (placed[first_student] + 1) % doc.group_lists[automatic].group_count
assert placed != dict(before.group_lists[automatic])

edited = collomatique.ColloscopeData(
    interrogations=interrogations, group_lists={automatic: placed})

landed = colloscope.install(edited)
assert isinstance(landed, collomatique.OpResult)
# Nothing in the document points at a colloscope row, so there is nothing for
# the cascade to repair, however much of the colloscope changed.
assert landed.warnings == []
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not isinstance(landed, collomatique.AddResult)
assert not hasattr(landed, "created")

# The document holds exactly the value's rows and no others: the rewritten
# cell is what the value says, and the dropped one is gone — this is a write
# of the whole colloscope and not an addition to it.
after = colloscope.to_data()
assert after == edited
assert colloscope.interrogation(first_slot, first_week) == frozenset({0, 1})
assert colloscope.interrogation(
    doc.slots[dropped_cell[0]], doc.weeks[dropped_cell[1]]) is None
assert dict(colloscope.group_list(doc.group_lists[automatic])) == {
    doc.students[student]: group for student, group in placed.items()}

# This is what rust reads back off the disk: exactly the rows the value named,
# in a document that opened with four cells and one placements row.
doc.save(target)

# One operation, and so one undo slot, however much changed: a single undo
# puts the whole colloscope back where it was.
assert doc.undo_name == install_label
doc.undo()
assert colloscope.to_data() == before
assert doc.can_undo is False

# The refusals are the model's, each a `ColloscopeError` naming the offending
# row — the same vocabulary the row-by-row writes speak, since the composite
# writes both kinds of row.
past_the_bound = collomatique.ColloscopeData(
    interrogations={first_cell: {bound}})
try:
    colloscope.install(past_the_bound)
except collomatique.ColloscopeError as error:
    assert isinstance(error, collomatique.UpdateError)
    assert isinstance(error, collomatique.Error)
    assert str(error)
    assert error.op == "InstallColloscope"
    assert error.case == "InvalidGroupNumInInterrogation"
    # The entities the model named, as the id classes — the very ones this
    # script is holding.
    assert error.details == (first_cell[0], first_cell[1])
else:
    raise AssertionError("a group number past the bound must refuse")

# Nothing of that was written: the refusal cost no undo slot either.
assert colloscope.to_data() == before
assert doc.can_undo is False

# A value naming something the document no longer holds is refused by the
# argument convention, above the write, where the message can say which id was
# wrong — so it never reaches the model at all.
doc.slots.remove(first_slot)
try:
    colloscope.install(edited)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a value naming a dead slot must not resolve")
