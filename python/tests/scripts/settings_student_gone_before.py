import collomatique

# The first half of a two-stage script: rust removes Hermione between this
# stage and the next — her limits override goes with her. Everything this
# stage leaves in the globals is what the next one asks questions about.
doc = collomatique.load(source)

hermione = [student for student in doc.students if student.surname == "Granger"][0]

resolved = doc.settings.limits_for(hermione)
override_view = doc.settings.override_for(hermione)
assert override_view is not None
assert resolved.interrogations_per_week_max is not None
assert override_view.interrogations_per_week_max is not None
