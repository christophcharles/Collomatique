import dataclasses

import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

teacher_list = list(doc.teachers)
student_list = list(doc.students)

# What a handle hands back detached. One per person, in the collection's order,
# which is the order rust compares them in.
teacher_values = [teacher.to_data() for teacher in teacher_list]
student_values = [student.to_data() for student in student_list]

assert all(isinstance(d, collomatique.TeacherData) for d in teacher_values)
assert all(isinstance(d, collomatique.StudentData) for d in student_values)

# The fields as python sees them, so that a conversion wrong in both directions
# at once — a tel and an email swapped, say — cannot pass rust's round-trip
# comparison by cancelling itself out.
value_firstnames = [d.firstname for d in teacher_values]
value_surnames = [d.surname for d in teacher_values]
value_tels = [d.tel for d in teacher_values]
value_emails = [d.email for d in teacher_values]
student_value_tels = [d.tel for d in student_values]

# A value holds ids, never handles: it is detached, and a handle would carry the
# document with it.
assert all(
    isinstance(subject, collomatique.SubjectId)
    for d in teacher_values
    for subject in d.subjects
)
assert all(isinstance(d.subjects, set) for d in teacher_values)
assert all(isinstance(d.excluded_periods, set) for d in student_values)

# A fresh object every call. Two of them are equal and share nothing, so writing
# to one is invisible to the other and to the document.
first = teacher_list[0]
again = first.to_data()
assert again == teacher_values[0]
assert again is not teacher_values[0]
again.surname = "Rogue-Prince"
assert teacher_values[0].surname != again.surname
assert first.surname != again.surname

# A value is dumb: no `__post_init__`, no property setters, nothing refused at
# birth. Both of these are answered for when the value is used, not here.
scratch = collomatique.TeacherData("", "")
scratch.tel = ""
scratch.email = ""

# And it has no identity: an id names a place in a document, and a value has
# none. Updating an existing teacher will pass the id as the method's argument.
assert not hasattr(teacher_values[0], "id")

# The field order of the class, which is what a positional call depends on.
assert [f.name for f in dataclasses.fields(collomatique.TeacherData)] == [
    "firstname",
    "surname",
    "tel",
    "email",
    "subjects",
]
assert [f.name for f in dataclasses.fields(collomatique.StudentData)] == [
    "firstname",
    "surname",
    "tel",
    "email",
    "excluded_periods",
]

# Firstname first, as §14 of the design writes it and as every screen of the
# application shows it.
positional = collomatique.TeacherData("Emmy", "Noether")
assert positional.firstname == "Emmy"
assert positional.surname == "Noether"

# The two classes are the module's, not a private submodule's, whichever door a
# script comes in through.
from collomatique._data import TeacherData as _same_class  # noqa: E402

assert _same_class is collomatique.TeacherData
assert collomatique.TeacherData.__module__ == "collomatique"

# A field that names an entity takes a handle or an id, interchangeably. The two
# values below extract to the same teacher, and — this is the wart §2.3 records —
# they do not compare equal, because a dataclass stores what it was given.
subject = list(doc.subjects)[0]
by_handle = collomatique.TeacherData(
    "Emmy", "Noether", email="noether@lycee.fr", subjects={subject}
)
by_id = collomatique.TeacherData(
    "Emmy", "Noether", email="noether@lycee.fr", subjects={subject.id}
)
assert by_handle != by_id

# A list is as good as a set on the way in: what is asked of this field is that
# it can be iterated over and that every item names an entity.
by_list = collomatique.TeacherData(
    "Emmy", "Noether", email="noether@lycee.fr", subjects=[subject]
)

# Everything left at its default, so that rust can pin the defaults against the
# model's own.
bare_teacher = collomatique.TeacherData("", "")
bare_student = collomatique.StudentData("", "")

# The four values the boundary must refuse. They are built without complaint —
# that is the point — and rust extracts each one and reads the message.
empty_tel = collomatique.TeacherData("Emmy", "Noether", tel="")
empty_email = collomatique.TeacherData("Emmy", "Noether", email="")
empty_student_tel = collomatique.StudentData("Harry", "Potter", tel="")
not_a_name = collomatique.TeacherData(3, "Noether")

# A handle of another document names nothing here, whatever its id says.
other = collomatique.load(source)
foreign_subject = collomatique.TeacherData(
    "Emmy", "Noether", subjects={list(other.subjects)[0]}
)
