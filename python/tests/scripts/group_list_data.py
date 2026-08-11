import dataclasses

import collomatique

# `source` is a throwaway copy of the document the read surface's commit 8
# built for the two filling shapes: an automatic group list and a prefilled
# one with named groups, side by side. The example has only prefilled lists
# and only unnamed groups, so it cannot carry this test.
doc = collomatique.load(source)

group_list_list = list(doc.group_lists)
assert len(group_list_list) == 2

# What a handle hands back detached, in the collection's order, which is the
# order rust compares them in.
gl_values = [gl.to_data() for gl in group_list_list]
assert all(isinstance(d, collomatique.GroupListData) for d in gl_values)

# The fields as python sees them, so that a conversion wrong in both
# directions at once — the name and a group name swapped, say — cannot pass
# rust's round-trip comparison by cancelling itself out.
gl_names = [d.name for d in gl_values]
gl_ranges = [d.students_per_group for d in gl_values]
group_names_lists = [d.group_names for d in gl_values]

# The filling keeps the sum: one of the two leaf values, both of them under
# the `Filling` base, and told apart by their class.
assert all(isinstance(d.filling, collomatique.Filling) for d in gl_values)
assert {type(d.filling) for d in gl_values} == {
    collomatique.AutomaticGroups,
    collomatique.PrefilledGroups,
}
by_kind = {type(d.filling): d for d in gl_values}
automatic_value = by_kind[collomatique.AutomaticGroups]
prefilled_value = by_kind[collomatique.PrefilledGroups]

# The students come out as ids, never as handles: the value is detached, and a
# handle would carry the document with it.
excluded = automatic_value.filling.excluded_students
assert isinstance(excluded, frozenset)
assert all(isinstance(student, collomatique.StudentId) for student in excluded)
excluded_surnames = sorted(
    student.surname for student in doc.students if student.id in excluded
)

groups = prefilled_value.filling.groups
assert isinstance(groups, tuple)
assert all(isinstance(group, frozenset) for group in groups)
assert all(
    isinstance(student, collomatique.StudentId)
    for group in groups
    for student in group
)
prefilled_members = [
    sorted(student.surname for student in doc.students if student.id in group)
    for group in groups
]

# The filling's containers are frozen — a leaf value is built whole and
# replaced whole — and `==` and `hash` compare the content, not the object.
assert hash(automatic_value.filling) == hash(
    collomatique.AutomaticGroups(excluded)
)
assert hash(prefilled_value.filling) == hash(collomatique.PrefilledGroups(groups))
assert automatic_value.filling == collomatique.AutomaticGroups(excluded)
assert prefilled_value.filling == collomatique.PrefilledGroups(groups)
assert automatic_value.filling != prefilled_value.filling

# A fresh object every call. Two of them are equal and share nothing, so
# writing to one is invisible to the other and to the document.
first = group_list_list[0]
again = first.to_data()
assert again == gl_values[0]
assert again is not gl_values[0]
again.group_names.append("Héphaïstos")
assert gl_values[0].group_names != again.group_names

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
scratch = collomatique.GroupListData(name=3)
scratch.students_per_group = "beaucoup"
scratch.filling = None

# And a value has no identity: an id names a place in a document, and a value
# has none. Updating an existing group list will pass the id as the method's
# argument.
assert not hasattr(gl_values[0], "id")

# The field order of each class, which is what a positional call depends on:
# required first, then the defaulted ones in the order the handle shows them.
assert [f.name for f in dataclasses.fields(collomatique.GroupListData)] == [
    "name",
    "students_per_group",
    "group_names",
    "filling",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import GroupListData as _same_class  # noqa: E402

assert _same_class is collomatique.GroupListData
assert collomatique.GroupListData.__module__ == "collomatique"

# All the model's own defaults, at once: `clm.GroupListData()` is what the
# application creates when a user adds a group list.
bare = collomatique.GroupListData()
assert bare.name == "Liste"
assert bare.students_per_group == (2, 3)
assert bare.group_names == [None] * 16
assert isinstance(bare.filling, collomatique.AutomaticGroups)
assert bare.filling.excluded_students == frozenset()

# A field that names an entity takes a handle or an id, interchangeably. The
# two fillings below extract to the same group list and — this is the wart
# §2.3 of the design records — do not compare equal, because a leaf value
# stores what it was given, and a handle and an id hash differently.
harry, hermione, ron, neville, _luna = list(doc.students)

by_handle = collomatique.GroupListData(
    "Maisons",
    group_names=["Aurore", None, "Serdaigle"],
    filling=collomatique.PrefilledGroups(({harry, hermione}, {ron}, {neville})),
)
by_id = collomatique.GroupListData(
    "Maisons",
    group_names=["Aurore", None, "Serdaigle"],
    filling=collomatique.PrefilledGroups(
        ({harry.id, hermione.id}, {ron.id}, {neville.id})
    ),
)
assert by_handle != by_id

# The automatic shape, by id, with the exclusions the fixture keeps.
automatic_by_id = collomatique.GroupListData(
    "Automatique",
    students_per_group=(1, 2),
    group_names=[None] * 4,
    filling=collomatique.AutomaticGroups({ron.id}),
)

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
not_a_name = collomatique.GroupListData(name=3)
not_a_range = collomatique.GroupListData(students_per_group=(5, 2))
not_a_names_list = collomatique.GroupListData(group_names=3)
not_an_entry = collomatique.GroupListData(group_names=["Aurore", ""])
not_a_filling = collomatique.GroupListData(filling="Aurore")

# The two sealed-constructor violations, with the group names written out so
# the count mismatch is about the count and not about the default of sixteen.
mismatched_count = collomatique.GroupListData(
    "Maisons",
    group_names=["Aurore", None, "Serdaigle"],
    filling=collomatique.PrefilledGroups(({harry}, {hermione})),
)
duplicated_student = collomatique.GroupListData(
    "Maisons",
    group_names=["Aurore", None],
    filling=collomatique.PrefilledGroups(({harry}, {harry, hermione})),
)

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_student = collomatique.GroupListData(
    "Maisons",
    filling=collomatique.AutomaticGroups({list(other.students)[0]}),
)
