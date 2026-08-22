import collomatique

# The first stage left the document as it opened it, and rust has since stopped
# the cell's subject from running on the cell's period — the subjects are a
# later piece of the mirror, so that is the one thing this script cannot say
# for itself, and it is the only way the last of the family's refusals is
# reachable at all.

# The coordinate could hold a colle a moment ago and cannot now: the read says
# so, and the write agrees with it.
assert doc.is_interrogation_possible(cell_slot, cell_week) is False

# The exclusion took the group list association of that coordinate with it, so
# the group bound there is zero and group 0 is out of range too. The model
# answers with the more telling of the two: the slot does not run there at all.
assert group_lists.association_for(cell_period, cell_subject) is None

try:
    colloscope.set_interrogation(cell_slot, cell_week, {0})
except collomatique.ColloscopeError as error:
    assert isinstance(error, collomatique.UpdateError)
    assert str(error)
    assert error.op == "UpdateColloscopeInterrogation"
    assert error.case == "SlotNotRunningOnPeriod"
    assert error.details == (cell_slot.id, cell_week.id)
else:
    raise AssertionError("a colle on a period the subject skips must refuse")

# Nothing was written, and the refusal cost no undo slot.
assert colloscope.interrogation(cell_slot, cell_week) is None
assert doc.undo_name == exclusion_label
