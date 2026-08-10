# `to_data()` is a read, so a dead handle refuses it the way every other read
# does — it does not hand back a value describing a teacher who is gone.
try:
    doomed_teacher.to_data()
except collomatique.StaleHandleError:
    pass
else:
    raise AssertionError("to_data() through a stale handle must raise")

# The survivor is untouched, and its value is the teacher as it is *now*: the
# subject that was removed is no longer in it, because `to_data()` reads through
# the handle rather than remembering what it saw.
survivor_data = survivor.to_data()
survivor_subjects_after = sorted(
    subject.index for subject in survivor.subjects
)
assert len(survivor_subjects_after) == len(survivor_subjects_before) - 1
assert len(survivor_data.subjects) == len(survivor_subjects_after)

# The values built before the removal are untouched objects — a dataclass knows
# nothing about a document, and nothing reached in to edit them. What has
# changed is that two of them no longer name anything, which is rust's half of
# this test.
assert len(naming_the_dead_by_handle.subjects) == 1
