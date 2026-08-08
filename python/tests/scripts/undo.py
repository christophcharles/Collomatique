import collomatique

# `first`, `second` and `third` are three mondays the rust side handed in, and
# `update_label` / `clear_label` are the french labels `ops` gives the two
# first-week operations. The labels come from rust rather than being spelled out
# here, so this pins that python shows *that* label and not merely some string.
doc = collomatique.load(source)

# A document that was just opened has no history: loading is not a write.
assert doc.can_undo is False
assert doc.can_redo is False
assert doc.undo_name is None
assert doc.redo_name is None

for step in (doc.undo, doc.redo):
    try:
        step()
    except collomatique.NothingToUndo:
        pass
    else:
        raise AssertionError("a document with no history must refuse %r" % step)

start = doc.periods.first_week

for date in (first, second, third):
    doc.periods.set_first_week(date)
    assert doc.periods.first_week == date

# Each write is its own step, and the top of the stack says what it was.
assert doc.can_undo is True
assert doc.can_redo is False
assert doc.undo_name == update_label

doc.undo()
assert doc.periods.first_week == second

# What was just undone is what redo would put back.
assert doc.can_redo is True
assert doc.redo_name == update_label
doc.redo()
assert doc.periods.first_week == third
assert doc.can_redo is False

# The history is a line, not a tree: writing after an undo drops the branch that
# was undone.
doc.undo()
assert doc.can_redo is True
doc.periods.clear_first_week()
assert doc.periods.first_week is None
assert doc.can_redo is False
assert doc.redo_name is None
assert doc.undo_name == clear_label

# Three writes are on the stack — the first two sets and the clear — so three
# undos go back to the document as it was loaded.
doc.undo()
assert doc.periods.first_week == second
doc.undo()
assert doc.periods.first_week == first
doc.undo()
assert doc.periods.first_week == start

assert doc.can_undo is False
assert doc.undo_name is None
try:
    doc.undo()
except collomatique.NothingToUndo:
    pass
else:
    raise AssertionError("undoing past the start must raise")

# Rust reads this back and compares it with the file the script opened: undoing
# everything gives the document that was loaded, not merely something close.
doc.save(target)

# `NothingToUndo` is an ordinary `collomatique.Error`, so a script that only
# cares that the call failed catches one thing.
assert issubclass(collomatique.NothingToUndo, collomatique.Error)
