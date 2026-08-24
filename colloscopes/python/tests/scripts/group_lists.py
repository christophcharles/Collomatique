import collomatique

# `source` is the example colloscope: two prefilled group lists with every
# group unnamed — « Liste principale » and « Divination » — and eighteen
# associations across three periods and eight subjects, which leaves six valid
# pairs unassociated. That is what pins the collection protocol, the group_name
# fallback, and the association reads.
doc = collomatique.load(source)

group_lists = doc.group_lists
assert isinstance(group_lists, collomatique.GroupLists)
assert repr(group_lists) == "<collomatique.GroupLists count=2>"

# The collection protocol: a plain collection, iterating in id order.
lists = list(group_lists)
assert all(
    isinstance(group_list, collomatique.GroupList) and group_list in doc.group_lists
    for group_list in lists
)
assert len(lists) == 2
first, second = lists
assert group_lists[first.id] == first
assert group_lists[first] == first
assert group_lists.get(first.id) == first
assert group_lists.get(first) == first
assert group_lists.get(second) == second
assert first in group_lists and second in group_lists

# The two lists iterate in id order, whatever the ids are.
assert [group_list.id for group_list in lists] == sorted(
    group_list.id for group_list in lists
)

first_repr = repr(first)

gl_names = [group_list.name for group_list in lists]
gl_students_per_group = [group_list.students_per_group for group_list in lists]
gl_group_counts = [group_list.group_count for group_list in lists]
gl_group_names = [group_list.group_names for group_list in lists]
gl_is_prefilled = [group_list.is_prefilled for group_list in lists]

# Both example lists are prefilled and every group is unnamed, so the groups
# read as real frozensets of live students, the exclusions answer None, and
# group_name is the fallback all the way down.
assert all(group_list.is_prefilled for group_list in lists)
assert all(group_list.excluded_students is None for group_list in lists)
assert all(name is None for group_list in lists for name in group_list.group_names)
assert all(
    group_list.groups is not None
    and len(group_list.groups) == group_list.group_count
    for group_list in lists
)
assert all(
    isinstance(group, frozenset)
    and all(isinstance(student, collomatique.Student) and student.surname for student in group)
    for group_list in lists
    for group in group_list.groups
)
gl_group_members = [
    [sorted(student.surname for student in group) for group in group_list.groups]
    for group_list in lists
]

# The fallback is the application's own: « Groupe 3 » for the third group of a
# list, number and nothing else.
fallback_names = [
    group_list.group_name(i)
    for group_list in lists
    for i in range(group_list.group_count)
]
assert "Groupe 1" in fallback_names

# A group number past the list's count is IndexError, and nothing else.
for group_list in lists:
    for bad in (group_list.group_count, group_list.group_count + 1):
        try:
            group_list.group_name(bad)
        except IndexError:
            pass
        else:
            raise AssertionError("a group number past the count is IndexError")

# The associations: the stored rows, in key order, as
# (Period, Subject, GroupList) triples.
rows = list(group_lists.associations())
assert all(
    isinstance(period, collomatique.Period) and period in doc.periods
    for period, _subject, _group_list in rows
)
assert all(
    isinstance(subject, collomatique.Subject) and subject in doc.subjects
    for _period, subject, _group_list in rows
)
assert all(
    isinstance(group_list, collomatique.GroupList) and group_list in doc.group_lists
    for _period, _subject, group_list in rows
)
row_period_indices = [period.index for period, _subject, _group_list in rows]
row_subject_indices = [subject.index for _period, subject, _group_list in rows]
row_group_positions = [lists.index(group_list) for _period, _subject, group_list in rows]

# The hop: an associated pair answers the stored list, by handle or by id.
first_period, first_subject, first_list = rows[0]
assert group_lists.association_for(first_period, first_subject) == first_list
assert group_lists.association_for(first_period.id, first_subject.id) == first_list

# The read is total over valid addresses: a pair the model stores no
# association for answers None, never an error.
stored = {(period, subject) for period, subject, _group_list in rows}
absent = [
    (period, subject)
    for period in doc.periods
    for subject in doc.subjects
    if (period, subject) not in stored
]
assert len(absent) > 0
assert group_lists.association_for(absent[0][0], absent[0][1]) is None

# A position that is not a reference at all was never a question about this
# document.
for bad in (3, "Maths"):
    try:
        group_lists.association_for(bad, first_subject)
    except TypeError:
        pass
    else:
        raise AssertionError("a period position takes a Period or a PeriodId")
for bad in (3, "Maths"):
    try:
        group_lists.association_for(first_period, bad)
    except TypeError:
        pass
    else:
        raise AssertionError("a subject position takes a Subject or a SubjectId")

# A reference of another document is stale, whatever its id says. `other` is
# this same file loaded twice, so its ids are this document's very ids — and
# the refusal must say « somebody else's » rather than « missing », because
# nothing is missing here.
other = collomatique.load(source)
other_period = list(other.periods)[0]
try:
    group_lists.association_for(other_period, first_subject)
except collomatique.StaleHandleError as error:
    assert "another document" in str(error)
    assert "is not in this document" not in str(error)
else:
    raise AssertionError("an address of another document must raise")
