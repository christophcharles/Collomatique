import collomatique

# The first half of a two-stage script: rust removes the entities these handles
# name between this stage and the next, because the read surface ships no
# removes of its own yet. Everything this stage leaves in the globals is what
# the next one asks questions about.
doc = collomatique.load(source)

subject = list(doc.subjects)[-1]
incompat = list(doc.incompats)[0]
pairing_rule = list(doc.pairings)[0]
slot_pairing_rule = list(doc.slot_pairings)[0]

# The four handles are about to become stale; the next stage asks each what
# points at it.
