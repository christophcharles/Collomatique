import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the five labels are the
# french names `ops` gives the operations this script runs — the colloscope's
# four, and the group list add it needs to have an automatic list at all —
# handed in from rust so that this pins the operations' own labels and not
# merely some strings.
#
# Everything else this script leans on it finds for itself, off the document;
# what rust asserts on its own side is that the example really holds those
# shapes. The two indices it leaves behind say which subject and which period
# the second stage's refusal is about, in the collections' own order.
doc = collomatique.load(source)
colloscope = doc.colloscope
group_lists = doc.group_lists

# The example carries no colloscope at all, so everything below is this
# script's own doing.
assert list(colloscope.interrogations()) == []
assert list(colloscope.group_lists()) == []

# The coordinate: a cell a colle can really sit on, whose subject holds a group
# list on that week's period — the list is what the group numbers are measured
# against, so a cell without one could hold nothing.
weeks = list(doc.weeks)
cell_slot, cell_week = next(
    (slot, week)
    for slot in doc.slots
    for week in weeks
    if doc.is_interrogation_possible(slot, week)
    and group_lists.association_for(week.period, slot.subject) is not None
)
cell_subject = cell_slot.subject
cell_period = cell_week.period
bound = group_lists.association_for(cell_period, cell_subject).group_count
assert bound > 2

# What the second stage needs to find the same subject and the same period from
# rust, in the collections' own order.
cell_subject_index = list(doc.subjects).index(cell_subject)
cell_period_index = list(doc.periods).index(cell_period)

# One cell, written whole. The groups are any iterable of numbers — here the
# set the read hands back the shape of.
written = colloscope.set_interrogation(cell_slot, cell_week, {0, 2})
assert isinstance(written, collomatique.OpResult)
# Nothing in the document points at a colloscope cell, so there is nothing for
# the cascade to repair. The result says so rather than the call saying nothing.
assert written.warnings == []
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not isinstance(written, collomatique.AddResult)
assert not hasattr(written, "created")

assert colloscope.interrogation(cell_slot, cell_week) == frozenset({0, 2})
assert list(colloscope.interrogations()) == [(cell_slot, cell_week, frozenset({0, 2}))]

# The coordinate is named by an id or by a handle, interchangeably — this is
# the write half of the argument convention — and the groups are a tuple this
# time: the cell becomes exactly what it is given, so the 0 and the 2 are gone.
colloscope.set_interrogation(cell_slot.id, cell_week.id, (1,))
assert colloscope.interrogation(cell_slot, cell_week) == frozenset({1})

# An empty iterable is the absent cell — the same `None` the read answers — so
# this is how a cell is cleared, and why there is no `remove` here.
cleared = colloscope.set_interrogation(cell_slot, cell_week, [])
assert cleared.warnings == []
assert colloscope.interrogation(cell_slot, cell_week) is None
assert list(colloscope.interrogations()) == []

# And back, from a list that names a group twice: the model stores a set, so
# what lands is the set.
colloscope.set_interrogation(cell_slot, cell_week, [0, 2, 2])
assert colloscope.interrogation(cell_slot, cell_week) == frozenset({0, 2})

# The other table needs an automatic list to hold a row for: a prefilled one
# has groups of its own, and the colloscope has no say in it. The example has
# none, so the script makes one — three groups, and one student it excludes.
students = list(doc.students)
harry, ron, malefoy = students[0], students[1], students[2]
automatic = group_lists.add(
    collomatique.GroupListData(
        "Colles automatiques",
        group_names=[None, None, None],
        filling=collomatique.AutomaticGroups(excluded_students={malefoy}),
    )
).created
assert automatic.is_prefilled is False
assert colloscope.group_list(automatic) is None

placed = colloscope.set_group_list(automatic, {harry: 0, ron: 2})
assert isinstance(placed, collomatique.OpResult)
assert placed.warnings == []

placements = colloscope.group_list(automatic)
assert dict(placements) == {harry: 0, ron: 2}
# The mapping is read-only: reading it is reading the document, and mutating it
# is a `TypeError`.
try:
    placements[harry] = 1
except TypeError:
    pass
else:
    raise AssertionError("the placements of a group list are read-only")

rows = list(colloscope.group_lists())
assert [group_list for group_list, _placements in rows] == [automatic]
assert dict(rows[0][1]) == {harry: 0, ron: 2}

# By id, and the row is written whole here too: the student the mapping leaves
# out is placed nowhere afterwards.
colloscope.set_group_list(automatic.id, {harry.id: 1})
assert dict(colloscope.group_list(automatic)) == {harry: 1}

# An empty mapping is the absent row, exactly as an empty iterable is the
# absent cell.
emptied = colloscope.set_group_list(automatic, {})
assert emptied.warnings == []
assert colloscope.group_list(automatic) is None
assert list(colloscope.group_lists()) == []

colloscope.set_group_list(automatic, {harry: 0, ron: 2})

# This is what rust reads back off the disk: one cell and one placement row, in
# a document that opened with neither.
doc.save(target)

# The refusals this family keeps for the model. Each of them is a statement
# about the document rather than about an argument's shape, so each arrives as
# the family's own exception rather than from the argument convention.
prefilled = next(group_list for group_list in group_lists if group_list.is_prefilled)
inactive = next(week for week in weeks if not week.interrogations)
for call, op, case, details in (
    # A prefilled list holds its groups itself, so the colloscope has no row
    # for it to write.
    (
        lambda: colloscope.set_group_list(prefilled, {harry: 0}),
        "UpdateColloscopeGroupList",
        "PrefilledGroupListInColloscope",
        (prefilled.id,),
    ),
    (
        lambda: colloscope.set_group_list(automatic, {malefoy: 0}),
        "UpdateColloscopeGroupList",
        "ExcludedStudentInGroupList",
        (automatic.id, malefoy.id),
    ),
    (
        lambda: colloscope.set_group_list(automatic, {harry: automatic.group_count}),
        "UpdateColloscopeGroupList",
        "InvalidGroupNumForStudentInGroupList",
        (automatic.id, harry.id),
    ),
    # A week the slot does not run on carries no colle, whatever the groups
    # say.
    (
        lambda: colloscope.set_interrogation(cell_slot, inactive, {0}),
        "UpdateColloscopeInterrogation",
        "InterrogationOnInactiveWeek",
        (cell_slot.id, inactive.id),
    ),
    (
        lambda: colloscope.set_interrogation(cell_slot, cell_week, {bound}),
        "UpdateColloscopeInterrogation",
        "InvalidGroupNumInInterrogation",
        (cell_slot.id, cell_week.id),
    ),
):
    try:
        call()
    except collomatique.ColloscopeError as error:
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

# The two writes are refused before any op is built when what they are given
# was never of the right shape.
for call in (
    lambda: colloscope.set_group_list(automatic, [harry]),
    lambda: colloscope.set_group_list(automatic, {harry: "premier"}),
    lambda: colloscope.set_group_list(automatic, {"Harry": 0}),
    lambda: colloscope.set_interrogation(cell_slot, cell_week, 3),
    lambda: colloscope.set_interrogation(cell_slot, cell_week, {"zéro"}),
):
    try:
        call()
    except TypeError as error:
        assert not isinstance(error, collomatique.Error)
        assert str(error)
    else:
        raise AssertionError("a placement or a group number of the wrong shape must refuse")

# A student named twice — once by handle and once by id, the only way one
# mapping can hold them both — is a call that says two things at once.
try:
    colloscope.set_group_list(automatic, {harry: 0, harry.id: 1})
except ValueError as error:
    assert not isinstance(error, collomatique.Error)
    assert str(error)
else:
    raise AssertionError("a student placed twice must refuse")

# Something that was never a reference to this document at all is a
# `TypeError`: it is not a stale anything.
for call in (
    lambda: colloscope.set_interrogation(3, cell_week, set()),
    lambda: colloscope.set_interrogation(cell_slot, "la première", set()),
    lambda: colloscope.set_group_list("Maisons", {}),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("a coordinate that is not a reference must not resolve")

# `other` is this same file loaded twice, so its slots, weeks, group lists and
# students carry the very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_slot = list(other.slots)[0]
foreign_week = list(other.weeks)[0]
foreign_list = list(other.group_lists)[0]
foreign_student = list(other.students)[0]

for call in (
    lambda: colloscope.set_interrogation(foreign_slot, cell_week, set()),
    lambda: colloscope.set_interrogation(cell_slot, foreign_week, set()),
    lambda: colloscope.set_group_list(foreign_list, {}),
    lambda: colloscope.set_group_list(automatic, {foreign_student: 0}),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a reference of another document must not resolve")

# Nothing of that was written: the document is what the last accepted write
# left.
assert colloscope.interrogation(cell_slot, cell_week) == frozenset({0, 2})
assert dict(colloscope.group_list(automatic)) == {harry: 0, ron: 2}

# The two erases are halves: the cells go and the placements stand, and then
# the other way about. Each is one operation however many rows it emptied.
erased = colloscope.erase()
assert erased.warnings == []
assert list(colloscope.interrogations()) == []
assert colloscope.interrogation(cell_slot, cell_week) is None
assert dict(colloscope.group_list(automatic)) == {harry: 0, ron: 2}

erased_lists = colloscope.erase_group_lists()
assert erased_lists.warnings == []
assert list(colloscope.group_lists()) == []
assert colloscope.group_list(automatic) is None

# Each accepted call was its own undo slot, named by the operation itself.
assert doc.undo_name == erase_group_lists_label
doc.undo()
assert doc.redo_name == erase_group_lists_label
assert dict(colloscope.group_list(automatic)) == {harry: 0, ron: 2}

assert doc.undo_name == erase_label
doc.undo()
assert colloscope.interrogation(cell_slot, cell_week) == frozenset({0, 2})

assert doc.undo_name == set_group_list_label
doc.undo()
assert colloscope.group_list(automatic) is None

doc.undo()
assert dict(colloscope.group_list(automatic)) == {harry: 1}

doc.undo()
assert dict(colloscope.group_list(automatic)) == {harry: 0, ron: 2}

doc.undo()
assert colloscope.group_list(automatic) is None

assert doc.undo_name == add_group_list_label
doc.undo()
assert automatic not in group_lists

assert doc.undo_name == set_interrogation_label
doc.undo()
assert colloscope.interrogation(cell_slot, cell_week) is None

doc.undo()
assert colloscope.interrogation(cell_slot, cell_week) == frozenset({1})

doc.undo()
assert colloscope.interrogation(cell_slot, cell_week) == frozenset({0, 2})

doc.undo()
assert list(colloscope.interrogations()) == []
assert doc.can_undo is False

# The added list went with its own undo, so the handle naming it is dead — and
# a dead group list is refused by the argument convention, not by the model.
for call in (
    lambda: colloscope.set_group_list(automatic, {harry: 0}),
    lambda: colloscope.set_group_list(automatic.id, {harry: 0}),
    # A call that is wrong about both names the *list*: a mapping meant for
    # nothing is moot, so the addressee is resolved first.
    lambda: colloscope.set_group_list(automatic, {"Harry": 0}),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead group list must not be written through")

assert doc.can_undo is False
