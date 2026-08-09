import collomatique

# The subject of the last stored row was removed between the stages, and the
# cascade took its rows with it. Reading one of them again is an argument, not
# a lookup: the address names a subject this document no longer holds, so it
# is StaleHandleError — the empty-frozenset answer is only for an address
# that was always valid.
try:
    doc.assignments[doomed_period, doomed_subject]
except collomatique.StaleHandleError as error:
    assert "Subject" in str(error)
    assert "is not in this document" in str(error)
else:
    raise AssertionError("an address whose subject was removed must raise")

# The stale handle the address named says so: `.id`, `==` and `hash` never
# touch the state and keep working, and the repr names the death instead of
# raising.
assert "(stale)" in repr(doomed_subject)
assert doomed_subject == doomed_subject
assert hash(doomed_subject) == hash(doomed_subject)

# The survivors read exactly as before the removal.
assert doc.assignments[survivor_period, survivor_subject] == survivor_students

# The walk shows no row of the dead subject, and exactly the rows it used to
# minus that subject's own — only the subject went, nothing else.
remaining = list(doc.assignments)
assert all(subject != doomed_subject for _period, subject, _students in remaining)
assert len(remaining) == sum(
    1 for _period, subject, _students in rows if subject != doomed_subject
)
