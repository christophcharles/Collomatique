import collomatique

# The second half: the entities the previous stage left here are gone, and a
# stale handle raises on `referenced_by` like on every other read — including
# for the three kinds whose alive answer is always `()`.
for handle in (subject, incompat, pairing_rule, slot_pairing_rule):
    try:
        handle.referenced_by()
    except collomatique.StaleHandleError:
        pass
    else:
        raise AssertionError("a stale handle's referenced_by must raise")
