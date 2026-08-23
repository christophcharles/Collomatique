import collomatique

# The second half: `result` is what deleting the subject answered, and rust ran
# that write between the two stages because the python write surface publishes
# no cascading op yet.

warnings = result.warnings
assert warnings, "removing a subject with slots repairs something"

# Two reads of the same getter hand back the same warnings, which is what makes
# the equality assertions at the bottom say something.
again = result.warnings


def position(target):
    """Where `target` sits in `warnings`, by identity rather than by equality."""
    for index, warning in enumerate(warnings):
        if warning is target:
            return index
    raise AssertionError("a parent is one of the warnings of the same list")


# Every repair says the same four things.
for w in warnings:
    assert isinstance(w, collomatique.Warning)
    assert str(w)  # the french sentence, rendered against the pre-state
    assert isinstance(w.kind, str)  # the model's own name for the repair
    assert isinstance(w.details, dict)  # its coordinates, keyed by field name
    # The whole-value argument the elementary op needed is not a coordinate.
    assert "rebuilt" not in w.details

# The coordinates are ids, and they are the very ids this script is holding.
slots_gone = {w.details["slot"] for w in warnings if w.kind == "DeleteSlot"}
assert slots_gone == doomed_slot_ids
assert all(isinstance(slot_id, collomatique.SlotId) for slot_id in slots_gone)

# A repair that names two entities names both, under the model's field names.
freed = [w for w in warnings if w.kind == "RemoveTeacherSubject"]
assert {w.details["teacher"] for w in freed} == doomed_teacher_ids
assert all(w.details["subject"] == doomed_id for w in freed)

# The list is a tree. A repair the write asked for directly has no parent …
assert any(w.parent is None for w in warnings)

# … and one the cascade needed for another names it — the object itself, not a
# copy of it, and always further down the list, since a repair lands before the
# one that needed it.
children = [(i, w) for i, w in enumerate(warnings) if w.parent is not None]
assert children, "deleting these slots takes a slot pairing rule with them"
for i, w in children:
    assert isinstance(w.parent, collomatique.Warning)
    assert position(w.parent) > i

# The one child this document guarantees: the rule that related two of the
# doomed subject's slots went because one of those slots went.
rule_gone = [w for w in warnings if w.kind == "DeleteSlotPairingRule"]
assert len(rule_gone) == 1
assert rule_gone[0].parent.kind == "DeleteSlot"
assert rule_gone[0].parent.details["slot"] in doomed_slot_ids

# Equality is about the repair, not about identity …
assert warnings[0] == again[0]
assert warnings[0] != warnings[1]
assert warnings[0] != "DeleteSlot"
# … and a warning carries a dict, so it does not hash, exactly like one.
try:
    hash(warnings[0])
except TypeError:
    pass
else:
    raise AssertionError("a warning carries a dict and must not hash")
