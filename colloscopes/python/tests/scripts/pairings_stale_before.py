import collomatique

# The first half of a two-stage script: rust removes one pairing rule between
# this stage and the next, because the read surface ships no removes of its own
# yet. Everything this stage leaves in the globals is what the next one asks
# questions about.
doc = collomatique.load(source)

rule_count_before = len(doc.pairings)

# The fixture's first rule in id order is the soft one, excluding the second
# period.
doomed, survivor = list(doc.pairings)
doomed_id = doomed.id
doomed_antecedent = doomed.antecedent
doomed_consequent = doomed.consequent

assert doomed.excluded_periods == frozenset({list(doc.periods)[1]})
assert len(doomed.excluded_periods) == 1

# The survivor, for the next stage to compare against.
survivor_soft = survivor.soft
survivor_antecedent_should_have = survivor.antecedent.should_have
survivor_antecedent_name = survivor.antecedent.subject.name

# Handles and views as dict keys: this is the thing that must not blow up when
# the entity they name dies.
by_handle = {
    doomed: "rule",
    doomed_antecedent: "antecedent",
    doomed_consequent: "consequent",
}
