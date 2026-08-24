import collomatique

# `first` … `fourth` are four mondays the rust side handed in, `batch_label`,
# `outer_label` and `empty_label` are the names blocks are opened under, and
# `update_label` is the french label `ops` gives the first-week operation. The
# labels come from rust rather than being spelled out here, so this pins that a
# block is stored under the name it was opened with, and a write outside one
# under the operation's own name.
doc = collomatique.load(source)

start = doc.periods.first_week
assert doc.can_undo is False


class Boom(Exception):
    """Raised inside a block, to be caught outside it."""


# A transaction is built by the document, and does nothing until it is entered.
assert isinstance(doc.transaction("built"), collomatique.Transaction)


# --- One block, one slot ---------------------------------------------------

with doc.transaction(batch_label):
    doc.periods.set_first_week(first)
    doc.periods.set_first_week(second)
    doc.periods.set_first_week(third)
    # The writes are in the document as they are made: a block groups what can
    # be taken back, it does not hold the writes until the end.
    assert doc.periods.first_week == third

assert doc.periods.first_week == third
assert doc.can_undo is True
assert doc.can_redo is False
# Three writes, one step, under the name the block was opened with.
assert doc.undo_name == batch_label

doc.undo()
assert doc.periods.first_week == start
assert doc.can_undo is False
assert doc.redo_name == batch_label

# And redo puts all three back at once, for the same reason.
doc.redo()
assert doc.periods.first_week == third
doc.undo()
assert doc.periods.first_week == start


# --- An exception rolls the block back, and carries on out of it -----------

try:
    with doc.transaction("rolled back"):
        doc.periods.set_first_week(first)
        assert doc.periods.first_week == first
        raise Boom()
except Boom:
    pass
else:
    raise AssertionError("the exception must leave the block")

# Nothing of the block is left: not in the document, and not in the history
# either, so there is no empty step to undo afterwards.
assert doc.periods.first_week == start
assert doc.can_undo is False
# The level below is untouched, redo branch included.
assert doc.can_redo is True


# --- Blocks really nest ----------------------------------------------------

with doc.transaction(outer_label):
    doc.periods.set_first_week(first)

    try:
        with doc.transaction("inner"):
            doc.periods.set_first_week(second)
            raise Boom()
    except Boom:
        pass
    else:
        raise AssertionError("the exception must leave the inner block")

    # The inner block took its own write back and nothing else, so the outer
    # block still has what it wrote before it — this is what nesting is for.
    assert doc.periods.first_week == first

    doc.periods.set_first_week(third)

# And the whole thing is still one step, named after the outer block.
assert doc.periods.first_week == third
assert doc.can_undo is True
assert doc.undo_name == outer_label
doc.undo()
assert doc.periods.first_week == start
assert doc.can_undo is False


# --- Cancelling from inside the block --------------------------------------

with doc.transaction("preview") as t:
    doc.periods.set_first_week(first)
    assert doc.periods.first_week == first

    # Everything the block wrote goes back at once, and the transaction is
    # closed there and then.
    t.cancel()
    assert doc.periods.first_week == start
    assert doc.can_undo is False
    assert "(fermée)" in repr(t)

    # The block keeps running, and this write is outside the transaction.
    doc.periods.set_first_week(second)

# So leaving the block adds nothing, and the write after the cancel is a step
# of its own, under the operation's name rather than the block's.
assert doc.periods.first_week == second
assert doc.undo_name == update_label
doc.undo()
assert doc.periods.first_week == start
assert doc.can_undo is False


# --- A transaction is one block, once --------------------------------------

with doc.transaction("one shot") as t:
    try:
        with t:
            pass
    except collomatique.Error:
        pass
    else:
        raise AssertionError("entering an open transaction must raise")

    t.cancel()

    try:
        with t:
            pass
    except collomatique.Error:
        pass
    else:
        raise AssertionError("entering a closed transaction must raise")

    try:
        t.cancel()
    except collomatique.Error:
        pass
    else:
        raise AssertionError("cancelling a closed transaction must raise")

assert doc.can_undo is False


# --- Blocks close in the order they opened ---------------------------------

with doc.transaction(outer_label) as outer:
    doc.periods.set_first_week(first)

    with doc.transaction("inner") as inner:
        doc.periods.set_first_week(second)

        # Cancelling the outer block from here would take the inner block's
        # write with it and leave `inner` holding a session that is gone.
        try:
            outer.cancel()
        except collomatique.Error:
            pass
        else:
            raise AssertionError("closing a block that is not the innermost must raise")

        # The refusal changed nothing: both blocks are still open, and the
        # inner one still holds what it wrote.
        assert doc.periods.first_week == second

        inner.cancel()

    assert doc.periods.first_week == first
    doc.periods.set_first_week(fourth)

assert doc.periods.first_week == fourth
assert doc.undo_name == outer_label
doc.undo()
assert doc.periods.first_week == start
assert doc.can_undo is False


# --- A transaction that is never entered does nothing ----------------------

never = doc.transaction("never entered")
assert "never entered" in repr(never)
assert "(non entrée)" in repr(never)

# It holds no block, so there is nothing to cancel — and saying so is better
# than doing nothing quietly, which would hide a script that lost its place.
try:
    never.cancel()
except collomatique.Error:
    pass
else:
    raise AssertionError("cancelling a transaction that was never entered must raise")

del never
assert doc.periods.first_week == start
assert doc.can_undo is False


# --- Inside a block, undo stops at the block's start ------------------------

doc.periods.set_first_week(first)
assert doc.can_undo is True

with doc.transaction("bounded"):
    doc.periods.set_first_week(second)

    # The write made inside the block can be taken back…
    doc.undo()
    assert doc.periods.first_week == first

    # …but the one made before it cannot, even though the document has a
    # history under the block.
    assert doc.can_undo is False
    try:
        doc.undo()
    except collomatique.NothingToUndo:
        pass
    else:
        raise AssertionError("undo inside a block must not reach past its start")

# The block wrote and then took it back, so its step changes nothing — but it
# is still a step, and the write from before the block is still under it.
assert doc.periods.first_week == first
assert doc.undo_name == "bounded"
doc.undo()
assert doc.periods.first_week == first
doc.undo()
assert doc.periods.first_week == start
assert doc.can_undo is False


# --- An empty block leaves a named, empty step ------------------------------

empty = doc.transaction(empty_label)
with empty:
    pass

assert doc.can_undo is True
assert doc.undo_name == empty_label
assert "(fermée)" in repr(empty)

doc.undo()
assert doc.periods.first_week == start
assert doc.can_undo is False


# Rust reads this back and compares it with the file the script opened: every
# rollback above landed on the document that was loaded, not merely on one that
# happens to carry the right start date.
doc.save(target)
