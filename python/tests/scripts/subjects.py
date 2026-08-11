import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

subjects = doc.subjects
assert isinstance(subjects, collomatique.Subjects)

subject_list = list(subjects)
assert len(subject_list) == len(subjects)
assert all(isinstance(subject, collomatique.Subject) for subject in subject_list)

# Iteration is user order, and `.index` is the position in it.
subject_indices = [subject.index for subject in subject_list]
subject_names = [subject.name for subject in subject_list]

# Indexing takes an id or a handle, and hands back an equal handle either way.
for subject in subject_list:
    assert subjects[subject.id] == subject
    assert subjects[subject] == subject
    assert subjects.get(subject.id) == subject
    assert subject.id in subjects
    assert subject in subjects

# A subject is never equal to something that is not one — that is an answer and
# not an error, so `!=` against a stranger is simply true. A handle of another
# kind is a stranger too, which the periods say at the end of this script.
# Handles identify; they do not order, which is what ids are for.
assert subject_list[0] != 3
assert subject_list[0] != "Sortilèges"
assert subject_list[0] != None  # noqa: E711 — `is not` would not call `__eq__`
try:
    subject_list[0] < subject_list[1]
except TypeError:
    pass
else:
    raise AssertionError("ordering two handles must raise")

assert 3 not in subjects
assert subjects.get(3) is None
try:
    subjects[3]
except KeyError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# A handle is something the document hands out, and it has no setters.
try:
    collomatique.Subject()
except TypeError:
    pass
else:
    raise AssertionError("a handle must not be constructible")
try:
    subject_list[0].name = "Sortilèges"
except AttributeError:
    pass
else:
    raise AssertionError("assigning to a handle attribute must raise")

# The example holds subjects of both shapes: some run colles, some are only
# there to take up room in the timetable.
interrogations = [subject.interrogation for subject in subject_list]
interrogation_present = [view is not None for view in interrogations]
assert any(interrogation_present)
assert not all(interrogation_present)

with_colles = [subject for subject in subject_list if subject.interrogation is not None]
assert all(
    isinstance(subject.interrogation, collomatique.Interrogation)
    for subject in with_colles
)

students_per_group = [subject.interrogation.students_per_group for subject in with_colles]
groups_per_interrogation = [
    subject.interrogation.groups_per_interrogation for subject in with_colles
]
durations = [subject.interrogation.duration for subject in with_colles]
take_duration_into_account = [
    subject.interrogation.take_duration_into_account for subject in with_colles
]
periodicity_class_names = [
    type(subject.interrogation.periodicity).__name__ for subject in with_colles
]

# A range is a plain `(min, max)` tuple of ints, inclusive at both ends.
for low, high in students_per_group + groups_per_interrogation:
    assert isinstance(low, int) and isinstance(high, int)
    assert low <= high

# Every periodicity is one of the four, and the base class catches all of them.
assert all(
    isinstance(subject.interrogation.periodicity, collomatique.Periodicity)
    for subject in with_colles
)
try:
    collomatique.Periodicity()
except TypeError:
    pass
else:
    raise AssertionError("the periodicity base class must not be constructible")

# The sub-view is a view, not the object the document keeps: two of them for the
# same subject are different objects that compare and hash the same.
first = with_colles[0]
again = first.interrogation
assert again is not first.interrogation
assert again == first.interrogation
assert hash(again) == hash(first.interrogation)
assert again != with_colles[1].interrogation

# And a view is never equal to something that is not one — not even the subject
# it was read out of, which is the handle standing nearest to it. It does not
# order either: it names a subject's colles, and that is not a position.
assert again != 3
assert again != "Sortilèges"
assert not (again == first)
assert again != None  # noqa: E711 — `is not` would not call `__eq__`
try:
    again < with_colles[1].interrogation
except TypeError:
    pass
else:
    raise AssertionError("ordering two views must raise")

try:
    collomatique.Interrogation()
except TypeError:
    pass
else:
    raise AssertionError("a sub-view must not be constructible")

# The periods a subject skips come back as a frozenset of live handles.
excluded_period_indices = [
    sorted(period.index for period in subject.excluded_periods)
    for subject in subject_list
]
assert all(isinstance(subject.excluded_periods, frozenset) for subject in subject_list)

# A handle from another document names nothing here, whatever its id says.
other = collomatique.load(source)
assert subject_list[0] not in other.subjects
assert other.subjects.get(subject_list[0]) is None
assert other.subjects[subject_list[0].id] == list(other.subjects)[0]

# Nor does a handle of another kind.
assert subject_list[0] != list(doc.periods)[0]
assert list(doc.periods)[0] not in subjects
