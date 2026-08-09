import collomatique

# `source` is a colloscope document written by the test: a sparse assignments
# table, three rows on six possible pairs, is what lets the absent-address
# shape show up at all.
doc = collomatique.load(source)

assignments = doc.assignments
assert isinstance(assignments, collomatique.Assignments)
assert repr(assignments) == "<collomatique.Assignments>"

# The walk: the stored rows, as (Period, Subject, frozenset) triples. Only the
# non-empty rows are stored, so iteration is exactly the content.
rows = list(assignments)
assert all(
    isinstance(period, collomatique.Period) and period in doc.periods
    for period, _subject, _students in rows
)
assert all(
    isinstance(subject, collomatique.Subject) and subject in doc.subjects
    for _period, subject, _students in rows
)
assert all(
    isinstance(students, frozenset)
    and all(isinstance(student, collomatique.Student) for student in students)
    and len(students) > 0
    for _period, _subject, students in rows
)

row_period_indices = [period.index for period, _subject, _students in rows]
row_subject_indices = [subject.index for _period, subject, _students in rows]
row_student_surnames = [
    sorted(student.surname for student in students)
    for _period, _subject, students in rows
]

# A stored row reads back as the frozenset of the students in it, and a handle
# or an id names the same pair.
first_period, first_subject, first_students = rows[0]
assert assignments[first_period, first_subject] == first_students
assert assignments[first_period.id, first_subject.id] == first_students

# The students are live handles, reading the document as it is now.
assert all(isinstance(student, collomatique.Student) for student in first_students)
assert all(student.surname for student in first_students)

# The read is total: every valid address answers a frozenset, and an address
# the model stores no row for is the empty one — never a KeyError. Whether a
# subject runs in a period at all is `subject.excluded_periods`'s question,
# not this table's.
stored = {(period, subject) for period, subject, _students in rows}
absent = [
    (period, subject)
    for period in doc.periods
    for subject in doc.subjects
    if (period, subject) not in stored
]
assert len(absent) > 0
assert assignments[absent[0][0], absent[0][1]] == frozenset()
for period in doc.periods:
    for subject in doc.subjects:
        row = assignments[period, subject]
        assert isinstance(row, frozenset)
        assert bool(row) == ((period, subject) in stored)

# There is no len, no in, no get: over a total mapping, row count and row
# membership are statements about the model's storage, not about the data.
# (Without a `__contains__`, `in` would fall back to scanning the iteration,
# which is not what a membership test means here — so the collection never
# defines one.)
assert not hasattr(assignments, "get")
assert not hasattr(assignments, "__contains__")
try:
    len(assignments)
except TypeError:
    pass
else:
    raise AssertionError("a total mapping has no row count")

# An address is a (period, subject) pair — the one spelling python has for a
# pair. A bare key, a pair of another length, or a list are TypeError.
for bad in (
    3,
    "Maths",
    [first_period, first_subject],
    (first_period,),
    (first_period, first_subject, first_students),
):
    try:
        assignments[bad]
    except TypeError:
        pass
    else:
        raise AssertionError("an address is a (period, subject) pair")

# A position that is not a reference at all was never a question about this
# document.
for bad in (3, "Maths"):
    try:
        assignments[bad, first_subject]
    except TypeError:
        pass
    else:
        raise AssertionError("a period position takes a Period or a PeriodId")
for bad in (3, "Maths"):
    try:
        assignments[first_period, bad]
    except TypeError:
        pass
    else:
        raise AssertionError("a subject position takes a Subject or a SubjectId")

# An address whose reference belongs to another document is stale, whatever its
# id says. `other` is this same file loaded twice, so its ids are this
# document's very ids — and the refusal must say « somebody else's » rather
# than « missing », because nothing is missing here.
other = collomatique.load(source)
other_period = list(other.periods)[0]
try:
    assignments[other_period, first_subject]
except collomatique.StaleHandleError as error:
    assert "another document" in str(error)
    assert "is not in this document" not in str(error)
else:
    raise AssertionError("an address of another document must raise")
