import collomatique

# The third stage: the subject itself is gone now, so the handle goes the way
# its sub-view already did.
assert len(doc.subjects) == subject_count_before - 1

for attribute in ("index", "name", "interrogation", "excluded_periods", "slots"):
    try:
        getattr(doomed, attribute)
    except collomatique.StaleHandleError as error:
        assert "Subject" in str(error)
        assert repr(doomed_id) in str(error)
    else:
        raise AssertionError(f"a stale Subject must raise on .{attribute}")

# The view says the other of its two things now: what it was viewing is gone
# because the subject is, not because the colles were switched off.
try:
    doomed_view.duration
except collomatique.StaleHandleError as error:
    assert "Interrogation" in str(error)
    assert repr(doomed_id) in str(error)
    assert "no longer in the document" in str(error)
else:
    raise AssertionError("a stale Interrogation must raise on .duration")

# Neither repr raises, and both say so.
assert repr(doomed).startswith("<Subject #")
assert repr(doomed).endswith("(périmé)>")
assert repr(doomed_view).endswith("(périmé)>")

# The id, `==` and `hash` never read the state, so they outlive the subject.
assert doomed.id == doomed_id
assert doomed == doomed
assert by_handle[doomed] == "subject"
assert by_handle[doomed_view] == "interrogation"

# The mapping conventions, for both an id and a handle that name nothing.
assert doc.subjects.get(doomed_id) is None
assert doc.subjects.get(doomed) is None
assert doomed_id not in doc.subjects
assert doomed not in doc.subjects
try:
    doc.subjects[doomed]
except KeyError:
    pass
else:
    raise AssertionError("a dead subject handle must not resolve")

# The survivor kept its place, since what went was after it in the list.
assert survivor.name == survivor_name
assert survivor.index == 0
assert survivor.interrogation.duration == survivor_duration
