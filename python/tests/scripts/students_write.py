import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the three labels are the
# french names `ops` gives the three operations of this family — handed in from
# rust so that this pins the operation's own label and not merely some string.
#
# `excluded_index` and `doomed_index` are places in the students' own order:
# rust cannot hand a script an id, so it names the two students whose fixtures
# this script leans on by where they sit, and asserts what it needs of them on
# its own side.
doc = collomatique.load(source)
students = doc.students

before = len(students)
periods = list(doc.periods)
first_period, last_period = periods[0], periods[-1]

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = students.add(
    collomatique.StudentData("Nymphadora", "Tonks", email="tonks@poudlard.fr")
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# A student arrives assigned to nothing and in no group, so there is nothing for
# the cascade to repair — but the result says so rather than the call saying
# nothing at all.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.Student)
assert isinstance(created.id, collomatique.StudentId)
assert created in students
assert students[created.id] == created
assert len(students) == before + 1

assert created.firstname == "Nymphadora"
assert created.surname == "Tonks"
assert created.tel is None
assert created.email == "tonks@poudlard.fr"
assert created.excluded_periods == frozenset()

assert added.created == created
assert repr(added).startswith("AddResult(created=<Student #")
assert "warnings=[]" in repr(added)

# A student who sits a period out is perfectly ordinary, and saying so at
# creation costs nothing: the new student is assigned nowhere, so the exclusion
# breaks nothing and the answer is as empty as the first one's.
joining_late = students.add(
    collomatique.StudentData("Sirius", "Black", excluded_periods={last_period})
)
assert joining_late.warnings == []
newcomer = joining_late.created
assert newcomer.excluded_periods == frozenset({last_period})
assert newcomer.tel is None
assert newcomer.email is None
assert len(students) == before + 2

# Rewriting replaces the whole value: the card and the excluded periods at once,
# and the id stays, so the handle a script is holding reads the new state. The
# email the value does not carry is gone rather than kept.
result = students.update(
    created,
    collomatique.StudentData(
        "Nymphadora", "Tonks-Lupin", tel="0102030405", excluded_periods={first_period}
    ),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")
# This student is assigned nowhere, so even the exclusion repairs nothing.
assert result.warnings == []

assert created.surname == "Tonks-Lupin"
assert created.tel == "0102030405"
assert created.email is None
assert created.excluded_periods == frozenset({first_period})
assert len(students) == before + 2

# The student is named by an id or by a handle, interchangeably — this is the
# write half of the argument convention.
students.update(created.id, collomatique.StudentData("Nymphadora", "Tonks"))
assert created.surname == "Tonks"
assert created.tel is None
assert created.excluded_periods == frozenset()

# The first cascade of this family, and it comes from an `update` rather than
# from a removal: `excluded` is assigned in several periods, and sitting one of
# them out means the rows of that period cannot go on naming them. The other
# periods' rows are untouched — the repair is precise — and so is everything
# that is not an assignment: a group list holds its members whatever periods
# they take part in.
excluded = list(students)[excluded_index]
excluded_rows = [
    (period, subject)
    for period, subject, members in doc.assignments
    if excluded in members
]
away_period = excluded_rows[0][0]
gone_rows = {(p.id, s.id) for p, s in excluded_rows if p == away_period}
kept_rows = {(p.id, s.id) for p, s in excluded_rows if p != away_period}
assert gone_rows and kept_rows

groups_before = {
    group_list.id
    for group_list in doc.group_lists
    if group_list.groups is not None
    and any(excluded in group for group in group_list.groups)
}
assert groups_before

exclusion = students.update(
    excluded,
    collomatique.StudentData(
        excluded.firstname,
        excluded.surname,
        tel=excluded.tel,
        email=excluded.email,
        excluded_periods={away_period},
    ),
)
assert excluded.excluded_periods == frozenset({away_period})
assert [w.kind for w in exclusion.warnings] == ["RemoveStudentFromAssignmentRow"] * len(
    gone_rows
)
assert {
    (w.details["period"], w.details["subject"]) for w in exclusion.warnings
} == gone_rows
assert all(w.details["student"] == excluded.id for w in exclusion.warnings)
# The write asked for those directly, so none of them hangs off another repair:
# a row that lost a name is still a row, and a group list that keeps one breaks
# nothing — this cascade is wide rather than deep.
assert all(w.parent is None for w in exclusion.warnings)

assert {
    (period.id, subject.id)
    for period, subject, members in doc.assignments
    if excluded in members
} == kept_rows
assert {
    group_list.id
    for group_list in doc.group_lists
    if group_list.groups is not None
    and any(excluded in group for group in group_list.groups)
} == groups_before

# This is what rust reads back off the disk.
doc.save(target)

# The removal cascade. `doomed` is named in three different kinds of place at
# once — the rows they are assigned in, the prefilled groups that hold them, and
# their own entry in the settings — and none of those can go on naming somebody
# the document no longer holds.
doomed = list(students)[doomed_index]
doomed_rows = {
    (period.id, subject.id)
    for period, subject, members in doc.assignments
    if doomed in members
}
doomed_lists = {
    group_list.id
    for group_list in doc.group_lists
    if group_list.groups is not None
    and any(doomed in group for group in group_list.groups)
}
assert doomed_rows and doomed_lists
assert doc.settings.override_for(doomed) is not None
overrides_before = len(doc.settings.overrides())

removed = students.remove(doomed)
assert isinstance(removed, collomatique.OpResult)
assert not hasattr(removed, "created")
warnings = removed.warnings
assert warnings, "removing a student who is named somewhere repairs something"

assert doomed not in students
assert len(students) == before + 1

# Every repair says the same four things, and its coordinates are the very ids
# this script was holding before the write.
for w in warnings:
    assert isinstance(w, collomatique.Warning)
    assert str(w)
    assert isinstance(w.details, dict)
    assert w.details["student"] == doomed.id

# The three kinds of site, each repaired in its own way: the rows let the name
# go, the prefilled groups let it go, and the settings entry goes whole, since
# an entry is nothing but the student it is keyed by.
assert {
    (w.details["period"], w.details["subject"])
    for w in warnings
    if w.kind == "RemoveStudentFromAssignmentRow"
} == doomed_rows
assert {
    w.details["group_list"]
    for w in warnings
    if w.kind == "RemoveStudentFromGroupListPrefill"
} == doomed_lists
assert [w.kind for w in warnings].count("ClearStudentSettings") == 1

# Nothing was taken away with them: an assignment row, a group list and a limits
# entry all survive losing one name, so no repair hangs off another.
assert all(w.parent is None for w in warnings)

assert not any(doomed in members for _p, _s, members in doc.assignments)
assert len(doc.settings.overrides()) == overrides_before - 1

try:
    doomed.surname
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed student must raise")

# A student nothing names takes nothing with them: the same op, and an empty
# answer, because this one had nothing to break.
gone = students.remove(created.id)
assert gone.warnings == []
assert len(students) == before
assert created not in students

try:
    created.excluded_periods
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed student must raise")

bare = collomatique.StudentData("Ailleurs", "Ailleurs")

# This family keeps no refusal for the model: the two things its ops can object
# to are a student the document does not hold and an excluded period that names
# nothing, and both are caught above the write, where the message can say which
# argument was wrong. A dead student is the argument convention's business …
for call in (
    lambda: students.remove(created),
    lambda: students.update(created, bare),
    lambda: students.update(created.id, bare),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead student must not be written through")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    students.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# `other` is this same file loaded twice, so its students and periods carry the
# very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_student = list(other.students)[0]
foreign_period = list(other.periods)[0]

try:
    students.remove(foreign_student)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a student of another document must not resolve")

# … and a period that names nothing is the value boundary's, so a value is
# refused before any op is built and nothing is written: a field naming another
# document's entity …
try:
    students.add(
        collomatique.StudentData(
            "Ailleurs", "Ailleurs", excluded_periods={foreign_period}
        )
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a period of another document names nothing here")

# … and a field that was never of the right shape.
try:
    students.add(collomatique.StudentData(3, "Ailleurs"))
except TypeError:
    pass
else:
    raise AssertionError("a firstname that is not a string must be refused")
assert len(students) == before

# A call that is wrong about both names the *student*: a value meant for nothing
# is moot, so the addressee is resolved first.
try:
    students.update(created, collomatique.StudentData(3, "Ailleurs"))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Each accepted call was its own undo slot, named by the operation itself.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert doc.undo_name == remove_label

# Undoing a removal puts back everything it took: the student, the rows they sat
# in, the groups that held them, and their entry in the settings.
assert created in students
assert created.surname == "Tonks"
doc.undo()
assert doomed in students
assert {
    (period.id, subject.id)
    for period, subject, members in doc.assignments
    if doomed in members
} == doomed_rows
assert doc.settings.override_for(doomed) is not None
assert len(doc.settings.overrides()) == overrides_before

assert doc.undo_name == update_label
doc.undo()
assert excluded.excluded_periods == frozenset()
assert {
    (period.id, subject.id)
    for period, subject, members in doc.assignments
    if excluded in members
} == gone_rows | kept_rows

doc.undo()
assert created.surname == "Tonks-Lupin"
doc.undo()
assert created.email == "tonks@poudlard.fr"
assert created.excluded_periods == frozenset()

assert doc.undo_name == add_label
doc.undo()
assert newcomer not in students
doc.undo()
assert len(students) == before
assert doc.can_undo is False
