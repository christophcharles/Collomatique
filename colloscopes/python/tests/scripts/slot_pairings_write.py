import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the three labels are the
# french names `ops` gives the three operations of this family — handed in from
# rust so that this pins the operation's own label and not merely some string.
doc = collomatique.load(source)
slot_pairings = doc.slot_pairings

before = len(slot_pairings)

# A slot pairing rule pairs two slots of one subject, so the three slots the
# script writes rules with come from the first subject that runs at least three
# of them. `elsewhere` is a slot of some other subject, and it is the whole of
# what the model's own refusal is asserted on further down.
all_slots = list(doc.slots)
subject = next(
    subject
    for subject in doc.subjects
    if len([slot for slot in all_slots if slot.subject == subject]) >= 3
)
first, second, third = [slot for slot in all_slots if slot.subject == subject][:3]
elsewhere = next(slot for slot in all_slots if slot.subject != subject)
period = list(doc.periods)[0]

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = slot_pairings.add(
    collomatique.SlotPairingRuleData(
        collomatique.SlotPairingRuleSideData(first),
        collomatique.SlotPairingRuleSideData(second, should_have=False),
    )
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# Nothing in the document points at a slot pairing rule, so no write of this
# family ever has anything to repair — but the result says so rather than the
# call saying nothing at all.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.SlotPairingRule)
assert isinstance(created.id, collomatique.SlotPairingRuleId)
assert created in slot_pairings
assert slot_pairings[created.id] == created
assert len(slot_pairings) == before + 1

assert created.antecedent.slot == first
assert created.antecedent.should_have is True
assert created.consequent.slot == second
assert created.consequent.should_have is False
assert created.excluded_periods == frozenset()
assert created.soft is False

assert added.created == created
assert repr(added).startswith("AddResult(created=<SlotPairingRule #")
assert "warnings=[]" in repr(added)

# The two ends are sub-views, and this one was handed out before the rewrite
# below: it reads the rule as it stands, not as it stood.
side = created.antecedent

# Rewriting replaces the whole value: both ends, the exclusions and the softness
# at once, and the id stays, so the handle a script is holding reads the new
# state.
result = slot_pairings.update(
    created,
    collomatique.SlotPairingRuleData(
        collomatique.SlotPairingRuleSideData(second),
        collomatique.SlotPairingRuleSideData(third, should_have=False),
        excluded_periods={period},
        soft=True,
    ),
)
assert isinstance(result, collomatique.OpResult)
assert not isinstance(result, collomatique.AddResult)
assert result.warnings == []
# A write that creates nothing has no `created` at all, rather than one holding
# `None`: different answers are different types.
assert not hasattr(result, "created")

assert created.antecedent.slot == second
assert created.consequent.slot == third
assert created.excluded_periods == frozenset({period})
assert created.soft is True
assert side.slot == second

# The rule is named by an id or by a handle, interchangeably — this is the write
# half of the argument convention.
slot_pairings.update(
    created.id,
    collomatique.SlotPairingRuleData(
        collomatique.SlotPairingRuleSideData(third, should_have=False),
        collomatique.SlotPairingRuleSideData(first),
        excluded_periods={period},
        soft=True,
    ),
)
assert created.antecedent.slot == third
assert created.antecedent.should_have is False

# The one refusal this family keeps for the model: the two slots of a rule must
# belong to one subject, on either end and for either op. Which subject a slot
# sits in is a statement about the document, not about the value, so it arrives
# as the family's own exception rather than from the value boundary.
for across, ends in (
    (
        lambda: collomatique.SlotPairingRuleData(
            collomatique.SlotPairingRuleSideData(elsewhere),
            collomatique.SlotPairingRuleSideData(first),
        ),
        (elsewhere, first),
    ),
    (
        lambda: collomatique.SlotPairingRuleData(
            collomatique.SlotPairingRuleSideData(first),
            collomatique.SlotPairingRuleSideData(elsewhere),
        ),
        (first, elsewhere),
    ),
):
    for call, op in (
        (lambda: slot_pairings.add(across()), "AddNewSlotPairingRule"),
        (lambda: slot_pairings.update(created, across()), "UpdateSlotPairingRule"),
    ):
        try:
            call()
        except collomatique.SlotPairingsError as error:
            assert isinstance(error, collomatique.UpdateError)
            assert isinstance(error, collomatique.Error)
            assert str(error)
            assert error.op == op
            assert error.case == "SlotsNotInSameSubject"
            # The two slots the model named, in the rule's own order — the very
            # slots this script is holding, as the id class.
            assert error.details == (ends[0].id, ends[1].id)
        else:
            raise AssertionError("a rule across two subjects must be refused")

# Nothing of that was written: the rule is what the last accepted write left.
assert len(slot_pairings) == before + 1
assert created.antecedent.slot == third

# This is what rust reads back off the disk.
doc.save(target)

# Removing takes it away and nothing else: the removal of a leaf entity has
# nothing to cascade.
removed = slot_pairings.remove(created.id)
assert isinstance(removed, collomatique.OpResult)
assert removed.warnings == []
assert len(slot_pairings) == before
assert created not in slot_pairings

# The handle is stale now, and so is the side view it handed out.
try:
    created.soft
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed slot pairing rule must raise")

try:
    side.slot
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a side view of a removed rule must raise")

bare = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(first),
    collomatique.SlotPairingRuleSideData(second),
)

# A dead rule is refused by the argument convention, and not by the model: the
# two ops that name one could object to an id the document does not hold, and
# that is caught here, where the message can say which argument was wrong.
for call in (
    lambda: slot_pairings.remove(created),
    lambda: slot_pairings.update(created, bare),
    lambda: slot_pairings.update(created.id, bare),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead slot pairing rule must not be written through")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    slot_pairings.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# `other` is this same file loaded twice, so its rules and its slots carry the
# very ids this document uses — and still name nothing here. The refusal is
# about the document the handle came from and not about the number it holds:
# the example ships two slot pairing rules, so `foreign_rule`'s id *is* one this
# document knows.
other = collomatique.load(source)
foreign_rule = list(other.slot_pairings)[0]
assert foreign_rule.id in slot_pairings
foreign_slot = list(other.slots)[0]
assert foreign_slot.id in doc.slots

try:
    slot_pairings.remove(foreign_rule)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a slot pairing rule of another document must not resolve")

# A value is refused before any op is built, and nothing is written: a field
# naming another document's entity …
try:
    slot_pairings.add(
        collomatique.SlotPairingRuleData(
            collomatique.SlotPairingRuleSideData(foreign_slot),
            collomatique.SlotPairingRuleSideData(second),
        )
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a slot of another document names nothing here")
assert len(slot_pairings) == before

# … a field that was never of the right shape, read at its own site inside the
# value …
try:
    slot_pairings.add(
        collomatique.SlotPairingRuleData(
            collomatique.SlotPairingRuleSideData(3),
            collomatique.SlotPairingRuleSideData(second),
        )
    )
except TypeError:
    pass
else:
    raise AssertionError("a slot that is not an id must be refused")
assert len(slot_pairings) == before

# … and the value's own invariant, which is the model's constructor speaking:
# an implication from a slot to itself says nothing.
try:
    slot_pairings.add(
        collomatique.SlotPairingRuleData(
            collomatique.SlotPairingRuleSideData(first),
            collomatique.SlotPairingRuleSideData(first),
        )
    )
except ValueError:
    pass
else:
    raise AssertionError("a rule from a slot to itself must be refused")
assert len(slot_pairings) == before

# A call that is wrong about both names the *rule*: a value meant for nothing is
# moot, so the addressee is resolved first.
try:
    slot_pairings.update(
        created,
        collomatique.SlotPairingRuleData(
            collomatique.SlotPairingRuleSideData(first),
            collomatique.SlotPairingRuleSideData(first),
        ),
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("the addressee is what a doubly-wrong call names")

# Each accepted call was its own undo slot, named by the operation itself — and
# the refused ones left no slot at all.
assert doc.undo_name == remove_label
doc.undo()
assert doc.redo_name == remove_label
assert doc.undo_name == update_label

# Undoing the removal puts the rule back under its own id, so the handle that
# went stale reads again — and so does the side view.
assert len(slot_pairings) == before + 1
assert created in slot_pairings
assert created.antecedent.slot == third
assert side.slot == third

doc.undo()
assert created.antecedent.slot == second
assert created.consequent.slot == third
doc.undo()
assert created.antecedent.slot == first
assert created.consequent.slot == second
assert created.soft is False
assert doc.undo_name == add_label

doc.undo()
assert len(slot_pairings) == before
assert doc.can_undo is False
