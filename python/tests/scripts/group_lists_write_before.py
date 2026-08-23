import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the six labels are the
# french names `ops` gives this family's operations — handed in from rust so
# that this pins the operations' own labels and not merely some strings.
# `served_index` is the place, in the group lists' own order, of the list that
# exactly one subject uses, and on every period.
#
# Everything else this script leans on it finds for itself, off the document;
# what rust asserts on its own side is that the example really holds those
# shapes.
doc = collomatique.load(source)
group_lists = doc.group_lists

before = len(group_lists)
rows_before = list(group_lists.associations())

lists = list(group_lists)
served = lists[served_index]
served_subject = next(
    subject for _period, subject, group_list in rows_before if group_list == served
)

# The list serves its subject on every period, so the first is as good as any —
# and the second one below is free for the copy to work on.
periods = list(doc.periods)
served_period = periods[0]
students = list(doc.students)
harry, ron = students[0], students[1]

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = group_lists.add(
    collomatique.GroupListData(
        "Maisons",
        students_per_group=(1, 4),
        group_names=["Gryffondor", None, "Serpentard"],
        filling=collomatique.PrefilledGroups(({harry}, {ron}, frozenset())),
    )
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# A brand new list serves no subject and holds no placement, so there is
# nothing for the cascade to repair. The result says so rather than the call
# saying nothing.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.GroupList)
assert isinstance(created.id, collomatique.GroupListId)
assert created in group_lists
assert group_lists[created.id] == created
assert len(group_lists) == before + 1

assert created.name == "Maisons"
assert created.students_per_group == (1, 4)
assert created.group_count == 3
assert created.group_names == ("Gryffondor", None, "Serpentard")
assert created.is_prefilled is True
assert created.groups == (frozenset({harry}), frozenset({ron}), frozenset())
assert created.excluded_students is None
assert created.group_name(0) == "Gryffondor"
assert created.group_name(1) == "Groupe 2"

# Adding a list associates it to nothing: which subject uses it on which period
# is the table beside the lists, and `set_association` is what writes there.
assert list(group_lists.associations()) == rows_before

assert added.created == created
assert repr(added).startswith("AddResult(created=<GroupList #")
assert "warnings=[]" in repr(added)

# Rewriting replaces the whole list at once — the parameters *and* the filling,
# because the model seals the two together — and the id stays, so the handle a
# script is holding reads the new state.
result = group_lists.update(
    created,
    collomatique.GroupListData(
        "Maisons de Poudlard",
        group_names=[None, None],
        filling=collomatique.AutomaticGroups(excluded_students={ron}),
    ),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")
# Nothing points at this list yet — no subject uses it and the colloscope holds
# nothing for it — so a filling that changes shape breaks nothing.
assert result.warnings == []

assert created.name == "Maisons de Poudlard"
assert created.students_per_group == (2, 3)
assert created.group_count == 2
assert created.is_prefilled is False
# The `None`-for-inapplicable rule reads the other way round now: the groups
# are the solver's business, and the exclusions are the question that applies.
assert created.groups is None
assert created.excluded_students == frozenset({ron})

# The list is named by an id or by a handle, interchangeably — this is the
# write half of the argument convention — and so are the students a value
# names.
group_lists.update(
    created.id,
    collomatique.GroupListData(
        "Maisons",
        group_names=[None, None, None],
        filling=collomatique.PrefilledGroups(({harry.id}, {ron.id}, frozenset())),
    ),
)
assert created.name == "Maisons"
assert created.group_count == 3
assert created.is_prefilled is True
assert created.groups == (frozenset({harry}), frozenset({ron}), frozenset())
assert created.excluded_students is None

# The table beside the lists: one row of `(period, subject) → group list`, and
# the write half of `association_for`. The document holds no colle at this
# point, so nothing is measured against the list that lands and nothing is
# repaired.
switched = group_lists.set_association(served_period, served_subject, created)
assert isinstance(switched, collomatique.OpResult)
assert switched.warnings == []
assert group_lists.association_for(served_period, served_subject) == created

# `None` is the missing row on the write exactly as it is on the read: the pair
# keeps no list at all afterwards, and the list itself stays — a list nobody
# uses is an ordinary document.
cleared = group_lists.set_association(served_period.id, served_subject.id, None)
assert cleared.warnings == []
assert group_lists.association_for(served_period, served_subject) is None
assert created in group_lists
assert (served_period, served_subject) not in {
    (period, subject) for period, subject, _group_list in group_lists.associations()
}

# Back to what the document came with.
group_lists.set_association(served_period, served_subject, served.id)
assert group_lists.association_for(served_period, served_subject) == served

# The copy: every subject that runs on both periods and holds interrogations is
# given the list the *previous* period gives it. The row emptied just above is
# what it has to put back.
second = periods[1]
group_lists.set_association(second, served_subject, None)
assert group_lists.association_for(second, served_subject) is None

copied = group_lists.duplicate_previous_period(second)
assert copied.warnings == []
assert all(
    group_lists.association_for(second, subject)
    == group_lists.association_for(periods[0], subject)
    for subject in doc.subjects
    if subject.interrogation is not None
)
assert list(group_lists.associations()) == rows_before

# This is what rust reads back off the disk.
doc.save(target)

# The refusals this family keeps for the model. Each of them is a statement
# about the document rather than about an argument's shape, so each arrives as
# the family's own exception rather than from the argument convention.
no_colles = next(subject for subject in doc.subjects if subject.interrogation is None)
for call, op, case, details in (
    (
        lambda: group_lists.set_association(served_period, no_colles, created),
        "AssignGroupListToSubject",
        "SubjectHasNoInterrogation",
        (no_colles.id,),
    ),
    # And in the other direction too: there is no row to clear where there
    # could never be one.
    (
        lambda: group_lists.set_association(served_period, no_colles, None),
        "AssignGroupListToSubject",
        "SubjectHasNoInterrogation",
        (no_colles.id,),
    ),
    (
        lambda: group_lists.duplicate_previous_period(periods[0]),
        "DuplicatePreviousPeriod",
        "FirstPeriodHasNoPreviousPeriod",
        (periods[0].id,),
    ),
):
    try:
        call()
    except collomatique.GroupListsError as error:
        assert isinstance(error, collomatique.UpdateError)
        assert isinstance(error, collomatique.Error)
        assert str(error)
        assert error.op == op
        assert error.case == case
        # The entities the model named, as the id classes — the very ones this
        # script is holding.
        assert error.details == details
    else:
        raise AssertionError(f"{op}/{case} must refuse")

# Nothing of that was written: the document is what the last accepted write
# left.
assert len(group_lists) == before + 1
assert list(group_lists.associations()) == rows_before

# A list nobody uses takes nothing with it: the removal is the same op the
# cascading one is, and the answer is empty because this one was named by
# nothing.
gone = group_lists.remove(created)
assert isinstance(gone, collomatique.OpResult)
assert not hasattr(gone, "created")
assert gone.warnings == []
assert created not in group_lists
assert len(group_lists) == before

try:
    created.name
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed group list must raise")

bare = collomatique.GroupListData("Maisons", group_names=[None])

# A dead group list is refused by the argument convention, and not by the
# model: the three ops that name one can object to an id the document does not
# hold, and that is caught here, where the message can say which argument was
# wrong.
for call in (
    lambda: group_lists.remove(created),
    lambda: group_lists.update(created, bare),
    lambda: group_lists.update(created.id, bare),
    lambda: group_lists.set_association(served_period, served_subject, created),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead group list must not be written through")

# Something that was never a reference to this document at all is a
# `TypeError`: it is not a stale anything.
for call in (
    lambda: group_lists.remove(3),
    lambda: group_lists.set_association(3, served_subject, None),
    lambda: group_lists.set_association(served_period, "Divination", None),
    lambda: group_lists.duplicate_previous_period("la deuxième"),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("a key that is not a reference must not resolve")

# `other` is this same file loaded twice, so its lists, periods, subjects and
# students carry the very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_list = list(other.group_lists)[0]
foreign_period = list(other.periods)[0]
foreign_student = list(other.students)[0]

for call in (
    lambda: group_lists.remove(foreign_list),
    lambda: group_lists.duplicate_previous_period(foreign_period),
    lambda: group_lists.set_association(served_period, served_subject, foreign_list),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a reference of another document must not resolve")

# A value is refused before any op is built, and nothing is written: a filling
# naming another document's student …
try:
    group_lists.add(
        collomatique.GroupListData(
            "Maisons",
            group_names=[None],
            filling=collomatique.PrefilledGroups(({foreign_student},)),
        )
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a student of another document names nobody here")

# … one whose prefilled groups do not match the names it gives them, which is
# the model's own invariant, checked by its own constructor …
try:
    group_lists.add(
        collomatique.GroupListData(
            "Maisons",
            group_names=[None, None],
            filling=collomatique.PrefilledGroups(({harry},)),
        )
    )
except ValueError as error:
    assert not isinstance(error, collomatique.Error)
    assert str(error)
else:
    raise AssertionError("a prefilled filling names every group or none")

# … and one whose field was never of the right shape.
try:
    group_lists.add(collomatique.GroupListData("Maisons", group_names=[""]))
except ValueError:
    pass
else:
    raise AssertionError("a group name is a non-empty string or None")
assert len(group_lists) == before

# A call that is wrong about both names the *list*: a value meant for nothing
# is moot, so the addressee is resolved first.
try:
    group_lists.update(created, collomatique.GroupListData("Maisons", group_names=[""]))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Each accepted call was its own undo slot, named by the operation itself.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert created in group_lists

assert doc.undo_name == duplicate_label
doc.undo()
assert group_lists.association_for(second, served_subject) is None

assert doc.undo_name == unassign_label
doc.undo()
assert group_lists.association_for(second, served_subject) == served

assert doc.undo_name == assign_label
doc.undo()
assert group_lists.association_for(served_period, served_subject) is None

assert doc.undo_name == unassign_label
doc.undo()
assert group_lists.association_for(served_period, served_subject) == created

assert doc.undo_name == assign_label
doc.undo()
assert group_lists.association_for(served_period, served_subject) == served

assert doc.undo_name == update_label
doc.undo()
assert created.name == "Maisons de Poudlard"
assert created.excluded_students == frozenset({ron})

doc.undo()
assert created.name == "Maisons"
assert created.groups == (frozenset({harry}), frozenset({ron}), frozenset())

assert doc.undo_name == add_label
doc.undo()
assert created not in group_lists
assert len(group_lists) == before
assert list(group_lists.associations()) == rows_before
assert doc.can_undo is False
