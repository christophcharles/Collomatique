import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the labels are the
# french names `ops` gives the operations this script makes — handed in from
# rust so that this pins the operations' own labels and not merely some strings.
# `incompat_label` is the one write of another family the script makes, and it
# carries that family's name rather than this one's.
#
# `followed_index` is a place in the patterns' own order: rust cannot hand a
# script an id, so it names the pattern this script leans on by where it sits,
# and asserts what it needs of it on its own side.
doc = collomatique.load(source)
patterns = doc.week_patterns

before = len(patterns)
slots_before = len(doc.slots)
incompats_before = len(doc.incompats)
weeks = list(doc.weeks)

# The pattern the removal is made on, and what follows it. A slot follows a
# pattern in the example; nothing else does, so the script points an
# incompatibility at it first — through `doc.incompats`, which is a published
# mutator — to give the removal both of its kinds of site.
doomed = list(patterns)[followed_index]
doomed_slots = [slot for slot in doc.slots if slot.week_pattern == doomed]
assert len(doomed_slots) == 1

incompat = list(doc.incompats)[0]
following = incompat.to_data()
assert following.week_pattern is None
following.week_pattern = doomed
doc.incompats.update(incompat, following)
assert incompat.week_pattern == doomed

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = patterns.add(
    collomatique.WeekPatternData(
        "Semaines de rentrée", excluded_weeks={weeks[0], weeks[1]}
    )
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# Nothing follows a pattern that has just been made — a slot picks one up
# afterwards — so there is nothing for the cascade to repair, whatever the new
# pattern switches off. The result says so rather than the call saying nothing.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.WeekPattern)
assert isinstance(created.id, collomatique.WeekPatternId)
assert created in patterns
assert patterns[created.id] == created
assert len(patterns) == before + 1

assert created.name == "Semaines de rentrée"
assert created.excluded_weeks == frozenset({weeks[0], weeks[1]})

assert added.created == created
assert repr(added).startswith("AddResult(created=<WeekPattern #")
assert "warnings=[]" in repr(added)

# A pattern that excludes nothing is an ordinary pattern and not an unfinished
# one: it leaves every week alone. So is one nobody has named — the model types
# the name as a plain string, and `""` is what a pattern the user never named
# reads as.
plain = patterns.add(collomatique.WeekPatternData("")).created
assert plain.name == ""
assert plain.excluded_weeks == frozenset()
assert len(patterns) == before + 2

# Rewriting replaces the whole value: the name and the excluded weeks at once,
# and the id stays, so the handle a script is holding reads the new state.
result = patterns.update(
    created,
    collomatique.WeekPatternData("Semaines de reprise", excluded_weeks={weeks[2]}),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")
# Nothing follows this pattern, and the document holds no colle at this point,
# so switching a week off here repairs nothing.
assert result.warnings == []

assert created.name == "Semaines de reprise"
assert created.excluded_weeks == frozenset({weeks[2]})
assert len(patterns) == before + 2

# The pattern is named by an id or by a handle, interchangeably — this is the
# write half of the argument convention. The set takes handles and ids in any
# mix, the way every other place naming entities does.
patterns.update(
    created.id,
    collomatique.WeekPatternData(
        "Semaines de rentrée", excluded_weeks={weeks[0], weeks[1].id}
    ),
)
assert created.name == "Semaines de rentrée"
assert created.excluded_weeks == frozenset({weeks[0], weeks[1]})

# This is what rust reads back off the disk.
doc.save(target)

# The removal cascade, and the whole point of this family: what followed the
# pattern lets go of it and *stays*. « Pas de modèle » is a legal value for a
# slot and for an incompatibility alike — it means every week — so nothing dies
# here, and the two repairs are the only account of a slot that has just changed
# meaning.
removed = patterns.remove(doomed)
assert isinstance(removed, collomatique.OpResult)
assert not hasattr(removed, "created")
warnings = removed.warnings

assert doomed not in patterns
assert len(patterns) == before + 1
assert len(doc.slots) == slots_before
assert len(doc.incompats) == incompats_before
assert doomed_slots[0].week_pattern is None
assert incompat.week_pattern is None

# Every repair says the same four things, and its coordinates are the very ids
# this script was holding before the write.
for w in warnings:
    assert isinstance(w, collomatique.Warning)
    assert str(w)
    assert isinstance(w.details, dict)

by_kind = {w.kind: w for w in warnings}
assert sorted(by_kind) == ["ClearIncompatWeekPattern", "ClearSlotWeekPattern"]
assert len(warnings) == 2
assert by_kind["ClearSlotWeekPattern"].details == {"slot": doomed_slots[0].id}
assert by_kind["ClearIncompatWeekPattern"].details == {"incompat": incompat.id}

# The write asked for both directly — neither reference needed the other — so
# this cascade is flat where a removal that takes entities with it is a tree.
assert all(w.parent is None for w in warnings)

# The pattern's own handle is the only one that went stale: a slot that lost its
# pattern is still a slot, and reads.
try:
    doomed.name
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed week pattern must raise")

assert doomed_slots[0] in doc.slots
assert incompat in doc.incompats

# A pattern nothing follows takes nothing with it: the same op, and an empty
# answer, because this one had nothing to let go of it.
gone = patterns.remove(plain.id)
assert gone.warnings == []
assert len(patterns) == before
assert plain not in patterns

try:
    plain.excluded_weeks
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed week pattern must raise")

bare = collomatique.WeekPatternData("Ailleurs")

# A dead pattern is refused by the argument convention, and not by the model:
# the two ops that name one can only object to an id the document does not hold,
# and that is caught here, where the message can say which argument was wrong.
for call in (
    lambda: patterns.remove(plain),
    lambda: patterns.update(plain, bare),
    lambda: patterns.update(plain.id, bare),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead week pattern must not be written through")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    patterns.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# `other` is this same file loaded twice, so its patterns and weeks carry the
# very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_pattern = list(other.week_patterns)[0]
foreign_week = list(other.weeks)[0]

try:
    patterns.remove(foreign_pattern)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a pattern of another document must not resolve")

# A value is refused before any op is built, and nothing is written: a field
# naming another document's entity …
try:
    patterns.add(
        collomatique.WeekPatternData("Ailleurs", excluded_weeks={foreign_week})
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a week of another document names nothing here")

# … and a field that was never of the right shape.
try:
    patterns.add(collomatique.WeekPatternData(3))
except TypeError:
    pass
else:
    raise AssertionError("a name that is not a string must be refused")
assert len(patterns) == before

# A call that is wrong about both names the *pattern*: a value meant for nothing
# is moot, so the addressee is resolved first.
try:
    patterns.update(plain, collomatique.WeekPatternData(3))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Each accepted call was its own undo slot, named by the operation itself.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert doc.undo_name == remove_label
assert plain in patterns
assert plain.name == ""

# Undoing the removal puts the pattern back, and the two references with it.
doc.undo()
assert doomed in patterns
assert doomed_slots[0].week_pattern == doomed
assert incompat.week_pattern == doomed

assert doc.undo_name == update_label
doc.undo()
assert created.name == "Semaines de reprise"
assert created.excluded_weeks == frozenset({weeks[2]})
doc.undo()
assert created.name == "Semaines de rentrée"
assert created.excluded_weeks == frozenset({weeks[0], weeks[1]})

assert doc.undo_name == add_label
doc.undo()
assert plain not in patterns
doc.undo()
assert created not in patterns
assert len(patterns) == before

# The last slot is the one write of another family, and it carries that family's
# name: an undo slot is named by the operation that filled it.
assert doc.undo_name == incompat_label
doc.undo()
assert incompat.week_pattern is None
assert doc.can_undo is False
