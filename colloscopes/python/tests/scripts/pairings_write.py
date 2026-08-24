import collomatique

# `source` is a throwaway copy of a real colloscope, `target` is where the
# script leaves the document for rust to read back, and the three labels are the
# french names `ops` gives the three operations of this family — handed in from
# rust so that this pins the operation's own label and not merely some string.
doc = collomatique.load(source)
pairings = doc.pairings

before = len(pairings)

# A pairing rule is an implication between two subjects that hold
# interrogations. The document also holds subjects that run no colles at all —
# they exist so that incompatibilities can block slots for their students — and
# one of those is what the model's own refusal is asserted on further down.
with_colles = [subject for subject in doc.subjects if subject.interrogation is not None]
first, second, third = with_colles[0], with_colles[1], with_colles[2]
without_colles = next(
    subject for subject in doc.subjects if subject.interrogation is None
)
period = list(doc.periods)[0]

# The creating op answers the `AddResult` subclass, so a script that only reads
# warnings treats it like any other result.
added = pairings.add(
    collomatique.PairingRuleData(
        collomatique.PairingRuleSideData(first),
        collomatique.PairingRuleSideData(second, should_have=False),
    )
)
assert isinstance(added, collomatique.AddResult)
assert isinstance(added, collomatique.OpResult)

# Nothing in the document points at a pairing rule, so no write of this family
# ever has anything to repair — but the result says so rather than the call
# saying nothing at all.
assert added.warnings == []

# What it created is a *handle* of this document, not an id: the id is one
# attribute away, and the handle reads.
created = added.created
assert isinstance(created, collomatique.PairingRule)
assert isinstance(created.id, collomatique.PairingRuleId)
assert created in pairings
assert pairings[created.id] == created
assert len(pairings) == before + 1

assert created.antecedent.subject == first
assert created.antecedent.should_have is True
assert created.consequent.subject == second
assert created.consequent.should_have is False
assert created.excluded_periods == frozenset()
assert created.soft is False

assert added.created == created
assert repr(added).startswith("AddResult(created=<PairingRule #")
assert "warnings=[]" in repr(added)

# The two ends are sub-views, and this one was handed out before the rewrite
# below: it reads the rule as it stands, not as it stood.
side = created.antecedent

# Rewriting replaces the whole value: both ends, the exclusions and the softness
# at once, and the id stays, so the handle a script is holding reads the new
# state.
result = pairings.update(
    created,
    collomatique.PairingRuleData(
        collomatique.PairingRuleSideData(second),
        collomatique.PairingRuleSideData(third, should_have=False),
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

assert created.antecedent.subject == second
assert created.consequent.subject == third
assert created.excluded_periods == frozenset({period})
assert created.soft is True
assert side.subject == second

# The rule is named by an id or by a handle, interchangeably — this is the write
# half of the argument convention.
pairings.update(
    created.id,
    collomatique.PairingRuleData(
        collomatique.PairingRuleSideData(third, should_have=False),
        collomatique.PairingRuleSideData(first),
        excluded_periods={period},
        soft=True,
    ),
)
assert created.antecedent.subject == third
assert created.antecedent.should_have is False

# The one refusal this family keeps for the model: an implication about a
# subject that runs no interrogations is vacuous, on either end and for either
# op. It is a statement about the document, not about the value, so it arrives
# as the family's own exception rather than from the value boundary.
for vacuous in (
    lambda: collomatique.PairingRuleData(
        collomatique.PairingRuleSideData(without_colles),
        collomatique.PairingRuleSideData(first),
    ),
    lambda: collomatique.PairingRuleData(
        collomatique.PairingRuleSideData(first),
        collomatique.PairingRuleSideData(without_colles),
    ),
):
    for call, op in (
        (lambda: pairings.add(vacuous()), "AddNewPairingRule"),
        (lambda: pairings.update(created, vacuous()), "UpdatePairingRule"),
    ):
        try:
            call()
        except collomatique.PairingsError as error:
            assert isinstance(error, collomatique.UpdateError)
            assert isinstance(error, collomatique.Error)
            assert str(error)
            assert error.op == op
            assert error.case == "SubjectWithoutInterrogations"
            # The subject the model named, as the id class — the very subject
            # this script is holding.
            assert error.details == (without_colles.id,)
        else:
            raise AssertionError("a rule about a subject without colles is vacuous")

# Nothing of that was written: the rule is what the last accepted write left.
assert len(pairings) == before + 1
assert created.antecedent.subject == third

# This is what rust reads back off the disk.
doc.save(target)

# Removing takes it away and nothing else: the removal of a leaf entity has
# nothing to cascade.
removed = pairings.remove(created.id)
assert isinstance(removed, collomatique.OpResult)
assert removed.warnings == []
assert len(pairings) == before
assert created not in pairings

# The handle is stale now, and so is the side view it handed out.
try:
    created.soft
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("reading a removed pairing rule must raise")

try:
    side.subject
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a side view of a removed rule must raise")

bare = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(first),
    collomatique.PairingRuleSideData(second),
)

# A dead rule is refused by the argument convention, and not by the model: the
# two ops that name one could object to an id the document does not hold, and
# that is caught here, where the message can say which argument was wrong.
for call in (
    lambda: pairings.remove(created),
    lambda: pairings.update(created, bare),
    lambda: pairings.update(created.id, bare),
):
    try:
        call()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a dead pairing rule must not be written through")

# Something that was never a reference to this document at all is a `TypeError`:
# it is not a stale anything.
try:
    pairings.remove(3)
except TypeError:
    pass
else:
    raise AssertionError("a key that is not an id must not resolve")

# `other` is this same file loaded twice, so its subjects carry the very ids
# this document uses — and still name nothing here. The example holds no pairing
# rule of its own, so the foreign one is one `other` makes for itself.
other = collomatique.load(source)
other_subjects = [
    subject for subject in other.subjects if subject.interrogation is not None
]
foreign_rule = other.pairings.add(
    collomatique.PairingRuleData(
        collomatique.PairingRuleSideData(other_subjects[0]),
        collomatique.PairingRuleSideData(other_subjects[1]),
    )
).created
foreign_subject = other_subjects[0]

try:
    pairings.remove(foreign_rule)
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a pairing rule of another document must not resolve")

# A value is refused before any op is built, and nothing is written: a field
# naming another document's entity …
try:
    pairings.add(
        collomatique.PairingRuleData(
            collomatique.PairingRuleSideData(foreign_subject),
            collomatique.PairingRuleSideData(second),
        )
    )
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("a subject of another document names nothing here")
assert len(pairings) == before

# … a field that was never of the right shape, read at its own site inside the
# value …
try:
    pairings.add(
        collomatique.PairingRuleData(
            collomatique.PairingRuleSideData(3),
            collomatique.PairingRuleSideData(second),
        )
    )
except TypeError:
    pass
else:
    raise AssertionError("a subject that is not an id must be refused")
assert len(pairings) == before

# … and the value's own invariant, which is the model's constructor speaking:
# an implication from a subject to itself says nothing.
try:
    pairings.add(
        collomatique.PairingRuleData(
            collomatique.PairingRuleSideData(first),
            collomatique.PairingRuleSideData(first),
        )
    )
except ValueError:
    pass
else:
    raise AssertionError("a rule from a subject to itself must be refused")
assert len(pairings) == before

# A call that is wrong about both names the *rule*: a value meant for nothing is
# moot, so the addressee is resolved first.
try:
    pairings.update(
        created,
        collomatique.PairingRuleData(
            collomatique.PairingRuleSideData(first),
            collomatique.PairingRuleSideData(first),
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
assert len(pairings) == before + 1
assert created in pairings
assert created.antecedent.subject == third
assert side.subject == third

doc.undo()
assert created.antecedent.subject == second
assert created.consequent.subject == third
doc.undo()
assert created.antecedent.subject == first
assert created.consequent.subject == second
assert created.soft is False
assert doc.undo_name == add_label

doc.undo()
assert len(pairings) == before
assert doc.can_undo is False
