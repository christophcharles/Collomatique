import collomatique

# `source` is a throwaway copy of a real colloscope — a global entry setting all
# three limits, Hermione the one student holding an override of her own, and
# Harry a student inheriting the global entry — `target` is where the script
# leaves the document for rust to read back, and the three labels are the french
# names `ops` gives the three operations of this family, handed in from rust so
# that this pins the operation's own label and not merely some string.
doc = collomatique.load(source)
settings = doc.settings

hermione = [student for student in doc.students if student.surname == "Granger"][0]
harry = [student for student in doc.students if student.surname == "Potter"][0]

before = len(settings.overrides())
opened_global = settings.global_limits.to_data()
hermione_before = settings.override_for(hermione).to_data()
assert settings.override_for(harry) is None

# Two views held from before every write: the global one, which is bound to an
# entry that is always there, and the resolved view over Harry, which re-resolves
# on every read and so follows an override appearing and vanishing.
global_view = settings.global_limits
harry_view = settings.limits_for(harry)
assert harry_view.to_data() == opened_global

# The global entry is rewritten whole: what the value says is what it becomes,
# and the two fields left at `None` are not left alone — they disable those
# limits for every student without an override.
new_global = collomatique.LimitsData(
    interrogations_per_week_max=collomatique.Limit(3, collomatique.Enforcement.STRICT))
result = settings.set_global_limits(new_global)
assert isinstance(result, collomatique.OpResult)
# A write of this family creates nothing, so it answers a plain `OpResult` and
# never the `AddResult` subclass: there is no `created` at all, rather than one
# holding `None`.
assert not isinstance(result, collomatique.AddResult)
assert not hasattr(result, "created")
# Nothing in the document points at a limits entry, so no write of this family
# ever has anything to repair — but the result says so rather than the call
# saying nothing at all.
assert result.warnings == []

# The views read the document, so the ones held from before the write read the
# entry it left.
assert global_view.to_data() == new_global
assert global_view.interrogations_per_week_min is None
assert global_view.interrogations_per_week_max == collomatique.Limit(
    3, collomatique.Enforcement.STRICT)
# Harry still has no override, so his resolved view still answers the global
# entry — the new one.
assert harry_view.to_data() == new_global
assert settings.override_for(harry) is None
assert len(settings.overrides()) == before

# An override is a whole entry too, and it replaces the global one **verbatim**
# for that student: the two fields left at `None` here disable the global limits
# rather than inheriting them.
partial = collomatique.LimitsData(
    interrogations_per_week_min=collomatique.Limit(4, collomatique.Enforcement.OBJECTIVE))
assert settings.set_student_limits(harry, partial).warnings == []
assert len(settings.overrides()) == before + 1

override_view = settings.override_for(harry)
assert override_view is not None
assert override_view.to_data() == partial
# The resolved view followed the override appearing, masking included.
assert harry_view.to_data() == partial
assert harry_view.interrogations_per_week_max is None

# The student is named by an id or by a handle, interchangeably — this is the
# write half of the argument convention. The whole entry is replaced again:
# the minimum changes and the per-day limit appears.
rewritten = collomatique.LimitsData(
    interrogations_per_week_min=collomatique.Limit(1, collomatique.Enforcement.STRICT),
    max_interrogations_per_day=collomatique.Limit(2, collomatique.Enforcement.OBJECTIVE))
settings.set_student_limits(harry.id, rewritten)
assert override_view.to_data() == rewritten
assert harry_view.to_data() == rewritten
assert len(settings.overrides()) == before + 1

# Nobody else was touched: the one override the document opened with is what it
# was.
assert settings.override_for(hermione).to_data() == hermione_before

# `other` is this same file loaded twice, so its students carry the very ids
# this document uses — and still name nothing here. The refusal is about the
# document the handle came from and not about the number it holds.
other = collomatique.load(source)
foreign = [student for student in other.students if student.surname == "Potter"][0]
assert foreign.id in doc.students

# A student this document does not hold is refused by the argument convention,
# and not by the model: both ops that name one could object to an id the
# document does not know, and that is caught here, where the message can say
# which argument was wrong.
for call in (
    lambda: settings.set_student_limits(foreign, partial),
    lambda: settings.remove_student_limits(foreign),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a student of another document must not resolve")

# Something that was never a reference to a student at all is a `TypeError`: it
# is not a stale anything.
for call in (
    lambda: settings.set_student_limits(3, partial),
    lambda: settings.remove_student_limits(3),
):
    try:
        call()
    except TypeError:
        pass
    else:
        raise AssertionError("a key that is not a student must not resolve")

# A value is refused before any op is built, and nothing is written: the model's
# own at-least-one rule on the per-day limit …
try:
    settings.set_global_limits(collomatique.LimitsData(
        max_interrogations_per_day=collomatique.Limit(0, collomatique.Enforcement.STRICT)))
except ValueError:
    pass
else:
    raise AssertionError("a per-day limit of zero must be refused")

# … and a field that was never of the right shape, read at its own site inside
# the value.
try:
    settings.set_student_limits(
        harry, collomatique.LimitsData(interrogations_per_week_min=3))
except TypeError:
    pass
else:
    raise AssertionError("a limit that is not a Limit must be refused")

# A call that is wrong about both names the *student*: a value meant for nobody
# is moot, so the addressee is resolved first.
try:
    settings.set_student_limits(
        foreign, collomatique.LimitsData(interrogations_per_week_min=3))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Nothing of that was written: both entries are what the last accepted writes
# left.
assert global_view.to_data() == new_global
assert override_view.to_data() == rewritten

# This is what rust reads back off the disk.
doc.save(target)

# Removing the override gives the student the global entry back, and takes
# nothing else with it.
removed = settings.remove_student_limits(harry)
assert isinstance(removed, collomatique.OpResult)
assert removed.warnings == []
assert settings.override_for(harry) is None
assert len(settings.overrides()) == before

# The raw view was bound to the entry, so it is stale now …
try:
    override_view.interrogations_per_week_min
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed override must raise")

# … while the resolved view lives on and falls back to the global entry, which
# is the point of its being a view.
assert harry_view.to_data() == new_global

# The one refusal this family keeps for the model: a student with no override
# has nothing to remove, and the op says so rather than quietly doing nothing.
# That is a statement about the document and not about an argument's shape, so
# it arrives as the family's own exception.
try:
    settings.remove_student_limits(harry)
except collomatique.SettingsError as error:
    assert isinstance(error, collomatique.UpdateError)
    assert isinstance(error, collomatique.Error)
    assert str(error)
    assert error.op == "RemoveStudentLimits"
    assert error.case == "NoLimitsForStudent"
    # The student the model named — the very one this script is holding, as the
    # id class.
    assert error.details == (harry.id,)
else:
    raise AssertionError("removing an override that is not there must be refused")

# The global entry has no such refusal to keep: it names no entity, so setting
# it twice in a row is two writes and no complaint.
assert settings.set_global_limits(new_global).warnings == []

# Each accepted call was its own undo slot, named by the operation itself — and
# the refused ones left no slot at all.
assert doc.undo_name == global_label
doc.undo()
assert doc.redo_name == global_label
assert doc.undo_name == remove_label

doc.undo()
# Undoing the removal puts the entry back under the student it was keyed by, so
# the raw view that went stale reads again.
assert settings.override_for(harry) is not None
assert override_view.to_data() == rewritten
assert harry_view.to_data() == rewritten
assert doc.undo_name == student_label

doc.undo()
assert override_view.to_data() == partial
doc.undo()
assert settings.override_for(harry) is None
assert len(settings.overrides()) == before
assert doc.undo_name == global_label

doc.undo()
assert global_view.to_data() == opened_global
assert harry_view.to_data() == opened_global
assert doc.can_undo is False
