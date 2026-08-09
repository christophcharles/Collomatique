import collomatique

# The second half: the rule is gone, so the handle and both its side views go
# with it, loudly.
assert len(doc.slot_pairings) == rule_count_before - 1

for attribute in ("antecedent", "consequent", "excluded_periods", "soft"):
    try:
        getattr(doomed, attribute)
    except collomatique.StaleHandleError as error:
        assert "SlotPairingRule" in str(error)
        assert repr(doomed_id) in str(error)
        assert "is no longer in the document" in str(error)
    else:
        raise AssertionError(f"a stale SlotPairingRule must raise on .{attribute}")

for view in (doomed_antecedent, doomed_consequent):
    for attribute in ("slot", "should_have"):
        try:
            getattr(view, attribute)
        except collomatique.StaleHandleError as error:
            assert "SlotPairingRuleSide" in str(error)
            assert repr(doomed_id) in str(error)
            assert "is no longer in the document" in str(error)
        else:
            raise AssertionError(f"a stale SlotPairingRuleSide must raise on .{attribute}")

# The id, `==` and `hash` never read the state, so they outlive the rule — and
# the two sides stay distinct views of the same dead rule.
assert doomed.id == doomed_id
assert doomed == doomed
assert by_handle[doomed] == "rule"
assert by_handle[doomed_antecedent] == "antecedent"
assert by_handle[doomed_consequent] == "consequent"
assert doomed_antecedent != doomed_consequent
assert doomed_antecedent == doomed_antecedent
assert hash(doomed_antecedent) == hash(doomed_antecedent)
assert len({doomed_antecedent, doomed_consequent}) == 2

# Neither repr raises, and both say so.
assert repr(doomed).startswith("<SlotPairingRule #")
assert repr(doomed).endswith("(stale)>")
assert repr(doomed_antecedent).startswith("<SlotPairingRuleSide #")
assert repr(doomed_antecedent).endswith("(stale)>")

# The mapping conventions, for both an id and a handle that name nothing.
assert doc.slot_pairings.get(doomed_id) is None
assert doc.slot_pairings.get(doomed) is None
assert doomed_id not in doc.slot_pairings
assert doomed not in doc.slot_pairings
try:
    doc.slot_pairings[doomed]
except KeyError:
    pass
else:
    raise AssertionError("a dead slot pairing rule handle must not resolve")

# The survivor reads exactly as before.
assert survivor.soft == survivor_soft
assert survivor.antecedent.should_have == survivor_antecedent_should_have
assert survivor.antecedent.slot.subject.name == survivor_antecedent_slot_subject
