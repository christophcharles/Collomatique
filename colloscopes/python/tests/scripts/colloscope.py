import collomatique

# `source` is a colloscope document written by the test: a filled colloscope,
# with cells on several (slot, week) pairs and placements for one automatic
# group list — the two shapes the example file never shows, since it was never
# resolved. `other` is that example, whose colloscope is empty: it is what
# lets the empty iteration and the empty cell show up too.
doc = collomatique.load(source)
other = collomatique.load(other_source)

colloscope = doc.colloscope
assert isinstance(colloscope, collomatique.Colloscope)
assert repr(colloscope) == "<collomatique.Colloscope>"

# The stored cells, as (Slot, Week, frozenset) triples, in key order. The
# frozenset holds group numbers — ints, not handles — and the two handles in
# the triple read the document.
cells = list(colloscope.interrogations())
assert len(cells) > 0
assert all(
    isinstance(slot, collomatique.Slot) and slot in doc.slots
    for slot, _week, _groups in cells
)
assert all(
    isinstance(week, collomatique.Week) and week in doc.weeks
    for _slot, week, _groups in cells
)
assert all(
    isinstance(groups, frozenset)
    and all(isinstance(group, int) for group in groups)
    and len(groups) > 0
    for _slot, _week, groups in cells
)

cell_reads = [
    (slot.index, week.index, tuple(sorted(groups)))
    for slot, week, groups in cells
]

# A handle and an id name the same cell.
first_slot, first_week, first_groups = cells[0]
assert colloscope.interrogation(first_slot, first_week) == first_groups
assert colloscope.interrogation(first_slot.id, first_week.id) == first_groups

# The empty cell is one thing, `None` — the single absent answer, whatever the
# cell could have held: the week right after the first cell's is a possible
# one nobody scheduled, and the fixture's last week is switched off entirely.
weeks = list(doc.weeks)
empty_possible = colloscope.interrogation(first_slot, weeks[1])
empty_impossible = colloscope.interrogation(first_slot, weeks[-1])
assert empty_possible is None
assert empty_impossible is None
empty_cell_reads = (empty_possible, empty_impossible)

# The placements of an automatic list, as a read-only mapping of Student
# handles to group numbers — a `types.MappingProxyType` over a fresh dict.
automatic = next(gl for gl in doc.group_lists if not gl.is_prefilled)
prefilled = next(gl for gl in doc.group_lists if gl.is_prefilled)
placements = colloscope.group_list(automatic)
assert type(placements).__name__ == "mappingproxy"
assert all(isinstance(student, collomatique.Student) for student in placements)
assert all(isinstance(group, int) for group in placements.values())
assert colloscope.group_list(automatic.id) == placements

placement_items = sorted(
    (student.surname, group) for student, group in placements.items()
)

# Reading the mapping is reading the document; writing it is TypeError.
harry = next(student for student in doc.students if student.surname == "Potter")
assert placements[harry] == 0
try:
    placements[harry] = 0
except TypeError:
    pass
else:
    raise AssertionError("the placements mapping refuses assignment")
try:
    del placements[harry]
except TypeError:
    pass
else:
    raise AssertionError("the placements mapping refuses deletion")

# A prefilled list never appears here: it has groups of its own, so the
# question does not apply and the answer is `None`, not an empty mapping.
assert colloscope.group_list(prefilled) is None

# The stored rows, as (GroupList, mapping) pairs, in key order — the same
# read-only shape the single lookup hands out.
rows = list(colloscope.group_lists())
assert len(rows) > 0
assert all(
    isinstance(gl, collomatique.GroupList) and gl in doc.group_lists
    for gl, _placements in rows
)
assert all(type(placements).__name__ == "mappingproxy" for _gl, placements in rows)
list_order = list(doc.group_lists)
group_list_rows = [
    (
        list_order.index(gl),
        sorted((student.surname, group) for student, group in placements.items()),
    )
    for gl, placements in rows
]

# The example's colloscope is empty, and the empty iteration is exactly that.
assert list(other.colloscope.interrogations()) == []
assert list(other.colloscope.group_lists()) == []
assert other.colloscope.interrogation(list(other.slots)[0], list(other.weeks)[0]) is None

# A reference that belongs to another document is stale, whatever its id says.
# The refusal must say « somebody else's » rather than « missing », because
# nothing is missing here.
other_slot = list(other.slots)[0]
other_week = list(other.weeks)[0]
other_group_list = list(other.group_lists)[0]
for call in (
    lambda: colloscope.interrogation(other_slot, first_week),
    lambda: colloscope.interrogation(first_slot, other_week),
    lambda: colloscope.group_list(other_group_list),
):
    try:
        call()
    except collomatique.StaleHandleError as error:
        assert "another document" in str(error)
        assert "is not in this document" not in str(error)
    else:
        raise AssertionError("an argument of another document must raise")

# A position that is not a reference at all was never a question about this
# document.
for bad in (3, "Maths"):
    try:
        colloscope.interrogation(bad, first_week)
    except TypeError:
        pass
    else:
        raise AssertionError("a slot argument takes a Slot or a SlotId")
    try:
        colloscope.interrogation(first_slot, bad)
    except TypeError:
        pass
    else:
        raise AssertionError("a week argument takes a Week or a WeekId")
for bad in (3, "Maths"):
    try:
        colloscope.group_list(bad)
    except TypeError:
        pass
    else:
        raise AssertionError("a group list argument takes a GroupList or a GroupListId")
