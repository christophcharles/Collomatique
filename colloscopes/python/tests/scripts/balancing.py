import collomatique

# `source` is a copy of the hogwarts example: global balancing options that
# pursue teacher and slot rotation as objectives, plus one per-subject override
# (Métamorphose's) that hardens a rotation the global entry does not pursue at
# all. The numeric content is compared against the model on the rust side;
# what this script pins is the api's own structure.
doc = collomatique.load(source)

balancing = doc.balancing
assert isinstance(balancing, collomatique.Balancing)
assert repr(balancing) == "<collomatique.Balancing overrides=%d>" % len(balancing.overrides())

global_options = balancing.global_options
assert isinstance(global_options, collomatique.BalancingOptions)
assert global_options.teacher_rotation == collomatique.Enforcement.OBJECTIVE
assert global_options.slot_rotation == collomatique.Enforcement.OBJECTIVE
assert global_options.avoid_twice_in_a_row is None
assert global_options.year_teacher_rotation is False
assert global_options.period_teacher_rotation is False

# Métamorphose's override wins verbatim: where the global entry does not
# pursue a goal at all, the override's strict one applies.
metamorphose = [subject for subject in doc.subjects if subject.name == "Métamorphose"][0]
potions = [subject for subject in doc.subjects if subject.name == "Potions"][0]

metamorphose_options = balancing.options_for(metamorphose)
metamorphose_override = balancing.override_for(metamorphose)
assert metamorphose_override is not None
assert metamorphose_options.avoid_twice_in_a_row == collomatique.Enforcement.STRICT
assert metamorphose_options.year_teacher_rotation is True
assert metamorphose_options == balancing.options_for(metamorphose.id)
assert metamorphose_options != metamorphose_override  # same reading, different binding
assert metamorphose_override == balancing.override_for(metamorphose.id)
assert metamorphose_override.teacher_rotation == collomatique.Enforcement.OBJECTIVE
assert metamorphose_override.avoid_twice_in_a_row == collomatique.Enforcement.STRICT
assert metamorphose_override.year_teacher_rotation is True

# The three states of a rotation goal all appear across the document: not
# pursued, objective, strict.
assert global_options.avoid_twice_in_a_row is None
assert {global_options.avoid_twice_in_a_row, metamorphose_options.avoid_twice_in_a_row} == {
    None,
    collomatique.Enforcement.STRICT,
}

# Potions has an override of its own; a subject without one inherits the
# global entry — the resolution is the model's, not a merge.
assert balancing.override_for(potions) is not None
arithmancie = [subject for subject in doc.subjects if subject.name == "Arithmancie"][0]
assert balancing.override_for(arithmancie) is None
defense_options = balancing.options_for(arithmancie)
assert defense_options.teacher_rotation == global_options.teacher_rotation
assert defense_options != global_options  # same reading, different binding

# The stored overrides come back as live `(Subject, BalancingOptions)` pairs,
# in id order, the view bound to the entry it describes.
rows = balancing.overrides()
assert all(isinstance(subject, collomatique.Subject) for subject, _ in rows)
assert all(isinstance(options, collomatique.BalancingOptions) for _, options in rows)
metamorphose_row = [options for subject, options in rows if subject.name == "Métamorphose"]
assert len(metamorphose_row) == 1
assert metamorphose_row[0] == metamorphose_override

# What the rust half compares against the same document read from the model.
global_rotation_objectives = [global_options.teacher_rotation == collomatique.Enforcement.OBJECTIVE,
                              global_options.slot_rotation == collomatique.Enforcement.OBJECTIVE]
global_bools = [global_options.year_teacher_rotation, global_options.period_teacher_rotation]
metamorphose_objectives = [metamorphose_options.teacher_rotation == collomatique.Enforcement.OBJECTIVE,
                           metamorphose_options.slot_rotation == collomatique.Enforcement.OBJECTIVE,
                           metamorphose_options.avoid_twice_in_a_row == collomatique.Enforcement.OBJECTIVE]
metamorphose_bools = [metamorphose_options.year_teacher_rotation,
                      metamorphose_options.period_teacher_rotation]
override_subject_names = [subject.name for subject, _ in rows]
global_repr = repr(global_options)
metamorphose_repr = repr(metamorphose_options)
