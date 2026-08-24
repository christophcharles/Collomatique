import collomatique

# The first half of a two-stage script: rust removes the example's second slot
# pairing rule between this stage and the next, because the read surface ships
# no removes of its own yet. Everything this stage leaves in the globals is
# what the next one asks questions about.
doc = collomatique.load(source)

rule_count_before = len(doc.slot_pairings)

# In id order, the survivor is the first rule and the doomed one the second.
survivor, doomed = list(doc.slot_pairings)
doomed_id = doomed.id
doomed_antecedent = doomed.antecedent
doomed_consequent = doomed.consequent

assert len(doomed.excluded_periods) == 0

# The survivor, for the next stage to compare against.
survivor_soft = survivor.soft
survivor_antecedent_should_have = survivor.antecedent.should_have
survivor_antecedent_slot_subject = survivor.antecedent.slot.subject.name

# Handles and views as dict keys: this is the thing that must not blow up when
# the entity they name dies.
by_handle = {
    doomed: "rule",
    doomed_antecedent: "antecedent",
    doomed_consequent: "consequent",
}
