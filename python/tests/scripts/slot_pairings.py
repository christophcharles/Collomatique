import collomatique

# `source` is a throwaway copy of a real colloscope — the example's two slot
# pairing rules: both strict, both excluding no period, a used antecedent
# against an unused consequent.
doc = collomatique.load(source)

slot_pairings = doc.slot_pairings
assert isinstance(slot_pairings, collomatique.SlotPairings)
assert repr(slot_pairings) == "<collomatique.SlotPairings count=%d>" % len(slot_pairings)

rule_list = list(slot_pairings)
assert len(rule_list) == len(slot_pairings)
assert all(isinstance(rule, collomatique.SlotPairingRule) for rule in rule_list)
assert all(isinstance(rule.antecedent, collomatique.SlotPairingRuleSide) for rule in rule_list)
assert all(isinstance(rule.consequent, collomatique.SlotPairingRuleSide) for rule in rule_list)

# The two ends are live sub-views reading this document: their slots are this
# document's slots, and both slots of one rule sit on one subject.
assert all(rule.antecedent.slot in doc.slots for rule in rule_list)
assert all(rule.consequent.slot in doc.slots for rule in rule_list)
assert all(
    rule.antecedent.slot.subject == rule.consequent.slot.subject
    for rule in rule_list
)

# The example's shape: strict rules on every week, a used antecedent against
# an unused consequent.
assert all(rule.antecedent.should_have for rule in rule_list)
assert all(not rule.consequent.should_have for rule in rule_list)
assert all(not rule.soft for rule in rule_list)
assert all(len(rule.excluded_periods) == 0 for rule in rule_list)
assert all(
    isinstance(period, collomatique.Period)
    for rule in rule_list
    for period in rule.excluded_periods
)

# What the script leaves for rust to compare with the model.
antecedent_should_have = [rule.antecedent.should_have for rule in rule_list]
consequent_should_have = [rule.consequent.should_have for rule in rule_list]
softs = [rule.soft for rule in rule_list]
antecedent_slot_start_times = [rule.antecedent.slot.start_time for rule in rule_list]
rule_subject_names = [rule.antecedent.slot.subject.name for rule in rule_list]

# Indexing takes an id or a handle, and hands back an equal handle either way.
for rule in rule_list:
    assert slot_pairings[rule.id] == rule
    assert slot_pairings[rule] == rule
    assert slot_pairings.get(rule.id) == rule
    assert rule.id in slot_pairings
    assert rule in slot_pairings

# The side is part of a side view's identity: the two ends of one rule differ,
# and a re-read of the same end is equal.
for rule in rule_list:
    assert rule.antecedent != rule.consequent
    assert slot_pairings[rule.id].antecedent == rule.antecedent
    assert len({rule.antecedent, rule.consequent}) == 2
assert len(set(rule_list)) == len(rule_list)

# Handles identify; they do not order, which is what ids are for.
assert rule_list[0] != 3
assert rule_list[0] != "Physique"
try:
    rule_list[0] < rule_list[1]
except TypeError:
    pass
else:
    raise AssertionError("ordering two handles must raise")

# A handle is something the document hands out, and it has no setters.
try:
    collomatique.SlotPairingRule()
except TypeError:
    pass
else:
    raise AssertionError("a handle must not be constructible")
try:
    rule_list[0].soft = True
except AttributeError:
    pass
else:
    raise AssertionError("assigning to a handle attribute must raise")

# A handle from another document names nothing here, whatever its id says.
# `other` is this same file loaded twice, so its rules carry the very ids this
# document uses.
other = collomatique.load(source)
other_rule = list(other.slot_pairings)[0]
assert other_rule not in slot_pairings
assert slot_pairings.get(other_rule) is None
assert other_rule.id in slot_pairings
assert other.slot_pairings[other_rule.id] == other_rule
try:
    slot_pairings[other_rule]
except KeyError:
    pass
else:
    raise AssertionError("a handle of another document must not resolve")

# The reprs name the rules the way the application does — the notation of
# `ops::rendering`, which the rust half pins exactly.
first_repr = repr(rule_list[0])
first_side_repr = repr(rule_list[0].antecedent)
assert first_repr.startswith("<SlotPairingRule #")
assert "⟹" in first_repr
assert first_side_repr.startswith("<SlotPairingRuleSide #")
assert "(antécédent)" in first_side_repr
