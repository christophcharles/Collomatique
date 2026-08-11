import collomatique

# `source` is the filled synthetic colloscope of the read surface's commit: a
# document written by the test, with cells on several (slot, week) pairs and
# placements for one automatic group list — the two shapes the example file
# never shows, since it was never resolved. `other` is that example: the
# document whose handles the key resolution must refuse.
doc = collomatique.load(source)
other = collomatique.load(other_source)

colloscope = doc.colloscope

# The whole colloscope, detached. The keys of both tables are ids — a value
# carries no document around with it — and the group numbers are plain ints,
# not handles.
tree = colloscope.to_data()
assert isinstance(tree, collomatique.ColloscopeData)
assert all(
    isinstance(key, tuple)
    and len(key) == 2
    and isinstance(key[0], collomatique.SlotId)
    and isinstance(key[1], collomatique.WeekId)
    for key in tree.interrogations
)
assert all(
    isinstance(group, int)
    for groups in tree.interrogations.values()
    for group in groups
)
assert all(isinstance(gl, collomatique.GroupListId) for gl in tree.group_lists)
assert all(
    isinstance(student, collomatique.StudentId) and isinstance(group, int)
    for placements in tree.group_lists.values()
    for student, group in placements.items()
)

# A fresh object every call. Two of them are equal and share nothing.
fresh = colloscope.to_data()
assert fresh == tree
assert fresh is not tree

# A value has no identity: an id names a place in a document, and a value has
# none.
assert not hasattr(tree, "id")

# The value's tables agree with the handle reads, cell by cell and row by
# row. The positions are read the way a script reads them: the slot's place
# within its subject, the week's global index, the sorted group numbers, and
# the placements by surname.
cell_reads = [
    (doc.slots[slot].index, doc.weeks[week].index, tuple(sorted(groups)))
    for (slot, week), groups in tree.interrogations.items()
]
group_lists = list(doc.group_lists)
row_reads = [
    (
        group_lists.index(doc.group_lists[group_list]),
        sorted(
            (doc.students[student].surname, group)
            for student, group in placements.items()
        ),
    )
    for group_list, placements in tree.group_lists.items()
]
# A field that names an entity takes a handle or an id, interchangeably. The
# two spellings of one row are the same payload — the handle form is the
# natural one to write, and the id form is what `to_data()` hands back.
first_slot, first_week, first_groups = next(colloscope.interrogations())
automatic = next(gl for gl in doc.group_lists if not gl.is_prefilled)
harry = next(student for student in doc.students if student.surname == "Potter")
by_handles = collomatique.ColloscopeData(
    interrogations={(first_slot, first_week): set(first_groups)},
    group_lists={automatic: {harry: 0}},
)
by_ids = collomatique.ColloscopeData(
    interrogations={(first_slot.id, first_week.id): set(first_groups)},
    group_lists={automatic.id: {harry.id: 0}},
)

# A hand-built value need not be canonical: an empty group set and an empty
# placement map just mean "no row", which is what the payload promises its
# callers.
with_empty_rows = collomatique.ColloscopeData(
    interrogations={(first_slot.id, first_week.id): set()},
    group_lists={automatic.id: {}},
)

# The default is the empty colloscope, what `clm.new_document()` holds.
defaults = collomatique.ColloscopeData()
assert defaults == collomatique.ColloscopeData()

# The field order, which is what a positional call depends on: the
# interrogation rows, then the placements rows.
import dataclasses

assert [f.name for f in dataclasses.fields(collomatique.ColloscopeData)] == [
    "interrogations",
    "group_lists",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import ColloscopeData as _same_class  # noqa: E402

assert _same_class is collomatique.ColloscopeData
assert collomatique.ColloscopeData.__module__ == "collomatique"

# A value is dumb: nothing is refused at birth. All of these are answered for
# when the value is used, not here.
bad_table = collomatique.ColloscopeData(interrogations=[0])
bad_cell_key = collomatique.ColloscopeData(interrogations={3: {0}})
bad_week_key = collomatique.ColloscopeData(
    interrogations={(first_slot, 3): {0}})
bad_groups = collomatique.ColloscopeData(
    interrogations={(first_slot, first_week): "x"})
bad_list_key = collomatique.ColloscopeData(group_lists={3: {}})
bad_student_key = collomatique.ColloscopeData(
    group_lists={automatic: {3: 0}})
bad_group_number = collomatique.ColloscopeData(
    group_lists={automatic: {harry: "x"}})
foreign_slot = collomatique.ColloscopeData(
    interrogations={(next(iter(other.slots)), first_week): {0}})
foreign_group_list = collomatique.ColloscopeData(
    group_lists={next(iter(other.group_lists)): {harry: 0}})
