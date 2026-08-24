import collomatique

# `source` is a document written by the test: an automatic group list and a
# prefilled one with named groups, side by side. The example has only prefilled
# lists and only unnamed groups, so the `None`-for-inapplicable rule and the
# stored-name half of group_name need a document of their own.
doc = collomatique.load(source)

group_lists = doc.group_lists
assert len(group_lists) == 2
assert repr(group_lists) == "<collomatique.GroupLists count=2>"

# The two filling shapes, told apart by the attribute that says so.
by_kind = {group_list.is_prefilled: group_list for group_list in group_lists}
assert set(by_kind) == {False, True}
automatic = by_kind[False]
prefilled = by_kind[True]

# The automatic list: `.groups` is None — the question does not apply — and
# `.excluded_students` is a real frozenset of live students, empty or not.
assert automatic.groups is None
excluded = automatic.excluded_students
assert isinstance(excluded, frozenset)
assert all(isinstance(student, collomatique.Student) for student in excluded)
excluded_surnames = sorted(student.surname for student in excluded)

# The prefilled list: `.excluded_students` is None, and `.groups` is one
# frozenset per group, in group order.
assert prefilled.excluded_students is None
groups = prefilled.groups
assert isinstance(groups, tuple)
assert len(groups) == prefilled.group_count
assert all(
    isinstance(group, frozenset)
    and all(isinstance(student, collomatique.Student) for student in group)
    for group in groups
)
prefilled_members = [
    sorted(student.surname for student in group) for group in groups
]

# group_name reads the stored name where there is one, and falls back to the
# application's « Groupe N » only for an unnamed group — here the middle one,
# so the fallback is « Groupe 2 ».
assert any(name is None for name in prefilled.group_names)
assert any(name is not None for name in prefilled.group_names)
shown_names = [prefilled.group_name(i) for i in range(prefilled.group_count)]
assert "Groupe 2" in shown_names
assert any("Groupe" not in name for name in shown_names)

# Both lists read their own shape: a named list, a range, a count.
automatic_name = automatic.name
automatic_students_per_group = automatic.students_per_group
automatic_group_count = automatic.group_count
prefilled_name = prefilled.name
prefilled_students_per_group = prefilled.students_per_group
prefilled_group_count = prefilled.group_count

# The associations reach the automatic list too — the hop is not prefilled-only.
rows = list(group_lists.associations())
row_count = len(rows)
assert any(group_list == automatic for _period, _subject, group_list in rows)
assert any(group_list == prefilled for _period, _subject, group_list in rows)
