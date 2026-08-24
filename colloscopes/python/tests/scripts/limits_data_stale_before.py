import collomatique

# The first of three stages. Harry has no override in the file; rust installs
# a partial one — a minimum of four interrogations a week, objective rather
# than strict, and the two other fields unset — between this stage and the
# next. The `None` fields must *disable* the corresponding global limits, not
# inherit them: the verbatim whole-entry rule the model's own tests pin.
doc = collomatique.load(source)

harry = [student for student in doc.students if student.surname == "Potter"][0]
assert doc.settings.override_for(harry) is None
