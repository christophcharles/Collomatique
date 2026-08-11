import dataclasses

import collomatique

# `source` is a copy of the hogwarts example: global balancing options that
# pursue teacher and slot rotation as objectives, plus one per-subject override
# (Métamorphose's) that hardens a rotation the global entry does not pursue at
# all. The content is compared against the model on the rust side; what this
# script pins is the api's own shape.
doc = collomatique.load(source)

# What the views hand back detached, as the values a script builds.
global_value = doc.balancing.global_options.to_data()
assert isinstance(global_value, collomatique.BalancingData)

# The five fields come out whole: an `Enforcement` for each pursued goal,
# `None` for each goal the entry does not pursue, and the two switches. The
# global entry pursues teacher and slot rotation as objectives, and does not
# pursue the rest — `avoid_twice_in_a_row` is the `None` field, the one that
# must stay `None` across a round trip.
assert global_value.teacher_rotation == collomatique.Enforcement.OBJECTIVE
assert global_value.slot_rotation == collomatique.Enforcement.OBJECTIVE
assert global_value.avoid_twice_in_a_row is None
assert global_value.year_teacher_rotation is False
assert global_value.period_teacher_rotation is False

# Métamorphose's override wins verbatim: it hardens the rotation the global
# entry does not pursue at all, and turns the year switch on. The whole raw
# entry comes back, whatever it holds.
metamorphose = [subject for subject in doc.subjects if subject.name == "Métamorphose"][0]
metamorphose_value = doc.balancing.override_for(metamorphose).to_data()
assert isinstance(metamorphose_value, collomatique.BalancingData)
assert metamorphose_value.teacher_rotation == collomatique.Enforcement.OBJECTIVE
assert metamorphose_value.avoid_twice_in_a_row == collomatique.Enforcement.STRICT
assert metamorphose_value.year_teacher_rotation is True

# A subject without an override resolves to the global entry, and `to_data()`
# hands back what the view reads — the resolved entry.
arithmancie = [subject for subject in doc.subjects if subject.name == "Arithmancie"][0]
arithmancie_value = doc.balancing.options_for(arithmancie).to_data()
assert arithmancie_value == global_value

# A fresh object every call. Two of them are equal and share nothing, so
# writing to one is invisible to the other and to the document.
again = doc.balancing.global_options.to_data()
assert again == global_value
assert again is not global_value
again.teacher_rotation = None
assert global_value.teacher_rotation is not None

# A value has no identity: an id names a place in a document, and a value has
# none. Updating an entry will pass the id as the method's argument.
assert not hasattr(global_value, "id")

# The field order, which is what a positional call depends on. Everything is
# defaulted here.
assert [f.name for f in dataclasses.fields(collomatique.BalancingData)] == [
    "teacher_rotation",
    "slot_rotation",
    "avoid_twice_in_a_row",
    "year_teacher_rotation",
    "period_teacher_rotation",
]

# The class is the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import BalancingData as _same_class  # noqa: E402

assert _same_class is collomatique.BalancingData
assert collomatique.BalancingData.__module__ == "collomatique"

# The defaults: teacher rotation pursued as an objective, and nothing else —
# the model's own default. Rust pins it against
# `BalancingOptions::default()`.
defaults = collomatique.BalancingData()

# Built by hand, a partial entry: one goal set, one goal hardened, one goal
# left at its `None` default — which is the point, it must stay *not pursued*
# across the extraction — and the year switch on. Rust compares it whole.
hand_built = collomatique.BalancingData(
    teacher_rotation=collomatique.Enforcement.OBJECTIVE,
    avoid_twice_in_a_row=collomatique.Enforcement.STRICT,
    year_teacher_rotation=True,
)
assert hand_built.slot_rotation is None

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. All of these are answered for when the value is used, not here.
not_an_enforcement = collomatique.BalancingData(teacher_rotation=3)
not_a_goal = collomatique.BalancingData(slot_rotation="OBJECTIVE")
not_a_switch = collomatique.BalancingData(year_teacher_rotation=1)
