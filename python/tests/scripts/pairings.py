import collomatique

# `source` is a throwaway copy of a document built for this test: two pairing
# rules, both `should_have` polarities on each side, soft both ways, one rule
# excluding a period and one excluding none.
doc = collomatique.load(source)

pairings = doc.pairings
assert isinstance(pairings, collomatique.Pairings)
assert repr(pairings) == "<collomatique.Pairings count=%d>" % len(pairings)

rule_list = list(pairings)
assert len(rule_list) == len(pairings)
assert all(isinstance(rule, collomatique.PairingRule) for rule in rule_list)
assert all(isinstance(rule.antecedent, collomatique.PairingRuleSide) for rule in rule_list)
assert all(isinstance(rule.consequent, collomatique.PairingRuleSide) for rule in rule_list)

# The two ends are live sub-views reading this document: their subjects are
# this document's subjects.
assert all(rule.antecedent.subject in doc.subjects for rule in rule_list)
assert all(rule.consequent.subject in doc.subjects for rule in rule_list)

# The fixture's shape: both polarities on each side, soft both ways, one rule
# excluding a period and one excluding none.
assert {rule.antecedent.should_have for rule in rule_list} == {True, False}
assert {rule.consequent.should_have for rule in rule_list} == {True, False}
assert {rule.soft for rule in rule_list} == {True, False}
assert sorted(len(rule.excluded_periods) for rule in rule_list) == [0, 1]
assert all(
    isinstance(period, collomatique.Period)
    for rule in rule_list
    for period in rule.excluded_periods
)
assert all(
    period in doc.periods
    for rule in rule_list
    for period in rule.excluded_periods
)

# What the script leaves for rust to compare with the model.
antecedent_subject_names = [rule.antecedent.subject.name for rule in rule_list]
consequent_subject_names = [rule.consequent.subject.name for rule in rule_list]
antecedent_should_have = [rule.antecedent.should_have for rule in rule_list]
consequent_should_have = [rule.consequent.should_have for rule in rule_list]
softs = [rule.soft for rule in rule_list]
excluded_period_indices = [
    sorted(period.index for period in rule.excluded_periods)
    for rule in rule_list
]

# Indexing takes an id or a handle, and hands back an equal handle either way.
for rule in rule_list:
    assert pairings[rule.id] == rule
    assert pairings[rule] == rule
    assert pairings.get(rule.id) == rule
    assert rule.id in pairings
    assert rule in pairings

# The side is part of a side view's identity: the two ends of one rule differ,
# and a re-read of the same end is equal.
for rule in rule_list:
    assert rule.antecedent != rule.consequent
    assert pairings[rule.id].antecedent == rule.antecedent
    assert len({rule.antecedent, rule.consequent}) == 2
assert len(set(rule_list)) == len(rule_list)

# A handle is a view, not the object the collection keeps: two of them for the
# same rule are different objects that compare and hash the same.
again = pairings[rule_list[0].id]
assert again is not rule_list[0]
assert hash(again) == hash(rule_list[0])
assert len({again, rule_list[0]}) == 1

# Handles identify; they do not order, which is what ids are for.
assert rule_list[0] != 3
assert rule_list[0] != "Avoir Sortilèges"
try:
    rule_list[0] < rule_list[1]
except TypeError:
    pass
else:
    raise AssertionError("ordering two handles must raise")

# A handle is something the document hands out, and it has no setters.
try:
    collomatique.PairingRule()
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
other_rule = list(other.pairings)[0]
assert other_rule not in pairings
assert pairings.get(other_rule) is None
assert other_rule.id in pairings
assert other.pairings[other_rule.id] == other_rule
try:
    pairings[other_rule]
except KeyError:
    pass
else:
    raise AssertionError("a handle of another document must not resolve")

# The reprs name the rules the way the application does — the notation of
# `ops::rendering`, which the rust half pins exactly.
first_repr = repr(rule_list[0])
first_side_repr = repr(rule_list[0].antecedent)
assert first_repr.startswith("<PairingRule #")
assert "⟹" in first_repr
assert first_side_repr.startswith("<PairingRuleSide #")
assert "(antécédent)" in first_side_repr
