import collomatique

# The second half: `dead_period`, `no_previous`, `too_long` and `dead_subject`
# are the exceptions four refused writes raised, in the order rust applied them.

# Every refusal is an `UpdateError`, and so a `collomatique.Error`: a script
# that only wants « the write failed » still catches one thing. The message is
# the model's own sentence, which is what a script prints.
for error in (dead_period, no_previous, too_long, dead_subject):
    assert isinstance(error, collomatique.UpdateError)
    assert isinstance(error, collomatique.Error)
    assert str(error)

# The class says which family refused …
assert isinstance(dead_period, collomatique.GeneralPlanningError)
assert isinstance(dead_subject, collomatique.SubjectsError)
assert not isinstance(dead_subject, collomatique.GeneralPlanningError)

# … and the attributes say which op, which case, and about what.
assert dead_period.op == "UpdatePeriodWeekCount"
assert dead_period.case == "InvalidPeriodId"
assert dead_period.details == (doomed_id,)
# An id among the details is the id class, not a number a script could do
# nothing with: it is the very id this script is still holding.
assert isinstance(dead_period.details[0], collomatique.PeriodId)

# A case that carries nothing carries the empty tuple, rather than carrying
# nothing at all — every case reads the same way.
assert no_previous.op == "MergeWithPreviousPeriod"
assert no_previous.case == "NoPreviousPeriodToMergeWith"
assert no_previous.details == ()

# One that carries two numbers carries both, in the model's own order: what was
# asked for, then what there was.
assert too_long.op == "CutPeriod"
assert too_long.case == "RemainingWeekCountTooBig"
assert too_long.details == (first_week_count + 5, first_week_count)

assert dead_subject.op == "DeleteSubject"
assert dead_subject.case == "InvalidSubjectId"
assert dead_subject.details == (doomed_subject_id,)
