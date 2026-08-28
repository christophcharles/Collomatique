import dataclasses

import collomatique

# `source` is a throwaway copy of a real colloscope — the example's two slot
# pairing rules: both strict, both excluding no period, a used antecedent
# against an unused consequent. The example carries no subject pairing rule,
# which is the pairing script's own document's job.
doc = collomatique.load(source)

rule_list = list(doc.slot_pairings)
assert len(rule_list) == 2

# What a handle hands back detached, in the collection's order, which is the
# order rust compares them in.
rule_values = [rule.to_data() for rule in rule_list]
assert all(isinstance(d, collomatique.SlotPairingRuleData) for d in rule_values)
assert all(
    isinstance(d.antecedent, collomatique.SlotPairingRuleSideData)
    for d in rule_values
)
assert all(
    isinstance(d.consequent, collomatique.SlotPairingRuleSideData)
    for d in rule_values
)

# The fields as python sees them, so that a conversion wrong in both
# directions at once cannot pass rust's round-trip comparison by cancelling
# itself out.
rule_softs = [d.soft for d in rule_values]
side_should_haves = [
    (d.antecedent.should_have, d.consequent.should_have) for d in rule_values
]
antecedent_slot_start_times = [
    doc.slots[d.antecedent.slot].start_time for d in rule_values
]
consequent_slot_start_times = [
    doc.slots[d.consequent.slot].start_time for d in rule_values
]
rule_subject_names = [
    doc.slots[d.antecedent.slot].subject.name for d in rule_values
]

# The slots come out as ids, never as handles: the value is detached, and a
# handle would carry the document with it. They name live slots of this
# document.
assert all(
    isinstance(d.antecedent.slot, collomatique.SlotId) for d in rule_values
)
assert all(
    isinstance(d.consequent.slot, collomatique.SlotId) for d in rule_values
)
assert all(d.antecedent.slot in doc.slots for d in rule_values)
assert all(d.consequent.slot in doc.slots for d in rule_values)
assert all(len(d.excluded_periods) == 0 for d in rule_values)

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
again.consequent.should_have = not again.consequent.should_have
assert rule_values[0].consequent.should_have != again.consequent.should_have

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
    isinstance(d, collomatique.SlotPairingRuleSideData) for d in side_values
)
assert all(d == rv.antecedent for rv, d in zip(rule_values, side_values[::2]))
assert all(d == rv.consequent for rv, d in zip(rule_values, side_values[1::2]))
assert all(d is not rv.antecedent for rv, d in zip(rule_values, side_values[::2]))

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
scratch = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(3), collomatique.SlotPairingRuleSideData(4)
)
scratch.soft = "beaucoup"
scratch.excluded_periods = None

# And a value has no identity: an id names a place in a document, and a value
# has none. Updating an existing rule will pass the id as the method's
# argument.
assert not hasattr(rule_values[0], "id")

# The field order of each class, which is what a positional call depends on:
# required first, then the defaulted ones in the order the handle shows them.
assert [f.name for f in dataclasses.fields(collomatique.SlotPairingRuleSideData)] == [
    "slot",
    "should_have",
]
assert [f.name for f in dataclasses.fields(collomatique.SlotPairingRuleData)] == [
    "antecedent",
    "consequent",
    "excluded_periods",
    "soft",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import SlotPairingRuleData as _same_class  # noqa: E402

assert _same_class is collomatique.SlotPairingRuleData
assert collomatique.SlotPairingRuleData.__module__ == "collomatique"
assert collomatique.SlotPairingRuleSideData.__module__ == "collomatique"

# A field that names an entity takes a handle or an id, interchangeably. The
# two rules below extract to the example's first rule and — this is the wart —
# do not compare equal, because a dataclass stores what it was given, and a
# handle and an id hash differently.
first_rule = rule_list[0]
ante_slot = first_rule.antecedent.slot
con_slot = first_rule.consequent.slot

by_handle = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(ante_slot),
    collomatique.SlotPairingRuleSideData(con_slot, should_have=False),
)
by_id = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(ante_slot.id),
    collomatique.SlotPairingRuleSideData(con_slot.id, should_have=False),
)
assert by_handle != by_id

# The shape the example does not carry — a soft rule with a period excluded —
# built by hand, so that rust can compare it whole.
soft_with_exclusion = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(ante_slot.id),
    collomatique.SlotPairingRuleSideData(con_slot.id, should_have=False),
    excluded_periods={list(doc.periods)[0].id},
    soft=True,
)

# The defaults: `should_have` is True on each side and `soft` is False, the
# spellings the application itself starts a new rule with. The model has no
# default for the rule to pin them to, so rust re-reads this value whole.
defaults = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(ante_slot.id),
    collomatique.SlotPairingRuleSideData(con_slot.id),
)

# The values the boundary must refuse. They are built without complaint — that
# is the point — and rust extracts each one and reads the message.
not_a_slot = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(3),
    collomatique.SlotPairingRuleSideData(con_slot),
)
not_a_side = collomatique.SlotPairingRuleData(
    "Aurore",
    collomatique.SlotPairingRuleSideData(con_slot),
)
not_a_side_flag = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(ante_slot, should_have=1),
    collomatique.SlotPairingRuleSideData(con_slot),
)

# The sealed-constructor violation, in the model's own words.
same_slot_twice = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(ante_slot),
    collomatique.SlotPairingRuleSideData(ante_slot),
)

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
other_rule = list(other.slot_pairings)[0]
foreign_rule = collomatique.SlotPairingRuleData(
    collomatique.SlotPairingRuleSideData(other_rule.antecedent.slot),
    collomatique.SlotPairingRuleSideData(other_rule.consequent.slot),
)
