import collomatique

# `source` is the fixture holding every shape of a person's card: both contact
# details, one, the other, neither — and exclusion sets running from empty to
# whole.
doc = collomatique.load(source)

teacher_values = [teacher.to_data() for teacher in doc.teachers]
student_values = [student.to_data() for student in doc.students]

value_tels = [d.tel for d in teacher_values]
value_emails = [d.email for d in teacher_values]

# Every shape is here, which is what makes this document worth reading.
assert any(d.tel is None and d.email is None for d in teacher_values)
assert any(d.tel is not None and d.email is not None for d in teacher_values)

# A set is a set, empty included, and it holds ids.
subject_counts = [len(d.subjects) for d in teacher_values]
excluded_counts = [len(d.excluded_periods) for d in student_values]
assert 0 in subject_counts
assert 0 in excluded_counts
assert max(excluded_counts) > 1
assert all(
    isinstance(period, collomatique.PeriodId)
    for d in student_values
    for period in d.excluded_periods
)

# A value's containers are the mutable ones: that is the whole point of a value,
# and the read surface's frozensets are a handle rule. This is done on a value
# of its own rather than on one of the two lists above, which rust is about to
# read back.
scratch = list(doc.students)[0].to_data()
scratch.excluded_periods.add(list(doc.periods)[0].id)
