import collomatique

# The first half of a two-stage script: rust removes Métamorphose's balancing
# override between this stage and the next. Everything this stage leaves in
# the globals is what the next one asks questions about.
doc = collomatique.load(source)

metamorphose = [subject for subject in doc.subjects if subject.name == "Métamorphose"][0]

resolved = doc.balancing.options_for(metamorphose)
override_view = doc.balancing.override_for(metamorphose)
assert override_view is not None
assert resolved.avoid_twice_in_a_row == collomatique.Enforcement.STRICT
assert resolved.year_teacher_rotation is True
assert override_view.avoid_twice_in_a_row == collomatique.Enforcement.STRICT

# The override rows, for the last stage to see shrink by one.
override_count = len(doc.balancing.overrides())
