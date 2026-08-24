import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

# The first and the last stored row, in the model's key order: the last row's
# subject is the one the test removes between the stages, and the first row's
# subject survives it — so the second stage can tell a dead address from a
# live one.
rows = list(doc.assignments)
assert len(rows) > 1
survivor_period, survivor_subject, survivor_students = rows[0]
doomed_period, doomed_subject, doomed_students = rows[-1]
assert doomed_subject != survivor_subject

# Both rows read now, and the doomed one is non-empty — the cascade has
# something to clean up.
assert doc.assignments[doomed_period, doomed_subject] == doomed_students
assert len(doomed_students) > 0
assert doc.assignments[survivor_period, survivor_subject] == survivor_students
