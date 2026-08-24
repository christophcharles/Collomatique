import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the three labels are the
# french names `ops` gives the three operations of this family — handed in from
# rust so that this pins the operation's own label and not merely some string.
#
# `pruned_index` and `doomed_index` are places in the teachers' own order: rust
# cannot hand a script an id, so it names the two teachers whose fixtures this
# script leans on by where they sit, and asserts what it needs of them on its
# own side.
doc = collomatique.load(source)
teachers = doc.teachers

before = len(teachers)
slots_before = len(doc.slots)
rules_before = len(doc.slot_pairings)

# A teacher may only interrogate in a subject that runs colles. The document
# also holds subjects that run none — they exist so that incompatibilities can
# block slots for their students — and one of those is what the model's own
# refusal is asserted on further down.
with_colles = [subject for subject in doc.subjects if subject.interrogation is not None]
maths, physics = with_colles[0], with_colles[1]
without_colles = next(
    subject for subject in doc.subjects if subject.interrogation is None
)

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = teachers.add(
    collomatique.TeacherData(
        "Emmy", "Noether", email="noether@lycee.fr", subjects={maths}
    )
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# A teacher arrives holding no slot, so there is nothing for the cascade to
# repair — but the result says so rather than the call saying nothing at all.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.Teacher)
assert isinstance(created.id, collomatique.TeacherId)
assert created in teachers
assert teachers[created.id] == created
assert len(teachers) == before + 1

assert created.firstname == "Emmy"
assert created.surname == "Noether"
assert created.tel is None
assert created.email == "noether@lycee.fr"
assert created.subjects == frozenset({maths})

assert added.created == created
assert repr(added).startswith("AddResult(created=<Teacher #")
assert "warnings=[]" in repr(added)

# A teacher who interrogates in nothing at all is perfectly ordinary — it is
# what somebody who has just been typed in looks like.
newcomer = teachers.add(collomatique.TeacherData("Argus", "Rusard")).created
assert newcomer.subjects == frozenset()
assert newcomer.tel is None
assert newcomer.email is None
assert len(teachers) == before + 2

# Rewriting replaces the whole value: the card and the subjects at once, and the
# id stays, so the handle a script is holding reads the new state. The email the
# value does not carry is gone rather than kept.
result = teachers.update(
    created,
    collomatique.TeacherData(
        "Emmy", "Noether-Tietze", tel="0102030405", subjects={maths, physics}
    ),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")
assert result.warnings == []

assert created.surname == "Noether-Tietze"
assert created.tel == "0102030405"
assert created.email is None
assert created.subjects == frozenset({maths, physics})
assert len(teachers) == before + 2

# The teacher is named by an id or by a handle, interchangeably — this is the
# write half of the argument convention.
teachers.update(
    created.id,
    collomatique.TeacherData("Emmy", "Noether", subjects={maths}),
)
assert created.surname == "Noether"
assert created.tel is None
assert created.subjects == frozenset({maths})

# The first cascade of this family, and it comes from an `update` rather than
# from a removal: `pruned` interrogates in two subjects and holds slots in each,
# so dropping one subject leaves those slots with nobody to hold them, and they
# go. The other subject's slots are untouched — the repair is precise.
pruned = list(teachers)[pruned_index]
pruned_subjects = [subject for subject in doc.subjects if subject in pruned.subjects]
kept, dropped = pruned_subjects[0], pruned_subjects[1]

pruned_slots = [slot for slot in doc.slots if slot.teacher == pruned]
kept_slot_ids = {slot.id for slot in pruned_slots if slot.subject == kept}
dropped_slot_ids = {slot.id for slot in pruned_slots if slot.subject == dropped}
assert kept_slot_ids and dropped_slot_ids

pruning = teachers.update(
    pruned,
    collomatique.TeacherData(
        pruned.firstname,
        pruned.surname,
        tel=pruned.tel,
        email=pruned.email,
        subjects={kept},
    ),
)
assert pruned.subjects == frozenset({kept})
assert [w.kind for w in pruning.warnings] == ["DeleteSlot"] * len(dropped_slot_ids)
assert {w.details["slot"] for w in pruning.warnings} == dropped_slot_ids
# The write asked for those directly, so none of them hangs off another repair.
assert all(w.parent is None for w in pruning.warnings)
assert {slot.id for slot in doc.slots if slot.teacher == pruned} == kept_slot_ids
assert len(doc.slots) == slots_before - len(dropped_slot_ids)

# The one refusal this family keeps for the model: nobody can be declared to
# teach a subject that runs no colles — there is no interrogation for them to
# hold. It is a statement about the document, not about the value, so it arrives
# as the family's own exception rather than from the value boundary.
for call, op in (
    (
        lambda: teachers.add(
            collomatique.TeacherData("Sybille", "Trelawney", subjects={without_colles})
        ),
        "AddNewTeacher",
    ),
    (
        lambda: teachers.update(
            created,
            collomatique.TeacherData("Emmy", "Noether", subjects={without_colles}),
        ),
        "UpdateTeacher",
    ),
):
    try:
        call()
    except collomatique.TeachersError as error:
        assert isinstance(error, collomatique.UpdateError)
        assert isinstance(error, collomatique.Error)
        assert str(error)
        assert error.op == op
        assert error.case == "SubjectHasNoInterrogation"
        # The subject the model named, as the id class — the very subject this
        # script is holding.
        assert error.details == (without_colles.id,)
    else:
        raise AssertionError("a teacher of a subject without colles holds nothing")

# Nothing of that was written: the teacher is what the last accepted write left.
assert len(teachers) == before + 2
assert created.subjects == frozenset({maths})

# This is what rust reads back off the disk.
doc.save(target)

# The removal cascade. `doomed` holds slots, and a slot pairing rule relates two
# of them: the slots go because a slot cannot do without its teacher, and the
# rule goes because one of its slots did.
doomed = list(teachers)[doomed_index]
doomed_slots = [slot for slot in doc.slots if slot.teacher == doomed]
doomed_slot_ids = {slot.id for slot in doomed_slots}
doomed_rule_ids = {
    rule.id
    for rule in doc.slot_pairings
    if rule.antecedent.slot in doomed_slots or rule.consequent.slot in doomed_slots
}
assert doomed_slot_ids
assert len(doomed_rule_ids) == 1

removed = teachers.remove(doomed)
assert isinstance(removed, collomatique.OpResult)
assert not hasattr(removed, "created")
warnings = removed.warnings
assert warnings, "removing a teacher who holds slots repairs something"

assert doomed not in teachers
assert len(teachers) == before + 1
assert len(doc.slots) == slots_before - len(dropped_slot_ids) - len(doomed_slot_ids)
assert len(doc.slot_pairings) == rules_before - 1

# Every repair says the same four things, and its coordinates are the very ids
# this script was holding before the write.
for w in warnings:
    assert isinstance(w, collomatique.Warning)
    assert str(w)
    assert isinstance(w.details, dict)

assert {w.details["slot"] for w in warnings if w.kind == "DeleteSlot"} == doomed_slot_ids

# The list is a tree: the rule went because a slot did, so it names the repair
# that needed it — the object itself, and one that is further down the list,
# since a repair lands before the one that needed it.
gone_rules = [w for w in warnings if w.kind == "DeleteSlotPairingRule"]
assert len(gone_rules) == 1
assert {w.details["rule"] for w in gone_rules} == doomed_rule_ids
assert isinstance(gone_rules[0].parent, collomatique.Warning)
assert gone_rules[0].parent.kind == "DeleteSlot"
assert gone_rules[0].parent.details["slot"] in doomed_slot_ids
assert any(w.parent is None for w in warnings)

# The slots the cascade took stale their handles, exactly as the teacher's own
# handle does.
try:
    doomed.subjects
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed teacher must raise")

try:
    doomed_slots[0].teacher
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a slot the cascade removed must read as gone")

# A teacher who holds no slot takes nothing with them: the same op, and an empty
# answer, because this one had nothing to break.
gone = teachers.remove(created.id)
assert gone.warnings == []
assert len(teachers) == before
assert created not in teachers

try:
    created.surname
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed teacher must raise")

bare = collomatique.TeacherData("Ailleurs", "Ailleurs")

# A dead teacher is refused by the argument convention, and not by the model:
# the two ops that name one can only object to an id the document does not hold,
# and that is caught here, where the message can say which argument was wrong.
for call in (
    lambda: teachers.remove(created),
    lambda: teachers.update(created, bare),
    lambda: teachers.update(created.id, bare),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead teacher must not be written through")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    teachers.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# `other` is this same file loaded twice, so its teachers and subjects carry the
# very ids this document uses — and still name nothing here.
other = collomatique.load(source)
foreign_teacher = list(other.teachers)[0]
foreign_subject = list(other.subjects)[0]

try:
    teachers.remove(foreign_teacher)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a teacher of another document must not resolve")

# A value is refused before any op is built, and nothing is written: a field
# naming another document's entity …
try:
    teachers.add(
        collomatique.TeacherData("Ailleurs", "Ailleurs", subjects={foreign_subject})
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a subject of another document names nothing here")

# … and a field that was never of the right shape.
try:
    teachers.add(collomatique.TeacherData(3, "Ailleurs"))
except TypeError:
    pass
else:
    raise AssertionError("a firstname that is not a string must be refused")
assert len(teachers) == before

# A call that is wrong about both names the *teacher*: a value meant for nothing
# is moot, so the addressee is resolved first.
try:
    teachers.update(created, collomatique.TeacherData(3, "Ailleurs"))
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Each accepted call was its own undo slot, named by the operation itself.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert doc.undo_name == remove_label

# Undoing a removal puts back everything it took: the teacher, their slots, and
# the rule that went with one of them.
assert created in teachers
assert created.surname == "Noether"
doc.undo()
assert doomed in teachers
assert len(doc.slots) == slots_before - len(dropped_slot_ids)
assert len(doc.slot_pairings) == rules_before

assert doc.undo_name == update_label
doc.undo()
assert pruned.subjects == frozenset({kept, dropped})
assert len(doc.slots) == slots_before

doc.undo()
assert created.surname == "Noether-Tietze"
doc.undo()
assert created.email == "noether@lycee.fr"
assert created.subjects == frozenset({maths})

assert doc.undo_name == add_label
doc.undo()
assert newcomer not in teachers
doc.undo()
assert len(teachers) == before
assert doc.can_undo is False
