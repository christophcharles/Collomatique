import collomatique

# `source` is a throwaway copy of a real colloscope.
doc = collomatique.load(source)

teachers = doc.teachers
students = doc.students
assert isinstance(teachers, collomatique.Teachers)
assert isinstance(students, collomatique.Students)

teacher_list = list(teachers)
student_list = list(students)
assert len(teacher_list) == len(teachers)
assert len(student_list) == len(students)
assert all(isinstance(teacher, collomatique.Teacher) for teacher in teacher_list)
assert all(isinstance(student, collomatique.Student) for student in student_list)

# Iteration is id order — the one order the document has for these two, since
# the model keeps no display order for them. So there is no `.index` either:
# there would be nothing for it to be the position in.
assert [teacher.id for teacher in teacher_list] == sorted(
    teacher.id for teacher in teacher_list
)
assert [student.id for student in student_list] == sorted(
    student.id for student in student_list
)
assert not hasattr(teacher_list[0], "index")
assert not hasattr(student_list[0], "index")

# The card the model keeps for a person, flattened onto the handle.
teacher_surnames = [teacher.surname for teacher in teacher_list]
teacher_firstnames = [teacher.firstname for teacher in teacher_list]
teacher_tels = [teacher.tel for teacher in teacher_list]
teacher_emails = [teacher.email for teacher in teacher_list]

student_surnames = [student.surname for student in student_list]
student_firstnames = [student.firstname for student in student_list]
student_tels = [student.tel for student in student_list]
student_emails = [student.email for student in student_list]

# A number or an address is a string or `None`, and never the empty string. The
# example carries both shapes for both kinds of person.
for contacts in (teacher_tels, teacher_emails, student_tels, student_emails):
    assert all(
        value is None or (isinstance(value, str) and value != "") for value in contacts
    )
assert any(tel is None for tel in teacher_tels)
assert any(tel is not None for tel in teacher_tels)
assert any(email is None for email in student_emails)
assert any(email is not None for email in student_emails)

# The names are plain strings, empty allowed: the model types them that way.
assert all(isinstance(name, str) for name in teacher_surnames + teacher_firstnames)
assert all(isinstance(name, str) for name in student_surnames + student_firstnames)

# What a teacher interrogates in comes back as a frozenset of live handles: they
# are this document's subjects, not names copied out of it.
assert all(isinstance(teacher.subjects, frozenset) for teacher in teacher_list)
assert all(
    subject in doc.subjects for teacher in teacher_list for subject in teacher.subjects
)
teacher_subject_indices = [
    sorted(subject.index for subject in teacher.subjects) for teacher in teacher_list
]

# And the periods a student sits out, the same way.
assert all(isinstance(student.excluded_periods, frozenset) for student in student_list)
assert all(
    period in doc.periods
    for student in student_list
    for period in student.excluded_periods
)
student_excluded_period_indices = [
    sorted(period.index for period in student.excluded_periods)
    for student in student_list
]

# Indexing takes an id or a handle, and hands back an equal handle either way.
for teacher in teacher_list:
    assert teachers[teacher.id] == teacher
    assert teachers[teacher] == teacher
    assert teachers.get(teacher.id) == teacher
    assert teacher.id in teachers
    assert teacher in teachers
for student in student_list:
    assert students[student.id] == student
    assert students[student] == student
    assert students.get(student.id) == student
    assert student.id in students
    assert student in students

# A handle is a view, not the object the collection keeps: two of them for the
# same person are different objects that compare and hash the same. So a set or
# a dict can be keyed on people, which is what a script sorting a class list or
# counting a teacher's slots does all day.
again = teachers[teacher_list[0].id]
assert again is not teacher_list[0]
assert hash(again) == hash(teacher_list[0])
assert len({again, teacher_list[0]}) == 1
assert len(set(teacher_list)) == len(teacher_list)

by_handle = {student: student.surname for student in student_list}
assert len(by_handle) == len(student_list)
assert by_handle[students[student_list[0].id]] == student_list[0].surname

for collection in (teachers, students):
    assert 3 not in collection
    assert collection.get(3) is None
    try:
        collection[3]
    except KeyError:
        pass
    else:
        raise AssertionError("a key that is not an id must not resolve")

# A handle is something the document hands out, and it has no setters.
for cls in (collomatique.Teacher, collomatique.Student):
    try:
        cls()
    except TypeError:
        pass
    else:
        raise AssertionError("a handle must not be constructible")
try:
    teacher_list[0].email = "severus@poudlard.fr"
except AttributeError:
    pass
else:
    raise AssertionError("assigning to a handle attribute must raise")

# A handle from another document names nothing here, whatever its id says.
other = collomatique.load(source)
assert teacher_list[0] not in other.teachers
assert other.teachers.get(teacher_list[0]) is None
assert other.teachers[teacher_list[0].id] == list(other.teachers)[0]
assert student_list[0] not in other.students
assert other.students.get(student_list[0]) is None
assert other.students[student_list[0].id] == list(other.students)[0]
try:
    other.students[student_list[0]]
except KeyError:
    pass
else:
    raise AssertionError("a handle of another document must not resolve")

# Nor does a handle of another kind: a teacher and a student are two things,
# whatever their ids say.
assert teacher_list[0] != student_list[0]
assert teacher_list[0] not in students
assert student_list[0] not in teachers
