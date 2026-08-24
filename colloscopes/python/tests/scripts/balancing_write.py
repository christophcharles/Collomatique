import collomatique

# `source` is a throwaway copy of a real colloscope — a global entry pursuing
# teacher and slot rotation as objectives, Métamorphose one of the subjects
# holding an override of its own, Arithmancie a subject inheriting the global
# entry, and the Quidditch training a subject that runs no interrogations at all
# — `target` is where the script leaves the document for rust to read back, and
# the three labels are the french names `ops` gives the three operations of this
# family, handed in from rust so that this pins the operation's own label and not
# merely some string.
doc = collomatique.load(source)
balancing = doc.balancing

metamorphose = [subject for subject in doc.subjects if subject.name == "Métamorphose"][0]
arithmancie = [subject for subject in doc.subjects if subject.name == "Arithmancie"][0]
quidditch = [subject for subject in doc.subjects
             if subject.name == "Entrainement de Quidditch"][0]
assert quidditch.interrogation is None

before = len(balancing.overrides())
opened_global = balancing.global_options.to_data()
metamorphose_before = balancing.override_for(metamorphose).to_data()
assert balancing.override_for(arithmancie) is None

# Two views held from before every write: the global one, which is bound to an
# entry that is always there, and the resolved view over Arithmancie, which
# re-resolves on every read and so follows an override appearing and vanishing.
global_view = balancing.global_options
arithmancie_view = balancing.options_for(arithmancie)
assert arithmancie_view.to_data() == opened_global

# The global entry is rewritten whole: what the value says is what it becomes,
# and the goal left at `None` is not left alone — it stops being pursued for
# every subject without an override.
new_global = collomatique.BalancingData(
    teacher_rotation=None,
    slot_rotation=collomatique.Enforcement.STRICT,
    year_teacher_rotation=True)
result = balancing.set_global(new_global)
assert isinstance(result, collomatique.OpResult)
# A write of this family creates nothing, so it answers a plain `OpResult` and
# never the `AddResult` subclass: there is no `created` at all, rather than one
# holding `None`.
assert not isinstance(result, collomatique.AddResult)
assert not hasattr(result, "created")
# Nothing in the document points at a balancing entry, so no write of this
# family ever has anything to repair — but the result says so rather than the
# call saying nothing at all.
assert result.warnings == []

# The views read the document, so the ones held from before the write read the
# entry it left.
assert global_view.to_data() == new_global
assert global_view.teacher_rotation is None
assert global_view.slot_rotation == collomatique.Enforcement.STRICT
assert global_view.year_teacher_rotation is True
# Arithmancie still has no override, so its resolved view still answers the
# global entry — the new one.
assert arithmancie_view.to_data() == new_global
assert balancing.override_for(arithmancie) is None
assert len(balancing.overrides()) == before

# An override is a whole entry too, and it replaces the global one **verbatim**
# for that subject: the goals left at `None` here stop being pursued rather than
# being inherited from the global entry.
partial = collomatique.BalancingData(
    teacher_rotation=None,
    avoid_twice_in_a_row=collomatique.Enforcement.STRICT)
assert balancing.set_subject(arithmancie, partial).warnings == []
assert len(balancing.overrides()) == before + 1

override_view = balancing.override_for(arithmancie)
assert override_view is not None
assert override_view.to_data() == partial
# The resolved view followed the override appearing, masking included: the
# global entry's strict slot rotation is not inherited.
assert arithmancie_view.to_data() == partial
assert arithmancie_view.slot_rotation is None
assert arithmancie_view.year_teacher_rotation is False

# The subject is named by an id or by a handle, interchangeably — this is the
# write half of the argument convention. The whole entry is replaced again: the
# teacher rotation comes back as a constraint and the period switch turns on.
rewritten = collomatique.BalancingData(
    teacher_rotation=collomatique.Enforcement.STRICT,
    period_teacher_rotation=True)
balancing.set_subject(arithmancie.id, rewritten)
assert override_view.to_data() == rewritten
assert arithmancie_view.to_data() == rewritten
assert len(balancing.overrides()) == before + 1

# Nobody else was touched: one of the overrides the document opened with is
# what it was.
assert balancing.override_for(metamorphose).to_data() == metamorphose_before

# `other` is this same file loaded twice, so its subjects carry the very ids
# this document uses — and still name nothing here. The refusal is about the
# document the handle came from and not about the number it holds.
other = collomatique.load(source)
foreign = [subject for subject in other.subjects if subject.name == "Arithmancie"][0]
assert foreign.id in doc.subjects

# A subject this document does not hold is refused by the argument convention,
# and not by the model: both ops that name one could object to an id the
# document does not know, and that is caught here, where the message can say
# which argument was wrong.
for call in (
    lambda: balancing.set_subject(foreign, partial),
    lambda: balancing.remove_subject(foreign),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a subject of another document must not resolve")

# Something that was never a reference to a subject at all is a `TypeError`: it
# is not a stale anything.
for call in (
    lambda: balancing.set_subject(3, partial),
    lambda: balancing.remove_subject(3),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("a key that is not a subject must not resolve")

# A value is refused before any op is built, and nothing is written: a field
# that was never of the right shape, read at its own site inside the value.
try:
    balancing.set_global(collomatique.BalancingData(teacher_rotation=True))
except TypeError:
    pass
else:
    raise AssertionError("a rotation goal that is not an Enforcement must be refused")

try:
    balancing.set_subject(
        arithmancie, collomatique.BalancingData(year_teacher_rotation="oui"))
except TypeError:
    pass
else:
    raise AssertionError("a switch that is not True or False must be refused")

# A call that is wrong about both names the *subject*: a value meant for nobody
# is moot, so the addressee is resolved first.
try:
    balancing.set_subject(
        foreign, collomatique.BalancingData(teacher_rotation=True))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# The first of the two refusals this family keeps for the model: only a subject
# that runs interrogations may carry an override, since a subject without them
# has nothing to balance. The address is right and the value is fine; it is the
# document that says no, so it arrives as the family's own exception.
try:
    balancing.set_subject(quidditch, partial)
except collomatique.BalancingError as error:
    assert isinstance(error, collomatique.UpdateError)
    assert isinstance(error, collomatique.Error)
    assert str(error)
    assert error.op == "UpdateSubjectOptions"
    assert error.case == "SubjectHasNoInterrogation"
    assert error.details == (quidditch.id,)
else:
    raise AssertionError("a subject without interrogations must not carry an override")

assert balancing.override_for(quidditch) is None

# Nothing of that was written: both entries are what the last accepted writes
# left.
assert global_view.to_data() == new_global
assert override_view.to_data() == rewritten
assert len(balancing.overrides()) == before + 1

# This is what rust reads back off the disk.
doc.save(target)

# Removing the override gives the subject the global entry back, and takes
# nothing else with it.
removed = balancing.remove_subject(arithmancie)
assert isinstance(removed, collomatique.OpResult)
assert removed.warnings == []
assert balancing.override_for(arithmancie) is None
assert len(balancing.overrides()) == before

# The raw view was bound to the entry, so it is stale now …
try:
    override_view.teacher_rotation
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed override must raise")

# … while the resolved view lives on and falls back to the global entry, which
# is the point of its being a view.
assert arithmancie_view.to_data() == new_global

# The second refusal the family keeps for the model: a subject with no override
# has nothing to remove, and the op says so rather than quietly doing nothing.
try:
    balancing.remove_subject(arithmancie)
except collomatique.BalancingError as error:
    assert isinstance(error, collomatique.UpdateError)
    assert str(error)
    assert error.op == "RemoveSubjectOptions"
    assert error.case == "NoOptionsForSubject"
    # The subject the model named — the very one this script is holding, as the
    # id class.
    assert error.details == (arithmancie.id,)
else:
    raise AssertionError("removing an override that is not there must be refused")

# The global entry has no such refusal to keep: it names no subject, so setting
# it twice in a row is two writes and no complaint.
assert balancing.set_global(new_global).warnings == []

# Each accepted call was its own undo slot, named by the operation itself — and
# the refused ones left no slot at all.
assert doc.undo_name == global_label
doc.undo()
assert doc.redo_name == global_label
assert doc.undo_name == remove_label

doc.undo()
# Undoing the removal puts the entry back under the subject it was keyed by, so
# the raw view that went stale reads again.
assert balancing.override_for(arithmancie) is not None
assert override_view.to_data() == rewritten
assert arithmancie_view.to_data() == rewritten
assert doc.undo_name == subject_label

doc.undo()
assert override_view.to_data() == partial
doc.undo()
assert balancing.override_for(arithmancie) is None
assert len(balancing.overrides()) == before
assert doc.undo_name == global_label

doc.undo()
assert global_view.to_data() == opened_global
assert arithmancie_view.to_data() == opened_global
assert doc.can_undo is False
