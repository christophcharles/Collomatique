import collomatique

# The first half of a two-stage script. The write surface publishes no
# cascading op yet, so rust deletes a subject between this stage and the next
# and leaves the `OpResult` it answered in the globals. What this stage keeps is
# what the next one reads those warnings against — and it has to be kept *now*,
# because after the write none of it is in the document any more.
doc = collomatique.load(source)

# Named by its place in the user order: rust cannot hand a script an id, so the
# script reads its own off the document, as every other stage does.
doomed = list(doc.subjects)[doomed_index]
doomed_id = doomed.id

# The slots that go with the subject, and the teachers that stop interrogating
# in it: two repairs the cascade must report, named the way this script names
# them.
doomed_slot_ids = {slot.id for slot in doomed.slots}
doomed_teacher_ids = {
    teacher.id for teacher in doc.teachers if doomed_id in teacher.to_data().subjects
}
