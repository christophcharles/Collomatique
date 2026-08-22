import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the labels are the
# french names `ops` gives the operations this script drives — handed in from
# rust so that this pins the operations' own labels and not merely some
# strings.
#
# `partial_index` and `excluded_index` are places in the subjects' and the
# students' own order: rust cannot hand a script an id, so it names the subject
# only some students take, and the student this script excludes from a period,
# by where they sit.
#
# This is the first half of a two-stage script: what a subject that does not
# run on a period refuses is the next stage's, since nothing on the write
# surface can stop a subject from running yet.
doc = collomatique.load(source)
assignments = doc.assignments

periods = list(doc.periods)
first_period, second_period = periods[0], periods[1]
subjects = list(doc.subjects)
partial = subjects[partial_index]
students = list(doc.students)
everybody = frozenset(students)

# The row this script rewrites five times over: some students take this subject
# on the first period and some do not, so both directions of `set` have
# something to do there.
before = assignments[first_period, partial]
assert before and before < everybody
before_second = assignments[second_period, partial]

outsider = next(student for student in students if student not in before)
insider = next(student for student in students if student in before)

# A row is not an entity — it is the three ids it is made of — so this write
# creates nothing and answers a plain `OpResult` rather than the `AddResult`
# subclass.
joined = assignments.set(first_period, partial, outsider, True)
assert isinstance(joined, collomatique.OpResult)
assert not isinstance(joined, collomatique.AddResult)
assert not hasattr(joined, "created")

# Nothing in the document points *at* a row, so no write of this family ever
# gives the cascade something to repair. Every result below says so in turn.
assert joined.warnings == []
assert assignments[first_period, partial] == before | {outsider}

# The same write again, spelled with ids: the three addresses are handles or
# ids interchangeably, which is the write half of the argument convention. And
# `assigned` says what the row must hold afterwards rather than toggling
# anything, so a student who is already there stays there — the write is
# accepted and changes nothing. It still costs an undo slot: a write that had
# nothing to do is a write.
again = assignments.set(first_period.id, partial.id, outsider.id, True)
assert again.warnings == []
assert assignments[first_period, partial] == before | {outsider}

# The other direction, one student at a time.
left = assignments.set(first_period, partial, insider, False)
assert left.warnings == []
assert assignments[first_period, partial] == (before | {outsider}) - {insider}

# The whole row in one write. An emptied row is stored as no row at all, and
# that canonical form is invisible from python except in the walk: the address
# still reads, as the empty frozenset.
emptied = assignments.set_all(first_period, partial, False)
assert emptied.warnings == []
assert assignments[first_period, partial] == frozenset()
assert (first_period, partial) not in {
    (period, subject) for period, subject, _members in doc.assignments
}

filled = assignments.set_all(first_period.id, partial, True)
assert filled.warnings == []
assert assignments[first_period, partial] == everybody

# The three writes below need a student the second period takes no part in, and
# saying so is the students family's business rather than this one's: excluding
# them takes them out of the rows of that period, and that removal is *their*
# cascade, not an assignments write at all.
excluded = students[excluded_index]
assert excluded in assignments[second_period, partial]
doc.students.update(
    excluded,
    collomatique.StudentData(
        excluded.firstname,
        excluded.surname,
        tel=excluded.tel,
        email=excluded.email,
        excluded_periods={second_period},
    ),
)
assert not any(
    excluded in members
    for period, _subject, members in doc.assignments
    if period == second_period
)

# The first of the model's own refusals: a student who takes no part in a
# period cannot be assigned in it, and that is a statement about the document
# rather than about an argument's shape.
try:
    assignments.set(second_period, partial, excluded, True)
except collomatique.AssignmentsError as error:
    assert isinstance(error, collomatique.UpdateError)
    assert isinstance(error, collomatique.Error)
    assert str(error)
    assert error.op == "Assign"
    assert error.case == "StudentIsNotPresentOnPeriod"
    # The student the model named, then the period — the very ids this script
    # is holding, as the id classes.
    assert error.details == (excluded.id, second_period.id)
    assert isinstance(error.details[0], collomatique.StudentId)
    assert isinstance(error.details[1], collomatique.PeriodId)
else:
    raise AssertionError("a student who sits a period out cannot be assigned in it")

# `set_all` does not refuse over that same student: it assigns everybody the
# period does not exclude, so skipping them is the rule rather than an error.
everyone_else = assignments.set_all(second_period, partial, True)
assert everyone_else.warnings == []
assert assignments[second_period, partial] == everybody - {excluded}

# The second period takes the first one's rows, subject by subject …
duplicated = assignments.duplicate_previous_period(second_period)
assert duplicated.warnings == []
for subject in subjects:
    assert (
        assignments[second_period, subject]
        == assignments[first_period, subject] - {excluded}
    )

# … except for the student either of the two periods excludes, who keeps what
# they have: they sit in the first period's rows and stay out of the second's.
assert excluded in assignments[first_period, partial]
assert not any(
    excluded in members
    for period, _subject, members in doc.assignments
    if period == second_period
)

# This is what rust reads back off the disk.
doc.save(target)

# Everything below is refused, so this is the table every one of them must
# leave alone.
table_before = {
    (period.id, subject.id): frozenset(student.id for student in members)
    for period, subject, members in doc.assignments
}

# The second of the model's own refusals: the first period has nothing before
# it, and asking anyway is told so rather than quietly doing nothing.
try:
    assignments.duplicate_previous_period(first_period)
except collomatique.AssignmentsError as error:
    assert error.op == "DuplicatePreviousPeriod"
    assert error.case == "FirstPeriodHasNoPreviousPeriod"
    assert error.details == (first_period.id,)
else:
    raise AssertionError("the first period has no previous period to copy")

# Something that was never a reference to this document is a `TypeError`: it is
# not a stale anything. And `assigned` is a flag rather than a truthy value —
# the model stores a bool, so python takes one.
for call in (
    lambda: assignments.set(3, partial, outsider, True),
    lambda: assignments.set(first_period, 3, outsider, True),
    lambda: assignments.set(first_period, partial, 3, True),
    lambda: assignments.set_all(3, partial, True),
    lambda: assignments.set_all(first_period, 3, True),
    lambda: assignments.duplicate_previous_period(3),
    # A handle of the wrong kind is no better: a student is not a subject.
    lambda: assignments.set(first_period, outsider, partial, True),
    lambda: assignments.set(first_period, partial, outsider, 1),
    lambda: assignments.set_all(first_period, partial, "oui"),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("an argument of the wrong kind must not resolve")

# `other` is this same file loaded twice, so its periods, subjects and students
# carry the very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_period = list(other.periods)[0]
foreign_subject = list(other.subjects)[0]
foreign_student = list(other.students)[0]

for call in (
    lambda: assignments.set(foreign_period, partial, outsider, True),
    lambda: assignments.set(first_period, foreign_subject, outsider, True),
    lambda: assignments.set(first_period, partial, foreign_student, True),
    lambda: assignments.set_all(foreign_period, partial, True),
    lambda: assignments.set_all(first_period, foreign_subject, True),
    lambda: assignments.duplicate_previous_period(foreign_period),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a reference of another document must not resolve")

# A call wrong about several of its addresses names the first: they are
# resolved in the order they are written, so a period that names nothing is
# what the caller is told about even when the subject was never a subject.
try:
    assignments.set(foreign_period, 3, outsider, True)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addresses are resolved in the order they are written")

# Not one of those wrote anything.
assert {
    (period.id, subject.id): frozenset(student.id for student in members)
    for period, subject, members in doc.assignments
} == table_before

# Each accepted write was its own undo slot, named by the operation itself —
# and the one write of another family in the middle carries that family's name
# rather than this one's.
for label in (
    duplicate_label,
    assign_all_label,
    student_update_label,
    assign_all_label,
    unassign_all_label,
    unassign_label,
    assign_label,
    assign_label,
):
    assert doc.undo_name == label
    doc.undo()

# Undone one by one, the document is the one the script opened.
assert doc.can_undo is False
assert assignments[first_period, partial] == before
assert assignments[second_period, partial] == before_second
assert excluded.excluded_periods == frozenset()
