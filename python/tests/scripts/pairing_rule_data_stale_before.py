import collomatique

doc = collomatique.load(source)

# Rust removes one pairing rule between this stage and the next — the read
# surface ships no writes of its own.
rule_list = list(doc.pairings)
doomed = rule_list[-1]
living = rule_list[0]
assert doomed != living

# Written down while everything is alive, and read again afterwards.
living_soft = living.soft

# The sides are sub-views, and they go stale with their rule: both ends are
# held here so that `to_data()` through each of them is tried in the second
# half.
doomed_antecedent = doomed.antecedent
doomed_consequent = doomed.consequent

# A value is a detached object: the removal of the rule it describes cannot
# reach it, since nothing in it names the rule.
doomed_value = doomed.to_data()
doomed_soft = doomed_value.soft
