import collomatique

# The second half. Between the stages rust stopped `partial` from running on
# the last period — nothing on the write surface can say that yet, the subjects
# being a later piece of the mirror — and the cascade took that row with it.
# What this stage says is what the table refuses once a subject does not run
# somewhere, which is the third and last of the model's own refusals here.
gone_period = periods[-1]

# The row went with the subject's period status, and the address reads as the
# empty frozenset like any other pair the model stores nothing for.
assert assignments[gone_period, partial] == frozenset()

for call, op in (
    (lambda: assignments.set(gone_period, partial, outsider, True), "Assign"),
    (lambda: assignments.set_all(gone_period, partial, True), "AssignAll"),
    # Both directions of `set_all`, and for the same reason: there is no row to
    # empty there either.
    (lambda: assignments.set_all(gone_period, partial, False), "AssignAll"),
):
    try:
        call()
    except collomatique.AssignmentsError as error:
        assert isinstance(error, collomatique.UpdateError)
        assert isinstance(error, collomatique.Error)
        assert str(error)
        assert error.op == op
        assert error.case == "SubjectDoesNotRunOnPeriod"
        # The subject the model named, then the period — the very ids this
        # script is holding, as the id classes.
        assert error.details == (partial.id, gone_period.id)
        assert isinstance(error.details[0], collomatique.SubjectId)
        assert isinstance(error.details[1], collomatique.PeriodId)
    else:
        raise AssertionError(
            "a subject that does not run on a period holds nobody there"
        )

# Only that pair is refused: the subject still runs on the other periods, and
# the table takes writes there as it always did.
still_running = assignments.set(first_period, partial, insider, False)
assert still_running.warnings == []
assert insider not in assignments[first_period, partial]

# A reference the document no longer holds is stale rather than refused by the
# model: the argument convention answers before the op is built, where the
# message can say which argument was wrong.
newcomer = doc.students.add(collomatique.StudentData("Nymphadora", "Tonks")).created
doc.students.remove(newcomer)

try:
    assignments.set(first_period, partial, newcomer, True)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError(
        "a student the document no longer holds must not be written through"
    )
