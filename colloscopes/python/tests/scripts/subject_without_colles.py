import collomatique

# The second stage: the subject is still there, but it no longer holds
# interrogations. The handle is untouched; the sub-view is what died.
assert len(doc.subjects) == subject_count_before
assert doomed in doc.subjects
assert doomed.name == doomed_name
assert doomed.index == doomed_index

# Asked again, the subject answers the current truth rather than the view it
# handed out before.
assert doomed.interrogation is None

# The view that was handed out is stale, and says which of the two ways: the
# subject is still in the document, it just stopped holding colles.
for attribute in (
    "students_per_group",
    "groups_per_interrogation",
    "duration",
    "take_duration_into_account",
    "periodicity",
):
    try:
        getattr(doomed_view, attribute)
    except collomatique.StaleHandleError as error:
        assert "Interrogation" in str(error)
        assert repr(doomed_id) in str(error)
        assert "no longer holds interrogations" in str(error)
    else:
        raise AssertionError(f"a switched-off Interrogation must raise on .{attribute}")

# A repr never raises, and says the view is dead.
assert repr(doomed_view).startswith("<Interrogation #")
assert repr(doomed_view).endswith("(périmé)>")

# What still works, because it never reads the state.
assert doomed_view == doomed_view
assert hash(doomed_view) == hash(doomed_view)
assert by_handle[doomed_view] == "interrogation"
assert by_handle[doomed] == "subject"

# The other subject is untouched, view and all.
assert survivor.name == survivor_name
assert survivor.index == 0
assert survivor_view.duration == survivor_duration
assert survivor.interrogation == survivor_view
