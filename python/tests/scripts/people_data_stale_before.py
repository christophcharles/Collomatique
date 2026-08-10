import collomatique

doc = collomatique.load(source)

# `doomed_subject_index` and `doomed_teacher_index` are handed in by rust, which
# picks a subject at least one teacher interrogates in and a teacher who does
# not — so every teacher of that subject survives the removal and can be looked
# at afterwards.
doomed_subject = list(doc.subjects)[doomed_subject_index]
doomed_teacher = list(doc.teachers)[doomed_teacher_index]

survivor = next(
    teacher
    for teacher in doc.teachers
    if doomed_subject in teacher.subjects and teacher != doomed_teacher
)

# Written down while everything is alive, and read afterwards.
survivor_subjects_before = sorted(
    subject.index for subject in survivor.subjects
)

# Two values naming the subject that is about to die, one by handle and one by
# id, and one naming a subject that will survive.
naming_the_dead_by_handle = collomatique.TeacherData(
    "Emmy", "Noether", subjects={doomed_subject}
)
naming_the_dead_by_id = collomatique.TeacherData(
    "Emmy", "Noether", subjects={doomed_subject.id}
)
living_subject = next(
    subject for subject in doc.subjects if subject != doomed_subject
)
naming_the_living = collomatique.TeacherData(
    "Emmy", "Noether", subjects={living_subject}
)
