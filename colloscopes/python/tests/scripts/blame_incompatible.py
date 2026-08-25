import collomatique

# `source` is the filled synthetic colloscope: a document written by the test,
# with cells on several (slot, week) pairs and placements for one automatic
# group list. Everything below is refused before an engine is ever looked for,
# so this script runs on a machine that has none.
doc = collomatique.load(source)
model = doc.build_colloscope_model(collomatique.ColloscopeSolveConfig())

intact = doc.colloscope.to_data()
assert intact.group_lists, "the fixture places students"
assert intact.interrogations, "the fixture holds cells"

# ------------------------------------------------- a colloscope of another shape

# A group number past the last group of the list it is placed in: the value
# crosses the boundary — the group numbers are plain ints, and nothing at the
# boundary knows how many groups a list has — and the model refuses it.
group_list, placements = next(iter(intact.group_lists.items()))
student = next(iter(placements))

broken = doc.colloscope.to_data()
broken.group_lists[group_list][student] = 99

try:
    model.blame(broken)
except ValueError as e:
    out_of_range = str(e)
else:
    raise AssertionError("a group number the model has no group for must be refused")

# ------------------------------------------------------------ what a key may be

# The model is detached: it holds a snapshot of the parameters and no document,
# so there is nothing here to check a handle against. Ids are what `to_data()`
# hands back, and ids are what a blame reads.
handle_keyed = doc.colloscope.to_data()
cell, groups = next(iter(handle_keyed.interrogations.items()))
slot, week = cell
del handle_keyed.interrogations[cell]
handle_keyed.interrogations[(doc.slots[slot], week)] = groups

try:
    model.blame(handle_keyed)
except TypeError as e:
    handle_refused = str(e)
else:
    raise AssertionError("a detached colloscope names entities by id alone")

# ------------------------------------------------------------- what is refused

# Not a colloscope at all, refused where every other value is — at the
# boundary, by the field it does not have.
try:
    model.blame(3)
except TypeError as e:
    not_a_colloscope = str(e)
else:
    raise AssertionError("blame takes a ColloscopeData")

# The colloscope is required: there is no second meaning for its absence.
try:
    model.blame()
except TypeError:
    pass
else:
    raise AssertionError("blame takes a colloscope")

# `engine` and `on_log` are keyword-only, so a script that hands one over
# positionally is handing over a colloscope.
try:
    model.blame(intact, None)
except TypeError:
    pass
else:
    raise AssertionError("engine is keyword-only")

# ---------------------------------------------------------------- and the rest

# A blame reads; it writes nothing, to a document this model is not attached to
# anyway.
assert doc.can_undo is False
assert doc.can_redo is False

# A violation is something a blame hands back, not something a script writes.
try:
    collomatique.ConstraintViolation()
except TypeError:
    pass
else:
    raise AssertionError("ConstraintViolation has no constructor")
