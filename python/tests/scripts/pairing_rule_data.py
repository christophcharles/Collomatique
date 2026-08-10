import dataclasses

import collomatique

# `source` is a throwaway copy of a document built for this test: two pairing
# rules, both `should_have` polarities on each side, soft both ways, one rule
# excluding a period and one excluding none. The example has no subject
# pairing rules at all, so it cannot carry this test.
doc = collomatique.load(source)

rule_list = list(doc.pairings)
assert len(rule_list) == 2

# What a handle hands back detached, in the collection's order, which is the
# order rust compares them in.
rule_values = [rule.to_data() for rule in rule_list]
assert all(isinstance(d, collomatique.PairingRuleData) for d in rule_values)

# The two ends are plain values, not leaf values: a `PairingRuleData` nests
# them, so `d.antecedent.should_have = False` is a real mutation of a
# detached builder.
assert all(
    isinstance(d.antecedent, collomatique.PairingRuleSideData) for d in rule_values
)
assert all(
    isinstance(d.consequent, collomatique.PairingRuleSideData) for d in rule_values
)

# The fields as python sees them, so that a conversion wrong in both
# directions at once cannot pass rust's round-trip comparison by cancelling
# itself out.
rule_softs = [d.soft for d in rule_values]
side_should_haves = [
    (d.antecedent.should_have, d.consequent.should_have) for d in rule_values
]
antecedent_subject_names = [doc.subjects[d.antecedent.subject].name for d in rule_values]
consequent_subject_names = [doc.subjects[d.consequent.subject].name for d in rule_values]
excluded_period_indices = [
    sorted(doc.periods[p].index for p in d.excluded_periods) for d in rule_values
]

# The subjects and periods come out as ids, never as handles: the value is
# detached, and a handle would carry the document with it.
assert all(
    isinstance(d.antecedent.subject, collomatique.SubjectId) for d in rule_values
)
assert all(
    isinstance(d.consequent.subject, collomatique.SubjectId) for d in rule_values
)
assert all(
    isinstance(period, collomatique.PeriodId)
    for d in rule_values
    for period in d.excluded_periods
)

# A fresh object every call. Two of them are equal and share nothing, so
# writing to one is invisible to the other and to the document — and a side
# of one is a fresh value too.
first = rule_list[0]
again = first.to_data()
assert again == rule_values[0]
assert again is not rule_values[0]
assert again.antecedent is not rule_values[0].antecedent
again.soft = not again.soft
assert rule_values[0].soft != again.soft
again.antecedent.should_have = not again.antecedent.should_have
assert rule_values[0].antecedent.should_have != again.antecedent.should_have

# A side view detaches on its own too: `rule.antecedent.to_data()` hands back
# the same value the rule's own `to_data()` nests inside it, as a fresh
# object. The standalone side boundary is a reader of its own, so rust reads
# these back whole.
side_values = [
    end
    for rule in rule_list
    for end in (rule.antecedent.to_data(), rule.consequent.to_data())
]
assert all(
    isinstance(d, collomatique.PairingRuleSideData) for d in side_values
)
assert all(d == rv.antecedent for rv, d in zip(rule_values, side_values[::2]))
assert all(d == rv.consequent for rv, d in zip(rule_values, side_values[1::2]))
assert all(d is not rv.antecedent for rv, d in zip(rule_values, side_values[::2]))

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
scratch = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(3), collomatique.PairingRuleSideData(4)
)
scratch.soft = "beaucoup"
scratch.excluded_periods = None

# And a value has no identity: an id names a place in a document, and a value
# has none. Updating an existing rule will pass the id as the method's
# argument.
assert not hasattr(rule_values[0], "id")

# The field order of each class, which is what a positional call depends on:
# required first, then the defaulted ones in the order the handle shows them.
assert [f.name for f in dataclasses.fields(collomatique.PairingRuleSideData)] == [
    "subject",
    "should_have",
]
assert [f.name for f in dataclasses.fields(collomatique.PairingRuleData)] == [
    "antecedent",
    "consequent",
    "excluded_periods",
    "soft",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import PairingRuleData as _same_class  # noqa: E402

assert _same_class is collomatique.PairingRuleData
assert collomatique.PairingRuleData.__module__ == "collomatique"
assert collomatique.PairingRuleSideData.__module__ == "collomatique"

# A field that names an entity takes a handle or an id, interchangeably. The
# two rules below extract to the same rule and — this is the wart §2.3 of the
# design records — do not compare equal, because a dataclass stores what it
# was given, and a handle and an id hash differently.
sortileges, metamorphose = list(doc.subjects)

by_handle = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges),
    collomatique.PairingRuleSideData(metamorphose, should_have=False),
)
by_id = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges.id),
    collomatique.PairingRuleSideData(metamorphose.id, should_have=False),
)
assert by_handle != by_id

# The soft rule with a period excluded, named entirely by id — reproducing
# the fixture's first rule, so that rust can compare it whole.
period_list = list(doc.periods)
soft_by_id = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges.id),
    collomatique.PairingRuleSideData(metamorphose.id, should_have=False),
    excluded_periods={period_list[1].id},
    soft=True,
)

# The defaults: `should_have` is True on each side and `soft` is False, the
# spellings the application itself starts a new rule with. The model has no
# default for the rule to pin them to, so rust re-reads this value whole.
defaults = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges.id),
    collomatique.PairingRuleSideData(metamorphose.id),
)

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
not_a_subject = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(3),
    collomatique.PairingRuleSideData(metamorphose),
)
not_a_side = collomatique.PairingRuleData(
    "Aurore",
    collomatique.PairingRuleSideData(metamorphose),
)
not_a_side_flag = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges, should_have=1),
    collomatique.PairingRuleSideData(metamorphose),
)
not_a_rule_flag = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges),
    collomatique.PairingRuleSideData(metamorphose),
    soft=1,
)
not_a_periods_set = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges),
    collomatique.PairingRuleSideData(metamorphose),
    excluded_periods=3,
)

# The sealed-constructor violation, in the model's own words.
same_subject_twice = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(sortileges),
    collomatique.PairingRuleSideData(sortileges),
)

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_rule = collomatique.PairingRuleData(
    collomatique.PairingRuleSideData(list(other.subjects)[0]),
    collomatique.PairingRuleSideData(list(other.subjects)[1]),
)
