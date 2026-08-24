import datetime

import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the three labels are the
# french names `ops` gives the three operations of this family — handed in from
# rust so that this pins the operation's own label and not merely some string.
doc = collomatique.load(source)
incompats = doc.incompats

before = len(incompats)
subject = list(doc.subjects)[0]
pattern = list(doc.week_patterns)[0]

noon = collomatique.TimeSlot(collomatique.Weekday.MONDAY, datetime.time(12, 0), 60)
one_oclock = collomatique.TimeSlot(
    collomatique.Weekday.MONDAY, datetime.time(13, 0), 60
)

# The first creating op of the api: what comes back is the `AddResult` subclass,
# so a script that only reads warnings treats it like any other result.
added = incompats.add(
    collomatique.IncompatData("Lundi Midi (essai)", subject, slots=[noon])
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)
assert issubclass(collomatique.AddResult, collomatique.OpResult)

# Nothing in the document points at an incompatibility, so no write of this
# family ever has anything to repair — but the result says so rather than the
# call saying nothing at all.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.Incompat)
assert isinstance(created.id, collomatique.IncompatId)
assert created in incompats
assert incompats[created.id] == created
assert len(incompats) == before + 1

assert created.name == "Lundi Midi (essai)"
assert created.subject == subject
assert created.slots == (noon,)
assert created.minimum_free_slots == 1
assert created.week_pattern is None

# The getter hands back the same handle every time — a handle is a view, so two
# of them for one incompatibility are equal objects rather than one object.
assert added.created == created
assert repr(added).startswith("AddResult(created=<Incompat #")
assert "warnings=[]" in repr(added)

# Rewriting replaces the whole value: every field at once, and the id stays, so
# the handle a script is holding reads the new state.
result = incompats.update(
    created,
    collomatique.IncompatData(
        "Lundi Midi et une heure",
        subject,
        slots=[noon, one_oclock],
        minimum_free_slots=2,
        week_pattern=pattern,
    ),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
assert result.warnings == []
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")

assert created.name == "Lundi Midi et une heure"
assert created.slots == (noon, one_oclock)
assert created.minimum_free_slots == 2
assert created.week_pattern == pattern
assert len(incompats) == before + 1

# The incompatibility is named by an id or by a handle, interchangeably — this
# is the write half of the argument convention.
incompats.update(
    created.id,
    collomatique.IncompatData(
        "Lundi Midi (par id)",
        subject,
        slots=[noon, one_oclock],
        minimum_free_slots=2,
        week_pattern=pattern,
    ),
)
assert created.name == "Lundi Midi (par id)"

# This is what rust reads back off the disk.
doc.save(target)

# Removing takes it away and nothing else: the removal of a leaf entity has
# nothing to cascade.
removed = incompats.remove(created.id)
assert isinstance(removed, collomatique.OpResult)
assert removed.warnings == []
assert len(incompats) == before
assert created not in incompats

# The handle is stale now, and says so on every read.
try:
    created.name
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed incompatibility must raise")

bare = collomatique.IncompatData("Ailleurs", subject)

# A dead incompatibility is refused by the argument convention, and not by the
# model: the two ops that name one can only object to an id the document does
# not hold, and that is caught here, where the message can say which argument
# was wrong.
for call in (
    lambda: incompats.remove(created),
    lambda: incompats.update(created, bare),
    lambda: incompats.update(created.id, bare),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead incompatibility must not be written through")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    incompats.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# `other` is this same file loaded twice, so its incompatibilities and subjects
# carry the very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_incompat = list(other.incompats)[0]
foreign_subject = list(other.subjects)[0]

try:
    incompats.remove(foreign_incompat)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("an incompatibility of another document must not resolve")

# A value is refused before any op is built, and nothing is written: a field
# naming another document's entity …
try:
    incompats.add(collomatique.IncompatData("Ailleurs", foreign_subject))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a subject of another document names nothing here")
assert len(incompats) == before

# … and a field that was never of the right shape.
try:
    incompats.add(collomatique.IncompatData(3, subject))
except TypeError:
    pass
else:
    raise AssertionError("a name that is not a string must be refused")
assert len(incompats) == before

# A call that is wrong about both names the *incompatibility*: a value meant for
# nothing is moot, so the addressee is resolved first.
try:
    incompats.update(created, collomatique.IncompatData(3, subject))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Each call was its own undo slot, named by the operation itself.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert doc.undo_name == update_label

# Undoing the removal puts the incompatibility back under its own id, so the
# handle that went stale reads again.
assert len(incompats) == before + 1
assert created in incompats
assert created.name == "Lundi Midi (par id)"

doc.undo()
assert created.name == "Lundi Midi et une heure"
doc.undo()
assert created.name == "Lundi Midi (essai)"
assert doc.undo_name == add_label

doc.undo()
assert len(incompats) == before
assert doc.can_undo is False
