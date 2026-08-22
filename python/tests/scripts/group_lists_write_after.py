import collomatique

# The stage before this one undid every write it made, so the document is the
# example again — and rust has since written four things the write surface
# cannot reach yet. Three of them are the colloscope's, which is a later piece
# of the mirror: the served list turned automatic, a placement row for it, and
# one colle at a coordinate that list bounds. The fourth is the subjects', also
# a later piece: the served subject stopped running on one period, which is
# what makes the last refusal reachable at all.
assert prepared_filling.warnings == []
assert prepared_placements.warnings == []
assert prepared_cell.warnings == []

group_lists = doc.group_lists

# The list is the only automatic one, and the cell is the only colle: rust
# cannot hand a script an id, so the script reads its own off the document.
served = next(
    group_list for group_list in group_lists if not group_list.is_prefilled
)
slot, week, groups = list(doc.colloscope.interrogations())[0]
cell_period, cell_subject = week.period, slot.subject

# The colle is measured against the list its coordinate uses — that is the hop
# every cascade below walks.
assert group_lists.association_for(cell_period, cell_subject) == served
assert groups == frozenset({0, 2})
assert served.group_count > 2

placements = doc.colloscope.group_list(served)
placed_low = next(student for student, group in placements.items() if group == 0)
placed_high = next(student for student, group in placements.items() if group == 2)

# The `update` cascade, both halves at once: a list with fewer groups leaves
# out of range both the colles that name a dropped group and the students
# placed in one. A read-modify-write is what a script does here, and it carries
# the filling along with the parameters, since the model seals the two.
narrowed = served.to_data()
assert len(narrowed.group_names) == served.group_count
narrowed.group_names = narrowed.group_names[:2]
result = group_lists.update(served, narrowed)

for w in result.warnings:
    assert isinstance(w, collomatique.Warning)
    assert str(w)
    assert isinstance(w.details, dict)
    # The whole-value argument the elementary op needed is not a coordinate.
    assert "rebuilt" not in w.details

# The colle goes first: the interrogation row is the predicate the model
# declares ahead of the placement one.
assert [w.kind for w in result.warnings] == [
    "RemoveGroupsFromInterrogationCell",
    "RemoveStudentColloscopePlacement",
]
trimmed, unplaced = result.warnings
assert trimmed.details == {"slot": slot.id, "week": week.id, "groups": [2]}
assert unplaced.details == {"group_list": served.id, "student": placed_high.id}
# The write asked for the list itself, so neither repair hangs off another.
assert all(w.parent is None for w in result.warnings)

# What still fits was left alone, in both tables.
assert doc.colloscope.interrogation(slot, week) == frozenset({0})
assert dict(doc.colloscope.group_list(served)) == {placed_low: 0}
assert served.group_count == 2

doc.undo()
assert doc.colloscope.interrogation(slot, week) == frozenset({0, 2})
assert dict(doc.colloscope.group_list(served)) == {placed_low: 0, placed_high: 2}

# A prefilled list holds its groups itself, so it has no placement row at all:
# turning this one prefilled retires the whole row in a single repair, and not
# one repair per student. The colles are untouched — they name group numbers,
# and the list still has as many groups as before.
whole = served.to_data()
whole.filling = collomatique.PrefilledGroups(
    [frozenset() for _name in whole.group_names]
)
result = group_lists.update(served, whole)

assert [w.kind for w in result.warnings] == ["ClearColloscopeGroupListRow"]
assert result.warnings[0].details == {"group_list": served.id}
assert result.warnings[0].parent is None
assert doc.colloscope.group_list(served) is None
assert doc.colloscope.interrogation(slot, week) == frozenset({0, 2})

doc.undo()
assert dict(doc.colloscope.group_list(served)) == {placed_low: 0, placed_high: 2}

# Taking the list away from the coordinate takes the group bound there to zero,
# so every group of every colle written at it is out of range and the cell
# empties in one repair naming all of them.
result = group_lists.set_association(cell_period, cell_subject, None)

assert [w.kind for w in result.warnings] == ["RemoveGroupsFromInterrogationCell"]
assert result.warnings[0].details == {
    "slot": slot.id,
    "week": week.id,
    "groups": [0, 2],
}
# An emptied cell is stored as no cell at all, which is what `None` reads.
assert doc.colloscope.interrogation(slot, week) is None
assert group_lists.association_for(cell_period, cell_subject) is None
# The list itself is untouched: a list nobody uses is an ordinary document.
assert served in group_lists
assert dict(doc.colloscope.group_list(served)) == {placed_low: 0, placed_high: 2}

doc.undo()
assert group_lists.association_for(cell_period, cell_subject) == served
assert doc.colloscope.interrogation(slot, week) == frozenset({0, 2})

# The family's second model refusal, and the one the example alone cannot show:
# a subject that does not run on a period holds no group list there. Rust
# stopped the subject from running on `gone_period` between the stages.
gone_period = next(
    period for period in doc.periods if period in cell_subject.excluded_periods
)
assert gone_period != cell_period
for group_list in (served, None):
    try:
        group_lists.set_association(gone_period, cell_subject, group_list)
    except collomatique.GroupListsError as error:
        assert isinstance(error, collomatique.UpdateError)
        assert str(error)
        assert error.op == "AssignGroupListToSubject"
        assert error.case == "SubjectDoesNotRunOnPeriod"
        # The subject the model named, then the period — the very ids this
        # script is holding, as the id classes.
        assert error.details == (cell_subject.id, gone_period.id)
        assert isinstance(error.details[0], collomatique.SubjectId)
        assert isinstance(error.details[1], collomatique.PeriodId)
    else:
        raise AssertionError(
            "a subject that does not run on a period takes no group list there"
        )

# The removal cascade, and the whole of what a group list drags along: every
# association that gave it to a subject, the colles those associations bounded,
# and its placement row.
rows = [
    (period, subject)
    for period, subject, group_list in group_lists.associations()
    if group_list == served
]
assert len(rows) > 1
lists_before = len(group_lists)

removed = group_lists.remove(served)
warnings = removed.warnings

for w in warnings:
    assert isinstance(w, collomatique.Warning)
    assert str(w)
    assert isinstance(w.details, dict)

assert [w.kind for w in warnings] == (
    ["RemoveGroupsFromInterrogationCell"]
    + ["UnassignGroupList"] * len(rows)
    + ["ClearColloscopeGroupListRow"]
)
assert [
    (w.details["period"], w.details["subject"])
    for w in warnings
    if w.kind == "UnassignGroupList"
] == [(period.id, subject.id) for period, subject in rows]
assert warnings[-1].details == {"group_list": served.id}

# The list is a tree, and this is where it says something: the colle went
# because the association that bounded it went, so the trim hangs off that
# unassignment — the object itself, and further down the list, since a repair
# lands before the one that needed it.
trimmed = warnings[0]
assert trimmed.details == {"slot": slot.id, "week": week.id, "groups": [0, 2]}
assert trimmed.parent is not None
assert trimmed.parent.kind == "UnassignGroupList"
assert trimmed.parent.details == {"period": cell_period.id, "subject": cell_subject.id}


def position(target):
    """Where `target` sits in `warnings`, by identity rather than by equality."""
    for index, warning in enumerate(warnings):
        if warning is target:
            return index
    raise AssertionError("a parent is one of the warnings of the same list")


assert position(trimmed.parent) > 0
# What the write asked for itself hangs off nothing.
assert all(
    w.parent is None
    for w in warnings
    if w.kind != "RemoveGroupsFromInterrogationCell"
)

assert served not in group_lists
assert len(group_lists) == lists_before - 1
assert all(
    group_lists.association_for(period, subject) is None for period, subject in rows
)
assert doc.colloscope.interrogation(slot, week) is None

try:
    served.name
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed group list must raise")

# Undoing the removal puts back everything it took: the list, the associations
# that named it, the colle at the bounded coordinate and the placement row.
doc.undo()
assert served in group_lists
assert len(group_lists) == lists_before
assert all(
    group_lists.association_for(period, subject) == served for period, subject in rows
)
assert doc.colloscope.interrogation(slot, week) == frozenset({0, 2})
assert dict(doc.colloscope.group_list(served)) == {placed_low: 0, placed_high: 2}
